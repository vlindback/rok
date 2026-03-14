// lib.rs
//
// The Engine DLL's only public surface is the single C entry point below.
// Everything else is an implementation detail confined to engine.rs.

mod engine;

use rok_abi::EngineVTable;

/// Called by the Host immediately after dlopen / LoadLibrary to obtain the
/// complete Engine interface. The returned vtable contains only function
/// pointers — it is trivially copyable and carries no hidden state.
///
/// The symbol name is the null-terminated byte string ENGINE_ENTRY_SYMBOL.
#[unsafe(no_mangle)]
pub extern "C" fn rok_engine_vtable_get() -> EngineVTable {
    engine::make_vtable()
}
