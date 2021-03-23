use crate::timer::{get_time_us, USEC_PER_SEC};

#[repr(C)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let ts = unsafe { &mut *ts };
    // Todo: check if ts is within userspace
    let tims_us = get_time_us();
    ts.sec = tims_us / USEC_PER_SEC;
    ts.usec = tims_us % USEC_PER_SEC;
    0
}
