use crate::config::PAGE_SIZE;
use crate::mm::{MapPermission, VirtAddr};
use crate::task::TASK_MANAGER;

const PROT_READ: usize = 0x1;
const PROT_WRITE: usize = 0x2;
const PROT_EXEC: usize = 0x4;
const PROT_ALL: usize = PROT_READ | PROT_WRITE | PROT_EXEC;

fn ceil(num: usize, bound: usize) -> usize {
    (num + bound - 1) / bound * bound
}

fn get_map_permission(prot: usize) -> Option<MapPermission> {
    if (prot & PROT_ALL == 0) || (prot & !PROT_ALL != 0) {
        return None;
    }
    let mut perm = MapPermission::U;
    if prot & PROT_READ != 0 {
        perm.insert(MapPermission::R);
    }
    if prot & PROT_WRITE != 0 {
        perm.insert(MapPermission::W);
    }
    if prot & PROT_EXEC != 0 {
        perm.insert(MapPermission::X);
    }
    Some(perm)
}

pub fn mmap(start: usize, len: usize, prot: usize) -> isize {
    let start_va = VirtAddr::from(start);
    if !start_va.aligned() {
        warn!("start address not aligned");
        return -1;
    }
    if let Some(permission) = get_map_permission(prot) {
        let end_va = VirtAddr::from(start + len);
        if let Err(e) = TASK_MANAGER.current_insert_framed_area(start_va, end_va, permission) {
            warn!("{}", e);
            -1
        } else {
            ceil(len, PAGE_SIZE) as isize
        }
    } else {
        warn!("invalid protection bits");
        -1
    }
}

pub fn munmap(start: usize, len: usize) -> isize {
    let start_va = VirtAddr::from(start);
    if !start_va.aligned() {
        warn!("start address not aligned");
        return -1;
    }
    let end_va = VirtAddr::from(start + len);
    if let Err(e) = TASK_MANAGER.current_delete_framed_area(start_va, end_va) {
        warn!("{}", e);
        -1
    } else {
        ceil(len, PAGE_SIZE) as isize
    }
}
