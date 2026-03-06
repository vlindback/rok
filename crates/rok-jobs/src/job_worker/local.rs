// local.rs

use std::sync::Arc;

use crossbeam::{deque::Worker, sync::Parker};

use crate::{
    job_node_handle::JobNodeHandle,
    job_priority::JobPriority,
    job_scheduler::JobScheduler,
    job_worker::{JobWorkerInit, JobWorkerShared, send_ptr::SendPtr},
    stop::StopToken,
};

pub(crate) struct JobWorkerLocal {
    pub(crate) id: usize,
    pub(crate) stop_token: StopToken,
    pub(crate) scheduler: Arc<JobScheduler>,
    pub(crate) shared: SendPtr<JobWorkerShared>,
    pub(crate) parker: Parker,
    deques: [Worker<JobNodeHandle>; JobPriority::COUNT],
}

impl JobWorkerLocal {
    pub(crate) fn new(
        id: usize,
        stop_token: StopToken,
        scheduler: Arc<JobScheduler>,
        shared: SendPtr<JobWorkerShared>,
        init: JobWorkerInit,
    ) -> Self {
        Self {
            id,
            stop_token,
            scheduler,
            shared,
            deques: init.deques,
            parker: init.parker,
        }
    }

    pub(crate) fn shared(&self) -> &JobWorkerShared {
        // SAFETY: shared is guaranteed non-null and valid for the lifetime of
        // this JobWorkerLocal, as the scheduler owns the Box<[JobWorkerShared]>
        // and outlives all workers via Arc<JobScheduler>.
        unsafe { &*self.shared.0 }
    }

    pub(crate) fn push_work(&self, task: JobNodeHandle, prio: JobPriority) {
        let deque = unsafe { self.deques.get_unchecked(prio.index()) };
        deque.push(task);
    }

    pub(crate) fn pop_local(&self) -> Option<JobNodeHandle> {
        for prio in JobPriority::ALL {
            let i = prio.index();
            let deque = &self.deques[i];
            if let Some(handle) = deque.pop() {
                return Some(handle);
            }
        }
        None
    }

    pub(crate) fn drain_shared_inbox(&self, prio: JobPriority) {
        let deque = unsafe { self.deques.get_unchecked(prio.index()) };
        self.shared().drain_inbox_into(prio, |item| {
            deque.push(item);
        });
    }

    #[inline]
    pub fn has_work_anywhere(&self) -> bool {
        // Check local deque, inbox, and maybe do a quick peek at stealers
        !self.local_deque_is_empty() || !self.inbox_is_empty()
    }
}
