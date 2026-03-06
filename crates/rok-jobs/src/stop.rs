// stop.rs

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub struct StopSource {
    flag: Arc<AtomicBool>,
}

impl StopSource {
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn token(&self) -> StopToken {
        StopToken {
            flag: Arc::clone(&self.flag),
        }
    }

    pub fn request_stop(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }
}

#[derive(Clone)]
pub struct StopToken {
    flag: Arc<AtomicBool>,
}

impl StopToken {
    pub fn is_stop_requested(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }
}
