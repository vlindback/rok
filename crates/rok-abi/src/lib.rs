// rok-abi/src/lib.rs

use std::ffi::c_void;

#[repr(C)]
pub struct EngineState {
    _private: [u8; 0],
}

#[repr(C)]
pub struct EngineVTable {
    // Constructor/Destructor
    pub init: extern "C" fn() -> *mut EngineState,
    pub shutdown: extern "C" fn(*mut EngineState),

    // Logic
    pub update: extern "C" fn(*mut EngineState, f32),
    pub render: extern "C" fn(*mut EngineState),
}

pub const ENGINE_ENTRY_SYMBOL: &[u8] = b"rok_engine_vtable_get";
pub type VTableGetter = extern "C" fn() -> EngineVTable;
