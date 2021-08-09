#![no_std]
#![no_main]
#![feature(llvm_asm)]
#![feature(global_asm)]
#![feature(panic_info_message)]
#![feature(const_in_array_repeat_expressions)]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate bitflags;
extern crate alloc;

#[macro_use]
mod console;
mod config;
mod lang_items;
mod loader;
mod logging;
mod mm;
mod sbi;
mod syscall;
mod task;
mod timer;
mod trap;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    println!("[kernel] Hello, world!");
    logging::init();
    mm::init();
    trap::init();
    trap::enable_timer_interrupt();
    loader::list_apps();
    task::add_initproc();
    task::run_tasks();
    panic!("Unreachable in rust_main!");
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) })
}
