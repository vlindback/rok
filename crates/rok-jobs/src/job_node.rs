// jon_node.rs

use std::{
    ptr::NonNull,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    job::Job, job_fence::JobFence, job_node_handle::JobNodeHandle, job_priority::JobPriority,
    job_record::JobRecord,
};

pub(crate) struct JobNode {
    /// Optional fence.
    fence: Option<NonNull<JobFence>>,

    /// Optional performance record.
    record: Option<NonNull<JobRecord>>,

    /// The head of the list of jobs to "poke" when this one finishes.
    /// Atomic so multiple threads can add dependencies simultaneously.
    successor_head: AtomicU64,

    /// Our own link in a parent's successor list.
    /// This is what allows us to be "one of many" children.
    next_successor: JobNodeHandle,

    /// Job payload.
    job: std::cell::UnsafeCell<Job>,
    // TODO: dependencies counter
}

impl JobNode {
    pub fn new() -> Self {
        JobNode {
            fence: None,
            record: None,
            successor_head: AtomicU64::new(JobNodeHandle::INVALID_BITS),
            next_successor: JobNodeHandle::INVALID,
            job: std::cell::UnsafeCell::new(Job::NOOP),
        }
    }

    pub(crate) fn run_job(&self) {
        let execution_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            (&mut *self.job.get()).execute();
        }));
    }

    pub(crate) fn process_continuations(&self) {
        let current_raw: u64 = self.successor_head.load(Ordering::Acquire);
        while current_raw != JobNodeHandle::INVALID_BITS {
            let current_handle = JobNodeHandle::from_u64(current_raw);
        }
    }

    pub(crate) fn reset(&mut self) {
        *self.job.get_mut() = Job::NOOP;
        self.fence = None;
        self.record = None;
        self.successor_head
            .store(JobNodeHandle::INVALID_BITS, Ordering::Relaxed);
        self.next_successor = JobNodeHandle::INVALID;
        // TODO: clear dependency counter
    }

    pub(crate) fn arm(
        &self,
        prio: JobPriority,
        deps: u32,
        fence: Option<&JobFence>,
        record: Option<&JobRecord>,
        job: Job,
    ) {
    }
}
