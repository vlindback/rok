use rok_abi::{EngineState, EngineVTable};
use std::panic::catch_unwind;

// The actual internal struct in the engine
struct MyEngine {
    tick: u64,
}

// #[unsafe(no_mangle)]
// pub extern "C" fn rok_engine_entry() -> EngineVTable {
//     EngineVTable {
//         init: engine_init,
//         shutdown: engine_shutdown,
//         update: engine_update,
//     }
// }

extern "C" fn engine_init() -> *mut EngineState {
    let state = Box::new(MyEngine { tick: 0 });
    Box::into_raw(state) as *mut EngineState
}
