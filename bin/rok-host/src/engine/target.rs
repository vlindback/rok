// target.s

use rok_abi::TargetVTable;

pub(crate) struct Target {
    pub(super) _lib: libloading::Library,
    pub(super) vtable: TargetVTable,
}

impl Target {}
