use core::mem;

use crate::config::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE};
use crate::mm::{MapPermission, VirtAddr, KERNEL_SPACE};
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

struct PidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl PidAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            recycled: Vec::new(),
        }
    }

    fn alloc(&mut self) -> PidHandle {
        if let Some(pid) = self.recycled.pop() {
            PidHandle(pid)
        } else {
            assert!(self.current < usize::MAX);
            self.current += 1;
            PidHandle(self.current - 1)
        }
    }

    fn dealloc(&mut self, pid: usize) {
        if pid >= self.current || self.recycled.contains(&pid) {
            panic!("pid {} has not been allocated!", pid);
        }
        self.recycled.push(pid);
    }
}

lazy_static! {
    static ref PID_ALLOCATOR: Mutex<PidAllocator> = Mutex::new(PidAllocator::new());
}

pub fn pid_alloc() -> PidHandle {
    PID_ALLOCATOR.lock().alloc()
}

pub struct PidHandle(pub usize);

impl Drop for PidHandle {
    fn drop(&mut self) {
        PID_ALLOCATOR.lock().dealloc(self.0);
    }
}

pub struct KernelStack {
    pid: usize,
}

impl KernelStack {
    /// Return (bottom, top) of a kernel stack in kernel space.
    fn position(pid: usize) -> (usize, usize) {
        let top = TRAMPOLINE - pid * (KERNEL_STACK_SIZE + PAGE_SIZE);
        let buttom = top - KERNEL_STACK_SIZE;
        (buttom, top)
    }

    pub fn new(pid_handle: &PidHandle) -> Self {
        let pid = pid_handle.0;
        let (kernel_stack_bottom, kernel_stack_top) = Self::position(pid);
        KERNEL_SPACE
            .lock()
            .insert_framed_area(
                kernel_stack_bottom.into(),
                kernel_stack_top.into(),
                MapPermission::R | MapPermission::W,
            )
            .unwrap();
        Self { pid }
    }

    pub fn push_on_top<T>(&self, value: T) -> *mut T {
        let kernel_stack_top = self.get_top();
        let ptr = (kernel_stack_top - mem::size_of::<T>()) as *mut T;
        unsafe {
            *ptr = value;
        }
        ptr
    }

    pub fn get_bottom(&self) -> usize {
        Self::position(self.pid).0
    }

    pub fn get_top(&self) -> usize {
        Self::position(self.pid).1
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let kernel_stack_bottom_va = VirtAddr::from(self.get_bottom());
        KERNEL_SPACE
            .lock()
            .remove_area_with_start_vpn(kernel_stack_bottom_va.into())
            .unwrap();
    }
}
