// host_api.rs
//
// The Host's side of the ABI: what the Engine is allowed to call back into.
//

use core::ffi::c_char;

use crate::log::LogRecord;

/// Opaque host state pointer. The Engine passes this back into every HostVTable
/// callback so the Host implementation can reach its own context without globals.
#[repr(C)]
pub struct HostState {
    _private: [u8; 0],
}

// Host API Structs

// Host API Functions

type FnLogSubmit = extern "C" fn(*const LogRecord);
type FnRequestQuit = extern "C" fn(host: *mut HostState);

// Vtable
#[repr(C)]
pub struct HostVTable {
    pub fn_log_submit: FnLogSubmit,
    pub fn_request_quit: FnRequestQuit,
}
