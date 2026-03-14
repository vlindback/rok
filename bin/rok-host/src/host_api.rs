// host_api.rs

// hosts implementation of the rok-abi HostVtable

use crate::host_state::HostState;
use rok_abi::{HostVTable, LogLevel};
use std::ffi::c_char;

extern "C" fn host_log(
    _host: *mut rok_abi::HostState,
    level: LogLevel,
    msg: *const c_char,
    len: usize,
) {
    // Safety: Engine guarantees msg is valid UTF-8 for `len` bytes.
    let text = unsafe {
        let slice = std::slice::from_raw_parts(msg as *const u8, len);
        std::str::from_utf8_unchecked(slice)
    };
    // TODO: replace with a real logger (tracing, env_logger, etc.)
    eprintln!("[{level:?}] {text}");
}

extern "C" fn host_request_quit(host: *mut rok_abi::HostState) {
    // Safety: Host creates ConcreteHostState and passes it as *mut HostState.
    let state = unsafe { &mut *(host as *mut HostState) };
    state.should_quit = true;
}

extern "C" fn host_read_file(
    _host: *mut rok_abi::HostState,
    path: *const c_char,
    buf: *mut u8,
    buf_len: usize,
) -> usize {
    let path = unsafe { std::ffi::CStr::from_ptr(path) };
    let path = match path.to_str() {
        Ok(s) => s,
        Err(_) => return usize::MAX,
    };
    match std::fs::read(path) {
        Ok(data) => {
            let n = data.len().min(buf_len);
            unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), buf, n) };
            n
        }
        Err(_) => usize::MAX,
    }
}

extern "C" fn host_file_size(_host: *mut rok_abi::HostState, path: *const c_char) -> usize {
    let path = unsafe { std::ffi::CStr::from_ptr(path) };
    let path = match path.to_str() {
        Ok(s) => s,
        Err(_) => return usize::MAX,
    };
    std::fs::metadata(path)
        .map(|m| m.len() as usize)
        .unwrap_or(usize::MAX)
}

// Public API

pub(crate) fn create_host_vtable() -> HostVTable {
    HostVTable {
        log: host_log,
        request_quit: host_request_quit,
        read_file: Some(host_read_file),
        file_size: Some(host_file_size),
    }
}
