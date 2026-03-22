use crate::io_capabilities::IoCapabilities;

use std::os::unix::io::AsRawFd;

use crate::io_capabilities::IoCapabilities;
use crate::{Completion, IoToken};
use ioring_api::*;

mod io_buffer;
pub use io_buffer::IoBuffer;

pub struct IoRingLinux {}

impl IoRingLinux {
    pub fn new(_caps: IoCapabilities) -> Result<Self, &'static str> {
        todo!("io_uring setup + mmap")
    }

    pub fn submit_read(
        &mut self,
        _fd: i32,
        _buffer: &mut IoBuffer,
        _offset: u64,
        _size: u32,
        _token: IoToken,
    ) -> Result<(), &'static str> {
        debug_assert!(
            _size as usize <= _buffer.size(),
            "read size exceeds buffer size"
        );
        todo!("build SQE")
    }

    pub fn submit(&mut self, _wait_for: u32) -> Result<(), &'static str> {
        todo!("io_uring_enter")
    }

    pub fn pop_completion(&mut self) -> Option<Completion> {
        todo!("drain CQE")
    }
}

pub fn get_io_capabilities() -> Result<IoCapabilities, &'static str> {
    todo!("linux io_uring capabilities probe")
}
