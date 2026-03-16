// host_api.rs
//
// The Host's side of the ABI: what the Engine is allowed to call back into.
//

use core::ffi::c_char;

use crate::{LogLevel, log::LogRecord};

/// Opaque host state pointer. The Engine passes this back into every HostVTable
/// callback so the Host implementation can reach its own context without globals.
#[repr(C)]
pub struct HostState {
    _private: [u8; 0],
}

/// Callbacks from Engine -> Host
#[repr(C)]
pub struct HostVTable {
    /// Submit a log record to the host.
    pub log_submit: extern "C" fn(*const LogRecord),

    /// Ask the Host to begin an orderly shutdown after the current frame.
    /// The Host will stop its event loop and call Engine::shutdown.
    pub request_quit: extern "C" fn(host: *mut HostState),

    // -------------------------------------------------------------------------
    // OPTIONAL: File I/O  (set to null if host does not support)
    // -------------------------------------------------------------------------
    /// Synchronously read an entire file into a caller-supplied buffer.
    /// Returns the number of bytes read, or usize::MAX on error.
    ///
    /// `path` is a null-terminated UTF-8 string.
    /// `buf` and `buf_len` describe the output buffer.
    ///
    /// OPTIONAL — null if not provided.
    pub read_file: Option<
        extern "C" fn(
            host: *mut HostState,
            path: *const c_char,
            buf: *mut u8,
            buf_len: usize,
        ) -> usize,
    >,

    /// Return the byte length of a file without reading it.
    /// Returns usize::MAX on error.
    ///
    /// OPTIONAL — null if not provided.
    pub file_size: Option<extern "C" fn(host: *mut HostState, path: *const c_char) -> usize>,
}
