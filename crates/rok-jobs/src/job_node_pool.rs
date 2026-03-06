// job_node_pool.rs

use crossbeam::queue::ArrayQueue;
use std::{cell::UnsafeCell, num::NonZeroU32};

use crate::{job::JobHeader, job_node::JobNode, job_node_handle::JobNodeHandle};

/// Internal container for a job and its metadata.
///
/// By wrapping the [`JobNode`] in a slot with a generation counter, we can detect
/// "stale" handles. This prevents the ABA problem where an index is reused but
/// a thread still holds a handle to the old data.
struct JobNodeSlot {
    pub(crate) node: JobNode,
    pub(crate) generation: u32,
}

/// A generational arena for managing job lifetimes without global locks.
///
/// The pool uses a pre-allocated block of memory and a lock-free queue to manage
/// free indices. This ensures that job allocation and deallocation are wait-free
/// operations, critical for high-performance task systems.
pub(crate) struct JobNodePool {
    /// Fixed-size storage for all potential jobs in the system.
    slots: Box<[UnsafeCell<JobNodeSlot>]>,

    /// Lock-free queue of indices currently available for allocation.
    free: ArrayQueue<u32>,
}

unsafe impl Send for JobNodePool {}
unsafe impl Sync for JobNodePool {}

impl JobNodePool {
    /// Creates a new pool with a fixed maximum number of concurrent jobs.
    ///
    /// # Arguments
    /// * `cap` - The maximum number of jobs the pool can hold. This memory is
    ///   allocated upfront and never grows.
    pub(crate) fn with_capacity(cap: NonZeroU32) -> Self {
        let slots: Box<[UnsafeCell<JobNodeSlot>]> = (0..cap.get())
            .map(|_| {
                UnsafeCell::new(JobNodeSlot {
                    node: JobNode::new(),
                    generation: 0,
                })
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        let queue_size = cap.get() as usize;
        let free = ArrayQueue::new(queue_size);
        for i in 0..queue_size {
            free.push(i as u32).ok().unwrap();
        }

        Self { slots, free }
    }

    /// Attempts to allocate a new job node from the pool.
    ///
    /// This operation is wait-free. It increments the slot's generation and
    /// resets the node's internal state to ensure a clean slate for the new job.
    ///
    /// Returns `None` if the pool is exhausted.
    pub(crate) fn try_allocate_node(&self) -> Option<JobNodeHandle> {
        let slot_index = self.free.pop()?;
        let slot = unsafe { &mut *self.slots[slot_index as usize].get() };

        slot.generation = slot.generation.wrapping_add(1);
        slot.node.reset();

        Some(JobNodeHandle::new(slot_index, slot.generation))
    }

    /// Returns a node index to the pool for future reuse.
    ///
    /// Note: This does not reset the node memory. Resetting is deferred until
    /// [`try_allocate_node`] to improve cache locality during job setup.
    pub(crate) fn free_node(&self, handle: JobNodeHandle) {
        let slot_index = handle.index() as usize;
        debug_assert!(slot_index < self.slots.len(), "Job index out of bounds");
        let result = self.free.push(slot_index as u32);

        // This should not be possible, but its good hygiene to check for if our internal logic is broken.
        debug_assert!(
            result.is_ok(),
            "JobPool: Failed to push index {} back to free queue. Double free detected?",
            slot_index
        );
    }

    /// Safely retrieves a reference to a job node if the handle is still valid.
    ///
    /// Returns `None` if the handle's generation does not match the current
    /// generation of the slot (i.e., the job has been freed and potentially reused).
    pub(crate) fn get_node(&self, handle: &JobNodeHandle) -> Option<&JobNode> {
        let slot_index = handle.index() as usize;
        if slot_index >= self.slots.len() {
            debug_assert!(false, "JobNodeHandle index out of bounds: {}", slot_index);
            return None;
        }
        unsafe {
            let slot = &*self.slots[slot_index].get();
            if slot.generation == handle.generation() {
                Some(&slot.node)
            } else {
                None
            }
        }
    }

    /// Retrieves a reference to a job node without performing generation checks.
    ///
    /// # Safety
    /// The caller must guarantee that the `handle` is valid and the job has
    /// not been freed. This is typically used in internal hot-paths where
    /// the graph structure guarantees existence.
    pub(crate) unsafe fn get_node_unchecked(&self, handle: &JobNodeHandle) -> &JobNode {
        let slot_index = handle.index() as usize;

        debug_assert!(
            slot_index < self.slots.len(),
            "Index out of bounds in get_node_unchecked!"
        );

        let slot = unsafe { &mut *self.slots[slot_index].get() };

        debug_assert!(
            slot.generation == handle.generation(),
            "Generation mismatch in get_node_unchecked!"
        );

        &slot.node
    }

    /// Returns the number of nodes in the pool.
    pub(crate) fn node_capacity(&self) -> usize {
        self.slots.len()
    }
}
