// host_api.rs

// hosts implementation of the rok-abi HostVtable

use crate::host_state::HostState;
use rok_abi::HostVTable;
use rok_abi::log::LogRecord;

use rok_log::logger::log_record;

pub(crate) extern "C" fn host_log_submit(record: *const LogRecord) {
    if !record.is_null() {
        log_record(unsafe { *record });
    }
}

extern "C" fn host_request_quit(host: *mut rok_abi::HostState) {
    // Safety: Host creates ConcreteHostState and passes it as *mut HostState.
    let state = unsafe { &mut *(host as *mut HostState) };
    state.should_quit = true;
}

// Public API

pub(crate) fn create_host_vtable() -> HostVTable {
    HostVTable {
        fn_log_submit: host_log_submit,
        fn_request_quit: host_request_quit,
    }
}
