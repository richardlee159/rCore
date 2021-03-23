use crate::task::{exit_current_and_run_next, set_current_prio, suspend_current_and_run_next};

pub fn sys_exit(exit_code: i32) -> ! {
    info!("Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_set_priority(prio: isize) -> isize {
    set_current_prio(prio)
}
