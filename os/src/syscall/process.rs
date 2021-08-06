use crate::task::TASK_MANAGER;

pub fn sys_exit(exit_code: i32) -> ! {
    info!("Application exited with code {}", exit_code);
    TASK_MANAGER.exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    TASK_MANAGER.suspend_current_and_run_next();
    0
}

pub fn sys_set_priority(prio: isize) -> isize {
    TASK_MANAGER.set_current_prio(prio)
}
