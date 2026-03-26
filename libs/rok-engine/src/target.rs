// target.s

use rok_abi::{TARGET_ENTRY_SYMBOL, TargetVTable, TargetVTableGetter};

use crate::error::EngineError;

pub(crate) struct Target {
    _lib: libloading::Library,
    _vtable: TargetVTable,
}

impl Target {
    pub(crate) fn from_filepath(filepath: &str) -> Result<Self, EngineError> {
        let _lib = unsafe { libloading::Library::new(filepath).map_err(EngineError::Library)? };
        let getter: libloading::Symbol<TargetVTableGetter> = unsafe {
            _lib.get(TARGET_ENTRY_SYMBOL)
                .map_err(EngineError::Library)?
        };
        let _vtable = getter();
        Ok(Target { _lib, _vtable })
    }
}
