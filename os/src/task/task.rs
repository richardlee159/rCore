use super::{
    pid::{pid_alloc, KernelStack, PidHandle},
    TaskContext,
};
use crate::{
    config::TRAP_CONTEXT,
    fs::{File, STDIN, STDOUT},
    mm::{MemorySet, PhysPageNum, VirtAddr},
    trap::TrapContext,
};
use alloc::{
    sync::{Arc, Weak},
    vec,
    vec::Vec,
};
use spin::{Mutex, MutexGuard};

#[derive(PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Zombie,
}

pub struct TaskControlBlockInner {
    pub task_ctx_ptr: usize,
    pub task_status: TaskStatus,
    pub task_prio: usize,
    pub task_stride: usize,
    pub memory_set: MemorySet,
    pub trap_ctx_ppn: PhysPageNum,
    pub base_size: usize,
    pub parent: Option<Weak<TaskControlBlock>>,
    pub children: Vec<Arc<TaskControlBlock>>,
    pub exit_code: i32,
    pub fd_table: Vec<Option<Arc<dyn File>>>,
}

impl TaskControlBlockInner {
    pub fn get_task_ctx_ptr2(&self) -> *const usize {
        &self.task_ctx_ptr
    }

    pub fn get_trap_ctx(&self) -> &'static mut TrapContext {
        self.trap_ctx_ppn.get_mut()
    }

    pub fn get_user_token(&self) -> usize {
        self.memory_set.page_table.token()
    }

    pub fn is_zombie(&self) -> bool {
        self.task_status == TaskStatus::Zombie
    }

    pub fn alloc_fd(&mut self) -> usize {
        if let Some(fd) = self.fd_table.iter().position(|f| f.is_none()) {
            fd
        } else {
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }
}

pub struct TaskControlBlock {
    // immutable
    pub pid: PidHandle,
    pub kernel_stack: KernelStack,
    // mutable
    inner: Mutex<TaskControlBlockInner>,
}

impl TaskControlBlock {
    pub fn acquire_inner_lock(&self) -> MutexGuard<TaskControlBlockInner> {
        self.inner.lock()
    }

    pub fn getpid(&self) -> usize {
        self.pid.0
    }

    pub fn new(elf_data: &[u8]) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_ctx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        // push a task context which goes to trap_return to the top of kernel stack
        let task_ctx_ptr = kernel_stack.push_on_top(TaskContext::goto_trap_return());
        let task_control_block = Self {
            pid: pid_handle,
            kernel_stack,
            inner: Mutex::new(TaskControlBlockInner {
                task_ctx_ptr: task_ctx_ptr as usize,
                task_status: TaskStatus::Ready,
                task_prio: 16,
                task_stride: 0,
                memory_set,
                trap_ctx_ppn,
                base_size: user_sp,
                parent: None,
                children: Vec::new(),
                exit_code: 0,
                fd_table: vec![
                    Some(Arc::new(STDIN)),
                    Some(Arc::new(STDOUT)),
                    Some(Arc::new(STDOUT)),
                ],
            }),
        };
        // prepare TrapContext in user space
        let trap_ctx = task_control_block.acquire_inner_lock().get_trap_ctx();
        *trap_ctx = TrapContext::app_init_context(entry_point, user_sp, kernel_stack_top);
        task_control_block
    }

    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        // ---- hold parent PCB lock
        let mut parent_inner = self.acquire_inner_lock();
        // copy user space (include trap context)
        let memory_set = MemorySet::from_existed_user(&parent_inner.memory_set);
        let trap_ctx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        // push a task context which goes to trap_return to the top of kernel stack
        let task_ctx_ptr = kernel_stack.push_on_top(TaskContext::goto_trap_return());
        let task_control_block = Arc::new(Self {
            pid: pid_handle,
            kernel_stack,
            inner: Mutex::new(TaskControlBlockInner {
                task_ctx_ptr: task_ctx_ptr as usize,
                task_status: TaskStatus::Ready,
                task_prio: 16,
                task_stride: 0,
                memory_set,
                trap_ctx_ppn,
                base_size: parent_inner.base_size,
                parent: Some(Arc::downgrade(self)),
                children: Vec::new(),
                exit_code: 0,
                fd_table: parent_inner.fd_table.clone(),
            }),
        });
        // add child
        parent_inner.children.push(task_control_block.clone());
        // modify kernel_sp in trap_ctx
        // **** acquire child PCB lock
        let trap_ctx = task_control_block.acquire_inner_lock().get_trap_ctx();
        // **** release child PCB lock
        trap_ctx.kernel_sp = kernel_stack_top;
        // return
        task_control_block
        // ---- release parent PCB lock
    }

    pub fn exec(&self, elf_data: &[u8]) {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_ctx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // **** hold current PCB lock
        let mut inner = self.acquire_inner_lock();
        // substitute memory_set
        inner.memory_set = memory_set;
        // update trap_ctx ppn
        inner.trap_ctx_ppn = trap_ctx_ppn;
        // initialize trap_ctx
        let trap_ctx = inner.get_trap_ctx();
        *trap_ctx =
            TrapContext::app_init_context(entry_point, user_sp, self.kernel_stack.get_top());
        // **** release current PCB lock
    }

    pub fn spawn_child(self: &Arc<Self>, elf_data: &[u8]) -> Arc<Self> {
        let task_control_block = Arc::new(TaskControlBlock::new(elf_data));
        task_control_block.acquire_inner_lock().parent = Some(Arc::downgrade(self));
        self.acquire_inner_lock()
            .children
            .push(task_control_block.clone());
        task_control_block
    }
}
