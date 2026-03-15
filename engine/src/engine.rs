// engine.rs
//
// ConcreteEngineState is the real type behind the opaque `*mut EngineState`
// pointer the Host holds. Everything flows through here.
//
// Lifetime contracts (mirroring what the ABI comments say):
//
//   host_state / host_vtable  — borrowed from the Host for the full Engine
//                               session. The Host owns them; we never free them.
//
//   target_state              — owned by us from load_target to unload_target.
//                               We call target.shutdown() before dropping.
//
//   engine_api                — Box<EngineApi> kept alive for the duration of
//                               a Target session (load → unload). Its address
//                               is stable because it lives on the heap. The
//                               Target holds a *const EngineApi to it.
//                               Created in load_target, dropped after
//                               target.shutdown() returns in unload_target.

use std::ffi::c_void;

use rok_abi::engine_api::{EngineApi, EngineState, EngineVTable, Fence, FfiJobPriority};
use rok_abi::frame::FrameInput;
use rok_abi::input::DeviceInfo;
use rok_abi::input::DeviceState;
use rok_abi::surface::NativeSurfaceHandle;
use rok_abi::target_api::{HotReloadBuffer, TargetState, TargetVTable};
use rok_abi::{HostState, HostVTable};

use rok_jobs::{JobFence, JobPriority, JobSystem};
use rok_log::log_error;
use rok_log::log_info;
use rok_log::log_warn;

// ---------------------------------------------------------------------------
// Concrete state
// ---------------------------------------------------------------------------

struct ConcreteEngineState {
    // --- Host interface (borrowed, never freed by us) ---
    host_state: *mut HostState,
    host_vtable: *const HostVTable,

    // --- Engine subsystems (owned) ---
    job_system: JobSystem,

    // --- Surface dimensions (updated by on_surface_changed) ---
    surface_width: u32,
    surface_height: u32,

    // --- Target slot ---
    // Both fields are populated together by load_target and cleared by
    // unload_target. Invariant: they are either both Some/non-null or both None/null.
    target_vtable: Option<TargetVTable>,
    target_state: *mut TargetState, // null when no target is loaded

    // Stable-address EngineApi lent to the Target. Lives from load_target
    // to just after target shutdown in unload_target.
    engine_api: Option<Box<EngineApi>>,
}

// extern "C" fn host_log(
//     _host: *mut rok_abi::HostState,
//     level: LogLevel,
//     msg: *const c_char,
//     len: usize,
// ) {
//     // Safety: Engine guarantees msg is valid UTF-8 for `len` bytes.
//     let text = unsafe {
//         let slice = std::slice::from_raw_parts(msg as *const u8, len);
//         std::str::from_utf8_unchecked(slice)
//     };
//     let record = make_record(
//         timestamp_ns(),
//         level,
//         b"<engine>\0".as_ptr() as *const c_char,
//         0,
//         text.as_bytes(),
//     );
//     log_record(record);
// }

impl ConcreteEngineState {}

// Safety: ConcreteEngineState is only ever accessed from the host thread
// (the single thread that owns the EngineState pointer). The JobSystem
// internally manages its own cross-thread safety.
unsafe impl Send for ConcreteEngineState {}
unsafe impl Sync for ConcreteEngineState {}

// ---------------------------------------------------------------------------
// Helper: type-erase / recover ConcreteEngineState through the opaque ptr
// ---------------------------------------------------------------------------

#[inline]
unsafe fn as_engine(state: *mut EngineState) -> &'static mut ConcreteEngineState {
    // Safety: caller guarantees `state` came from engine_init and has not
    // been passed to engine_shutdown yet.
    unsafe { &mut *(state as *mut ConcreteEngineState) }
}

// ---------------------------------------------------------------------------
// EngineVTable implementations (Host → Engine)
// ---------------------------------------------------------------------------

extern "C" fn engine_init(
    host_state: *mut HostState,
    host_vtable: *const HostVTable,
    surface: *const NativeSurfaceHandle,
) -> *mut EngineState {
    // set up logging first

    rok_log::init_remote(unsafe { (*host_vtable).log_submit });

    let (w, h) = unsafe { ((*surface).width, (*surface).height) };

    let state = Box::new(ConcreteEngineState {
        host_state,
        host_vtable,
        job_system: JobSystem::new(),
        surface_width: w,
        surface_height: h,
        target_vtable: None,
        target_state: std::ptr::null_mut(),
        engine_api: None,
    });

    let ptr = Box::into_raw(state) as *mut EngineState;

    // Log through the host now that we have a stable pointer.
    // Safety: ptr was just created and is valid.
    log_info!("rok-engine: init");

    ptr
}

extern "C" fn engine_shutdown(state: *mut EngineState) {
    // Safety: state was produced by engine_init and this is the last call on it.
    let engine = unsafe { Box::from_raw(state as *mut ConcreteEngineState) };
    log_info!("rok-engine: shutdown");
    // Drop order: engine_api then job_system then rest — Rust handles this
    // correctly via field declaration order in ConcreteEngineState.
    drop(engine);
}

extern "C" fn engine_update(state: *mut EngineState, input: *const FrameInput) {
    let engine = unsafe { as_engine(state) };

    // Safety: input is valid for the duration of this call (Host contract).
    let frame = unsafe { &*input };

    // Surface resize: notify the Target before the tick so it can resize
    // its own render targets in the same frame.
    if frame.lifecycle.surface_changed != 0 {
        engine.surface_width = frame.lifecycle.surface_width;
        engine.surface_height = frame.lifecycle.surface_height;

        if let Some(vtable) = &engine.target_vtable {
            if let Some(on_resize) = vtable.on_resize {
                if !engine.target_state.is_null() {
                    on_resize(
                        engine.target_state,
                        frame.lifecycle.surface_width,
                        frame.lifecycle.surface_height,
                    );
                }
            }
        }
    }

    // TODO: tick engine subsystems that run before the Target
    // (e.g. input binding layer, physics pre-step).

    // Forward to Target.
    if let Some(vtable) = &engine.target_vtable {
        if !engine.target_state.is_null() {
            (vtable.update)(engine.target_state, frame.delta_time);
        }
    }

    // TODO: tick engine subsystems that run after the Target.
}

extern "C" fn engine_render(state: *mut EngineState) {
    let engine = unsafe { as_engine(state) };

    // TODO: begin frame (acquire swapchain image, reset command pools, etc.)

    if let Some(vtable) = &engine.target_vtable {
        if let Some(render) = vtable.render {
            if !engine.target_state.is_null() {
                render(engine.target_state);
            }
        }
    }

    // TODO: submit command buffers, present.
}

extern "C" fn engine_on_surface_changed(
    state: *mut EngineState,
    surface: *const NativeSurfaceHandle,
) {
    let engine = unsafe { as_engine(state) };
    // surface is borrowed for this call only.
    let (w, h) = unsafe { ((*surface).width, (*surface).height) };
    engine.surface_width = w;
    engine.surface_height = h;

    log_info!("rok-engine: surface changed — swapchain recreation TODO");
    // TODO: recreate Vulkan swapchain here.
}

extern "C" fn engine_load_target(state: *mut EngineState, vtable: *const TargetVTable) -> u8 {
    let engine = unsafe { as_engine(state) };

    if engine.target_state != std::ptr::null_mut() {
        log_warn!("rok-engine: load_target called while a target is already loaded. Unload first");
        return 0;
    }

    // Build the EngineApi we hand to the Target. Heap-allocate for stable address.
    let api = Box::new(make_engine_api(state));
    let api_ptr: *const EngineApi = &*api;
    engine.engine_api = Some(api);

    // Safety: vtable is borrowed for the duration of load_target; we copy it
    // so we own it going forward and the caller can free their copy.
    let vtable_copy = unsafe { *vtable };

    // Initialise the Target (no hot-reload buffer on first load).
    let target_state = (vtable_copy.init)(api_ptr, std::ptr::null());

    if target_state.is_null() {
        log_error!("rok-engine: target init returned null");
        engine.engine_api = None;
        return 0;
    }

    engine.target_vtable = Some(vtable_copy);
    engine.target_state = target_state;

    log_info!("rok-engine: target loaded");

    return 1;
}

extern "C" fn engine_unload_target(state: *mut EngineState) {
    let engine = unsafe { as_engine(state) };

    let (vtable, target_state) = match engine.target_vtable.take() {
        Some(v) => (v, engine.target_state),
        None => {
            log_warn!("rok-engine: unload_target called with no target loaded");
            return;
        }
    };

    engine.target_state = std::ptr::null_mut();

    // TODO: drain any in-flight jobs that touch TargetState before calling
    // shutdown. When rok-jobs exposes a "drain by tag" API this is the place
    // to call it.

    // Shut down the Target. Passing null for hot_reload = final shutdown,
    // not a reload. When hot-reload is implemented pass a pre-allocated buffer.
    (vtable.shutdown)(target_state, std::ptr::null_mut());

    // EngineApi is safe to drop now that target.shutdown has returned.
    engine.engine_api = None;

    log_info!("rok-engine: target unloaded");
}

// ---------------------------------------------------------------------------
// EngineApi implementations (Engine → Target callbacks)
// ---------------------------------------------------------------------------

extern "C" fn api_fence_create(_engine: *mut EngineState) -> *mut Fence {
    // Allocate a real JobFence on the heap and type-erase the pointer.
    let fence = Box::new(JobFence::new());
    Box::into_raw(fence) as *mut Fence
}

extern "C" fn api_fence_free(_engine: *mut EngineState, fence: *mut Fence) {
    // Safety: fence was produced by api_fence_create and has not been freed yet.
    // The Target must guarantee no jobs still reference this fence.
    unsafe { drop(Box::from_raw(fence as *mut JobFence)) };
}

extern "C" fn api_schedule(
    engine: *mut EngineState,
    priority: FfiJobPriority,
    fence: *mut Fence, // may be null
    userdata: *mut c_void,
    f: extern "C" fn(*mut c_void),
) {
    let engine = unsafe { as_engine(engine) };

    let prio = match priority {
        FfiJobPriority::High => JobPriority::High,
        FfiJobPriority::Normal => JobPriority::Normal,
        FfiJobPriority::Low => JobPriority::Low,
    };

    // userdata is a raw pointer; we need to cross a thread boundary.
    // Wrapping it in a newtype that asserts Send is correct here — the Target
    // contract requires that userdata outlives the job and is not aliased
    // without synchronisation.
    //
    // The `invoke` method is intentional: in Rust 2021 closures capture the
    // minimal field needed. `move || f(ud.0)` would capture `ud.0: *mut c_void`
    // (not Send), bypassing the wrapper entirely. By calling a method, the
    // closure captures `ud: SendUserdata` as a whole, keeping the Send impl.
    struct SendUserdata(*mut c_void);
    unsafe impl Send for SendUserdata {}
    impl SendUserdata {
        fn invoke(self, f: extern "C" fn(*mut c_void)) {
            f(self.0);
        }
    }
    let ud = SendUserdata(userdata);

    if fence.is_null() {
        engine
            .job_system
            .submit(move || ud.invoke(f))
            .with_priority(prio)
            .dispatch()
            .detach();
    } else {
        // The fence pointer is really a *mut JobFence allocated by api_fence_create.
        // We need a &JobFence with a lifetime the borrow checker accepts.
        //
        // Safety: fence is a valid *mut JobFence (created by api_fence_create),
        // it is kept alive by the caller until the jobs complete (Target contract),
        // and JobFence itself is Send + Sync. We transmute the lifetime to
        // 'static so JobBuilder::with_fence accepts it; the actual liveness is
        // upheld by the caller, not the type system.
        let job_fence: &'static JobFence =
            unsafe { std::mem::transmute(&*(fence as *const JobFence)) };

        engine
            .job_system
            .submit(move || ud.invoke(f))
            .with_priority(prio)
            .with_fence(job_fence)
            .dispatch()
            .detach();
    }
}

extern "C" fn api_fence_wait(_engine: *mut EngineState, fence: *mut Fence) {
    // Safety: fence is a valid *mut JobFence, valid until the caller frees it.
    let job_fence = unsafe { &*(fence as *const JobFence) };
    job_fence.wait();
}

extern "C" fn api_fence_is_complete(_engine: *mut EngineState, fence: *mut Fence) -> u8 {
    let job_fence = unsafe { &*(fence as *const JobFence) };
    job_fence.is_complete() as u8
}

extern "C" fn api_input_get_devices(
    engine: *mut EngineState,
    buf: *mut DeviceInfo,
    buf_len: usize,
) -> usize {
    todo!();
}

extern "C" fn api_input_get_device_state(
    engine: *mut EngineState,
    device_id: u64,
    state: *mut DeviceState,
) -> u8 {
    todo!();
}

// ---------------------------------------------------------------------------
// EngineApi constructor
// ---------------------------------------------------------------------------

fn make_engine_api(engine: *mut EngineState) -> EngineApi {
    let fn_log_submit = unsafe { (*(*as_engine(engine)).host_vtable).log_submit };

    EngineApi::new(
        engine,
        fn_log_submit,
        api_fence_create,
        api_fence_free,
        api_schedule,
        api_fence_wait,
        api_fence_is_complete,
        api_input_get_devices,
        api_input_get_device_state,
    )
}

// ---------------------------------------------------------------------------
// VTable constructor (called from lib.rs)
// ---------------------------------------------------------------------------

pub fn make_vtable() -> EngineVTable {
    EngineVTable {
        init: engine_init,
        shutdown: engine_shutdown,
        update: engine_update,
        render: engine_render,
        on_surface_changed: engine_on_surface_changed,
        load_target: engine_load_target,
        unload_target: engine_unload_target,
    }
}
