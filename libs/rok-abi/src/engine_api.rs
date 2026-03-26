// engine_api.rs

use core::ffi::c_void;

use crate::input::{DeviceInfo, DeviceState};
use crate::log::LogRecord;

// ---------------------------------------------------------------------------
// Opaque engine state
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct EngineHandle {
    _private: [u8; 0],
}

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
pub struct Fence {
    _private: [u8; 0],
}

// Engine API fn Type Aliases

type FnFenceCreate = extern "C" fn(engine: *mut EngineHandle) -> *mut Fence;
type FnFenceFree = extern "C" fn(engine: *mut EngineHandle, fence: *mut Fence);
type FnFenceWait = extern "C" fn(engine: *mut EngineHandle, fence: *mut Fence);
type FnFenceIsComplete = extern "C" fn(engine: *mut EngineHandle, fence: *mut Fence) -> u8;
type FnSchedule = extern "C" fn(
    engine: *mut EngineHandle,
    priority: FfiJobPriority,
    fence: *mut Fence,
    userdata: *mut c_void,
    f: extern "C" fn(*mut c_void),
);

type FnGetInputDevices =
    extern "C" fn(engine: *mut EngineHandle, buf: *mut DeviceInfo, buf_len: usize) -> usize;

type FnGetDeviceState =
    extern "C" fn(engine: *mut EngineHandle, device_id: u64, state: *mut DeviceState) -> u8;

// Log is special due to DLL boundaries.

pub type FnLogSubmit = extern "C" fn(*const LogRecord);

/// Engine services that the Target can call.
#[repr(C)]
pub struct EngineApi {
    handle: *mut EngineHandle,
    fn_log_submit: FnLogSubmit,
    fn_fence_create: FnFenceCreate,
    fn_fence_free: FnFenceFree,
    fn_schedule: FnSchedule,
    fn_fence_wait: FnFenceWait,
    fn_fence_is_complete: FnFenceIsComplete,
    fn_input_get_devices: FnGetInputDevices,
    fn_input_get_device_state: FnGetDeviceState,
}

impl EngineApi {
    // Construction

    pub fn new(
        handle: *mut EngineHandle,
        fn_log_submit: FnLogSubmit,
        fn_fence_create: FnFenceCreate,
        fn_fence_free: FnFenceFree,
        fn_schedule: FnSchedule,
        fn_fence_wait: FnFenceWait,
        fn_fence_is_complete: FnFenceIsComplete,
        fn_input_get_devices: FnGetInputDevices,
        fn_input_get_device_state: FnGetDeviceState,
    ) -> Self {
        Self {
            handle,
            fn_log_submit,
            fn_fence_create,
            fn_fence_free,
            fn_schedule,
            fn_fence_wait,
            fn_fence_is_complete,
            fn_input_get_devices,
            fn_input_get_device_state,
        }
    }

    // ---- UTILITY ----

    #[inline]
    pub fn log_submit(&self) -> FnLogSubmit {
        self.fn_log_submit
    }

    // ---- API ----

    // Job system

    #[inline]
    pub fn fence_create(&self) -> *mut Fence {
        (self.fn_fence_create)(self.handle)
    }

    #[inline]
    pub fn fence_free(&self, fence: *mut Fence) {
        (self.fn_fence_free)(self.handle, fence)
    }

    #[inline]
    pub fn fence_wait(&self, fence: *mut Fence) {
        (self.fn_fence_wait)(self.handle, fence)
    }

    #[inline]
    pub fn fence_is_complete(&self, fence: *mut Fence) -> bool {
        (self.fn_fence_is_complete)(self.handle, fence) == 1
    }

    #[inline]
    pub fn schedule(
        &self,
        priority: FfiJobPriority,
        fence: *mut Fence, // may be null
        userdata: *mut c_void,
        f: extern "C" fn(*mut c_void),
    ) {
        (self.fn_schedule)(self.handle, priority, fence, userdata, f)
    }

    // Input System

    // TODO: Maybe this can take a vec or something output iterator?
    #[inline]
    pub fn input_get_devices(&self, buf: *mut DeviceInfo, buf_len: usize) -> usize {
        (self.fn_input_get_devices)(self.handle, buf, buf_len)
    }

    #[inline]
    pub fn input_get_device_state(&self, device_id: u64, state: *mut DeviceState) -> bool {
        (self.fn_input_get_device_state)(self.handle, device_id, state) == 1
    }
}
