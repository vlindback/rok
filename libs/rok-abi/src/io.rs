// rok-abi/src/io.rs

/// Index into the host's registered file handle pool.
pub type HandleIndex = u32;

/// Index into the host's registered staging buffer pool.
pub type BufferIndex = u32;

/// A pending read request submitted to the IO ring.
///
/// Both handle_index and buffer_index must refer to slots that were
/// registered with the ring via IoRing::register_handles and
/// IoRing::register_buffers before submission.
#[repr(C)]
pub struct IoRequest {
    pub handle_index: HandleIndex,
    pub buffer_index: BufferIndex,
    pub offset: u64,
    pub size: u32,
    /// Opaque token - returned unchanged in IoCompletion.
    /// The engine puts whatever it needs here to identify the asset on completion.
    /// Typically a pointer to the AssetSlot or an asset ID.
    pub user_data: u64,
}

/// Result of a completed IO operation harvested from the completion queue.
#[repr(C)]
pub struct IoCompletion {
    /// The user_data token from the originating IoRequest.
    pub user_data: u64,
    /// Bytes transferred on success. Negative values are platform error codes.
    pub result: i32,
}
