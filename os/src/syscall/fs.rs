use crate::mm::{translated_byte_buffer, translated_byte_buffer_copy};
use crate::sbi::console_getchar;
use crate::task::{current_user_token, suspend_current_and_run_next};

const FD_STDIN: usize = 0;
const FD_STDOUT: usize = 1;

pub fn sys_read(fd: usize, buf: *mut u8, len: usize) -> isize {
    match fd {
        FD_STDIN => {
            assert_eq!(len, 1, "Only support len = 1 in sys_read!");
            let ch = loop {
                let c = console_getchar();
                if c == 0 {
                    suspend_current_and_run_next();
                } else {
                    break c;
                }
            } as u8;
            let input = [ch];
            let token = current_user_token();
            if translated_byte_buffer_copy(token, buf, len, &input).is_some() {
                1
            } else {
                warn!("Illegal memory region in sys_read!");
                -1
            }
        }
        _ => {
            warn!("Unsupported fd in sys_read!");
            -1
        }
    }
}

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let token = current_user_token();
            if let Some(buffers) = translated_byte_buffer(token, buf, len) {
                for buffer in buffers {
                    let str = core::str::from_utf8(buffer).unwrap();
                    print!("{}", str);
                }
                len as isize
            } else {
                warn!("Illegal memory region in sys_write!");
                -1
            }
        }
        _ => {
            warn!("Unsupported fd in sys_write!");
            -1
        }
    }
}
