// job_system.rs

// Central API for the Job System.

use std::sync::Arc;

use crate::job_scheduler::{JobScheduler, JobSchedulerConfig};

pub struct JobSystem {
    scheduler: Arc<JobScheduler>,
}

impl JobSystem {
    pub fn with_config(scheduler_config: JobSchedulerConfig) -> Self {
        let scheduler = JobScheduler::with_config(scheduler_config);

        Self { scheduler }
    }
}
