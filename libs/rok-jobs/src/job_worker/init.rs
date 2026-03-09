// init.rs

use crossbeam::{deque::Worker, sync::Parker};

use crate::job_pool::JobIndex;
use crate::job_priority::JobPriority;

pub(crate) struct JobWorkerInit {
    pub(crate) deques: [Worker<JobIndex>; JobPriority::COUNT],
    pub(crate) parker: Parker,
}
