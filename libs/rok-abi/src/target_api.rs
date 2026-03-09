// target_api.rs
//
// The Target is the game / application DLL that the Engine loads.
// It is separate from the Engine so it can be hot-reloaded independently.
//
// The Engine calls TargetVTable functions. The Target calls back into the
// Engine via the EngineApi it received at init.
//
// Ownership:
//   - TargetState is allocated by `TargetVTable::init` and freed by
//     `TargetVTable::shutdown`. The Engine holds the pointer between those calls.
//   - The EngineApi pointer passed to `init` is borrowed; the Target must not
//     free it. It is valid from `init` until `shutdown` returns.

use crate::engine_api::EngineApi;

// ---------------------------------------------------------------------------
// Opaque target state
// ---------------------------------------------------------------------------

/// Opaque handle to the Target's internal state.
/// Allocated by `TargetVTable::init`, freed by `TargetVTable::shutdown`.
#[repr(C)]
pub struct TargetState {
    _private: [u8; 0],
}

// ---------------------------------------------------------------------------
// Hot-reload serialisation buffer
// ---------------------------------------------------------------------------

/// A flat byte buffer used to carry Target state across a hot-reload.
///
/// Before unloading the old DLL the Engine calls `TargetVTable::save_state`,
/// which writes whatever the Target needs to survive a reload into this buffer.
/// After loading the new DLL the Engine calls `TargetVTable::load_state` with
/// the same buffer so the new Target can restore itself.
///
/// Format is entirely up to the Target — treat it as an opaque blob.
/// The Engine allocates `buf` on the heap, hands ownership to save_state,
/// and frees it after load_state returns (or on shutdown if reload failed).
///
/// `capacity` is the buffer size; `len` is the number of bytes written.
#[repr(C)]
pub struct HotReloadBuffer {
    pub buf: *mut u8,
    pub len: usize,
    pub capacity: usize,
}

// ---------------------------------------------------------------------------
// TargetVTable — Engine → Target
// ---------------------------------------------------------------------------

/// Symbol name exported by the Target DLL.
pub const TARGET_ENTRY_SYMBOL: &[u8] = b"rok_target_vtable_get\0";

/// Type of the Target DLL's entry point.
pub type TargetVTableGetter = extern "C" fn() -> TargetVTable;

/// The interface the Target exposes to the Engine.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct TargetVTable {
    /// Initialise the Target.
    ///
    /// - `api`         : borrowed engine services. Valid until `shutdown` returns.
    /// - `hot_reload`  : if non-null, a buffer from a previous `save_state` call.
    ///                   The Target should deserialise its state from it.
    ///                   The Engine frees the buffer after this call returns.
    ///
    /// Returns null on failure.
    pub init: extern "C" fn(
        api: *const EngineApi,
        hot_reload: *const HotReloadBuffer, // null on first load
    ) -> *mut TargetState,

    /// Shut down the Target and free `state`.
    /// `hot_reload` is non-null when this is a hot-reload (not final shutdown);
    /// the Target should serialise surviving state into it.
    /// The buffer is pre-allocated by the Engine; set `len` to bytes written.
    pub shutdown: extern "C" fn(
        state: *mut TargetState,
        hot_reload: *mut HotReloadBuffer, // null on final shutdown
    ),

    /// Advance the Target's simulation by one frame.
    /// Called by the Engine after it has run its own pre-update pass.
    pub update: extern "C" fn(state: *mut TargetState, delta_time: f32),

    // -------------------------------------------------------------------------
    // OPTIONAL callbacks (set to null if not needed)
    // -------------------------------------------------------------------------
    /// Called once the Engine's renderer is ready for the frame.
    /// The Target may submit draw calls, update GPU buffers, etc.
    /// OPTIONAL — null if not needed.
    pub render: Option<extern "C" fn(state: *mut TargetState)>,

    /// Called when the surface dimensions change (after the Engine has
    /// rebuilt its swapchain). Useful for resizing render targets.
    /// OPTIONAL — null if not needed.
    pub on_resize: Option<extern "C" fn(state: *mut TargetState, width: u32, height: u32)>,
}
