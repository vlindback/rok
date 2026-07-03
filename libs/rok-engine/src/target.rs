// target.s

use rok_abi::{
    EngineApi, HotReloadBuffer, TARGET_ENTRY_SYMBOL, TargetState, TargetVTable, TargetVTableGetter,
};

use crate::error::EngineError;

pub(crate) struct Target {
    state: *mut TargetState,
    vtable: TargetVTable,
    _lib: libloading::Library,
}

impl Target {
    pub(crate) fn load(
        path: &str,
        api: &EngineApi,
        hot_reload: Option<&HotReloadBuffer>,
    ) -> Result<Self, EngineError> {
        // Safety: loading an arbitrary DLL is inherently unsafe; we trust the configured target path.
        // Errors (missing file, bad image) surface as libloading errors.
        let lib = unsafe { libloading::Library::new(path).map_err(EngineError::Library)? };

        // Fetch the vtable through the pre-defined entry symbol.
        let vtable = {
            let getter: libloading::Symbol<TargetVTableGetter> =
                unsafe { lib.get(TARGET_ENTRY_SYMBOL)? };
            getter()
        };

        let hot_reload_ptr = match hot_reload {
            Some(buf) => buf as *const HotReloadBuffer,
            None => core::ptr::null(),
        };

        let state = (vtable.init)(api as *const EngineApi, hot_reload_ptr);
        if state.is_null() {
            return Err(EngineError::TargetInitFailure);
        }

        Ok(Self {
            state,
            vtable,
            _lib: lib,
        })
    }

    #[inline]
    pub(crate) fn update(&mut self, dt: f32) {
        (self.vtable.update)(self.state, dt);
    }

    #[inline]
    pub(crate) fn render(&mut self) {
        if let Some(render) = self.vtable.render {
            render(self.state);
        }
    }

    #[inline]
    pub(crate) fn on_resize(&mut self, width: u32, height: u32) {
        if let Some(on_resize) = self.vtable.on_resize {
            on_resize(self.state, width, height);
        }
    }
}

impl Drop for Target {
    fn drop(&mut self) {
        // Null hot-reload buffer means final shutdown.
        (self.vtable.shutdown)(self.state, std::ptr::null_mut());
    }
}
