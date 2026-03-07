// shared.rs

use crossbeam::{deque::Stealer, queue::ArrayQueue, sync::Unparker};

use crate::job_pool::JobIndex;
use crate::job_priority::JobPriority;

pub(crate) struct JobWorkerShared {
    pub(crate) stealers: [Stealer<JobIndex>; JobPriority::COUNT],
    pub(crate) inboxes: [ArrayQueue<JobIndex>; JobPriority::COUNT],
    pub(crate) unparker: Unparker,
}

impl JobWorkerShared {
    pub(crate) fn push_to_inbox(&self, mut index: JobIndex, prio: JobPriority) {
        let inbox = unsafe { self.inboxes.get_unchecked(prio.index()) };
        while let Err(returned) = inbox.push(index) {
            index = returned;
            std::hint::spin_loop();
            std::thread::yield_now();
        }
    }

    pub(crate) fn drain_inbox_into<F>(&self, prio: JobPriority, mut f: F) -> bool
    where
        F: FnMut(JobIndex),
    {
        // Safety: prio is an enum and this cannot exceed bounds.
        let inbox = unsafe { self.inboxes.get_unchecked(prio.index()) };

        let mut found_any = false;

        while let Some(item) = inbox.pop() {
            f(item);
            found_any = true;
        }

        found_any
    }
}
