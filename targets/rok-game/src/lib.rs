// lib.rs
//
// Entry point only. Game logic lives in game.rs.
// This file should never grow beyond the exported symbol.

mod game;

use rok_abi::TargetVTable;

#[unsafe(no_mangle)]
pub extern "C" fn rok_target_vtable_get() -> TargetVTable {
    game::make_vtable()
}
