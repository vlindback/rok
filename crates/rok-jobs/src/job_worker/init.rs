// init.rs

use crossbeam::{deque::Worker, sync::Parker};

use crate::{job_node_handle::JobNodeHandle, job_priority::JobPriority};

pub(crate) struct JobWorkerInit {
    pub(crate) deques: [Worker<JobNodeHandle>; JobPriority::COUNT],
    pub(crate) parker: Parker,
}

impl JobWorkerInit {
    pub(crate) fn new(deques: [Worker<JobNodeHandle>; JobPriority::COUNT], parker: Parker) -> Self {
        Self { deques, parker }
    }
}
