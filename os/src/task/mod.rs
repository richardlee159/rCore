mod context;
mod manager;
mod pid;
mod processor;
mod switch;
mod task;

use crate::loader::get_app_data_by_name;
use crate::mm::{MapPermission, VirtAddr};
use alloc::sync::Arc;
use context::TaskContext;
use lazy_static::lazy_static;
use processor::schedule;
use task::{TaskControlBlock, TaskStatus};

pub use manager::{add_task, get_task_by_pid};
pub use processor::{
    current_task, current_trap_ctx, current_user_token, run_tasks, take_current_task,
};

pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();

    // ---- hold current PCB lock
    let mut task_inner = task.acquire_inner_lock();
    let task_ctx_ptr2 = task_inner.get_task_ctx_ptr2();
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- release current PCB lock

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_ctx_ptr2);
}

pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();
    if task.getpid() == 0 {
        panic!("initproc exited!");
    }
    // **** hold current PCB lock
    let mut task_inner = task.acquire_inner_lock();
    // Change status to Zombie
    task_inner.task_status = TaskStatus::Zombie;
    // Record exit code
    task_inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ hold initproc PCB lock here
    {
        let mut initproc_inner = INITPROC.acquire_inner_lock();
        for child in &task_inner.children {
            child.acquire_inner_lock().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB lock here

    task_inner.children.clear();
    // deallocate user space
    task_inner.memory_set.recycle_data_pages();
    drop(task_inner);
    // **** release current PCB lock
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let unused: usize = 0;
    schedule(&unused as *const _);
}

pub fn set_current_prio(prio: isize) -> isize {
    if prio > 1 {
        current_task().unwrap().acquire_inner_lock().task_prio = prio as usize;
        prio
    } else {
        -1
    }
}

pub fn current_insert_framed_area(
    start_va: VirtAddr,
    end_va: VirtAddr,
    permission: MapPermission,
) -> Result<(), &'static str> {
    current_task()
        .unwrap()
        .acquire_inner_lock()
        .memory_set
        .insert_framed_area(start_va, end_va, permission)
}

pub fn current_delete_framed_area(
    start_va: VirtAddr,
    end_va: VirtAddr,
) -> Result<(), &'static str> {
    current_task()
        .unwrap()
        .acquire_inner_lock()
        .memory_set
        .delete_framed_area(start_va, end_va)
}

lazy_static! {
    static ref INITPROC: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new(
        get_app_data_by_name("initproc").unwrap()
    ));
}

pub fn add_initproc() {
    add_task(INITPROC.clone());
}
