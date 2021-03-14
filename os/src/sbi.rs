const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_SHUTDOWN: usize = 8;

#[inline(always)]
fn sbi_call(id: usize, args: [usize; 3]) -> usize {
    let mut ret;
    unsafe {
        llvm_asm!("ecall"
            : "={x10}" (ret)
            : "{x10}" (args[0]), "{x11}" (args[1]), "{x12}" (args[2]), "{x17}" (id)
            : "memory"
            : "volatile"
        );
    }
    ret
}

pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, [c, 0, 0]);
}

pub fn shutdown() -> ! {
    sbi_call(SBI_SHUTDOWN, [0, 0, 0]);
    panic!("It should shutdown!");
}
