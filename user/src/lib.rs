#![no_std]
#![feature(llvm_asm)]
#![feature(linkage)]
#![feature(panic_info_message)]

#[macro_use]
pub mod console;
mod lang_items;
mod syscall;

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
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

use syscall::*;

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
