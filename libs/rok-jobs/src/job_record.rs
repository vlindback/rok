// job_record.rs

use std::{
    num::NonZeroUsize,
    sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering},
};

pub struct RingBuffer {
    samples: Box<[AtomicU64]>,
    head: AtomicUsize,
}

impl RingBuffer {
    pub(crate) fn with_capacity(capacity: NonZeroUsize) -> Self {
        let samples: Vec<AtomicU64> = (0..capacity.get()).map(|_| AtomicU64::new(0)).collect();
        Self {
            samples: samples.into_boxed_slice(),
            head: AtomicUsize::new(0),
        }
    }

    pub fn capacity(&self) -> NonZeroUsize {
        // Safety: with_capacity guarantees non zero.
        unsafe { NonZeroUsize::new_unchecked(self.samples.len()) }
    }

    /// Push a new sample, overwriting the oldest if full.
    pub(crate) fn push(&self, nanos: u64) {
        let slot = self.head.fetch_add(1, Ordering::Relaxed) % self.samples.len();
        self.samples[slot].store(nanos, Ordering::Relaxed);
    }

    /// Rolling average over all slots in the buffer.
    ///
    /// Note: slots not yet written contain 0, which will pull the average
    /// down until the buffer is fully populated. This is intentional —
    /// it accurately reflects that we don't have N samples yet.
    pub fn average(&self) -> u64 {
        let sum: u64 = self.samples.iter().map(|s| s.load(Ordering::Relaxed)).sum();
        sum / self.samples.len() as u64
    }

    /// Rolling minimum over all slots in the buffer.
    ///
    /// Note: unwritten slots contain 0, so this will return 0 until
    /// the buffer is fully populated. Check `is_full()` if this matters.
    pub fn min(&self) -> u64 {
        self.samples
            .iter()
            .map(|s| s.load(Ordering::Relaxed))
            .min()
            .unwrap_or(0)
    }

    /// Rolling maximum over all slots in the buffer.
    pub fn max(&self) -> u64 {
        self.samples
            .iter()
            .map(|s| s.load(Ordering::Relaxed))
            .max()
            .unwrap_or(0)
    }

    /// Returns true once the buffer has been fully written at least once.
    pub fn is_full(&self) -> bool {
        self.head.load(Ordering::Relaxed) >= self.samples.len()
    }

    /// Iterates samples in chronological order, oldest first.
    ///
    /// The returned iterator is a snapshot - concurrent writes may cause
    /// individual samples to update mid-iteration, which is acceptable
    /// for a profiling display.
    pub fn samples_ordered(&self) -> impl Iterator<Item = u64> + '_ {
        let head = self.head.load(Ordering::Relaxed);
        let len = self.samples.len();
        // Start from the slot after the most recently written one
        // that is the oldest sample in a full buffer.
        (0..len).map(move |i| {
            let slot = (head + i) % len;
            self.samples[slot].load(Ordering::Relaxed)
        })
    }
}

/// Telemetry data for a specific type of job.
///
/// This is used to track performance trends and identify
/// bottlenecks in the task graph.
pub struct JobRecord {
    name: &'static str,
    lifetime_nanos: AtomicU64,
    lifetime_nanos_min: AtomicU64,
    lifetime_nanos_max: AtomicU64,

    /// Total number of times this job has successfully completed.
    lifetime_count: AtomicU32,

    history: Option<RingBuffer>,
}

impl JobRecord {
    /// Creates a new [`JobRecord`] with time and count set to zero.
    #[inline]
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            lifetime_nanos: AtomicU64::new(0),
            lifetime_nanos_min: AtomicU64::new(u64::MAX),
            lifetime_nanos_max: AtomicU64::new(0),
            lifetime_count: AtomicU32::new(0),
            history: None,
        }
    }

    pub fn with_history(name: &'static str, capacity: NonZeroUsize) -> Self {
        Self {
            history: Some(RingBuffer::with_capacity(capacity)),
            ..Self::new(name)
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Adds a new timing sample to the record.
    #[inline]
    pub fn record(&self, nanoseconds: u64) {
        self.lifetime_nanos
            .fetch_add(nanoseconds, Ordering::Relaxed);
        self.lifetime_count.fetch_add(1, Ordering::Relaxed);

        self.lifetime_nanos_max
            .fetch_max(nanoseconds, Ordering::Relaxed);
        self.lifetime_nanos_min
            .fetch_min(nanoseconds, Ordering::Relaxed);

        if let Some(history) = &self.history {
            history.push(nanoseconds);
        }
    }

    // --- Lifetime stats ---

    pub fn lifetime_average(&self) -> u64 {
        let count = self.lifetime_count.load(Ordering::Relaxed) as u64;
        if count == 0 {
            return 0;
        }
        self.lifetime_nanos.load(Ordering::Relaxed) / count
    }

    pub fn lifetime_min(&self) -> u64 {
        let min = self.lifetime_nanos_min.load(Ordering::Relaxed);
        // Return 0 if no samples recorded yet rather than u64::MAX
        if min == u64::MAX { 0 } else { min }
    }

    pub fn lifetime_max(&self) -> u64 {
        self.lifetime_nanos_max.load(Ordering::Relaxed)
    }

    pub fn lifetime_count(&self) -> u32 {
        self.lifetime_count.load(Ordering::Relaxed)
    }

    // --- Rolling stats (delegate to history) ---

    pub fn rolling_average(&self) -> Option<u64> {
        self.history.as_ref().map(|h| h.average())
    }
    pub fn rolling_min(&self) -> Option<u64> {
        self.history.as_ref().map(|h| h.min())
    }
    pub fn rolling_max(&self) -> Option<u64> {
        self.history.as_ref().map(|h| h.max())
    }
    pub fn history(&self) -> Option<&RingBuffer> {
        self.history.as_ref()
    }
}
