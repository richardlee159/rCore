mod context;

use crate::{
    config::{TRAMPOLINE, TRAP_CONTEXT},
    syscall::syscall,
    task::{
        current_trap_ctx, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next,
    },
};
pub use context::TrapContext;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
};

global_asm!(include_str!("trap.S"));
global_asm!(
    "
    .section .text
    .globl __ktrap
    .align 2
    __ktrap:
    j trap_from_kernel
    "
);

pub fn init() {
    set_kernel_trap_entry();
}

pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

fn set_kernel_trap_entry() {
    extern "C" {
        fn __ktrap();
    }
    unsafe {
        stvec::write(__ktrap as usize, TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
    }
}

#[no_mangle]
fn trap_from_kernel() -> ! {
    error!("{:?}, {:#x}", scause::read().cause(), stval::read());
    panic!("a trap from kernel!");
}

#[no_mangle]
fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let ctx = current_trap_ctx();
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            suspend_current_and_run_next();
        }
        Trap::Exception(Exception::UserEnvCall) => {
            ctx.sepc += 4;
            ctx.x[10] = syscall(ctx.x[17], [ctx.x[10], ctx.x[11], ctx.x[12]]) as usize;
        }
        Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault)
        | Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault) => {
            warn!("PageFault in application, core dumped.");
            exit_current_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            warn!("IllegalInstruction in application, core dumped.");
            exit_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    trap_return();
}

pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_ctx_ptr = TRAP_CONTEXT;
    let user_satp = current_user_token();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        llvm_asm!("fence.i" :::: "volatile");
        llvm_asm!("jr $0"
            :: "r" (restore_va), "{a0}" (trap_ctx_ptr), "{a1}" (user_satp)
            :: "volatile"
        );
    }
    panic!("Unreachable in back_to_user!");
}
