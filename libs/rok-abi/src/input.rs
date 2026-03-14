// input.rs
//
// Raw, lossless input events collected by the Host and forwarded to the Engine.
//
// Design rules:
//   1. NEVER collapse platform details here. Win32 gives us scan codes AND
//      virtual keys — keep both. Wayland gives relative pointer motion —
//      keep it separate from absolute position. The device-abstraction layer
//      that maps these to game actions lives above this, inside the Engine or
//      Target — not here.
//
//   2. All types must be #[repr(C)] with no padding surprises.
//      Use u8 instead of bool to avoid ABI ambiguity across compilers.
//
//   3. Event variants are a tagged union, not a Rust enum, so the layout is
//      stable if new variants are added (old consumers ignore unknown kinds).

/// Discriminant for RawInputEvent.
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InputEventKind {
    KeyDown = 0,
    KeyUp = 1,
    /// Raw relative mouse movement (from WM_INPUT / wp_relative_pointer).
    /// NOT screen-space. Use this for camera control.
    MouseDelta = 2,
    /// Absolute cursor position in physical pixels from top-left of client area.
    MouseMove = 3,
    MouseButtonDown = 4,
    MouseButtonUp = 5,
    MouseScroll = 6,
    /// Window gained OS focus.
    FocusGained = 7,
    /// Window lost OS focus. Treat all keys/buttons as released.
    FocusLost = 8,
}

/// Key event. Carries both the hardware scan code and the platform virtual key.
///
/// Use `scan_code` for anything that should be layout-independent (WASD movement).
/// Use `virtual_key` for anything that should respect the user's keyboard layout
/// (text input, menu shortcuts). The platform-specific value ranges are documented
/// in the host's platform layer, not here.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct KeyEvent {
    /// Hardware scan code. Platform-specific but layout-independent.
    pub scan_code: u32,
    /// Platform virtual key code (VK_* on Win32, XKB keysym on Wayland).
    pub virtual_key: u32,
    /// Non-zero if this is a key-repeat event (held down).
    pub is_repeat: u8,
    pub _pad: [u8; 3],
}

/// Raw relative mouse delta from hardware (not OS-accelerated).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct MouseDeltaEvent {
    /// Signed raw delta in hardware counts.
    pub dx: i32,
    pub dy: i32,
}

/// Absolute cursor position in physical pixels, client-area origin.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct MouseMoveEvent {
    pub x: i32,
    pub y: i32,
}

/// Mouse button indices follow the convention:
///   0 = Left, 1 = Right, 2 = Middle, 3/4 = X1/X2
#[repr(C)]
#[derive(Copy, Clone)]
pub struct MouseButtonEvent {
    pub button: u32,
    /// Cursor position at the moment of the click, physical pixels.
    pub x: i32,
    pub y: i32,
}

/// Scroll wheel delta. Positive Y = scroll up / away from user.
/// `delta_x` is populated by horizontal scroll wheels or touchpad two-finger scroll.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct MouseScrollEvent {
    pub delta_x: f32,
    pub delta_y: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union InputEventData {
    pub key: KeyEvent,
    pub mouse_delta: MouseDeltaEvent,
    pub mouse_move: MouseMoveEvent,
    pub mouse_button: MouseButtonEvent,
    pub mouse_scroll: MouseScrollEvent,
    /// Padding to keep the union a fixed size regardless of which arm is largest.
    /// Guarantees stable ABI if new variants are added later.
    pub _raw: [u8; 16],
}

/// A single raw input event. The Host queues these during its event loop
/// and hands a contiguous slice to the Engine each frame via FrameInput.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct RawInputEvent {
    pub kind: InputEventKind,
    pub data: InputEventData,
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DeviceType {
    Keyboard = 0,
    Mouse = 1,
    Gamepad = 2,
    Touch = 3,
    Unknown = 4,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DeviceState {
    pub device_id: u64,
    pub device_kind: DeviceType,
    pub data: DeviceStateData,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DeviceInfo {
    pub device_id: u64,
    pub device_type: DeviceType,
    pub data: DeviceStateData,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union DeviceStateData {
    pub keyboard: KeyboardState,
    pub mouse: MouseState,
    pub gamepad: GamepadState,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct KeyboardState {
    /// One bit per scan code. 256 scan codes = 32 bytes.
    pub keys_down: [u8; 32],
    /// Keys that transitioned down this frame.
    pub keys_pressed: [u8; 32],
    /// Keys that transitioned up this frame.
    pub keys_released: [u8; 32],
}

impl KeyboardState {
    pub fn is_down(&self, scan_code: u8) -> bool {
        self.keys_down[(scan_code / 8) as usize] & (1 << (scan_code % 8)) != 0
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MouseState {
    /// Absolute cursor position in physical pixels.
    pub x: i32,
    pub y: i32,
    /// Raw hardware delta this frame (not OS accelerated).
    pub delta_x: i32,
    pub delta_y: i32,
    pub scroll_x: f32,
    pub scroll_y: f32,
    /// Bits 0-4 = buttons down, same layout as RawInputEvent.
    pub buttons_down: u8,
    pub buttons_pressed: u8,
    pub buttons_released: u8,
    pub _pad: u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GamepadState {
    pub left_stick_x: f32,
    pub left_stick_y: f32,
    pub right_stick_x: f32,
    pub right_stick_y: f32,
    pub left_trigger: f32,
    pub right_trigger: f32,
    /// Bitmask of digital buttons. Assign bits via GamepadButton enum.
    pub buttons_down: u32,
    pub buttons_pressed: u32,
    pub buttons_released: u32,
}

#[repr(u32)]
pub enum GamepadButton {
    South = 1 << 0, // A / Cross
    East = 1 << 1,  // B / Circle
    West = 1 << 2,  // X / Square
    North = 1 << 3, // Y / Triangle
    L1 = 1 << 4,
    R1 = 1 << 5,
    L3 = 1 << 6,
    R3 = 1 << 7,
    Start = 1 << 8,
    Select = 1 << 9,
    DpadUp = 1 << 10,
    DpadDown = 1 << 11,
    DpadLeft = 1 << 12,
    DpadRight = 1 << 13,
}
