mod io_capabilities;

#[cfg(target_os = "windows")]
mod io_ring_windows;

#[cfg(target_os = "linux")]
mod io_ring_linux;

pub use io_capabilities::IoCapabilities;

#[cfg(target_os = "windows")]
pub use io_ring_windows::IoRingWindows as IoRing;

#[cfg(target_os = "linux")]
pub use io_ring_linux::IoRingLinux as IoRing;

#[cfg(target_os = "windows")]
pub use io_ring_windows::get_io_capabilities;

#[cfg(target_os = "linux")]
pub use io_ring_linux::get_io_capabilities;

// --- Common types ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IoToken(pub u64);

#[derive(Debug)]
pub struct Completion {
    pub token: IoToken,
    pub result: Result<usize, IoError>,
}

#[derive(Debug)]
pub struct IoError {
    pub code: i32,
}

#[derive(Debug)]
pub enum RingError {
    Setup(i32),
    Mmap(i32),
    Submit(i32),
}
