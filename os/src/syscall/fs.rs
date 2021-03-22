const FD_STDOUT: usize = 1;

use crate::loader::within_user_space;
use crate::task::get_current_task;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            let app_id = get_current_task();
            if within_user_space(app_id, slice) {
                let str = core::str::from_utf8(slice).unwrap();
                print!("{}", str);
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
