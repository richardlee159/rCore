use super::{
    address::{PhysAddr, PhysPageNum, StepByOne, VPNRange, VirtAddr, VirtPageNum},
    frame_allocator::{frame_alloc, FrameTracker},
    page_table::{PTEFlags, PageTable},
    PageTableEntry,
};
use crate::config::{MEMORY_END, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, USER_STACK_SIZE};
use alloc::{collections::BTreeMap, vec::Vec};
use lazy_static::lazy_static;
use riscv::register::satp;
use spin::Mutex;

extern "C" {
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss_with_stack();
    fn ebss();
    fn ekernel();
    fn strampoline();
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum MapType {
    Identical,
    Framed,
}

bitflags! {
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

struct MapArea {
    vpn_range: VPNRange,
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    map_perm: MapPermission,
}

impl MapArea {
    fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        let start_vpn = start_va.floor();
        let end_vpn = end_va.ceil();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
        }
    }

    fn from_another(another: &Self) -> Self {
        Self {
            data_frames: BTreeMap::new(),
            ..*another
        }
    }

    fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }

    fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }

    /// assume that all frames were cleared before
    fn copy_data(&mut self, page_table: &PageTable, data: &[u8], mut offset: usize) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start = 0;
        let mut current_vpn = self.vpn_range.get_start();
        let len = data.len();
        loop {
            let src = &data[start..len.min(start + PAGE_SIZE - offset)];
            let dst = &mut page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[offset..src.len() + offset];
            dst.copy_from_slice(src);
            start += PAGE_SIZE - offset;
            if start >= len {
                break;
            }
            current_vpn.step();
            offset = 0;
        }
    }

    fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn;
        match self.map_type {
            MapType::Identical => ppn = PhysPageNum(vpn.0),
            MapType::Framed => {
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
            }
        };
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
        page_table.map(vpn, ppn, pte_flags);
    }

    fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        match self.map_type {
            MapType::Identical => {}
            MapType::Framed => {
                self.data_frames.remove(&vpn);
            }
        }
        page_table.unmap(vpn);
    }

    fn overlap(&self, area: &MapArea) -> bool {
        self.vpn_range.overlap(&area.vpn_range)
    }
}

pub struct MemorySet {
    pub page_table: PageTable,
    areas: Vec<MapArea>,
}

impl MemorySet {
    fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    fn push(
        &mut self,
        mut map_area: MapArea,
        data: Option<(&[u8], usize)>,
    ) -> Result<(), &'static str> {
        if self.areas.iter().map(|a| a.overlap(&map_area)).any(|p| p) {
            return Err("map areas overlap");
        }
        map_area.map(&mut self.page_table);
        if let Some((data, offset)) = data {
            map_area.copy_data(&self.page_table, data, offset);
        }
        self.areas.push(map_area);
        Ok(())
    }

    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) -> Result<(), &'static str> {
        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission),
            None,
        )
    }

    pub fn delete_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
    ) -> Result<(), &'static str> {
        let range = VPNRange::new(start_va.floor(), end_va.ceil());
        if let Some(index) = self.areas.iter().position(|a| a.vpn_range == range) {
            let mut area = self.areas.remove(index);
            area.unmap(&mut self.page_table);
            Ok(())
        } else {
            Err("no such a map area")
        }
    }

    pub fn remove_area_with_start_vpn(&mut self, start_vpn: VirtPageNum) -> Result<(), &'static str> {
        if let Some(index) = self
            .areas
            .iter()
            .position(|area| area.vpn_range.get_start() == start_vpn)
        {
            let mut area = self.areas.remove(index);
            area.unmap(&mut self.page_table);
            Ok(())
        } else {
            Err("no such a map area")
        }
    }

    /// Mention that trampoline is not collected by areas.
    fn map_trampoline(&mut self) {
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        )
    }

    /// Without kernel stacks.
    fn new_kernel() -> Self {
        let mut memory_set = MemorySet::new_bare();
        // map trampoline
        memory_set.map_trampoline();
        // map kernel sections
        info!(".text   [{:#x}, {:#x})", stext as usize, etext as usize);
        info!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
        info!(".data   [{:#x}, {:#x})", sdata as usize, edata as usize);
        info!(
            ".bss    [{:#x}, {:#x})",
            sbss_with_stack as usize, ebss as usize
        );
        info!("mapping .text section");
        memory_set
            .push(
                MapArea::new(
                    (stext as usize).into(),
                    (etext as usize).into(),
                    MapType::Identical,
                    MapPermission::R | MapPermission::X,
                ),
                None,
            )
            .unwrap();
        info!("mapping .rodata section");
        memory_set
            .push(
                MapArea::new(
                    (srodata as usize).into(),
                    (erodata as usize).into(),
                    MapType::Identical,
                    MapPermission::R,
                ),
                None,
            )
            .unwrap();
        info!("mapping .data section");
        memory_set
            .push(
                MapArea::new(
                    (sdata as usize).into(),
                    (edata as usize).into(),
                    MapType::Identical,
                    MapPermission::R | MapPermission::W,
                ),
                None,
            )
            .unwrap();
        info!("mapping .bss section");
        memory_set
            .push(
                MapArea::new(
                    (sbss_with_stack as usize).into(),
                    (ebss as usize).into(),
                    MapType::Identical,
                    MapPermission::R | MapPermission::W,
                ),
                None,
            )
            .unwrap();
        info!("mapping physical memory");
        memory_set
            .push(
                MapArea::new(
                    (ekernel as usize).into(),
                    MEMORY_END.into(),
                    MapType::Identical,
                    MapPermission::R | MapPermission::W,
                ),
                None,
            )
            .unwrap();
        memory_set
    }

    /// Include sections in elf and trampoline and TrapContext and user stack,
    /// also returns user_sp and entry point.
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        let mut memory_set = MemorySet::new_bare();
        // map trampoline
        memory_set.map_trampoline();
        // map program headers of elf, with U flag
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, xmas_elf::header::MAGIC, "invalid elf!");
        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = VirtPageNum(0);
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                debug!(
                    "virtual_addr:{:#x}, mem_size:{:#x}",
                    ph.virtual_addr(),
                    ph.mem_size()
                );
                let start_va = (ph.virtual_addr() as usize).into();
                let end_va = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm.insert(MapPermission::R);
                }
                if ph_flags.is_write() {
                    map_perm.insert(MapPermission::W);
                }
                if ph_flags.is_execute() {
                    map_perm.insert(MapPermission::X);
                }
                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);
                max_end_vpn = map_area.vpn_range.get_end();
                memory_set
                    .push(
                        map_area,
                        Some((
                            &elf_data
                                [ph.offset() as usize..(ph.offset() + ph.file_size()) as usize],
                            start_va.page_offset(),
                        )),
                    )
                    .unwrap();
            }
        }
        // map user stack with U flag
        let max_end_va: VirtAddr = max_end_vpn.into();
        let user_stack_bottom = max_end_va.0 + PAGE_SIZE;
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;
        memory_set
            .insert_framed_area(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapPermission::R | MapPermission::W | MapPermission::U,
            )
            .unwrap();
        // map TrapContext
        memory_set
            .insert_framed_area(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                MapPermission::R | MapPermission::W,
            )
            .unwrap();
        (
            memory_set,
            user_stack_top,
            elf_header.pt2.entry_point() as usize,
        )
    }

    pub fn from_existed_user(user_space: &Self) -> Self {
        let mut memory_set = MemorySet::new_bare();
        // map trampoline
        memory_set.map_trampoline();
        // copy data sections/trap_context/user_stack
        for area in &user_space.areas {
            let new_area = MapArea::from_another(area);
            memory_set.push(new_area, None).unwrap();
            // copy data from another space
            for vpn in area.vpn_range {
                let src_ppn = user_space.translate(vpn).unwrap().ppn();
                let dst_ppn = memory_set.translate(vpn).unwrap().ppn();
                dst_ppn
                    .get_bytes_array()
                    .copy_from_slice(src_ppn.get_bytes_array());
            }
        }
        memory_set
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }

    pub fn activate(&self) {
        let satp = self.page_table.token();
        satp::write(satp);
        unsafe {
            llvm_asm!("sfence.vma" :::: "volatile");
        }
    }

    pub fn recycle_data_pages(&mut self) {
        self.areas.clear();
    }
}

lazy_static! {
    pub static ref KERNEL_SPACE: Mutex<MemorySet> = Mutex::new(MemorySet::new_kernel());
}

#[allow(unused)]
pub fn remap_test() {
    let kernel_space = KERNEL_SPACE.lock();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
    assert_eq!(
        kernel_space
            .page_table
            .translate(mid_text.floor())
            .unwrap()
            .writable(),
        false
    );
    assert_eq!(
        kernel_space
            .page_table
            .translate(mid_rodata.floor())
            .unwrap()
            .writable(),
        false,
    );
    assert_eq!(
        kernel_space
            .page_table
            .translate(mid_data.floor())
            .unwrap()
            .executable(),
        false,
    );
    println!("remap_test passed!");
}
