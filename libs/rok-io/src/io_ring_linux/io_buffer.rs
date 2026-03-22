// io_buffer.rs (linux)

use std::ptr;

extern "C" {
    fn mmap(addr: *mut u8, length: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut u8;

    fn munmap(addr: *mut u8, length: usize) -> i32;
}

const PROT_READ: i32 = 0x1;
const PROT_WRITE: i32 = 0x2;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;
const MAP_FAILED: *mut u8 = !0usize as *mut u8;

pub struct IoBuffer {
    ptr: *mut u8,
    size: usize,
}

// mmap with MAP_ANONYMOUS always returns page-aligned memory by contract
impl IoBuffer {
    /// Allocates a page-aligned buffer suitable for use with IoRing operations.
    ///
    /// `size` will be rounded up to the next page boundary by the OS.
    /// Callers that care about memory efficiency should round `size` up to
    /// the page size themselves using appropriate queries to avoid implicit waste.
    pub fn alloc(size: usize) -> Result<Self, &'static str> {
        unsafe {
            let ptr = mmap(
                ptr::null_mut(),
                size,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1, // no fd, anonymous mapping
                0,
            );

            if ptr == MAP_FAILED {
                return Err("mmap failed");
            }

            Ok(Self { ptr, size })
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
            munmap(self.ptr, self.size);
        }
    }
}

unsafe impl Send for IoBuffer {}
unsafe impl Sync for IoBuffer {}
