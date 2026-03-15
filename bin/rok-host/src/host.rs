// host.rs

use rok_abi::frame::LifecycleFlags;
use rok_abi::{FrameInput, HostVTable, NativeSurfaceHandle, RawInputEvent};

use crate::engine::engine::Engine;
use crate::engine::target::Target;
use crate::host_error::HostError;
use crate::host_state::HostState;

use crate::host_api::create_host_vtable;

pub(crate) struct Host {
    target: Option<Target>,
    state: HostState,
    _vtable: HostVTable,
    surface: NativeSurfaceHandle,
    engine: Engine,
}

impl Host {
    pub(crate) fn new(engine_path: &str, target_path: &str) -> Result<Box<Self>, HostError> {
        let mut state = HostState { should_quit: false };
        let _vtable = create_host_vtable();
        let surface = create_platform_window();
        let mut engine = Engine::load(engine_path)?;

        let opaque_state: &mut rok_abi::HostState =
            unsafe { &mut *(&mut state as *mut HostState as *mut rok_abi::HostState) };

        engine.init(&_vtable, opaque_state, &surface)?;

        let target = engine.load_target(target_path)?;

        Ok(Box::new(Host {
            target: Some(target),
            state,
            _vtable,
            surface,
            engine,
        }))
    }

    pub(crate) fn main_loop(&self) {
        let mut events: Vec<RawInputEvent> = Vec::with_capacity(256);
        let mut last_frame = std::time::Instant::now();
        let start = std::time::Instant::now();

        while !self.state.should_quit {
            events.clear();
            poll_platform_events(&mut events);

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
                    surface_width: self.surface.width,
                    surface_height: self.surface.height,
                    surface_valid: true as u8,
                    _pad: [0],
                },
            };

            self.engine.update(frame_input);
            self.engine.render();
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.engine.unload_target();
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
// Platform stub (replace with rok-platform implementations)
// ---------------------------------------------------------------------------

/// Placeholder: in the real implementation this creates a Win32 / Wayland window
/// and returns its native handles. For now it returns a zeroed handle so the
/// rest of the host structure compiles and can be tested without a GPU.
fn create_platform_window() -> NativeSurfaceHandle {
    use rok_abi::surface::{SurfaceData, SurfaceType, Win32Surface};
    NativeSurfaceHandle {
        kind: SurfaceType::Win32,
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
