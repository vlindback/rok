// job_node_graph.rs

use std::num::NonZeroU32;

use crate::{
    JobFence, job::Job, job_node_handle::JobNodeHandle, job_node_pool::JobNodePool,
    job_priority::JobPriority, job_record::JobRecord,
};

pub(crate) struct JobNodeGraph {
    pool: JobNodePool,
}

impl JobNodeGraph {
    pub(crate) fn with_capacity(node_pool_cap: NonZeroU32) -> Self {
        Self {
            pool: JobNodePool::with_capacity(node_pool_cap),
        }
    }

    pub(crate) fn allocate_node(&self) -> JobNodeHandle {
        // TODO: add exhaustion policy (spin, panic, etc)
        loop {
            if let Some(node) = self.pool.try_allocate_node() {
                return node;
            }
            std::hint::spin_loop();
            std::thread::yield_now();
        }
    }

    pub(crate) fn arm_node(
        &self,
        node_handle: &JobNodeHandle,
        prio: JobPriority,
        deps: u32,
        fence: Option<&JobFence>,
        record: Option<&JobRecord>,
        job: Job,
    ) {
        debug_assert!(
            (node_handle.index() as usize) < self.pool.node_capacity(),
            "JobNodeHandle index out of bounds!"
        );
        // Safety: any unsafety here is a logic error on our part so debug_assert makes sense.
        let node = unsafe { self.pool.get_node_unchecked(node_handle) };
        node.arm(prio, deps, fence, record, job);
    }

    pub(crate) fn execute_node(&self, handle: JobNodeHandle) {
        if let Some(node) = self.pool.get_node(&handle) {
            node.run_job();
        }
    }
}
