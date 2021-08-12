use super::File;
use crate::mm::UserBuffer;
use alloc::{collections::VecDeque, vec::Vec};
use spin::Mutex;

const MSG_COUNT: usize = 16;
const MSG_SIZE: usize = 256;

pub struct MailBox {
    buffer: Mutex<VecDeque<Vec<u8>>>,
}

impl MailBox {
    pub fn new() -> Self {
        Self {
            buffer: Mutex::new(VecDeque::new()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.lock().is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.buffer.lock().len() == MSG_COUNT
    }
}

impl File for MailBox {
    fn read(&self, buf: UserBuffer) -> usize {
        let mut ring_buffer = self.buffer.lock();
        let msg = ring_buffer.pop_front().unwrap();
        let read_size = msg.len().min(buf.len());
        buf.into_iter()
            .zip(msg.into_iter())
            .take(read_size)
            .for_each(|(p, v)| unsafe {
                *p = v;
            });
        read_size
    }

    fn write(&self, buf: UserBuffer) -> usize {
        let mut ring_buffer = self.buffer.lock();
        let write_size = MSG_SIZE.min(buf.len());
        let msg = buf
            .into_iter()
            .take(write_size)
            .map(|p| unsafe { *p })
            .collect();
        ring_buffer.push_back(msg);
        write_size
    }
}
