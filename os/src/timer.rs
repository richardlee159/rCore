use crate::config::CLOCK_FREQ;
use crate::sbi;
use riscv::register::time;

const TICKS_PER_SEC: usize = 200;
pub const USEC_PER_SEC: usize = 1000000;

fn get_time() -> usize {
    time::read()
}

pub fn get_time_us() -> usize {
    get_time() / (CLOCK_FREQ / USEC_PER_SEC)
}

pub fn set_next_trigger() {
    sbi::set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}
