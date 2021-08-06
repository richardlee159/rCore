const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_SET_PRIORITY: usize = 140;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_MMAP: usize = 222;

mod fs;
mod memory;
mod process;
mod time;

pub fn syscall(id: usize, args: [usize; 3]) -> isize {
    match id {
        SYSCALL_WRITE => fs::sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => process::sys_exit(args[0] as i32),
        SYSCALL_YIELD => process::sys_yield(),
        SYSCALL_SET_PRIORITY => process::sys_set_priority(args[0] as isize),
        SYSCALL_GET_TIME => time::sys_get_time(args[0] as *mut time::TimeVal, args[1]),
        SYSCALL_MUNMAP => memory::munmap(args[0], args[1]),
        SYSCALL_MMAP => memory::mmap(args[0], args[1], args[2]),
        _ => panic!("Unsupported syscall id: {}", id),
    }
}
