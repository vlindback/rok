// handle.rs

use std::{sync::Arc, thread::JoinHandle};

use crossbeam::utils::CachePadded;

use crate::{
    job_scheduler::JobScheduler,
    job_worker::{
        JobWorkerInit, JobWorkerLocal, JobWorkerShared, send_ptr::SendPtr,
        worker_loop::job_worker_loop,
    },
    stop::StopToken,
};

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
        shared: *const CachePadded<JobWorkerShared>,
    ) -> Self {
        let shared = SendPtr(shared);
        let join_handle = std::thread::Builder::new()
            .name(format!("rok-worker-{}", id))
            .spawn(move || {
                let mut local = JobWorkerLocal::new(id, stop_token, scheduler, shared, init);
                job_worker_loop(&mut local);
            })
            .expect("rok-jobs: failed to spawn worker thread");

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
