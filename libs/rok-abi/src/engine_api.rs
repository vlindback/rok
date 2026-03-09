// engine_api.rs
//
// Two things live here:
//
//   1. EngineVTable  — what the Host calls on the Engine (host → engine).
//   2. EngineApi     — what the Engine exposes to the Target (engine → target).
//      The Target receives this at load time and uses it to spawn jobs,
//      log messages, and access other engine services.
//
// Ownership summary:
//   - EngineState is allocated by Engine::init and freed by Engine::shutdown.
//     The Host holds the pointer between those two calls.
//   - EngineApi is allocated by the Engine and passed (borrowed) to the Target
//     during Target::init. The Target must not store it beyond unload.

use core::ffi::{c_char, c_void};

use crate::frame::FrameInput;
use crate::host_api::{HostState, HostVTable};
use crate::input::{DeviceInfo, DeviceState};
use crate::surface::NativeSurfaceHandle;
use crate::target_api::TargetVTable;

// ---------------------------------------------------------------------------
// Opaque engine state
// ---------------------------------------------------------------------------

/// Opaque handle to the Engine's internal state.
/// Allocated by `EngineVTable::init`, freed by `EngineVTable::shutdown`.
#[repr(C)]
pub struct EngineState {
    _private: [u8; 0],
}

// ---------------------------------------------------------------------------
// EngineVTable — Host → Engine
// ---------------------------------------------------------------------------

/// The complete interface the Engine exposes to the Host.
///
/// The Host obtains this by calling the exported `rok_engine_vtable_get`
/// symbol from the loaded DLL immediately after `dlopen` / `LoadLibrary`.
///
/// Call sequence per run:
/// ```text
///   vtable.init(...)        → EngineState*
///   loop {
///       vtable.update(state, &frame_input)
///       vtable.render(state)
///   }
///   vtable.shutdown(state)
/// ```
///
/// Hot-reload sequence (Target DLL only — Engine DLL stays loaded):
/// ```text
///   vtable.unload_target(state)       ← drain in-flight jobs, save state
///   dlclose(old_target)
///   dlopen(new_target)
///   vtable.load_target(state, vtable) ← restore state, re-register systems
/// ```
#[repr(C)]
pub struct EngineVTable {
    /// Initialise the Engine.
    ///
    /// - `host_state`  : borrowed. Host's opaque context pointer. Stored
    ///                   by Engine for the lifetime of the EngineState.
    /// - `host_vtable` : borrowed. Callbacks into the Host. Same lifetime.
    /// - `surface`     : borrowed for THIS CALL ONLY. Engine uses it to
    ///                   create a VkSurfaceKHR. Does not retain the pointer.
    ///
    /// Returns null on failure. Host must not call any other function if null
    /// is returned.
    pub init: extern "C" fn(
        host_state: *mut HostState,
        host_vtable: *const HostVTable,
        surface: *const NativeSurfaceHandle,
    ) -> *mut EngineState,

    /// Shut down the Engine, free all resources, and invalidate `state`.
    /// Must be the last call made on an EngineState.
    pub shutdown: extern "C" fn(state: *mut EngineState),

    /// Process one frame: drain input, tick all systems, advance the simulation.
    /// `input` is borrowed for the duration of this call only.
    pub update: extern "C" fn(state: *mut EngineState, input: *const FrameInput),

    /// Submit rendering commands for the frame. Must follow `update`.
    pub render: extern "C" fn(state: *mut EngineState),

    /// Notify the Engine that the platform surface was recreated or resized.
    /// The Engine must recreate its swapchain before the next `render` call.
    /// `surface` is borrowed for THIS CALL ONLY.
    pub on_surface_changed:
        extern "C" fn(state: *mut EngineState, surface: *const NativeSurfaceHandle),

    // --- Target (game/app DLL) management ---
    /// Load (or hot-reload) the Target. The Engine drains any in-flight work,
    /// then calls `TargetVTable::init` on the new target.
    ///
    /// `vtable` is borrowed; the Engine does NOT free it. The caller is
    /// responsible for keeping it valid until `unload_target` returns.
    ///
    /// Returns 1 on success, 0 on failure.
    pub load_target: extern "C" fn(state: *mut EngineState, vtable: *const TargetVTable) -> u8,

    /// Unload the current Target. Drains all in-flight jobs that touch
    /// TargetState, then calls `TargetVTable::shutdown`. After this returns
    /// it is safe to `dlclose` the Target DLL.
    pub unload_target: extern "C" fn(state: *mut EngineState),
}

/// Symbol name exported by the Engine DLL.
pub const ENGINE_ENTRY_SYMBOL: &[u8] = b"rok_engine_vtable_get\0";

/// Type of the Engine DLL's entry point.
pub type EngineVTableGetter = extern "C" fn() -> EngineVTable;

// ---------------------------------------------------------------------------
// EngineApi — Engine → Target (services the Engine provides to the Target)
// ---------------------------------------------------------------------------

/// Job priority levels mirroring JobPriority in rok-jobs, but repr(u32)
/// for stable FFI.
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum FfiJobPriority {
    High = 0,
    Normal = 1,
    Low = 2,
}

/// Opaque fence handle. Allocated by `EngineApi::fence_create`,
/// freed by `EngineApi::fence_free`.
#[repr(C)]
pub struct FfiFence {
    _private: [u8; 0],
}

/// Engine services that the Target can call.
///
/// The Engine allocates one of these per Target load and passes it to
/// `TargetVTable::init`. The Target stores the pointer and uses it for
/// the duration of its session. The Engine frees it after `TargetVTable::shutdown`
/// returns.
///
/// All functions are safe to call from any thread unless noted otherwise.
#[repr(C)]
pub struct EngineApi {
    /// Back-pointer to the engine state, so the Target doesn't have to
    /// store it separately. Passed into every callback above.
    pub engine: *mut EngineState,

    // --- Logging ---
    /// Forward a log message through the Engine to the Host's logger.
    /// `msg` is UTF-8, not null-terminated; use `len`.
    pub log: extern "C" fn(
        engine: *mut EngineState,
        level: u32, // cast from host_api::LogLevel
        msg: *const c_char,
        len: usize,
    ),

    // --- Job system ---
    /// Allocate a new fence initialised to zero.
    /// Caller owns the fence; must free it with `fence_free`.
    pub fence_create: extern "C" fn(engine: *mut EngineState) -> *mut FfiFence,

    /// Free a fence. Must not be called while jobs still reference it.
    pub fence_free: extern "C" fn(engine: *mut EngineState, fence: *mut FfiFence),

    /// Submit a job. `userdata` is passed to `f` when the job runs.
    /// If `fence` is non-null the fence's pending count is incremented
    /// before dispatch and decremented on c
    /// ompletion.
    ///
    /// `f` and `userdata` must remain valid until `f` is called.
    /// The caller is responsible for ensuring this (e.g. via the fence).
    pub schedule: extern "C" fn(
        engine: *mut EngineState,
        priority: FfiJobPriority,
        fence: *mut FfiFence, // may be null
        userdata: *mut c_void,
        f: extern "C" fn(*mut c_void),
    ),

    /// Block the calling thread until all jobs on the fence complete.
    pub fence_wait: extern "C" fn(engine: *mut EngineState, fence: *mut FfiFence),

    /// Non-blocking check: returns 1 if all jobs on the fence are done.
    pub fence_is_complete: extern "C" fn(engine: *mut EngineState, fence: *mut FfiFence) -> u8,

    // Input
    /// Write all currently connected devices into `buf`.
    /// Returns the number of devices written.
    /// `buf_len` is the capacity of `buf`.
    pub input_get_devices:
        extern "C" fn(engine: *mut EngineState, buf: *mut DeviceInfo, buf_len: usize) -> usize,

    /// Fill `state` with the current snapshot for `device_id`.
    /// Returns 1 if the device exists and state was written, 0 otherwise.
    pub input_get_device_state:
        extern "C" fn(engine: *mut EngineState, device_id: u64, state: *mut DeviceState) -> u8,

    /// Returns 1 if the device was just connected this frame.
    pub input_device_just_connected: extern "C" fn(engine: *mut EngineState, device_id: u64) -> u8,

    /// Returns 1 if the device was just disconnected this frame.
    pub input_device_just_disconnected:
        extern "C" fn(engine: *mut EngineState, device_id: u64) -> u8,
}
