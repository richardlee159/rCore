#[derive(PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

pub struct TaskControlBlock {
    pub task_ctx_ptr: usize,
    pub task_status: TaskStatus,
    pub task_prio: usize,
    pub task_stride: usize,
}

impl TaskControlBlock {
    pub fn get_task_ctx_ptr2(&self) -> *const usize {
        &self.task_ctx_ptr
    }
}
