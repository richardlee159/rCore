#![no_std]
#![feature(llvm_asm)]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
pub mod console;
mod lang_items;
mod syscall;

use buddy_system_allocator::LockedHeap;
use syscall::*;

const USER_HEAP_SIZE: usize = 4096 * 4;

static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP_SPACE.as_ptr() as usize, USER_HEAP_SIZE);
    }
    exit(main());
    panic!("Unreachable after sys_exit!");
}

#[no_mangle]
#[linkage = "weak"]
fn main() -> i32 {
    panic!("Connot find main!");
}

#[repr(C)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

impl TimeVal {
    pub fn new() -> Self {
        TimeVal { sec: 0, usec: 0 }
    }
}

pub fn read(fd: usize, buffer: &mut [u8]) -> isize {
    sys_read(fd, buffer)
}

pub fn write(fd: usize, buffer: &[u8]) -> isize {
    sys_write(fd, buffer)
}

pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}

pub fn yield_() -> isize {
    sys_yield()
}

pub fn set_priority(prio: isize) -> isize {
    sys_set_priority(prio)
}

pub fn get_time() -> isize {
    // get time in milliseconds
    let mut ts = TimeVal::new();
    match sys_get_time(&mut ts, 0) {
        0 => (ts.sec * 1000 + ts.usec / 1000) as isize,
        _ => -1,
    }
}

pub fn fork() -> isize {
    sys_fork()
}

pub fn exec(path: &str) -> isize {
    sys_exec(path)
}

pub fn wait(exit_code: &mut i32) -> isize {
    waitpid(-1, exit_code)
}

pub fn waitpid(pid: isize, exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(pid, exit_code as *mut _) {
            -2 => {
                yield_();
            }
            // -1 or a real pid
            exit_pid => return exit_pid,
        }
    }
}
