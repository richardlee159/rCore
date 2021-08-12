const SYSCALL_CLOSE: usize = 57;
const SYSCALL_PIPE: usize = 59;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_SET_PRIORITY: usize = 140;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_WAITPID: usize = 260;
const SYSCALL_SPAWN: usize = 400;

mod fs;
mod memory;
mod process;
mod time;

pub fn syscall(id: usize, args: [usize; 3]) -> isize {
    match id {
        SYSCALL_CLOSE => fs::sys_close(args[0]),
        SYSCALL_PIPE => fs::sys_pipe(args[0] as *mut usize),
        SYSCALL_READ => fs::sys_read(args[0], args[1] as *mut u8, args[2]),
        SYSCALL_WRITE => fs::sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => process::sys_exit(args[0] as i32),
        SYSCALL_YIELD => process::sys_yield(),
        SYSCALL_SET_PRIORITY => process::sys_set_priority(args[0] as isize),
        SYSCALL_GET_TIME => time::sys_get_time(args[0] as *mut time::TimeVal, args[1]),
        SYSCALL_GETPID => process::sys_getpid(),
        SYSCALL_MUNMAP => memory::munmap(args[0], args[1]),
        SYSCALL_FORK => process::sys_fork(),
        SYSCALL_EXEC => process::sys_exec(args[0] as *const u8),
        SYSCALL_MMAP => memory::mmap(args[0], args[1], args[2]),
        SYSCALL_WAITPID => process::sys_waitpid(args[0] as isize, args[1] as *mut i32),
        SYSCALL_SPAWN => process::sys_spawn(args[0] as *const u8),
        _ => panic!("Unsupported syscall id: {}", id),
    }
}
