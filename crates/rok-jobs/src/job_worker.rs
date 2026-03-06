// job_worker.rs

use std::sync::Arc;
use std::thread::JoinHandle;

use crate::job_node_handle::JobNodeHandle;
use crate::job_priority::JobPriority;
use crate::job_scheduler::JobScheduler;
use crate::stop::{self, StopToken};

use crossbeam::deque::Steal;
use crossbeam::deque::{Stealer, Worker};
use crossbeam::queue::ArrayQueue;

// State local to thread worker
pub(crate) struct JobWorkerLocal {
    pub(crate) id: usize,
    pub(crate) stop_token: StopToken,
    pub(crate) scheduler: Arc<JobScheduler>,
    deques: [Worker<JobNodeHandle>; JobPriority::COUNT],
}

impl JobWorkerLocal {
    fn new(
        id: usize,
        stop_token: StopToken,
        scheduler: Arc<JobScheduler>,
        init: JobWorkerInit,
    ) -> Self {
        Self {
            id,
            stop_token,
            scheduler,
            deques: init.deques,
        }
    }
}

// State shared between spawner and worker thread
pub(crate) struct JobWorkerShared {
    pub(crate) stealers: [Stealer<JobNodeHandle>; JobPriority::COUNT],
    pub(crate) inboxes: [ArrayQueue<JobNodeHandle>; JobPriority::COUNT],
}

// State given to a worker when created.
// JobWorkerLocal is derived from this and takes over the state.
pub(crate) struct JobWorkerInit {
    pub(crate) deques: [Worker<JobNodeHandle>; JobPriority::COUNT],
}

// Worker Handle (The actual thread)
pub(crate) struct JobWorkerHandle {
    join_handle: Option<JoinHandle<()>>,
}

impl JobWorkerHandle {
    pub(crate) fn spawn(
        id: usize,
        stop_token: StopToken,
        scheduler: Arc<JobScheduler>,
        init: JobWorkerInit,
    ) -> Self {
        let join_handle = std::thread::spawn(move || {
            let mut local = JobWorkerLocal::new(id, stop_token, scheduler, init);
            job_worker_loop(&mut local);
        });

        Self {
            join_handle: Some(join_handle),
        }
    }

    pub(crate) fn join(mut self) {
        if let Some(h) = self.join_handle.take() {
            let _ = h.join();
        }
    }
}

// Worker loop
pub(crate) fn job_worker_loop(local: &mut JobWorkerLocal) {
    while !local.stop_token.is_stop_requested() {

        // We need a tiered backoff strategy here.
        //
        // 1. [Spin]    : core::hint::spin_loop()
        // 2. [Yield]   : std::thread::yield_now()
        // 3. [Park]    : std::thread::park()
        //
        //      Then if the simulation "resumes",
        //      it should call a wake all function.
        //
        //      NOTE: we can use an atomic counter for the
        //      number of parked threads and if parked_threads > 0
        //      we call unpark()
    }
}
