// job_scheduler.rs

use std::any::Any;
use std::mem::MaybeUninit;
use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::ThreadId;

use crossbeam::deque::{Stealer, Worker};
use crossbeam::queue::ArrayQueue;
use crossbeam::sync::Parker;
use crossbeam::utils::CachePadded;

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
    workers: Vec<CachePadded<JobWorkerShared>>,
    worker_handles: OnceLock<Vec<JobWorkerHandle>>,
    rr_counter: AtomicUsize,
    timer: TscTimer,
    panic_payload: Mutex<Option<Box<dyn Any + Send>>>,
    owner_thread: ThreadId,
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

            shared_vec.push(CachePadded::new(JobWorkerShared {
                stealers,
                inboxes,
                unparker,
            }));

            worker_inits.push(JobWorkerInit { deques, parker });
        }

        //
        // Pass 2: freeze shared slice inside Arc — addresses are now stable
        //

        let scheduler = Arc::new(JobScheduler {
            stop_source: StopSource::new(),
            pool: JobPool::with_capacity(pool_capacity),
            workers: shared_vec,
            worker_handles: OnceLock::new(),
            rr_counter: AtomicUsize::new(0),
            timer: TscTimer::calibrate(),
            panic_payload: Mutex::new(None),
            owner_thread: std::thread::current().id(),
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

        scheduler.worker_handles.set(worker_handles).ok();

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
        //
        // Safety: index was produced by try_push and has not been executed yet.
        unsafe {
            let job = self.pool.get_mut(index);

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let record = job.record;
                if record.is_some() {
                    let (_, ns) = self.timer.measure(|| job.execute());
                    if let Some(rec) = record {
                        (*rec).record(ns);
                    }
                } else {
                    job.execute();
                }
            }));

            self.pool.free(index);

            if let Err(payload) = result {
                let mut guard = self.panic_payload.lock().unwrap();
                if guard.is_none() {
                    *guard = Some(payload); // store first panic, discard subsequent
                }
            }
        }
    }

    pub(crate) fn try_steal(
        &self,
        thief_id: usize,
        loot: &mut [MaybeUninit<(JobIndex, JobPriority)>],
    ) -> usize {
        // NOTE:
        //
        //      The worker loop assumes that the loot filled here is done so in a priority order.
        //      if that ever changes we might get priority bugs. Just an FYI for refactoring
        //
        let n_workers = self.workers.len();
        let mut n_stolen: usize = 0;
        let capacity = loot.len();
        for prio in JobPriority::ALL {
            for i in 1..n_workers {
                // Start one past our own index, wrap around, stop before reaching ourselves.
                let target = (thief_id + i) % n_workers;
                while n_stolen < capacity {
                    match self.workers[target].stealers[prio.index()].steal() {
                        crossbeam::deque::Steal::Success(index) => {
                            loot[n_stolen] = MaybeUninit::new((index, prio));
                            n_stolen += 1;
                        }
                        crossbeam::deque::Steal::Retry => {
                            // Lost a race. Better to move on to a less contested worker
                            // than to spin here and burn CPU.
                            break;
                        }
                        crossbeam::deque::Steal::Empty => break,
                    }
                }
                if n_stolen == capacity {
                    return n_stolen;
                }
            }
        }
        n_stolen
    }

    pub fn check_panics(&self) {
        debug_assert!(
            std::thread::current().id() == self.owner_thread,
            "check_panics() must be called from the thread that created the JobSystem"
        );
        let payload = self.panic_payload.lock().unwrap().take();
        if let Some(payload) = payload {
            std::panic::resume_unwind(payload);
        }
    }
}

impl Drop for JobScheduler {
    fn drop(&mut self) {
        self.stop_source.request_stop();

        // Wake all workers so they see the stop flag immediately
        // rather than waiting out their full park/yield cycle.
        for worker in &self.workers {
            worker.unparker.unpark();
        }

        if let Some(handles) = self.worker_handles.get_mut() {
            for worker in handles.drain(..) {
                worker.join();
            }
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
