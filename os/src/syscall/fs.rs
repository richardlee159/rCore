const FD_STDOUT: usize = 1;

use crate::mm::translated_byte_buffer;
use crate::task::TASK_MANAGER;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let token = TASK_MANAGER.get_current_token();
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
