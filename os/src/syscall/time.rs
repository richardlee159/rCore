use crate::loader::within_user_space;
use crate::task::get_current_task;
use crate::timer::{get_time_us, USEC_PER_SEC};

#[repr(C)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let app_id = get_current_task();
    if within_user_space(app_id, ts as usize, core::mem::size_of::<TimeVal>()) {
        let ts = unsafe { &mut *ts };
        let tims_us = get_time_us();
        ts.sec = tims_us / USEC_PER_SEC;
        ts.usec = tims_us % USEC_PER_SEC;
        0
    } else {
        warn!("Illegal memory region in sys_get_time!");
        -1
    }
}
