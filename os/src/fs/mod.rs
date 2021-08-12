mod pipe;
mod stdio;

use crate::mm::UserBuffer;
pub use pipe::make_pipe;
pub use stdio::{STDIN, STDOUT};

pub trait File: Send + Sync {
    fn read(&self, buf: UserBuffer) -> usize;
    fn write(&self, buf: UserBuffer) -> usize;
}
