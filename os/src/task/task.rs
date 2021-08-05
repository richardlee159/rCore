use super::TaskContext;
use crate::{
    config::{kernel_stack_position, TRAP_CONTEXT},
    mm::{MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE},
    trap::TrapContext,
};
use core::mem;

#[derive(PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Exited,
}

pub struct TaskControlBlock {
    pub task_ctx_ptr: usize,
    pub task_status: TaskStatus,
    pub task_prio: usize,
    pub task_stride: usize,
    pub memory_set: MemorySet,
    pub trap_ctx_ppn: PhysPageNum,
    pub base_size: usize,
}

impl TaskControlBlock {
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_ctx_ppn = memory_set
            .page_table
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let task_status = TaskStatus::Ready;
        // map a kernel-stack in kernel space
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.lock().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        let task_ctx_ptr = kernel_stack_top - mem::size_of::<TaskContext>();
        unsafe {
            *(task_ctx_ptr as *mut TaskContext) = TaskContext::goto_trap_return();
        }
        let task_control_block = Self {
            task_ctx_ptr,
            task_status,
            task_prio: 16,
            task_stride: 0,
            memory_set,
            trap_ctx_ppn,
            base_size: user_sp,
        };
        // prepare TrapContext in user space
        let trap_ctx = task_control_block.get_trap_ctx();
        *trap_ctx = TrapContext::app_init_context(entry_point, user_sp, kernel_stack_top);
        task_control_block
    }

    pub fn get_task_ctx_ptr2(&self) -> *const usize {
        &self.task_ctx_ptr
    }

    pub fn get_trap_ctx(&self) -> &'static mut TrapContext {
        self.trap_ctx_ppn.get_mut()
    }

    pub fn get_user_token(&self) -> usize {
        self.memory_set.page_table.token()
    }
}
