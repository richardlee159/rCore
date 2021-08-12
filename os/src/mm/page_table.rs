use super::{
    address::{PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum},
    frame_allocator::{frame_alloc, FrameTracker},
};
use crate::config::PAGE_SIZE;
use alloc::{string::String, vec, vec::Vec};

bitflags! {
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct PageTableEntry {
    bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }

    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }

    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1 << 44) - 1)).into()
    }

    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    pub fn is_valid(&self) -> bool {
        self.flags().contains(PTEFlags::V)
    }

    pub fn readable(&self) -> bool {
        self.flags().contains(PTEFlags::R)
    }

    pub fn writable(&self) -> bool {
        self.flags().contains(PTEFlags::W)
    }

    pub fn executable(&self) -> bool {
        self.flags().contains(PTEFlags::X)
    }
}

pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

impl PageTable {
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        Self {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }

    /// Temporarily used to get arguments from user space.
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1 << 44) - 1)),
            frames: Vec::new(),
        }
    }

    pub fn token(&self) -> usize {
        8 << 60 | self.root_ppn.0
    }

    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        debug!("[ROOT {:?}] mapping {:?} to {:?}", self.root_ppn, vpn, ppn);
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "{:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }

    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        debug!("translate {:?}", vpn);
        self.find_pte(vpn).map(|pte| pte.clone())
    }

    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.floor()).map(|pte| {
            let aligned_pa: PhysAddr = pte.ppn().into();
            let offset = va.page_offset();
            (aligned_pa.0 + offset).into()
        })
    }

    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idx = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result = None;
        for i in 0..3 {
            let pte = &mut ppn.get_pte_array()[idx[i]];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }

    fn find_pte(&self, vpn: VirtPageNum) -> Option<&PageTableEntry> {
        let idx = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result = None;
        for i in 0..3 {
            let pte = &ppn.get_pte_array()[idx[i]];
            if !pte.is_valid() {
                break;
            }
            if i == 2 {
                result = Some(pte);
                break;
            }
            ppn = pte.ppn();
        }
        result
    }
}

pub fn translated_byte_buffer(
    token: usize,
    ptr: *const u8,
    len: usize,
) -> Option<Vec<&'static mut [u8]>> {
    debug!("translate_byte_buffer ptr:{:#x}, len:{}", ptr as usize, len);
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn)?.ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        let start_offset = start_va.page_offset();
        let mut end_offset = end_va.page_offset();
        if end_offset == 0 {
            end_offset = PAGE_SIZE
        }
        v.push(&mut ppn.get_bytes_array()[start_offset..end_offset]);
        start = end_va.into();
    }
    Some(v)
}

pub fn translated_byte_buffer_copy(
    token: usize,
    ptr: *mut u8,
    len: usize,
    data: &[u8],
) -> Option<usize> {
    assert_eq!(len, data.len());
    let mut start = 0;
    for buf in translated_byte_buffer(token, ptr, len)? {
        buf.copy_from_slice(&data[start..start + buf.len()]);
        start += buf.len();
    }
    Some(len)
}

pub fn translated_str(token: usize, ptr: *const u8) -> Option<String> {
    let page_table = PageTable::from_token(token);
    let mut string = String::new();
    let mut va = VirtAddr::from(ptr as usize);
    loop {
        let ch = *(page_table.translate_va(va)?.get_mut::<u8>());
        if ch == 0 {
            break;
        } else {
            string.push(ch as char);
            va.0 += 1;
        }
    }
    Some(string)
}

pub fn translated_refmut<T>(token: usize, ptr: *mut T) -> Option<&'static mut T> {
    let page_table = PageTable::from_token(token);
    let va = VirtAddr::from(ptr as usize);
    Some(page_table.translate_va(va)?.get_mut())
}

pub struct UserBuffer {
    pub buffers: Vec<&'static mut [u8]>,
}

impl UserBuffer {
    pub fn new(buffers: Vec<&'static mut [u8]>) -> Self {
        Self { buffers }
    }

    pub fn len(&self) -> usize {
        self.buffers.iter().map(|b| b.len()).sum()
    }
}

impl IntoIterator for UserBuffer {
    type Item = *mut u8;
    type IntoIter = UserBufferIterator;
    fn into_iter(self) -> Self::IntoIter {
        UserBufferIterator {
            buffers: self.buffers,
            current_buffer: 0,
            current_idx: 0,
        }
    }
}

pub struct UserBufferIterator {
    buffers: Vec<&'static mut [u8]>,
    current_buffer: usize,
    current_idx: usize,
}

impl Iterator for UserBufferIterator {
    type Item = *mut u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_buffer >= self.buffers.len() {
            None
        } else {
            let r = &mut self.buffers[self.current_buffer][self.current_idx] as *mut _;
            if self.current_idx + 1 == self.buffers[self.current_buffer].len() {
                self.current_idx = 0;
                self.current_buffer += 1;
            } else {
                self.current_idx += 1;
            }
            Some(r)
        }
    }
}
