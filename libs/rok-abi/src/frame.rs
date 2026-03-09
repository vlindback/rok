// frame.rs
//
// FrameInput bundles everything the Host delivers to the Engine each tick
// into a single borrowed snapshot. This keeps the Engine vtable stable — we
// don't need to add new function signatures every time we want to pass new
// per-frame data.
//
// Ownership: FrameInput is stack-allocated by the Host. The `events` pointer
// is a borrow into the Host's event buffer for THIS CALL ONLY. The Engine
// must not retain the pointer past the return of `EngineVTable::update`.

use crate::input::RawInputEvent;

#[repr(C)]
pub struct LifecycleFlags {
    pub should_quit: u8,     // OS close button / SIGTERM
    pub surface_changed: u8, // resize or recreate
    pub surface_width: u32,
    pub surface_height: u32,
    pub surface_valid: u8, // Android: 0 when app is paused/backgrounded
    pub _pad: [u8; 1],
}

/// Per-frame data bundle. Created by the Host, borrowed by the Engine.
#[repr(C)]
pub struct FrameInput {
    // --- Timing ---
    /// Seconds elapsed since the previous frame. Clamped to a sane max
    /// by the Host (e.g. 0.1s) so the Engine never receives a huge spike
    /// after a hitch or debugger pause.
    pub delta_time: f32,

    /// Monotonic timestamp of this frame in nanoseconds, from an arbitrary
    /// epoch. Useful for animation, shader uniforms, and profiling.
    pub timestamp_ns: u64,

    // --- Input ---
    /// Pointer to the event array collected this frame.
    /// May be null when event_count == 0. The slice is valid for the
    /// duration of the `update` call and must not be stored beyond it.
    pub events: *const RawInputEvent,
    pub event_count: usize,
    pub lifecycle: LifecycleFlags,
}
