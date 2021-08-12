use super::File;
use crate::mm::UserBuffer;
use crate::sbi::{console_getchar, console_putchar};
use crate::task::suspend_current_and_run_next;

pub struct STDIN;

impl File for STDIN {
    fn read(&self, mut buf: UserBuffer) -> usize {
        assert_eq!(buf.len(), 1, "Only support len = 1 in sys_read!");
        let ch = loop {
            let c = console_getchar();
            if c == 0 {
                suspend_current_and_run_next();
            } else {
                break c;
            }
        } as u8;
        buf.buffers[0][0] = ch;
        1
    }

    fn write(&self, _buf: UserBuffer) -> usize {
        panic!("Cannot write to stdin!");
    }
}

pub struct STDOUT;

impl File for STDOUT {
    fn read(&self, _buf: UserBuffer) -> usize {
        panic!("Cannot read from stdout!");
    }

    fn write(&self, buf: UserBuffer) -> usize {
        for buffer in &buf.buffers {
            for &c in buffer.iter() {
                console_putchar(c as usize);
            }
        }
        buf.len()
    }
}
