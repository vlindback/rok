// job_pool.rs

use crossbeam::queue::ArrayQueue;
use std::{cell::UnsafeCell, num::NonZeroU32};

use crate::job::Job;

/// A simple index into the [`JobPool`] array.
///
/// Without continuations nobody holds long-lived references to job slots,
/// so generational ABA protection is no longer necessary. A plain index is enough.
#[derive(Copy, Clone)]
pub(crate) struct JobIndex(pub(crate) u32);

/// A pre-allocated arena of job slots managed with a lock-free free list.
///
/// Allocation and deallocation are wait-free operations — critical for
/// high-frequency job submission on worker threads.
pub(crate) struct JobPool {
    /// Fixed-size storage for all concurrent jobs.
    slots: Box<[UnsafeCell<Job>]>,

    /// Lock-free queue of indices currently available for allocation.
    free: ArrayQueue<u32>,
}

unsafe impl Send for JobPool {}
unsafe impl Sync for JobPool {}

impl JobPool {
    /// Creates a new pool with a fixed maximum capacity.
    ///
    /// All memory is allocated upfront and never grows.
    pub(crate) fn with_capacity(cap: NonZeroU32) -> Self {
        let cap = cap.get() as usize;

        let slots = (0..cap)
            .map(|_| UnsafeCell::new(Job::NOOP))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        let free = ArrayQueue::new(cap);
        for i in 0..cap {
            free.push(i as u32).ok().unwrap();
        }

        Self { slots, free }
    }

    /// Attempts to claim a slot and write a job into it.
    ///
    /// Returns `Ok(JobIndex)` on success, or `Err(Job)` if the pool is
    /// exhausted — returning the job so the caller can retry without
    /// losing the closure.
    pub(crate) fn try_push(&self, job: Job) -> Result<JobIndex, Job> {
        match self.free.pop() {
            Some(index) => {
                // Safety: we just popped this index from the free queue,
                // so no other thread holds a reference to this slot.
                unsafe {
                    *self.slots[index as usize].get() = job; // assignment drops the old Job
                }
                Ok(JobIndex(index))
            }
            None => Err(job),
        }
    }

    /// # Safety
    /// - `index` must have been obtained from `try_push`
    /// - the slot must not have been freed yet
    /// - no other reference to this slot may exist
    pub(crate) unsafe fn get_mut(&self, index: JobIndex) -> &mut Job {
        unsafe { &mut *self.slots[index.0 as usize].get() }
    }

    /// Returns the job at `index` to the free list.
    ///
    /// # Safety
    /// Caller must guarantee `index` was obtained from [`try_push`]
    pub(crate) unsafe fn free(&self, index: JobIndex) {
        // Return slot to free list.
        // Pushing can only fail if the queue is full, which would mean
        // more frees than allocations — a logic error.
        debug_assert!(
            self.free.push(index.0).is_ok(),
            "JobPool: double free detected on slot {}",
            index.0
        );
    }

    /// Returns the maximum number of concurrent jobs this pool supports.
    pub(crate) fn capacity(&self) -> usize {
        self.slots.len()
    }
}
