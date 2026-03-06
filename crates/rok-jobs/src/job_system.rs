// job_system.rs

// Central API for the Job System.

use std::sync::Arc;

use crate::{
    job_scheduler::{JobScheduler, JobSchedulerConfig},
    job_worker::JobWorkerHandle,
    stop::StopSource,
};

pub struct JobSystem {
    stop_source: StopSource,
    scheduler: Arc<JobScheduler>,
    workers: Vec<JobWorkerHandle>,
}

impl JobSystem {
    pub fn with_config(scheduler_config: JobSchedulerConfig) -> Self {
        let stop_source = StopSource::new();
        let (scheduler, worker_inits) = JobScheduler::with_config(scheduler_config);

        let workers = worker_inits
            .into_iter()
            .enumerate()
            .map(|(id, init)| {
                JobWorkerHandle::spawn(id, stop_source.token(), scheduler.clone(), init)
            })
            .collect();

        Self {
            stop_source,
            scheduler,
            workers,
        }
    }

    fn stop(&mut self) {
        self.stop_source.request_stop();
        // TODO: Notify all workers, wake everything to quit cleanly.

        for worker in self.workers.drain(..) {
            worker.join();
        }
    }
}

impl Drop for JobSystem {
    fn drop(&mut self) {
        self.stop();
    }
}
