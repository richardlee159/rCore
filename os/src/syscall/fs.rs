const FD_STDOUT: usize = 1;

use crate::batch::within_user_space;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            if within_user_space(slice) {
                let str = core::str::from_utf8(slice).unwrap();
                print!("{}", str);
                len as isize
            } else {
                info!("Illegal memory region in sys_write!");
                -1
            }
        }
        _ => {
            info!("Unsupported fd in sys_write!");
            -1
        }
    }
}
