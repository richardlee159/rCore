use super::{
    manager::fetch_task,
    switch::__switch,
    task::{TaskControlBlock, TaskStatus},
};
use crate::{timer::set_next_trigger, trap::TrapContext};
use alloc::sync::Arc;
use core::cell::RefCell;
use lazy_static::lazy_static;

pub struct Processor {
    inner: RefCell<ProcessorInner>,
}

unsafe impl Sync for Processor {}

struct ProcessorInner {
    current: Option<Arc<TaskControlBlock>>,
    idle_task_ctx_ptr: usize,
}

impl Processor {
    fn new() -> Self {
        Self {
            inner: RefCell::new(ProcessorInner {
                current: None,
                idle_task_ctx_ptr: 0,
            }),
        }
    }

    fn take_current(&self) -> Option<Arc<TaskControlBlock>> {
        self.inner.borrow_mut().current.take()
    }

    fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.inner.borrow().current.clone()
    }

    fn get_idle_task_ctx_ptr2(&self) -> *const usize {
        &self.inner.borrow().idle_task_ctx_ptr as *const usize
    }

    fn run(&self) {
        loop {
            if let Some(task) = fetch_task() {
                let idle_task_ctx_ptr2 = self.get_idle_task_ctx_ptr2();
                // acquire
                let mut task_inner = task.acquire_inner_lock();
                let next_task_ctx_ptr2 = task_inner.get_task_ctx_ptr2();
                task_inner.task_status = TaskStatus::Running;
                drop(task_inner);
                // release
                self.inner.borrow_mut().current = Some(task);
                set_next_trigger();
                unsafe {
                    __switch(idle_task_ctx_ptr2, next_task_ctx_ptr2);
                }
            }
        }
    }
}

lazy_static! {
    static ref PROCESSOR: Processor = Processor::new();
}

pub fn run_tasks() {
    PROCESSOR.run();
}

pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.take_current()
}

pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.current()
}

pub fn current_user_token() -> usize {
    current_task()
        .unwrap()
        .acquire_inner_lock()
        .get_user_token()
}

pub fn current_trap_ctx() -> &'static mut TrapContext {
    current_task().unwrap().acquire_inner_lock().get_trap_ctx()
}

pub fn schedule(switched_task_ctx_ptr2: *const usize) {
    let idle_task_ctx_ptr2 = PROCESSOR.get_idle_task_ctx_ptr2();
    unsafe { __switch(switched_task_ctx_ptr2, idle_task_ctx_ptr2) }
}
