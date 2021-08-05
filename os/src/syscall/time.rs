use crate::mm::translated_byte_buffer_copy;
use crate::task::current_user_token;
use crate::timer::{get_time_us, USEC_PER_SEC};
use core::{mem, slice};

#[repr(C)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

impl TimeVal {
    fn as_bytes(&self) -> &[u8] {
        let len = mem::size_of::<TimeVal>();
        let data = self as *const _ as usize as *const u8;
        unsafe { slice::from_raw_parts(data, len) }
    }
}

pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let tims_us = get_time_us();
    let time_val = TimeVal {
        sec: tims_us / USEC_PER_SEC,
        usec: tims_us % USEC_PER_SEC,
    };
    if translated_byte_buffer_copy(
        current_user_token(),
        ts as *mut u8,
        mem::size_of::<TimeVal>(),
        time_val.as_bytes(),
    )
    .is_some()
    {
        0
    } else {
        warn!("Illegal memory region in sys_get_time!");
        -1
    }
}
