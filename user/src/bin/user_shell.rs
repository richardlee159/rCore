#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;

use alloc::string::String;
use user_lib::{console::getchar, exec, exit, fork, waitpid};

const LF: u8 = 0x0a;
const CR: u8 = 0x0d;
const BS: u8 = 0x08;
const DL: u8 = 0x7f;

#[no_mangle]
pub fn main() -> i32 {
    println!("Rust user shell");
    let mut line = String::new();
    print!(">> ");
    loop {
        match getchar() {
            LF | CR => {
                println!("");
                if !line.is_empty() {
                    match line.as_str() {
                        "exit" => {
                            exit(0);
                        }
                        _ => {
                            line.push('\0');
                            let pid = fork();
                            if pid == 0 {
                                // child process
                                if exec(line.as_str()) == -1 {
                                    println!("Error when executing!");
                                    return -4;
                                }
                                unreachable!();
                            } else {
                                // parent process
                                let mut exit_code = 0;
                                let exit_pid = waitpid(pid, &mut exit_code);
                                assert_eq!(pid, exit_pid);
                                println!("Shell: Process {} exited with code {}", pid, exit_code);
                            }
                            line.clear();
                        }
                    }
                }
                print!(">> ");
            }
            BS | DL => {
                print!("{0} {0}", BS as char);
                line.pop();
            }
            ch => {
                print!("{}", ch as char);
                line.push(ch as char);
            }
        }
    }
}
