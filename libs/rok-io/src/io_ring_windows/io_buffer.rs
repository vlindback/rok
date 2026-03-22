// io_buffer.rs (windows)

use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, VirtualAlloc, VirtualFree,
};

pub struct IoBuffer {
    ptr: *mut u8,
    size: usize,
}

// VirtualAlloc returns page-aligned memory by contract
impl IoBuffer {
    /// Allocates a page-aligned buffer suitable for use with IoRing operations.
    ///
    /// `size` will be rounded up to the next page boundary by the OS.
    /// Callers that care about memory efficiency should round `size` up to
    /// the page size themselves using appropriate queries to avoid implicit waste.
    pub fn alloc(size: usize) -> Result<Self, &'static str> {
        unsafe {
            let ptr = VirtualAlloc(
                std::ptr::null(),
                size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            );

            if ptr.is_null() {
                return Err("VirtualAlloc failed");
            }

            Ok(Self {
                ptr: ptr as *mut u8,
                size,
            })
        }
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for IoBuffer {
    fn drop(&mut self) {
        unsafe {
            VirtualFree(self.ptr as *mut _, 0, MEM_RELEASE);
        }
    }
}

unsafe impl Send for IoBuffer {}
unsafe impl Sync for IoBuffer {}
