// frame.rs

pub struct LifecycleFlags {
    pub surface_width: u32,
    pub surface_height: u32,
    pub should_quit: bool,     // OS close button / SIGTERM
    pub surface_valid: bool,   // Android: 0 when app is paused/backgrounded
    pub surface_changed: bool, // resize or recreate
}

/// Per-frame data bundle.
pub struct FrameInput {
    /// Seconds elapsed since the previous frame.
    pub delta_time: f32,

    /// Monotonic timestamp of this frame in nanoseconds, from an arbitrary
    /// epoch. Useful for animation, shader uniforms, and profiling.
    pub timestamp_ns: u64,

    /// Lifetime flags (state changes)
    pub lifecycle: LifecycleFlags,
}
