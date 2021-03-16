use crate::trap::TrapContext;
use core::{cell::RefCell, mem, slice};
use lazy_static::lazy_static;

const USER_STACK_SIZE: usize = 4096 * 2;
const KERNEL_STACK_SIZE: usize = 4096 * 2;
const MAX_APP_NUM: usize = 16;
const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x20000;

#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: KernelStack = KernelStack {
    data: [0; KERNEL_STACK_SIZE],
};

static USER_STACK: UserStack = UserStack {
    data: [0; USER_STACK_SIZE],
};

impl KernelStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    fn push_context(&self, ctx: TrapContext) -> &mut TrapContext {
        let ctx_ptr = (self.get_sp() - mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *ctx_ptr = ctx;
            ctx_ptr.as_mut().unwrap()
        }
    }
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }

    fn contain(&self, data: &[u8]) -> bool {
        let stack_ptr = self.data.as_ptr() as usize;
        let data_ptr = data.as_ptr() as usize;
        (stack_ptr <= data_ptr) && (stack_ptr + USER_STACK_SIZE >= data_ptr + data.len())
    }
}

struct AppManager {
    inner: RefCell<AppManagerInner>,
}

struct AppManagerInner {
    num_app: usize,
    current_app: usize,
    app_start: [usize; MAX_APP_NUM + 1],
}

unsafe impl Sync for AppManager {}

impl AppManagerInner {
    fn print_app_info(&self) {
        info!("num_app = {}", self.num_app);
        for i in 0..self.num_app {
            info!(
                "app_{} [{:#x}, {:#x})",
                i,
                self.app_start[i],
                self.app_start[i + 1]
            );
        }
    }

    fn get_current_app(&self) -> usize {
        self.current_app
    }

    fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }

    unsafe fn load_app(&self, app_id: usize) {
        if app_id >= self.num_app {
            panic!("All application completed!");
        }
        info!("Loading app_{}", app_id);
        // clear icache
        llvm_asm!("fence.i" :::: "volatile");
        (APP_BASE_ADDRESS..APP_BASE_ADDRESS + APP_SIZE_LIMIT).for_each(|addr| {
            (addr as *mut u8).write_volatile(0);
        });
        let app_src = slice::from_raw_parts(
            self.app_start[app_id] as *const u8,
            self.app_start[app_id + 1] - self.app_start[app_id],
        );
        let app_dst = slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, app_src.len());
        app_dst.copy_from_slice(app_src);
    }
}

lazy_static! {
    static ref APP_MANAGER: AppManager = AppManager {
        inner: RefCell::new({
            extern "C" {
                fn _num_app();
            }
            let num_app_ptr = _num_app as usize as *const usize;
            let num_app = unsafe { num_app_ptr.read_volatile() };
            let mut app_start: [usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM + 1];
            let app_start_raw = unsafe { slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
            app_start[..=num_app].copy_from_slice(app_start_raw);
            AppManagerInner {
                num_app,
                current_app: 0,
                app_start,
            }
        }),
    };
}

pub fn init() {
    APP_MANAGER.inner.borrow().print_app_info();
}

pub fn run_next_app() -> ! {
    let current_app = APP_MANAGER.inner.borrow().get_current_app();
    unsafe {
        APP_MANAGER.inner.borrow().load_app(current_app);
    }
    APP_MANAGER.inner.borrow_mut().move_to_next_app();
    extern "C" {
        fn __restore(ctx_addr: usize) -> !;
    }
    unsafe {
        __restore(KERNEL_STACK.push_context(TrapContext::app_init_context(
            APP_BASE_ADDRESS,
            USER_STACK.get_sp(),
        )) as *const _ as usize)
    }
}

fn within_app_space(data: &[u8]) -> bool {
    let data_ptr = data.as_ptr() as usize;
    (APP_BASE_ADDRESS <= data_ptr) && (APP_BASE_ADDRESS + APP_SIZE_LIMIT >= data_ptr + data.len())
}

pub fn within_user_space(data: &[u8]) -> bool {
    within_app_space(data) || USER_STACK.contain(data)
}
