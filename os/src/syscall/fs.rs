use crate::fs::{make_pipe, File};
use crate::mm::{translated_byte_buffer, translated_refmut, UserBuffer};
use crate::task::{current_task, current_user_token, get_task_by_pid};

pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.acquire_inner_lock();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

pub fn sys_pipe(pipe: *mut usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let mut inner = task.acquire_inner_lock();
    let (pipe_read, pipe_write) = make_pipe();
    let read_fd = inner.alloc_fd();
    inner.fd_table[read_fd] = Some(pipe_read);
    let write_fd = inner.alloc_fd();
    inner.fd_table[write_fd] = Some(pipe_write);
    *translated_refmut(token, pipe).unwrap() = read_fd;
    *translated_refmut(token, unsafe { pipe.add(1) }).unwrap() = write_fd;
    0
}

pub fn sys_read(fd: usize, buf: *mut u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.acquire_inner_lock();
    if let Some(Some(file)) = inner.fd_table.get(fd) {
        let file = file.clone();
        // release Task lock manually to avoid deadlock
        drop(inner);
        if let Some(buffers) = translated_byte_buffer(token, buf, len) {
            file.read(UserBuffer::new(buffers)) as isize
        } else {
            warn!("Illegal memory region in sys_read!");
            -1
        }
    } else {
        warn!("Invalid fd in sys_read!");
        -1
    }
}

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.acquire_inner_lock();
    if let Some(Some(file)) = inner.fd_table.get(fd) {
        let file = file.clone();
        // release Task lock manually to avoid deadlock
        drop(inner);
        if let Some(buffers) = translated_byte_buffer(token, buf, len) {
            file.write(UserBuffer::new(buffers)) as isize
        } else {
            warn!("Illegal memory region in sys_write!");
            -1
        }
    } else {
        warn!("Invalid fd in sys_write!");
        -1
    }
}

pub fn sys_mailread(buf: *mut u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.acquire_inner_lock();
    let mailbox = &inner.mailbox;
    if mailbox.is_empty() {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    if let Some(buffers) = translated_byte_buffer(token, buf, len) {
        mailbox.read(UserBuffer::new(buffers)) as isize
    } else {
        -1
    }
}

pub fn sys_mailwrite(pid: usize, buf: *mut u8, len: usize) -> isize {
    let token = current_user_token();
    let task = if current_task().unwrap().getpid() == pid {
        current_task().unwrap()
    } else {
        get_task_by_pid(pid).unwrap()
    };
    let inner = task.acquire_inner_lock();
    let mailbox = &inner.mailbox;
    if mailbox.is_full() {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    if let Some(buffers) = translated_byte_buffer(token, buf, len) {
        mailbox.write(UserBuffer::new(buffers)) as isize
    } else {
        -1
    }
}
