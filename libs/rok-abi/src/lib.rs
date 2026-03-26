// rok-abi/src/lib.rs
//
// Public ABI surface for the rok engine plugin system.
//
// ┌─────────────────────────────────────────────────────────────────────┐
// │  rok-host (exe)                                                     │
// │    owns: OS window, event loop, DLL lifetimes                       │
// │    calls: EngineVTable                                              │
// │    exposes: HostVTable (callbacks)                                  │
// │                                                                     │
// │    dlopen ──► rok-engine (cdylib)                                   │
// │                 owns: renderer, job system, asset system            │
// │                 calls: HostVTable, TargetVTable                     │
// │                 exposes: EngineVTable, EngineApi                    │
// │                                                                     │
// │                 dlopen ──► target-game (cdylib)                     │
// │                              owns: game state, scene                │
// │                              calls: EngineApi                       │
// │                              exposes: TargetVTable                  │
// └─────────────────────────────────────────────────────────────────────┘
//
// Data flow (per frame):
//   Host  fills  FrameInput { delta_time, events, surface_changed, ... }
//   Host  calls  EngineVTable::update(state, &frame_input)
//   Engine calls TargetVTable::update(target_state, delta_time)
//   Host  calls  EngineVTable::render(state)
//   Engine calls TargetVTable::render(target_state)      [optional]

pub mod engine_api;
pub mod input;
pub mod log;
pub mod surface;
pub mod target_api;

// Flatten the most-used types so callers can `use rok_abi::*`.
pub use engine_api::{EngineApi, Fence, FfiJobPriority};
pub use input::{InputEventKind, RawInputEvent};
pub use log::LogLevel;
pub use surface::{NativeSurfaceHandle, SurfaceType};
pub use target_api::{
    HotReloadBuffer, TARGET_ENTRY_SYMBOL, TargetState, TargetVTable, TargetVTableGetter,
};
