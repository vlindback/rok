// job_scheduler.rs

use std::any::Any;
use std::f32::MIN;
use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Arc, Mutex};

use crossbeam::deque::{Stealer, Worker};
use crossbeam::queue::ArrayQueue;

use crate::JobFence;
use crate::job::Job;
use crate::job_node_graph::JobNodeGraph;
use crate::job_node_handle::JobNodeHandle;
use crate::job_priority::JobPriority;
use crate::job_record::JobRecord;
use crate::job_worker::{JobWorkerInit, JobWorkerShared};

pub(crate) struct JobScheduler {
    graph: JobNodeGraph,
    workers: Box<[JobWorkerShared]>,
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
    pub(crate) fn with_config(config: JobSchedulerConfig) -> (Arc<Self>, Vec<JobWorkerInit>) {
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

        // Create the worker states

        let mut shared = Vec::with_capacity(worker_count);
        let mut inits = Vec::with_capacity(worker_count);

        for _ in 0..worker_count {
            let deques: [Worker<JobNodeHandle>; JobPriority::COUNT] =
                std::array::from_fn(|_| Worker::new_lifo());

            let stealers: [Stealer<JobNodeHandle>; JobPriority::COUNT] =
                std::array::from_fn(|i| deques[i].stealer());

            let inboxes: [ArrayQueue<JobNodeHandle>; JobPriority::COUNT] =
                std::array::from_fn(|_| ArrayQueue::new(worker_inbox_cap));

            shared.push(JobWorkerShared { stealers, inboxes });
            inits.push(JobWorkerInit { deques });
        }

        let scheduler = Arc::new(JobScheduler {
            graph: JobNodeGraph::with_capacity(graph_node_capacity),
            workers: shared.into_boxed_slice(),
            rr_counter: AtomicUsize::new(0),
            poisoned: AtomicBool::new(false),
            panic_payload: Mutex::new(None),
        });

        (scheduler, inits)
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

        if is_worker_thread() {
        } else {
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

    // SAFETY:
    // 1. If it came from worker_count, it was already a NonZeroU32.
    // 2. If it came from available_parallelism, we used .max(1).
    // 3. If it fell back to the constant, 4 is > 0.
    unsafe { NonZeroUsize::new_unchecked(count) }
}
