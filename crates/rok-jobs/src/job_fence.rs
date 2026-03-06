// job_fence.rs

use std::sync::{Condvar, Mutex, atomic::AtomicI32};

pub struct JobFence {
    count: AtomicI32,
    lock: Mutex<()>,
    cv: Condvar,
}

impl JobFence {
    fn new() -> Self {
        Self {
            count: AtomicI32::new(0),
            lock: Mutex::new(()),
            cv: Condvar::new(),
        }
    }

    fn wait(&self) {}
}
