// job_record.rs

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Telemetry data for a specific type of job.
///
/// This is used to track performance trends and identify
/// bottlenecks in the task graph.
pub struct JobRecord {
    /// Accumulated nanoseconds across all executions.
    total_nanos: AtomicU64,
    /// Total number of times this job has successfully completed.
    execution_count: AtomicU32,
}

impl JobRecord {
    /// Creates a new [`JobRecord`] with time and count set to zero.
    #[inline]
    pub fn new() -> Self {
        JobRecord {
            total_nanos: AtomicU64::new(0),
            execution_count: AtomicU32::new(0),
        }
    }

    /// Adds a new timing sample to the record.
    #[inline]
    pub fn record_execution(&self, nanoseconds: u64) {
        self.total_nanos.fetch_add(nanoseconds, Ordering::Relaxed);
        self.execution_count.fetch_add(1, Ordering::Relaxed);
    }
}
