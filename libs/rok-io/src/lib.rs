// lib.rs

#[cfg(target_os = "linux")]
pub mod io_backend_linux;

#[cfg(target_os = "windows")]
pub mod io_ring_windows;

#[cfg(target_os = "linux")]
pub use io_backend_linux as io_ring;

#[cfg(target_os = "windows")]
pub use io_ring_windows as io_ring;

pub mod io_capabilities;

pub use io_capabilities::IoCapabilities;

use std::{
    alloc::{Layout, alloc, dealloc},
    num::NonZeroUsize,
};

#[cfg(target_os = "linux")]
type BackendImpl = crate::io_backend_linux::IoRingLinux;

#[cfg(target_os = "windows")]
type BackendImpl = crate::io_ring_windows::IoRingWindows;

// --- Public Types ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IoToken(pub u64);

/// Result of a completed IO operation.
#[derive(Debug)]
pub struct Completion {
    pub token: IoToken,
    pub result: Result<usize, IoError>,
}

/// IO error. On Linux/Android this is a negated errno value from the CQE.
#[derive(Debug)]
pub struct IoError {
    pub code: i32,
}

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "io error: errno {}", self.code)
    }
}

impl std::error::Error for IoError {}

/// Errors that can occur when creating or operating the ring itself (not IO
/// errors on individual operations).
#[derive(Debug)]
pub enum RingError {
    /// Kernel returned an error during ring setup.
    Setup(i32),
    /// An mmap call failed.
    Mmap(i32),
    /// io_uring_enter returned an error when submitting.
    Submit(i32),
}

impl std::fmt::Display for RingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RingError::Setup(e) => write!(f, "ring setup failed: errno {e}"),
            RingError::Mmap(e) => write!(f, "ring mmap failed: errno {e}"),
            RingError::Submit(e) => write!(f, "ring submit failed: errno {e}"),
        }
    }
}

impl std::error::Error for RingError {}

pub struct IoRing {
    layout: std::alloc::Layout,
    buffer: *mut u8,
    backend: BackendImpl,
}

impl IoRing {
    pub fn new(buffer_size: NonZeroUsize) -> Self {
        let layout = Layout::from_size_align(buffer_size.get(), 4096).unwrap();
        let buffer = unsafe { alloc(layout) };
        let backend = BackendImpl {};

        Self {
            layout,
            buffer,
            backend,
        }
    }

    pub fn get_io_capabilities() -> Result<IoCapabilities, &'static str> {
        io_ring::get_io_capabilities()
    }
}
