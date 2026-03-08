// local.rs

use std::{mem::MaybeUninit, sync::Arc};

use crossbeam::{deque::Worker, sync::Parker, utils::CachePadded};

use crate::{
    job_pool::JobIndex,
    job_priority::JobPriority,
    job_scheduler::JobScheduler,
    job_worker::{JobWorkerInit, JobWorkerShared, send_ptr::SendPtr},
    stop::StopToken,
};

const MAX_STEAL_BATCH: usize = 8; // todo: we can tune this.

pub(crate) struct JobWorkerLocal {
    pub(crate) id: usize,
    pub(crate) stop_token: StopToken,
    pub(crate) scheduler: Arc<JobScheduler>,
    pub(super) shared: SendPtr<CachePadded<JobWorkerShared>>,
    pub(crate) parker: Parker,
    pub(crate) loot: [MaybeUninit<(JobIndex, JobPriority)>; MAX_STEAL_BATCH],
    deques: [Worker<JobIndex>; JobPriority::COUNT],
}

impl JobWorkerLocal {
    pub(super) fn new(
        id: usize,
        stop_token: StopToken,
        scheduler: Arc<JobScheduler>,
        shared: SendPtr<CachePadded<JobWorkerShared>>,
        init: JobWorkerInit,
    ) -> Self {
        Self {
            id,
            stop_token,
            scheduler,
            shared,
            deques: init.deques,
            parker: init.parker,
            loot: unsafe { MaybeUninit::uninit().assume_init() },
        }
    }

    pub(crate) fn shared(&self) -> &JobWorkerShared {
        // SAFETY: shared is guaranteed non-null and valid for the lifetime of
        // this JobWorkerLocal, as the scheduler owns the Vec<JobWorkerShared>
        // and outlives all workers via Arc<JobScheduler>.
        unsafe { &*self.shared.0 }
    }

    pub(crate) fn push_work(&self, index: JobIndex, prio: JobPriority) {
        let deque = unsafe { self.deques.get_unchecked(prio.index()) };
        deque.push(index);
    }

    pub(crate) fn pop_local(&self) -> Option<JobIndex> {
        for prio in JobPriority::ALL {
            if let Some(index) = self.deques[prio.index()].pop() {
                return Some(index);
            }
        }
        None
    }

    pub(crate) fn drain_shared_inbox(&self, prio: JobPriority) -> bool {
        let deque = unsafe { self.deques.get_unchecked(prio.index()) };
        self.shared().drain_inbox_into(prio, |item| {
            deque.push(item);
        })
    }

    #[inline]
    pub(crate) fn has_work_anywhere(&self) -> bool {
        !self.local_deques_empty() || !self.inbox_empty()
    }

    fn local_deques_empty(&self) -> bool {
        self.deques.iter().all(|d| d.is_empty())
    }

    fn inbox_empty(&self) -> bool {
        let shared = self.shared();
        JobPriority::ALL
            .iter()
            .all(|p| shared.inboxes[p.index()].is_empty())
    }
}
