// job_scheduler.rs

use std::any::Any;
use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crossbeam::deque::{Stealer, Worker};
use crossbeam::queue::ArrayQueue;
use crossbeam::sync::Parker;

use crate::JobFence;
use crate::job::Job;
use crate::job_node_graph::JobNodeGraph;
use crate::job_node_handle::JobNodeHandle;
use crate::job_priority::JobPriority;
use crate::job_record::JobRecord;
use crate::job_worker::{JobWorkerHandle, JobWorkerInit, JobWorkerShared};
use crate::job_worker_tls::get_job_worker_local_tls;
use crate::stop::StopSource;

pub(crate) struct JobScheduler {
    stop_source: StopSource,
    graph: JobNodeGraph,
    workers: Vec<JobWorkerShared>,
    worker_handles: Vec<JobWorkerHandle>,
    rr_counter: AtomicUsize,
    poisoned: AtomicBool,
    panic_payload: Mutex<Option<Box<dyn Any + Send>>>,
}

#[derive(Default)]
pub(crate) struct JobSchedulerConfig {
    // How many nodes the graph owned by the scheduler have.
    graph_node_capacity: Option<NonZeroU32>,

    // How large each workers inbox is for work.
    worker_inbox_capacity: Option<NonZeroU32>,

    // How many workers exist.
    worker_count: Option<NonZeroU32>,
}

impl JobScheduler {
    pub(crate) fn with_config(config: JobSchedulerConfig) -> Arc<Self> {
        /// Default minimum capacity for the job node graph storage.

        const MIN_GRAPH_NODES: NonZeroU32 = match NonZeroU32::new(128) {
            Some(n) => n,
            None => unreachable!(),
        };

        /// Default minimum capacity for worker job inboxes.

        const MIN_INBOX_CAP: NonZeroU32 = match NonZeroU32::new(32) {
            Some(n) => n,
            None => unreachable!(),
        };

        let graph_node_capacity = config
            .graph_node_capacity
            .unwrap_or(MIN_GRAPH_NODES)
            .max(MIN_GRAPH_NODES);

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

            let deques: [Worker<JobNodeHandle>; JobPriority::COUNT] =
                std::array::from_fn(|_| Worker::new_lifo());

            let stealers: [Stealer<JobNodeHandle>; JobPriority::COUNT] =
                std::array::from_fn(|i| deques[i].stealer());

            let inboxes: [ArrayQueue<JobNodeHandle>; JobPriority::COUNT] =
                std::array::from_fn(|_| ArrayQueue::new(worker_inbox_cap));

            shared_vec.push(JobWorkerShared {
                stealers,
                inboxes,
                unparker,
            });

            worker_inits.push(JobWorkerInit { deques, parker });
        }

        //
        // Pass 2: freeze the shared slice inside the Arc - addresses are now stable
        //

        let mut scheduler = Arc::new(JobScheduler {
            stop_source: StopSource::new(),
            graph: JobNodeGraph::with_capacity(graph_node_capacity),
            workers: shared_vec,
            worker_handles: Vec::new(),
            rr_counter: AtomicUsize::new(0),
            poisoned: AtomicBool::new(false),
            panic_payload: Mutex::new(None),
        });

        //
        // Pass 3: Spawn the workers.
        //

        let workers = worker_inits
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

        // expect() string is technically dead weight in release builds but serves
        // as documentation for future contributors. Remove if binary size becomes
        // a concern.
        Arc::get_mut(&mut scheduler)
            .expect("no external Arc clones exist before workers are published")
            .worker_handles = workers;

        scheduler
    }

    pub(crate) fn schedule<F>(
        &self,
        prio: JobPriority,
        fence: Option<&JobFence>,
        record: Option<&JobRecord>,
        f: F,
    ) where
        F: FnOnce() + Send + 'static,
    {
        let deps: u32 = 0;
        let node_handle: JobNodeHandle = self.graph.allocate_node();
        let job = Job::new(f);
        self.graph
            .arm_node(&node_handle, prio, deps, fence, record, job);

        if let Some(worker_local_ctx) = get_job_worker_local_tls() {
            // Fast path: push directly to local worker.

            // crossbeam::deque::worker is single producer but here we are in the producer thread.
            worker_local_ctx.push_work(node_handle, prio);

            // NOTE: it is crucial here that we wake up someone else than ourselves.
            // if we are the only worker thread and a job spawns 1000 jobs they will
            // all run single-threaded unless we start waking up another thread to
            // help stealing.

            let neighbor_idx = (worker_local_ctx.id + 1) % self.workers.len();
            let neighbor: &JobWorkerShared = unsafe { self.workers.get_unchecked(neighbor_idx) };
            neighbor.unparker.unpark();
        } else {
            // Round-robin pick a worker and add to that workers inbox.

            // The modulo ensures index is always < workers.len()
            let worker_index = self.rr_counter.fetch_add(1, Ordering::Relaxed) % self.workers.len();
            let worker: &JobWorkerShared = unsafe { self.workers.get_unchecked(worker_index) };
            worker.push_to_inbox(node_handle, prio);
            worker.unparker.unpark();
        }
    }

    pub(crate) fn run_job(&self, handle: JobNodeHandle) {
        self.graph.execute_node(handle);
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
    // Minimum 1, perferably optimal - 1, where optimal = available_parallelism.
    // sub for main spawning thread, optimizing for 1 scheduler and 1 main thread.

    let count: usize = config
        .worker_count
        .map(|n| n.get() as usize)
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get().saturating_sub(1).max(1))
                .unwrap_or(4)
        });

    // Safety:
    // 1. If it came from worker_count, it was already a NonZeroU32.
    // 2. If it came from available_parallelism, we used .max(1).
    // 3. If it fell back to the constant, 4 is > 0.
    NonZeroUsize::new(count).unwrap()
}
