// engine.rs

use rok_abi::{
    ENGINE_ENTRY_SYMBOL, EngineState, EngineVTable, EngineVTableGetter, FrameInput, HostState,
    HostVTable, NativeSurfaceHandle, TARGET_ENTRY_SYMBOL, TargetVTableGetter,
};
use rok_log::shutdown;

use super::target::Target;
use crate::host_error::HostError;

pub(crate) struct Engine {
    _lib: libloading::Library,
    state: *mut EngineState,
    vtable: EngineVTable,
}

impl Engine {
    pub(crate) fn load(dll_path: &str) -> Result<Self, HostError> {
        let _lib = unsafe { libloading::Library::new(dll_path).map_err(HostError::Library)? };
        let getter: libloading::Symbol<EngineVTableGetter> =
            unsafe { _lib.get(ENGINE_ENTRY_SYMBOL).map_err(HostError::Library)? };
        let vtable = getter();

        Ok(Self {
            _lib,
            state: std::ptr::null_mut(),
            vtable,
        })
    }

    pub(crate) fn init(
        &mut self,
        host_vtable: &HostVTable,
        host_state: &mut HostState,
        surface: &NativeSurfaceHandle,
    ) -> Result<(), HostError> {
        debug_assert!(self.state.is_null(), "Engine::init called twice");
        let engine_state = (self.vtable.init)(host_state, host_vtable, surface);
        if engine_state.is_null() {
            return Err(HostError::EngineInitFailure);
        }
        self.state = engine_state;
        Ok(())
    }

    pub(crate) fn load_target(&self, target_dll_path: &str) -> Result<Target, HostError> {
        debug_assert!(!self.state.is_null(), "Engine::init not called");

        let _lib =
            unsafe { libloading::Library::new(target_dll_path).map_err(HostError::Library)? };
        let getter: libloading::Symbol<TargetVTableGetter> =
            unsafe { _lib.get(TARGET_ENTRY_SYMBOL).map_err(HostError::Library)? };
        let _vtable = getter();

        let ok = (self.vtable.load_target)(self.state, &_vtable);
        if ok == 0 {
            return Err(HostError::TargetInitFailure);
        }
        Ok(Target { _lib, _vtable })
    }

    pub(crate) fn unload_target(&self) {
        (self.vtable.unload_target)(self.state);
    }

    pub(crate) fn update(&self, frame_input: FrameInput) {
        (self.vtable.update)(self.state, &frame_input as *const FrameInput)
    }

    pub(crate) fn render(&self) {
        (self.vtable.render)(self.state)
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if !self.state.is_null() {
            (self.vtable.shutdown)(self.state);
            self.state = std::ptr::null_mut();
        }
        rok_log::shutdown();
    }
}
