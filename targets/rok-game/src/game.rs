
// game.rs
//
// Concrete game state and TargetVTable implementations.
// This is where actual game code will live.

use std::ffi::c_char;

use rok_abi::engine_api::EngineApi;
use rok_abi::host_api::LogLevel;
use rok_abi::target_api::{HotReloadBuffer, TargetState, TargetVTable};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

struct GameState {
    /// Borrowed from the Engine. Valid from init until shutdown.
    api: *const EngineApi,

    frame_count: u64,

    // TODO: worlds, scene graph, asset handles, etc.
}

impl GameState {
    fn log(&self, level: LogLevel, msg: &str) {
        unsafe {
            let api = &*self.api;
            (api.log)(
                api.engine,
                level as u32,
                msg.as_ptr() as *const c_char,
                msg.len(),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Hot-reload
//
// Only frame_count survives for now — proves the path works.
// Extend this as state grows.
// ---------------------------------------------------------------------------

const SAVE_SIZE: usize = size_of::<u64>();

fn save(state: &GameState, buf: *mut HotReloadBuffer) {
    unsafe {
        let buf = &mut *buf;
        assert!(buf.capacity >= SAVE_SIZE);
        std::ptr::copy_nonoverlapping(
            state.frame_count.to_le_bytes().as_ptr(),
            buf.buf,
            SAVE_SIZE,
        );
        buf.len = SAVE_SIZE;
    }
}

fn load(buf: *const HotReloadBuffer) -> u64 {
    unsafe {
        let buf = &*buf;
        assert_eq!(buf.len, SAVE_SIZE);
        let mut bytes = [0u8; 8];
        std::ptr::copy_nonoverlapping(buf.buf, bytes.as_mut_ptr(), SAVE_SIZE);
        u64::from_le_bytes(bytes)
    }
}

// ---------------------------------------------------------------------------
// TargetVTable implementations
// ---------------------------------------------------------------------------

extern "C" fn on_init(
    api:        *const EngineApi,
    hot_reload: *const HotReloadBuffer,
) -> *mut TargetState {
    let frame_count = if hot_reload.is_null() { 0 } else { load(hot_reload) };

    let state = Box::new(GameState { api, frame_count });
    let ptr = Box::into_raw(state) as *mut TargetState;

    let s = unsafe { &*(ptr as *const GameState) };
    if hot_reload.is_null() {
        s.log(LogLevel::Info, "rok-game: init");
    } else {
        s.log(LogLevel::Info, "rok-game: hot-reload restore");
    }

    ptr
}

extern "C" fn on_shutdown(state: *mut TargetState, hot_reload: *mut HotReloadBuffer) {
    let boxed = unsafe { Box::from_raw(state as *mut GameState) };

    if hot_reload.is_null() {
        boxed.log(LogLevel::Info, "rok-game: shutdown");
    } else {
        boxed.log(LogLevel::Info, "rok-game: saving state for hot-reload");
        save(&boxed, hot_reload);
    }
}

extern "C" fn on_update(state: *mut TargetState, _dt: f32) {
    let state = unsafe { &mut *(state as *mut GameState) };
    state.frame_count += 1;

    // TODO: tick game systems
}

extern "C" fn on_render(state: *mut TargetState) {
    let _state = unsafe { &*(state as *const GameState) };

    // TODO: submit draw calls
}

extern "C" fn on_resize(state: *mut TargetState, width: u32, height: u32) {
    let state = unsafe { &*(state as *const GameState) };
    let msg = format!("rok-game: resize {width}x{height}");
    state.log(LogLevel::Info, &msg);

    // TODO: resize dependent render targets, update projection matrices
}

// ---------------------------------------------------------------------------
// VTable
// ---------------------------------------------------------------------------

pub fn make_vtable() -> TargetVTable {
    TargetVTable {
        init:      on_init,
        shutdown:  on_shutdown,
        update:    on_update,
        render:    Some(on_render),
        on_resize: Some(on_resize),
    }
}