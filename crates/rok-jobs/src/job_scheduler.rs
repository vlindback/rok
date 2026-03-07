// job_scheduler.rs

use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam::deque::{Stealer, Worker};
use crossbeam::queue::ArrayQueue;
use crossbeam::sync::Parker;

use crate::job::Job;
use crate::job_fence::JobFence;
use crate::job_pool::{JobIndex, JobPool};
use crate::job_priority::JobPriority;
use crate::job_record::JobRecord;
use crate::job_worker::{JobWorkerHandle, JobWorkerInit, JobWorkerShared};
use crate::job_worker_tls::get_job_worker_local_tls;
use crate::stop::StopSource;
use crate::tsc_timer::TscTimer;

pub(crate) struct JobScheduler {
    stop_source: StopSource,
    pool: JobPool,
    workers: Vec<JobWorkerShared>,
    worker_handles: Vec<JobWorkerHandle>,
    rr_counter: AtomicUsize,
}

#[derive(Default)]
pub struct JobSchedulerConfig {
    /// How many jobs the pool can hold concurrently.
    pub pool_capacity: Option<NonZeroU32>,

    /// How large each worker's inbox is.
    pub worker_inbox_capacity: Option<NonZeroU32>,

    /// How many worker threads to spawn.
    pub worker_count: Option<NonZeroU32>,
}

impl JobScheduler {
    pub(crate) fn with_config(config: JobSchedulerConfig) -> Arc<Self> {
        const MIN_POOL_CAP: NonZeroU32 = match NonZeroU32::new(128) {
            Some(n) => n,
            None => unreachable!(),
        };

        const MIN_INBOX_CAP: NonZeroU32 = match NonZeroU32::new(32) {
            Some(n) => n,
            None => unreachable!(),
        };

        let pool_capacity = config
            .pool_capacity
            .unwrap_or(MIN_POOL_CAP)
            .max(MIN_POOL_CAP);

        let worker_count = decide_worker_count(&config).get();

        let worker_inbox_cap = config
            .worker_inbox_capacity
            .unwrap_or(MIN_INBOX_CAP)
            .max(MIN_INBOX_CAP)
            .get() as usize;

        //
        // Pass 1: collect raw parts
        //

        let mut shared_vec = Vec::with_capacity(worker_count);
        let mut worker_inits = Vec::with_capacity(worker_count);

        for _ in 0..worker_count {
            let parker = Parker::new();
            let unparker = parker.unparker().clone();

            let deques: [Worker<JobIndex>; JobPriority::COUNT] =
                std::array::from_fn(|_| Worker::new_lifo());

            let stealers: [Stealer<JobIndex>; JobPriority::COUNT] =
                std::array::from_fn(|i| deques[i].stealer());

            let inboxes: [ArrayQueue<JobIndex>; JobPriority::COUNT] =
                std::array::from_fn(|_| ArrayQueue::new(worker_inbox_cap));

            shared_vec.push(JobWorkerShared {
                stealers,
                inboxes,
                unparker,
            });

            worker_inits.push(JobWorkerInit { deques, parker });
        }

        //
        // Pass 2: freeze shared slice inside Arc — addresses are now stable
        //

        let mut scheduler = Arc::new(JobScheduler {
            stop_source: StopSource::new(),
            pool: JobPool::with_capacity(pool_capacity),
            workers: shared_vec,
            worker_handles: Vec::new(),
            rr_counter: AtomicUsize::new(0),
        });

        //
        // Pass 3: spawn workers
        //

        let worker_handles = worker_inits
            .into_iter()
            .enumerate()
            .map(|(id, init)| {
                JobWorkerHandle::spawn(
                    id,
                    scheduler.stop_source.token(),
                    scheduler.clone(),
                    init,
                    &scheduler.workers[id],
                )
            })
            .collect();

        Arc::get_mut(&mut scheduler)
            .expect("no external Arc clones exist before workers are published")
            .worker_handles = worker_handles;

        scheduler
    }

    pub(crate) fn schedule<F>(
        &self,
        prio: JobPriority,
        fence: Option<*const JobFence>,
        record: Option<*const JobRecord>,
        f: F,
    ) where
        F: FnOnce() + Send + 'static,
    {
        // Increment the fence BEFORE pushing the job to any worker.
        // If we incremented after, the job could complete and decrement
        // before we increment, causing a spurious wakeup at count 0.
        if let Some(fence) = fence {
            unsafe {
                (*fence).increment(1);
            }
        }

        let mut job = Job::new(f, fence, record);
        let index = loop {
            match self.pool.try_push(job) {
                Ok(idx) => break idx,
                Err(returned_job) => {
                    job = returned_job;
                    std::hint::spin_loop();
                    std::thread::yield_now();
                }
            }
        };

        if let Some(worker_local) = get_job_worker_local_tls() {
            // Fast path: we are on a worker thread, push directly to local deque.
            worker_local.push_work(index, prio);

            // Wake a neighbor so they can steal if we produce many jobs.
            let neighbor_idx = (worker_local.id + 1) % self.workers.len();
            let neighbor = unsafe { self.workers.get_unchecked(neighbor_idx) };
            neighbor.unparker.unpark();
        } else {
            // External thread: round-robin into a worker inbox.
            let worker_idx = self.rr_counter.fetch_add(1, Ordering::Relaxed) % self.workers.len();
            let worker = unsafe { self.workers.get_unchecked(worker_idx) };
            worker.push_to_inbox(index, prio);
            worker.unparker.unpark();
        }
    }

    pub(crate) fn run_job(&self, index: JobIndex) {
        // Safety: index was produced by try_push and has not been executed yet.
        unsafe { self.pool.execute_and_free(index) }
    }
}

impl Drop for JobScheduler {
    fn drop(&mut self) {
        self.stop_source.request_stop();
        for worker in self.worker_handles.drain(..) {
            worker.join();
        }
    }
}

fn decide_worker_count(config: &JobSchedulerConfig) -> NonZeroUsize {
    let count = config
        .worker_count
        .map(|n| n.get() as usize)
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get().saturating_sub(1).max(1))
                .unwrap_or(4)
        });

    // Safety: all three branches produce a value >= 1.
    NonZeroUsize::new(count).unwrap()
}
