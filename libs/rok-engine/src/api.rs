// api.rs
//
// Engine-side construction of the EngineApi handed to the Target.
// The Target borrows this from `init` until `shutdown`; it lives in a
// field on the (boxed) Engine so its address is stable for that span.

use core::ffi::c_void;

use rok_abi::engine_api::{EngineApi, EngineHandle, Fence, FfiJobPriority};
use rok_abi::input::{DeviceInfo, DeviceState};
use rok_abi::log::LogRecord;

// The Target lives in a separate DLL with its OWN rok-log static, so its
// macros can't reach the host's logger directly. It calls init_remote with
// this pointer; every log_*! in the target then routes a LogRecord across
// the boundary into here, which drops it into the shared (host+engine)
// logger. Engine/host share one rok-log instance via static linking, so
// this just forwards.
extern "C" fn log_submit(record: *const LogRecord) {
    if record.is_null() {
        return;
    }
    // LogRecord is Copy + repr(C): deref-copy and forward.
    let record = unsafe { *record };
    rok_log::logger::log_record(record);
}

// --- Stubs: real extern "C" fns, just inert -------------------------------
//
// These fields are non-nullable fn pointers, so "not implemented yet" means
// a real function that does nothing sensible — not null. Nothing in the
// cube milestone calls any of these (engine owns the camera; the target
// schedules no jobs), so the defaults only need to be *harmless*.

extern "C" fn fence_create(_engine: *mut EngineHandle) -> *mut Fence {
    core::ptr::null_mut()
}
extern "C" fn fence_free(_engine: *mut EngineHandle, _fence: *mut Fence) {}
extern "C" fn fence_wait(_engine: *mut EngineHandle, _fence: *mut Fence) {}

// DECISION: report "complete" (1). If anything ever does wait on a stub
// fence, returning complete lets it fall through rather than spin forever.
extern "C" fn fence_is_complete(_engine: *mut EngineHandle, _fence: *mut Fence) -> u8 {
    1
}

extern "C" fn schedule(
    _engine: *mut EngineHandle,
    _priority: FfiJobPriority,
    _fence: *mut Fence,
    _userdata: *mut c_void,
    _f: extern "C" fn(*mut c_void),
) {
    // No-op. NOTE: a real impl would likely run `f(userdata)` inline as a
    // fallback so scheduled work isn't silently dropped — but nothing
    // schedules yet, so we don't even do that. Revisit when jobs cross the
    // boundary.
}

extern "C" fn input_get_devices(
    _engine: *mut EngineHandle,
    _buf: *mut DeviceInfo,
    _buf_len: usize,
) -> usize {
    0 // no devices reported
}

// DECISION: report "no such device" (0), leaving `state` untouched. The
// caller must treat 0 as "don't read state", which matches the ABI.
extern "C" fn input_get_device_state(
    _engine: *mut EngineHandle,
    _device_id: u64,
    _state: *mut DeviceState,
) -> u8 {
    0
}

pub(crate) fn build(handle: *mut EngineHandle) -> EngineApi {
    EngineApi::new(
        handle,
        log_submit,
        fence_create,
        fence_free,
        schedule,
        fence_wait,
        fence_is_complete,
        input_get_devices,
        input_get_device_state,
    )
}
