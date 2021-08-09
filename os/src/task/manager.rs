use super::task::TaskControlBlock;
use alloc::{collections::BinaryHeap, sync::Arc};
use lazy_static::lazy_static;
use spin::Mutex;

struct TaskManager {
    ready_queue: BinaryHeap<TaskControlBlockQueuer>,
}

/// A stride scheduler implemented with a priority queue.
impl TaskManager {
    fn new() -> Self {
        Self {
            ready_queue: BinaryHeap::new(),
        }
    }

    fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push(TaskControlBlockQueuer::new(task))
    }

    fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop().map(|q| q.into_task())
    }
}

lazy_static! {
    static ref TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());
}

pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.lock().add(task);
}

pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.lock().fetch()
}

const BIT_STRIDE: usize = 65536;

struct TaskControlBlockQueuer {
    stride: usize,
    task: Arc<TaskControlBlock>,
}

impl TaskControlBlockQueuer {
    fn new(task: Arc<TaskControlBlock>) -> Self {
        let stride = task.acquire_inner_lock().task_stride;
        Self { stride, task }
    }

    fn into_task(self) -> Arc<TaskControlBlock> {
        let task = self.task;
        let mut inner = task.acquire_inner_lock();
        inner.task_stride += BIT_STRIDE / inner.task_prio;
        drop(inner);
        task
    }
}

impl PartialEq for TaskControlBlockQueuer {
    fn eq(&self, other: &Self) -> bool {
        self.stride.eq(&other.stride)
    }
}
impl Eq for TaskControlBlockQueuer {}
impl PartialOrd for TaskControlBlockQueuer {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.stride.partial_cmp(&other.stride).map(|o| o.reverse())
    }
}
impl Ord for TaskControlBlockQueuer {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.stride.cmp(&other.stride).reverse()
    }
}
