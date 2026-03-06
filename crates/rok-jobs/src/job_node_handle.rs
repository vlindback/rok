// job_node_handle.rs

//! # Job Node Handles
//!
//! This module provides a packed 64-bit handle used for addressing jobs
//! within the `rok-jobs` system.
//!
//! By combining the index and generation into a single `u64`, we can perform
//! atomic swaps on the entire handle at once, which is critical for the
//! lock-free continuation list implementation.

/// A 64-bit handle for addressing jobs in the pool with ABA protection.
///
/// This handle is designed to be stored in an [`std::sync::atomic::AtomicU64`]
/// to allow for lock-free dependency management.
///
/// ### Bit Layout
/// | Bits  | Field      | Description |
/// | :---  | :---       | :---        |
/// | 0-31  | **Index** | The raw index into the `JobPool` array. |
/// | 32-63 | **Gen** | The generation counter to prevent ABA reuse bugs. |
pub(crate) struct JobNodeHandle(u64);

impl JobNodeHandle {
    /// Raw sentinel value
    pub const INVALID_BITS: u64 = u64::MAX;

    /// A sentinel value representing an uninitialized or null handle.
    pub const INVALID: Self = Self(u64::MAX);

    /// Creates a new [`JobNodeHandle`] from an index and generation.
    ///
    /// # Arguments
    ///
    /// * `index` - The position in the global `JobPool`.
    /// * `generation` - The ABA-protection counter for this slot.
    ///
    /// See also: [`JobNodeHandle::is_valid`]
    pub fn new(index: u32, generation: u32) -> Self {
        // Pack: [ Generation (32 bits) | Index (32 bits) ]
        Self(((generation as u64) << 32) | (index as u64))
    }

    /// Extracts the 32-bit array index.
    #[inline]
    pub fn index(&self) -> u32 {
        (self.0 & 0xFFFFFFFF) as u32
    }

    /// Extracts the 32-bit generation counter.
    #[inline]
    pub fn generation(&self) -> u32 {
        (self.0 >> 32) as u32
    }

    /// Returns `true` if this handle is the [`Self::INVALID`] sentinel.
    #[inline]
    pub fn is_invalid(&self) -> bool {
        self.0 == u64::MAX
    }

    /// Returns `true` if this handle points to a potentially valid slot.
    #[inline]
    pub fn is_valid(self) -> bool {
        !self.is_invalid()
    }

    #[inline]
    pub fn from_u64(val: u64) -> Self {
        Self(val)
    }

    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}
