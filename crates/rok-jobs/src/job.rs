// job.rs

use core::mem::{MaybeUninit, align_of, size_of};

use crate::{job_fence::JobFence, job_record::JobRecord};

const JOB_SIZE: usize = 64;
const JOB_ALIGN: usize = 16;

// Type aliases so size_of works cleanly in the storage size calculation.
type RunFn = unsafe fn(*mut u8);
type DropFn = unsafe fn(*mut u8);

#[repr(C, align(16))]
pub struct Aligned<const N: usize>(pub [u8; N]);

#[repr(C)]
pub struct Job {
    run: RunFn,
    drop: DropFn,
    pub(crate) fence: Option<*const JobFence>,
    pub(crate) record: Option<*const JobRecord>,
    storage: MaybeUninit<
        Aligned<
            {
                JOB_SIZE
                    - size_of::<RunFn>()
                    - size_of::<DropFn>()
                    - size_of::<Option<*const JobFence>>()
            },
        >,
    >,
}

impl Job {
    /// A constant representing a no-op job.
    pub const NOOP: Self = Self {
        run: noop_run,
        drop: noop_drop,
        fence: None,
        record: None,
        storage: MaybeUninit::uninit(),
    };

    pub fn new<F>(f: F, fence: Option<*const JobFence>, record: Option<*const JobRecord>) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        const {
            assert!(
                size_of::<F>()
                    <= JOB_SIZE
                        - size_of::<RunFn>()
                        - size_of::<DropFn>()
                        - size_of::<Option<*const JobFence>>(),
                "Closure too large for Job slot!"
            );
            assert!(
                align_of::<F>() <= JOB_ALIGN,
                "Alignment for job is invalid."
            );
        }

        fn run_impl<F: FnOnce()>(ptr: *mut u8) {
            unsafe { (ptr.cast::<F>().read())() }
        }

        fn drop_impl<F>(ptr: *mut u8) {
            unsafe { core::ptr::drop_in_place(ptr.cast::<F>()) }
        }

        let mut job = Job {
            run: run_impl::<F>,
            drop: drop_impl::<F>,
            fence,
            record,
            storage: MaybeUninit::uninit(),
        };

        unsafe {
            job.storage.as_mut_ptr().cast::<F>().write(f);
        }

        job
    }

    #[inline]
    pub fn execute(&mut self) {
        unsafe {
            let ptr = self.storage.as_mut_ptr().cast::<u8>();
            (self.run)(ptr);
            self.run = noop_run;
            self.drop = noop_drop; // closure is gone, don't touch that memory again
        }

        if let Some(fence) = self.fence {
            unsafe {
                (*fence).decrement();
            }
        }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            let ptr = self.storage.as_mut_ptr().cast::<u8>();
            (self.drop)(ptr);
        }
    }
}

unsafe fn noop_run(_: *mut u8) {}
unsafe fn noop_drop(_: *mut u8) {}
