use super::trap_handler;
use crate::mm::KERNEL_SPACE;
use riscv::register::sstatus::{self, Sstatus, SPP};

#[repr(C)]
pub struct TrapContext {
    pub x: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
    pub kernel_satp: usize,
    pub kernel_sp: usize,
    pub trap_handler: usize,
}

impl TrapContext {
    fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }

    pub fn app_init_context(entry: usize, sp: usize, kernel_sp: usize) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut ctx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,
            kernel_satp: KERNEL_SPACE.lock().page_table.token(),
            kernel_sp,
            trap_handler: trap_handler as usize,
        };
        ctx.set_sp(sp);
        ctx
    }
}
