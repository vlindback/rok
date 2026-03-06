// job.rs

use core::mem::{MaybeUninit, align_of, size_of};

const JOB_SIZE: usize = 64;
const JOB_ALIGN: usize = 16;

#[repr(C, align(16))]
pub struct Aligned<const N: usize>(pub [u8; N]);

#[repr(C)]
pub struct JobHeader {
    pub run: unsafe fn(*mut u8),
    pub drop: unsafe fn(*mut u8),
}

#[repr(C)]
pub struct Job {
    header: JobHeader,
    storage: MaybeUninit<Aligned<{ JOB_SIZE - size_of::<JobHeader>() }>>,
}

impl Job {
    /// A constant representing a no-op job.
    pub const NOOP: Self = Self {
        header: JobHeader {
            run: noop_run,
            drop: noop_drop,
        },
        storage: MaybeUninit::uninit(),
    };

    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        const {
            assert!(
                size_of::<F>() <= JOB_SIZE - size_of::<JobHeader>(),
                "Closure too large for Job slot!"
            );
            assert!(
                align_of::<F>() <= JOB_ALIGN,
                "Alignment for job is invalid."
            );
        }

        fn run_impl<F: FnOnce()>(ptr: *mut u8) {
            let f_ptr = ptr.cast::<F>();
            unsafe {
                (f_ptr.read())(); // move + call
            }
        }

        fn drop_impl<F>(ptr: *mut u8) {
            unsafe {
                core::ptr::drop_in_place(ptr.cast::<F>());
            }
        }

        let mut job = Job {
            header: JobHeader {
                run: run_impl::<F>,
                drop: drop_impl::<F>,
            },
            storage: MaybeUninit::uninit(),
        };

        unsafe {
            let dst = job.storage.as_mut_ptr().cast::<F>();
            dst.write(f);
        }

        job
    }

    #[inline]
    pub fn execute(&mut self) {
        unsafe {
            let ptr = self.storage.as_mut_ptr().cast::<u8>();

            // Move + run closure
            (self.header.run)(ptr);

            // Prevent future drop from running
            self.header.drop = noop_drop;
        }
    }
}

fn noop_drop(_: *mut u8) {}

fn noop_run(_ptr: *mut u8) {}

impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            let ptr = self.storage.as_mut_ptr().cast::<u8>();
            (self.header.drop)(ptr);
        }
    }
}
