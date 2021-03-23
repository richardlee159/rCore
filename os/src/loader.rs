use crate::{config::*, task::TaskContext, trap::TrapContext};
use core::{mem, slice};

#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

static KERNEL_STACKS: [KernelStack; MAX_APP_NUM] = [KernelStack {
    data: [0; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];

static USER_STACKS: [UserStack; MAX_APP_NUM] = [UserStack {
    data: [0; USER_STACK_SIZE],
}; MAX_APP_NUM];

impl KernelStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    fn push_context(&self, trap_ctx: TrapContext, task_ctx: TaskContext) -> &mut TaskContext {
        let trap_ctx_ptr = (self.get_sp() - mem::size_of::<TrapContext>()) as *mut TrapContext;
        let task_ctx_ptr =
            (trap_ctx_ptr as usize - mem::size_of::<TaskContext>()) as *mut TaskContext;
        unsafe {
            *trap_ctx_ptr = trap_ctx;
            *task_ctx_ptr = task_ctx;
            task_ctx_ptr.as_mut().unwrap()
        }
    }
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }

    fn contain(&self, ptr: usize, len: usize) -> bool {
        let stack_ptr = self.data.as_ptr() as usize;
        (stack_ptr <= ptr) && (stack_ptr + USER_STACK_SIZE >= ptr + len)
    }
}

fn get_base_i(app_id: usize) -> usize {
    APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT
}

pub fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as *const usize).read_volatile() }
}

pub fn load_apps() {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe { slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    // clear icache
    unsafe {
        llvm_asm!("fence.i" :::: "volatile");
    }
    // load apps
    for i in 0..num_app {
        let base_i = get_base_i(i);
        // clear region
        (base_i..base_i + APP_SIZE_LIMIT).for_each(|addr| unsafe {
            (addr as *mut u8).write_volatile(0);
        });
        // load app from data section to memory
        let app_src = unsafe {
            slice::from_raw_parts(app_start[i] as *const u8, app_start[i + 1] - app_start[i])
        };
        let app_dst = unsafe { slice::from_raw_parts_mut(base_i as *mut u8, app_src.len()) };
        app_dst.copy_from_slice(app_src);
    }
}

pub fn init_app_ctx(app_id: usize) -> &'static mut TaskContext {
    KERNEL_STACKS[app_id].push_context(
        TrapContext::app_init_context(get_base_i(app_id), USER_STACKS[app_id].get_sp()),
        TaskContext::goto_restore(),
    )
}

fn within_app_space(app_id: usize, ptr: usize, len: usize) -> bool {
    (get_base_i(app_id) <= ptr) && (get_base_i(app_id) + APP_SIZE_LIMIT >= ptr + len)
}

pub fn within_user_space(app_id: usize, ptr: usize, len: usize) -> bool {
    within_app_space(app_id, ptr, len) || USER_STACKS[app_id].contain(ptr, len)
}
