// rok-host/src/main.rs
//
// The Host is the thin, stable exe that owns:
//   - The OS window and native event loop
//   - The input event queue
//   - DLL lifetimes (engine + target)
//
// It knows almost nothing about rendering or game logic — those live in the DLLs.
// Its job is to pump the OS, collect raw input, and drive the engine tick.

use std::ffi::c_char;

use rok_abi::{
    ENGINE_ENTRY_SYMBOL, EngineVTable, EngineVTableGetter, FrameInput, HostState, HostVTable,
    LogLevel, NativeSurfaceHandle, RawInputEvent, TARGET_ENTRY_SYMBOL, TargetVTable,
    TargetVTableGetter, frame::LifecycleFlags,
};

// ---------------------------------------------------------------------------
// Host state
// ---------------------------------------------------------------------------

/// The host's concrete internal state.
/// `*mut HostState` in the ABI is really `*mut ConcreteHostState` under the hood —
/// we cast on the way in and out of every HostVTable callback.
struct ConcreteHostState {
    should_quit: bool,
    // Future: file system roots, profiler handle, etc.
}

// ---------------------------------------------------------------------------
// HostVTable implementations (called by the Engine)
// ---------------------------------------------------------------------------

extern "C" fn host_log(_host: *mut HostState, level: LogLevel, msg: *const c_char, len: usize) {
    // Safety: Engine guarantees msg is valid UTF-8 for `len` bytes.
    let text = unsafe {
        let slice = std::slice::from_raw_parts(msg as *const u8, len);
        std::str::from_utf8_unchecked(slice)
    };
    // TODO: replace with a real logger (tracing, env_logger, etc.)
    eprintln!("[{level:?}] {text}");
}

extern "C" fn host_request_quit(host: *mut HostState) {
    // Safety: Host creates ConcreteHostState and passes it as *mut HostState.
    let state = unsafe { &mut *(host as *mut ConcreteHostState) };
    state.should_quit = true;
}

extern "C" fn host_read_file(
    _host: *mut HostState,
    path: *const c_char,
    buf: *mut u8,
    buf_len: usize,
) -> usize {
    let path = unsafe { std::ffi::CStr::from_ptr(path) };
    let path = match path.to_str() {
        Ok(s) => s,
        Err(_) => return usize::MAX,
    };
    match std::fs::read(path) {
        Ok(data) => {
            let n = data.len().min(buf_len);
            unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), buf, n) };
            n
        }
        Err(_) => usize::MAX,
    }
}

extern "C" fn host_file_size(_host: *mut HostState, path: *const c_char) -> usize {
    let path = unsafe { std::ffi::CStr::from_ptr(path) };
    let path = match path.to_str() {
        Ok(s) => s,
        Err(_) => return usize::MAX,
    };
    std::fs::metadata(path)
        .map(|m| m.len() as usize)
        .unwrap_or(usize::MAX)
}

// ---------------------------------------------------------------------------
// Loaded DLL wrappers
// ---------------------------------------------------------------------------

/// A loaded Engine DLL. Owns the `libloading::Library` handle, which keeps
/// the DLL mapped. Dropping this struct unloads the DLL — only do that after
/// calling `vtable.shutdown`.
struct LoadedEngine {
    vtable: EngineVTable,
    // Kept alive so the vtable function pointers remain valid.
    _lib: libloading::Library,
}

impl LoadedEngine {
    fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Safety: we control the DLL and know it exports the entry symbol.
        let lib = unsafe { libloading::Library::new(path)? };
        let getter: libloading::Symbol<EngineVTableGetter> =
            unsafe { lib.get(ENGINE_ENTRY_SYMBOL)? };
        let vtable = getter();
        Ok(Self { vtable, _lib: lib })
    }
}

/// A loaded Target DLL.
struct LoadedTarget {
    vtable: TargetVTable,
    _lib: libloading::Library,
}

impl LoadedTarget {
    fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let lib = unsafe { libloading::Library::new(path)? };
        let getter: libloading::Symbol<TargetVTableGetter> =
            unsafe { lib.get(TARGET_ENTRY_SYMBOL)? };
        let vtable = getter();
        Ok(Self { vtable, _lib: lib })
    }
}

// ---------------------------------------------------------------------------
// Platform stub (replace with rok-platform implementations)
// ---------------------------------------------------------------------------

/// Placeholder: in the real implementation this creates a Win32 / Wayland window
/// and returns its native handles. For now it returns a zeroed handle so the
/// rest of the host structure compiles and can be tested without a GPU.
fn create_platform_window() -> NativeSurfaceHandle {
    use rok_abi::surface::{SurfaceData, SurfaceKind, Win32Surface};
    NativeSurfaceHandle {
        kind: SurfaceKind::Win32,
        data: SurfaceData {
            win32: Win32Surface {
                hwnd: std::ptr::null_mut(),
                hinstance: std::ptr::null_mut(),
            },
        },
        width: 1280,
        height: 720,
    }
}

/// Placeholder: drain the OS event queue into `events`.
/// The real implementation calls PeekMessageW / wl_display_dispatch_pending
/// and translates native events into RawInputEvents.
fn poll_platform_events(_events: &mut Vec<RawInputEvent>) {
    // TODO: Win32 PeekMessageW loop / Wayland dispatch

    // Win32: match on WM_* messages
    // WM_INPUT        → push to events
    // WM_SIZE         → set lifecycle.surface_changed
    // WM_CLOSE        → set lifecycle.should_quit
    // WM_KILLFOCUS    → push FocusLost to events (affects input state)
    //                   AND set lifecycle.focus_lost (engine may pause sim)
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

// Paths (in a real build these come from argv or a config file)
#[cfg(target_os = "linux")]
const ENGINE_PATH: &str = "./rok_engine.so";
#[cfg(target_os = "windows")]
const ENGINE_PATH: &str = "./rok_engine.dll";
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
const ENGINE_PATH: &str = "./rok_engine.dylib";

#[cfg(target_os = "linux")]
const TARGET_PATH: &str = "./rok_target.so";
#[cfg(target_os = "windows")]
const TARGET_PATH: &str = "./rok_target.dll";
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
const TARGET_PATH: &str = "./rok_target.dylib";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Host state (concrete, stack-allocated)
    let mut host_state = ConcreteHostState { should_quit: false };

    // Erase to opaque pointer for ABI boundary.
    let host_state_ptr = &mut host_state as *mut ConcreteHostState as *mut HostState;

    // HostVTable (static; only pointers, no heap)
    let host_vtable = HostVTable {
        log: host_log,
        request_quit: host_request_quit,
        read_file: Some(host_read_file),
        file_size: Some(host_file_size),
    };

    // --- Platform window ---
    // Must outlive the engine. The engine will create its VkSurface from this.
    let surface = create_platform_window();

    // --- Load Engine DLL ---
    let engine = LoadedEngine::load(ENGINE_PATH)?;

    // --- Initialise Engine ---
    // Safety: host_state_ptr and host_vtable are valid for the engine's lifetime.
    // surface is borrowed only for the duration of this call.
    let engine_state = (engine.vtable.init)(
        host_state_ptr,
        &host_vtable as *const HostVTable,
        &surface as *const NativeSurfaceHandle,
    );
    assert!(!engine_state.is_null(), "Engine init failed");

    // --- Load Target DLL and hand it to the Engine ---
    let target = LoadedTarget::load(TARGET_PATH)?;
    let ok = (engine.vtable.load_target)(engine_state, &target.vtable);
    assert!(ok != 0, "Target load failed");

    // --- Input event buffer (reused each frame) ---
    let mut events: Vec<RawInputEvent> = Vec::with_capacity(256);

    // --- Frame timing ---
    let mut last_frame = std::time::Instant::now();
    let start = std::time::Instant::now();

    // ---------------------------------------------------------------------------
    // Main loop
    // ---------------------------------------------------------------------------
    while !host_state.should_quit {
        // 1. Drain the OS event queue.
        //    This may set should_quit via host_request_quit.
        events.clear();
        poll_platform_events(&mut events);

        // 2. Build the frame input bundle (borrowed slice into `events`).
        let now = std::time::Instant::now();
        let dt = now.duration_since(last_frame).as_secs_f32().min(0.1);
        last_frame = now;

        let frame_input = FrameInput {
            delta_time: dt,
            timestamp_ns: now.duration_since(start).as_nanos() as u64,
            events: events.as_ptr(),
            event_count: events.len(),
            lifecycle: LifecycleFlags {
                should_quit: false as u8,
                surface_changed: 0,
                surface_width: surface.width,
                surface_height: surface.height,
                surface_valid: true as u8,
                _pad: [0],
            },
        };

        // 3. Tick the engine (which will tick the target internally).
        (engine.vtable.update)(engine_state, &frame_input as *const FrameInput);

        // 4. Render.
        (engine.vtable.render)(engine_state);

        // `events` Vec is alive until here — the borrow in frame_input is safe.
    }

    // ---------------------------------------------------------------------------
    // Shutdown (reverse order of init)
    // ---------------------------------------------------------------------------

    // Unload target first so the engine can drain in-flight jobs.
    (engine.vtable.unload_target)(engine_state);
    // `target` DLL is now safe to drop (unmap).
    drop(target);

    // Shut down engine, freeing EngineState.
    (engine.vtable.shutdown)(engine_state);
    // `engine` DLL is now safe to drop (unmap).
    drop(engine);

    // `surface` (window) is safe to destroy now that no Vulkan surface references it.
    drop(surface);

    Ok(())
}
