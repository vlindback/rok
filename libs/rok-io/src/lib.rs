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

/// A platform-agnostic raw file handle.
/// Obtain one from a `std::fs::File` via `RawHandle::from_file(&file)`.
#[derive(Copy, Clone)]
pub struct RawHandle(
    #[cfg(target_os = "windows")] pub(crate) std::os::windows::io::RawHandle,
    #[cfg(target_os = "linux")] pub(crate) std::os::unix::io::RawFd,
);

impl RawHandle {
    pub fn from_file(file: &std::fs::File) -> Self {
        #[cfg(target_os = "windows")]
        {
            Self(std::os::windows::io::AsRawHandle::as_raw_handle(file))
        }
        #[cfg(target_os = "linux")]
        {
            Self(std::os::unix::io::AsRawFd::as_raw_fd(file))
        }
    }
}
