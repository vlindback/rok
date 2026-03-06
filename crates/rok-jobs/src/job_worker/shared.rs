// shared.rs

use crossbeam::{
    deque::{Stealer, Worker},
    queue::ArrayQueue,
    sync::Unparker,
};

use crate::{job_node::JobNode, job_node_handle::JobNodeHandle, job_priority::JobPriority};

pub(crate) struct JobWorkerShared {
    pub(crate) stealers: [Stealer<JobNodeHandle>; JobPriority::COUNT],
    pub(crate) inboxes: [ArrayQueue<JobNodeHandle>; JobPriority::COUNT],
    pub(crate) unparker: Unparker,
}

impl JobWorkerShared {
    pub(crate) fn push_to_inbox(&self, mut task: JobNodeHandle, prio: JobPriority) {
        let inbox = unsafe { self.inboxes.get_unchecked(prio.index()) };
        while let Err(returned_task) = inbox.push(task) {
            task = returned_task;
            std::hint::spin_loop();
            std::thread::yield_now();
        }
    }

    pub(crate) fn drain_inbox_into<F>(&self, prio: JobPriority, mut f: F)
    where
        F: FnMut(JobNodeHandle),
    {
        let inbox = unsafe { self.inboxes.get_unchecked(prio.index()) };
        let len = inbox.len();
        for _ in 0..len {
            match inbox.pop() {
                Some(item) => f(item),
                None => break,
            }
        }
    }
}
