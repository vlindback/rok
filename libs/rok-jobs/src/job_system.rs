// job_system.rs

use std::sync::Arc;

use crate::job_fence::JobFence;
use crate::job_priority::JobPriority;
use crate::job_record::JobRecord;
use crate::job_scheduler::JobScheduler;
use crate::join_handle::JoinHandle;

pub use crate::job_scheduler::JobSchedulerConfig;

pub struct JobSystem {
    scheduler: Arc<JobScheduler>,
}

impl JobSystem {
    pub fn new() -> Self {
        Self::with_config(JobSchedulerConfig::default())
    }

    pub fn with_config(config: JobSchedulerConfig) -> Self {
        Self {
            scheduler: JobScheduler::with_config(config),
        }
    }

    /// Begin building a single job submission.
    ///
    /// # Example
    /// ```
    /// // Fire and forget, Normal priority.
    /// job_system.submit(|| do_work()).dispatch().detach();
    ///
    /// // With an explicit fence.
    /// let fence = JobFence::new();
    /// job_system.submit(|| do_work()).with_fence(&fence).dispatch().wait();
    ///
    /// // High priority with a fence.
    /// let fence = JobFence::new();
    /// job_system.submit(|| do_work())
    ///     .with_priority(JobPriority::High)
    ///     .with_fence(&fence)
    ///     .dispatch()
    ///     .wait();
    /// ```
    pub fn submit<F>(&self, f: F) -> JobBuilder<'_, F>
    where
        F: FnOnce() + Send + 'static,
    {
        JobBuilder::new(self, f)
    }

    /// Begin building a parallel_for submission.
    ///
    /// Splits `iter` into one job per item. For coarser granularity
    /// call `.chunks(n)` on your slice before passing it in.
    ///
    /// # Example
    /// ```
    /// let fence = JobFence::new();
    /// job_system
    ///     .parallel_for(entities.chunks(64), |chunk| {
    ///         for e in chunk { e.update(); }
    ///     })
    ///     .with_fence(&fence)
    ///     .dispatch()
    ///     .wait();
    /// ```
    pub fn parallel_for<I, F>(&self, iter: I, f: F) -> ParallelForBuilder<'_, I, F>
    where
        I: Iterator + Send,
        I::Item: Send + 'static,
        F: Fn(I::Item) + Send + Sync + Clone + 'static,
    {
        ParallelForBuilder::new(self, iter, f)
    }

    /// Checks if there's an active panic in the JobSystem, if there is
    /// it will resume_unwind.
    pub fn check_panics(&self) {
        self.scheduler.check_panics()
    }
}

impl Default for JobSystem {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// JobBuilder
// ---------------------------------------------------------------------------

pub struct JobBuilder<'sys, F> {
    system: &'sys JobSystem,
    f: F,
    priority: JobPriority,
    fence: Option<*const JobFence>,
    record: Option<*const JobRecord>,
}

impl<'sys, F> JobBuilder<'sys, F>
where
    F: FnOnce() + Send + 'static,
{
    fn new(system: &'sys JobSystem, f: F) -> Self {
        Self {
            system,
            f,
            priority: JobPriority::Normal,
            fence: None,
            record: None,
        }
    }

    pub fn with_priority(mut self, priority: JobPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Attach a fence. The fence lifetime is enforced by the borrow
    /// it must outlive this builder and the dispatched job.
    pub fn with_fence(mut self, fence: &'sys JobFence) -> Self {
        self.fence = Some(fence as *const JobFence);
        self
    }

    // Attach a record. The record lifetime is enforced by the borrow
    // it must outlive this builder and the dispatched job.
    pub fn with_record(mut self, record: &'sys JobRecord) -> Self {
        self.record = Some(record as *const JobRecord);
        self
    }

    /// Schedule the job and return a [`JoinHandle`].
    ///
    /// The handle will debug_assert if dropped without calling
    /// [`JoinHandle::wait`] or [`JoinHandle::detach`].
    #[must_use = "handle must be waited on or explicitly detached"]
    pub fn dispatch(self) -> JoinHandle<'sys> {
        self.system
            .scheduler
            .schedule(self.priority, self.fence, self.record, self.f);
        JoinHandle::new(self.fence)
    }
}

// ---------------------------------------------------------------------------
// ParallelForBuilder
// ---------------------------------------------------------------------------

pub struct ParallelForBuilder<'sys, I, F> {
    system: &'sys JobSystem,
    iter: I,
    f: F,
    priority: JobPriority,
    fence: Option<*const JobFence>,
    record: Option<*const JobRecord>,
}

impl<'sys, I, F> ParallelForBuilder<'sys, I, F>
where
    I: Iterator + Send,
    I::Item: Send + 'static,
    F: Fn(I::Item) + Send + Sync + Clone + 'static,
{
    fn new(system: &'sys JobSystem, iter: I, f: F) -> Self {
        Self {
            system,
            iter,
            f,
            priority: JobPriority::Normal,
            fence: None,
            record: None,
        }
    }

    pub fn with_priority(mut self, priority: JobPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_fence(mut self, fence: &'sys JobFence) -> Self {
        self.fence = Some(fence as *const JobFence);
        self
    }

    pub fn with_record(mut self, record: &'sys JobRecord) -> Self {
        self.record = Some(record as *const JobRecord);
        self
    }

    #[must_use = "handle must be waited on or explicitly detached"]
    pub fn dispatch(self) -> JoinHandle<'sys> {
        for item in self.iter {
            let f = self.f.clone();
            self.system
                .scheduler
                .schedule(self.priority, self.fence, self.record, move || f(item));
        }
        JoinHandle::new(self.fence)
    }
}
