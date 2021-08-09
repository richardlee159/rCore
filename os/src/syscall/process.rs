use alloc::sync::Arc;

use crate::{loader::get_app_data_by_name, mm::{translated_refmut, translated_str}, task::{
        add_task, current_task, current_user_token, exit_current_and_run_next, set_current_prio,
        suspend_current_and_run_next,
    }};

pub fn sys_exit(exit_code: i32) -> ! {
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_set_priority(prio: isize) -> isize {
    set_current_prio(prio)
}

pub fn sys_getpid() -> isize {
    current_task().unwrap().getpid() as isize
}

pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    // for child process, fork returns 0
    new_task.acquire_inner_lock().get_trap_ctx().x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    let token = current_user_token();
    if let Some(path) = translated_str(token, path) {
        if let Some(elf_data) = get_app_data_by_name(&path) {
            let task = current_task().unwrap();
            task.exec(elf_data);
            0
        } else {
            -1
        }
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let task = current_task().unwrap();
    // find a child process

    // ---- hold current PCB lock
    let mut inner = task.acquire_inner_lock();
    let mut target = inner
        .children
        .iter()
        .enumerate()
        .filter(|(_, t)| pid == -1 || pid as usize == t.getpid())
        .peekable();
    if target.peek().is_none() {
        // specified child process not found
        return -1;
    }
    if let Some((index, _)) = target.find(|(_, t)| t.acquire_inner_lock().is_zombie()) {
        let child = inner.children.remove(index);
        // confirm that child will be deallocated after removing from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        let exit_code =child.acquire_inner_lock().exit_code;
        if let Some(refmut) = translated_refmut(inner.get_user_token(), exit_code_ptr) {
            *refmut = exit_code;
            found_pid as isize
        } else {
            warn!("Illegal memory region in sys_waitpid!");
            -1
        }
    } else {
        // no zombie process in specified child process(es)
        -2
    }
    // ---- release current PCB lock automatically
}
