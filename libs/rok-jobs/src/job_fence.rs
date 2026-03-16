// job_fence.rs

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Condvar, Mutex};

pub struct JobFence {
    /// Counts pending jobs. Starts at 0, incremented before dispatch,
    /// decremented on completion. Waiters wake when it reaches 0.
    count: AtomicI32,

    /// Guards the condvar wait/notify pair to prevent lost wakeups.
    lock: Mutex<()>,
    cv: Condvar,
}

impl JobFence {
    pub fn new() -> Self {
        Self {
            count: AtomicI32::new(0),
            lock: Mutex::new(()),
            cv: Condvar::new(),
        }
    }

    /// Increments the pending job count before dispatch.
    ///
    /// Must be called by the scheduler *before* jobs are pushed to workers,
    /// to prevent a race where jobs complete before the count is set.
    pub(crate) fn increment(&self, n: i32) {
        self.count.fetch_add(n, Ordering::Relaxed);
    }

    /// Decrements the pending count. Called by a job on completion.
    ///
    /// If this brings the count to zero all waiters are notified.
    pub(crate) fn decrement(&self) {
        let prev = self.count.fetch_sub(1, Ordering::Release);

        debug_assert!(
            prev > 0,
            "JobFence decremented below zero — double completion detected"
        );

        if prev == 1 {
            // We were the last job. Wake all waiters.
            // Lock briefly to synchronize with threads currently entering wait().
            // This prevents the lost wakeup where a waiter checks count,
            // we decrement + notify, then the waiter parks and never wakes.
            let _guard = self.lock.lock().unwrap();
            self.cv.notify_all();
        }
    }

    /// Block the calling thread until all associated jobs are complete.
    /// This
    pub fn wait(&self) {
        use crate::job_priority::JobPriority;
        use crate::job_worker_tls::get_job_worker_local_tls;

        // Fast path: already done.
        if self.is_complete() {
            return;
        }

        // Help if we are a worker thread.
        if let Some(worker) = get_job_worker_local_tls() {
            while !self.is_complete() {
                for prio in JobPriority::ALL {
                    worker.drain_shared_inbox(prio);
                }

                if let Some(idx) = worker.pop_local() {
                    worker.scheduler.run_job(idx);
                } else {
                    std::hint::spin_loop();
                }
            }
        }

        let guard = self.lock.lock().unwrap();
        let _guard = self.cv.wait_while(guard, |_| !self.is_complete()).unwrap();
    }

    /// Blocks the calling thread by spinning until all associate djobs are complete.
    /// Prefer on time-sensitive threads with a hard time deadline where latency
    /// is a high priority.
    pub fn wait_spin(&self) {
        while !self.is_complete() {
            std::hint::spin_loop();
        }
        // Acquire fence to ensure all job writes are visible to this thread.
        std::sync::atomic::fence(Ordering::Acquire);
    }

    /// Returns true if all associated jobs have completed.
    #[inline]
    pub fn is_complete(&self) -> bool {
        self.count.load(Ordering::Acquire) <= 0
    }
}

impl Default for JobFence {
    fn default() -> Self {
        Self::new()
    }
}
