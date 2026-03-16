// win32.rs

// Abandon all hope ye who enter here

use std::sync::Once;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow, SetProcessDpiAwarenessContext,
};
use windows_sys::Win32::UI::Input::{
    GetRawInputData, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RID_INPUT, RIDEV_INPUTSINK,
    RIM_TYPEKEYBOARD, RIM_TYPEMOUSE, RegisterRawInputDevices,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use rok_abi::input::{
    InputEventData, InputEventKind, KeyEvent, MouseButtonEvent, MouseDeltaEvent, MouseMoveEvent,
    MouseScrollEvent, RawInputEvent,
};
use rok_abi::surface::{NativeSurfaceHandle, SurfaceData, SurfaceType, Win32Surface};

use crate::{PumpResult, WindowError};

// ---------------------------------------------------------------------------
// Event Loop
// ---------------------------------------------------------------------------

pub struct EventLoop {
    window: Option<Box<Window>>,
}

impl EventLoop {
    pub fn new() -> Self {
        Self { window: None }
    }

    pub fn create_window(
        &mut self,
        title: &str,
        width: u32,
        height: u32,
    ) -> Result<&Window, WindowError> {
        let window = Window::create(title, width, height)?;
        self.window = Some(window);
        Ok(self.window.as_ref().unwrap())
    }

    pub fn pump(&mut self, events: &mut Vec<RawInputEvent>) -> PumpResult {
        match self.window.as_mut() {
            Some(window) => window.pump(events),
            None => PumpResult {
                should_quit: false,
                surface_changed: false,
                new_width: 0,
                new_height: 0,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Class registration — once per process
// ---------------------------------------------------------------------------

static REGISTER_CLASS: Once = Once::new();
const CLASS_NAME: &[u16] = &[
    b'r' as u16,
    b'o' as u16,
    b'k' as u16,
    b'_' as u16,
    b'w' as u16,
    b'i' as u16,
    b'n' as u16,
    b'd' as u16,
    b'o' as u16,
    b'w' as u16,
    0,
];

fn register_class(hinstance: HINSTANCE) -> Result<(), WindowError> {
    let mut result = Ok(());

    REGISTER_CLASS.call_once(|| {
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: std::ptr::null_mut(),
            hCursor: unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW) },
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: CLASS_NAME.as_ptr() as *const u16,
            hIconSm: std::ptr::null_mut(),
        };

        // NOTE: WNDCLASSEXW uses wide strings — we need to convert.
        // CLASS_NAME above must be a wide string. See encode_wide() below.
        let atom = unsafe { RegisterClassExW(&wc) };
        if atom == 0 {
            result = Err(WindowError::ClassRegistrationFailed(unsafe {
                windows_sys::Win32::Foundation::GetLastError()
            }));
        }
    });

    result
}

// ---------------------------------------------------------------------------
// Window state
// ---------------------------------------------------------------------------

pub struct Window {
    hwnd: HWND,
    hinstance: HINSTANCE,
    width: u32,
    height: u32,
    dpi: u32,

    // Written by WndProc during DispatchMessage, read by pump after.
    should_quit: bool,
    surface_changed: bool,

    // Temporarily borrowed during pump() only. Null outside of pump.
    // WndProc appends events here directly.
    events_ptr: *mut Vec<RawInputEvent>,
    raw_input_buf: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

impl Window {
    pub(crate) fn create(title: &str, width: u32, height: u32) -> Result<Box<Self>, WindowError> {
        unsafe {
            // Per-monitor DPI awareness — must be set before any window creation.
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

            let hinstance = GetModuleHandleW(std::ptr::null());

            register_class(hinstance)?;

            // Encode title and class name as wide strings.
            let title_wide = encode_wide(title);
            let class_wide = encode_wide("rok_window");

            // Calculate window rect from desired client area.
            let style = WS_OVERLAPPEDWINDOW;
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: width as i32,
                bottom: height as i32,
            };
            AdjustWindowRect(&mut rect, style, FALSE);

            // Create with null lpParam for now — we'll set GWLP_USERDATA in WM_NCCREATE.
            // We pass a pointer to a partially-initialized Window via lpParam.
            let window = Box::new(Window {
                hwnd: std::ptr::null_mut(),
                hinstance,
                width,
                height,
                dpi: 96,
                should_quit: false,
                surface_changed: false,
                events_ptr: std::ptr::null_mut(),
                raw_input_buf: Vec::with_capacity(256),
            });

            let window_ptr = Box::into_raw(window);

            let hwnd = CreateWindowExW(
                0,
                class_wide.as_ptr(),
                title_wide.as_ptr(),
                style,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                rect.right - rect.left,
                rect.bottom - rect.top,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinstance,
                window_ptr as *mut _,
            );

            if hwnd.is_null() {
                return Err(WindowError::WindowCreationFailed(GetLastError()));
            }

            (*window_ptr).hwnd = hwnd;
            (*window_ptr).dpi = GetDpiForWindow(hwnd);

            register_raw_input(hwnd)?;

            ShowWindow(hwnd, SW_SHOW);

            Ok(Box::from_raw(window_ptr))
        }
    }

    pub fn surface_handle(&self) -> NativeSurfaceHandle {
        NativeSurfaceHandle {
            kind: SurfaceType::Win32,
            data: SurfaceData {
                win32: Win32Surface {
                    hwnd: self.hwnd as *mut _,
                    hinstance: self.hinstance as *mut _,
                },
            },
            width: self.width,
            height: self.height,
        }
    }

    pub(crate) fn pump(&mut self, events: &mut Vec<RawInputEvent>) -> PumpResult {
        // Stash the events pointer so WndProc can append to it.
        self.events_ptr = events as *mut Vec<RawInputEvent>;
        self.surface_changed = false;

        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    self.should_quit = true;
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // Clear the events pointer — WndProc must not touch it outside pump.
        self.events_ptr = std::ptr::null_mut();

        PumpResult {
            should_quit: self.should_quit,
            surface_changed: self.surface_changed,
            new_width: self.width,
            new_height: self.height,
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        if !self.hwnd.is_null() {
            unsafe { DestroyWindow(self.hwnd) };
            self.hwnd = std::ptr::null_mut();
        }
    }
}

// ---------------------------------------------------------------------------
// Raw input registration
// ---------------------------------------------------------------------------

fn register_raw_input(hwnd: HWND) -> Result<(), WindowError> {
    let devices = [
        RAWINPUTDEVICE {
            usUsagePage: 0x01, // Generic desktop
            usUsage: 0x02,     // Mouse
            dwFlags: RIDEV_INPUTSINK,
            hwndTarget: hwnd,
        },
        RAWINPUTDEVICE {
            usUsagePage: 0x01, // Generic desktop
            usUsage: 0x06,     // Keyboard
            dwFlags: RIDEV_INPUTSINK,
            hwndTarget: hwnd,
        },
    ];

    let ok = unsafe {
        RegisterRawInputDevices(
            devices.as_ptr(),
            devices.len() as u32,
            size_of::<RAWINPUTDEVICE>() as u32,
        )
    };

    if ok == 0 {
        return Err(WindowError::RawInputRegistrationFailed(unsafe {
            GetLastError()
        }));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// WndProc
// ---------------------------------------------------------------------------

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Retrieve our Window pointer from GWLP_USERDATA.
    // During WM_NCCREATE it isn't set yet — handle that first.
    if msg == WM_NCCREATE {
        unsafe {
            let create = &*(lparam as *const CREATESTRUCTW);
            let window_ptr = create.lpCreateParams as *mut Window;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, window_ptr as isize);
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
    }

    let window_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Window };

    if window_ptr.is_null() {
        unsafe {
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
    }

    match msg {
        // --- Lifecycle ---
        WM_CLOSE => {
            unsafe {
                (*window_ptr).should_quit = true;
            }
            // Don't call DestroyWindow yet — let the host decide when to exit.
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }

        // --- Surface ---
        WM_SIZE => {
            let new_width = (lparam & 0xFFFF) as u32;
            let new_height = ((lparam >> 16) & 0xFFFF) as u32;
            let is_minimized = wparam == SIZE_MINIMIZED as usize;

            unsafe {
                if !is_minimized
                    && (new_width != (*window_ptr).width || new_height != (*window_ptr).height)
                {
                    (*window_ptr).width = new_width;
                    (*window_ptr).height = new_height;
                    (*window_ptr).surface_changed = true;
                }
            }

            0
        }
        WM_PAINT => {
            // Validate the dirty rect so Windows stops sending WM_PAINT.
            // Actual presentation is handled by Vulkan.
            unsafe {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                BeginPaint(hwnd, &mut ps);
                EndPaint(hwnd, &ps);
            }
            0
        }
        WM_ENTERSIZEMOVE => {
            // TODO: pause simulation / fixed timestep while dragging
            0
        }
        WM_EXITSIZEMOVE => {
            // TODO: resume simulation
            0
        }

        // --- DPI ---
        WM_DPICHANGED => {
            // Windows provides the suggested rect at the new DPI.
            unsafe {
                (*window_ptr).dpi = (wparam & 0xFFFF) as u32;
                let suggested = &*(lparam as *const RECT);
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    suggested.left,
                    suggested.top,
                    suggested.right - suggested.left,
                    suggested.bottom - suggested.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                (*window_ptr).surface_changed = true;
            }
            0
        }

        // --- Focus ---
        WM_SETFOCUS => {
            push_event(
                window_ptr,
                RawInputEvent {
                    kind: InputEventKind::FocusGained,
                    data: InputEventData { _raw: [0; 16] },
                },
            );
            0
        }
        WM_KILLFOCUS => {
            push_event(
                window_ptr,
                RawInputEvent {
                    kind: InputEventKind::FocusLost,
                    data: InputEventData { _raw: [0; 16] },
                },
            );
            0
        }

        // --- Raw input (keyboard + mouse delta) ---
        WM_INPUT => unsafe {
            handle_raw_input(window_ptr, lparam);
            DefWindowProcW(hwnd, msg, wparam, lparam)
        },

        // --- Mouse absolute position ---
        WM_MOUSEMOVE => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            push_event(
                window_ptr,
                RawInputEvent {
                    kind: InputEventKind::MouseMove,
                    data: InputEventData {
                        mouse_move: MouseMoveEvent { x, y },
                    },
                },
            );
            0
        }

        // --- Mouse buttons ---
        WM_LBUTTONDOWN => {
            push_mouse_button(window_ptr, lparam, 0, true);
            0
        }
        WM_LBUTTONUP => {
            push_mouse_button(window_ptr, lparam, 0, false);
            0
        }
        WM_RBUTTONDOWN => {
            push_mouse_button(window_ptr, lparam, 1, true);
            0
        }
        WM_RBUTTONUP => {
            push_mouse_button(window_ptr, lparam, 1, false);
            0
        }
        WM_MBUTTONDOWN => {
            push_mouse_button(window_ptr, lparam, 2, true);
            0
        }
        WM_MBUTTONUP => {
            push_mouse_button(window_ptr, lparam, 2, false);
            0
        }
        WM_XBUTTONDOWN => {
            let button = if (wparam >> 16) & 0x1 != 0 { 3 } else { 4 };
            push_mouse_button(window_ptr, lparam, button, true);
            0
        }
        WM_XBUTTONUP => {
            let button = if (wparam >> 16) & 0x1 != 0 { 3 } else { 4 };
            push_mouse_button(window_ptr, lparam, button, false);
            0
        }

        // --- Scroll ---
        WM_MOUSEWHEEL => {
            let delta = (wparam >> 16) as i16 as f32 / 120.0;
            push_event(
                window_ptr,
                RawInputEvent {
                    kind: InputEventKind::MouseScroll,
                    data: InputEventData {
                        mouse_scroll: MouseScrollEvent {
                            delta_x: 0.0,
                            delta_y: delta,
                        },
                    },
                },
            );
            0
        }
        WM_MOUSEHWHEEL => {
            let delta = (wparam >> 16) as i16 as f32 / 120.0;
            push_event(
                window_ptr,
                RawInputEvent {
                    kind: InputEventKind::MouseScroll,
                    data: InputEventData {
                        mouse_scroll: MouseScrollEvent {
                            delta_x: delta,
                            delta_y: 0.0,
                        },
                    },
                },
            );
            0
        }

        // --- Keyboard (sys keys for Alt+F4, Alt+Enter etc) ---
        WM_SYSKEYDOWN | WM_SYSKEYUP => {
            // Let DefWindowProc handle Alt+F4 etc but still capture for input.
            // TODO: push KeyDown/KeyUp via WM_INPUT instead — syskeydown is a fallback.
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }

        // --- App activation ---
        WM_ACTIVATEAPP => {
            // TODO: pause/resume audio, reset input state on deactivate
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }

        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[inline]
fn push_event(window: *mut Window, event: RawInputEvent) {
    unsafe {
        if !(*window).events_ptr.is_null() {
            (*(*window).events_ptr).push(event);
        }
    }
}

fn push_mouse_button(window: *mut Window, lparam: LPARAM, button: u32, down: bool) {
    let x = (lparam & 0xFFFF) as i16 as i32;
    let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
    let kind = if down {
        InputEventKind::MouseButtonDown
    } else {
        InputEventKind::MouseButtonUp
    };
    push_event(
        window,
        RawInputEvent {
            kind,
            data: InputEventData {
                mouse_button: MouseButtonEvent { button, x, y },
            },
        },
    );
}

unsafe fn handle_raw_input(window: *mut Window, lparam: LPARAM) {
    let mut size: u32 = 0;
    unsafe {
        GetRawInputData(
            lparam as *mut _,
            RID_INPUT,
            std::ptr::null_mut(),
            &mut size,
            size_of::<RAWINPUTHEADER>() as u32,
        )
    };

    if size == 0 {
        return;
    }

    // Grow if needed — only happens if RAWINPUT somehow exceeds 256 bytes,
    // which won't occur in practice for mouse/keyboard.
    unsafe {
        if (*window).raw_input_buf.len() < size as usize {
            (*window).raw_input_buf.resize(size as usize, 0);
        }
    }

    let read = unsafe {
        GetRawInputData(
            lparam as *mut _,
            RID_INPUT,
            (*window).raw_input_buf.as_mut_ptr() as *mut _,
            &mut size,
            size_of::<RAWINPUTHEADER>() as u32,
        )
    };

    if read == u32::MAX {
        return;
    }

    let raw = unsafe { &*((*window).raw_input_buf.as_ptr() as *const RAWINPUT) };

    match raw.header.dwType {
        RIM_TYPEMOUSE => {
            let mouse = unsafe { &raw.data.mouse };
            // MOUSEEVENTF_MOVE, relative movement
            if mouse.usFlags & 0x01 == 0 {
                push_event(
                    window,
                    RawInputEvent {
                        kind: InputEventKind::MouseDelta,
                        data: InputEventData {
                            mouse_delta: MouseDeltaEvent {
                                dx: mouse.lLastX,
                                dy: mouse.lLastY,
                            },
                        },
                    },
                );
            }
        }
        RIM_TYPEKEYBOARD => {
            let kb = unsafe { &raw.data.keyboard };
            let is_down = kb.Message == WM_KEYDOWN || kb.Message == WM_SYSKEYDOWN;
            let kind = if is_down {
                InputEventKind::KeyDown
            } else {
                InputEventKind::KeyUp
            };
            push_event(
                window,
                RawInputEvent {
                    kind,
                    data: InputEventData {
                        key: KeyEvent {
                            scan_code: kb.MakeCode as u32,
                            virtual_key: kb.VKey as u32,
                            is_repeat: 0, // WM_INPUT doesn't give repeat, use WM_KEYDOWN flags if needed
                            _pad: [0; 3],
                        },
                    },
                },
            );
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Wide string encoding
// ---------------------------------------------------------------------------

fn encode_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
