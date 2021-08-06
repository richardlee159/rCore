mod context;
mod switch;
mod task;

use crate::loader::{get_app_data, get_num_app};
use crate::mm::{MapPermission, VirtAddr};
use crate::timer::set_next_trigger;
use crate::trap::TrapContext;
use alloc::vec::Vec;
use context::TaskContext;
use core::{cell::RefCell, mem};
use lazy_static::lazy_static;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};

const BIT_STRIDE: usize = 65536;

pub struct TaskManager {
    num_app: usize,
    inner: RefCell<TaskManagerInner>,
}

struct TaskManagerInner {
    tasks: Vec<TaskControlBlock>,
    current_task: usize,
}

unsafe impl Sync for TaskManager {}

lazy_static! {
    static ref TASK_MANAGER: TaskManager = {
        info!("init TASK_MANAGER");
        let num_app = get_num_app();
        info!("num_app = {}", num_app);
        let mut tasks = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }
        TaskManager {
            num_app,
            inner: RefCell::new(TaskManagerInner {
                tasks,
                current_task: 0,
            }),
        }
    };
}

impl TaskManager {
    fn run_first_task(&self) {
        self.inner.borrow_mut().tasks[0].task_status = TaskStatus::Running;
        let next_task_ctx_ptr2 = self.inner.borrow().tasks[0].get_task_ctx_ptr2();
        let _unused: usize = 0;
        set_next_trigger();
        unsafe {
            __switch(&_unused as *const _, next_task_ctx_ptr2);
        }
    }

    fn mark_current_suspended(&self) {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    fn mark_current_exited(&self) {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    fn find_next_task(&self) -> Option<usize> {
        // find next runnable task in O(n) time
        // todo: use a binary heap instead
        let inner = self.inner.borrow();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .filter(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
            .min_by_key(|id| inner.tasks[*id].task_stride)
    }

    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.borrow_mut();
            let current = inner.current_task;
            // update status
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            // update stride
            inner.tasks[next].task_stride += BIT_STRIDE / inner.tasks[next].task_prio;
            // ready to switch task
            let current_task_ctx_ptr2 = inner.tasks[current].get_task_ctx_ptr2();
            let next_task_ctx_ptr2 = inner.tasks[next].get_task_ctx_ptr2();
            mem::drop(inner);
            set_next_trigger();
            unsafe {
                __switch(current_task_ctx_ptr2, next_task_ctx_ptr2);
            }
        } else {
            panic!("All applications completed!");
        }
    }

    fn set_current_prio(&self, prio: isize) -> isize {
        if prio > 1 {
            let mut inner = self.inner.borrow_mut();
            let current = inner.current_task;
            inner.tasks[current].task_prio = prio as usize;
            prio
        } else {
            -1
        }
    }

    fn get_current_token(&self) -> usize {
        let inner = self.inner.borrow();
        let current = inner.current_task;
        inner.tasks[current].get_user_token()
    }

    fn get_current_trap_ctx(&self) -> &mut TrapContext {
        let inner = self.inner.borrow();
        let current = inner.current_task;
        inner.tasks[current].get_trap_ctx()
    }

    fn current_insert_framed_area(
        &self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) -> Result<(), &'static str> {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current]
            .memory_set
            .insert_framed_area(start_va, end_va, permission)
    }

    fn current_delete_framed_area(
        &self,
        start_va: VirtAddr,
        end_va: VirtAddr,
    ) -> Result<(), &'static str> {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current]
            .memory_set
            .delete_framed_area(start_va, end_va)
    }
}

pub fn suspend_current_and_run_next() {
    TASK_MANAGER.mark_current_suspended();
    TASK_MANAGER.run_next_task();
}

pub fn exit_current_and_run_next() {
    TASK_MANAGER.mark_current_exited();
    TASK_MANAGER.run_next_task();
}

pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

pub fn set_current_prio(prio: isize) -> isize {
    TASK_MANAGER.set_current_prio(prio)
}

pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

pub fn current_trap_ctx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_ctx()
}

pub fn current_insert_framed_area(
    start_va: VirtAddr,
    end_va: VirtAddr,
    permission: MapPermission,
) -> Result<(), &'static str> {
    TASK_MANAGER.current_insert_framed_area(start_va, end_va, permission)
}

pub fn current_delete_framed_area(
    start_va: VirtAddr,
    end_va: VirtAddr,
) -> Result<(), &'static str> {
    TASK_MANAGER.current_delete_framed_area(start_va, end_va)
}