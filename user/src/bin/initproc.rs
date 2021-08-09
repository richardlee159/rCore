#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exec, exit, fork, wait};

#[no_mangle]
fn main() {
    println!("[initproc] Hello!");
    if fork() == 0 {
        exec("user_shell\0");
    } else {
        loop {
            let mut exit_code = 0;
            let pid = wait(&mut exit_code);
            if pid == -1 {
                println!("[initproc] No child process!");
                exit(0);
            }
            println!(
                "[initproc] Released a zombie process, pid={}, exit_code={}",
                pid, exit_code,
            );
        }
    }
}
