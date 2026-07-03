// input.rs
// rok-engine

use rok_abi::input::{InputEventKind, KeyboardState, MouseState, RawInputEvent};

pub struct InputState {
    pub keyboard: KeyboardState,
    pub mouse: MouseState,
}

impl InputState {
    pub fn new() -> Self {
        // All-zero is a valid resting state for both (nothing down, cursor at
        // origin, no deltas). Both are repr(C) POD, so zeroing is sound.
        Self {
            keyboard: unsafe { core::mem::zeroed() },
            mouse: unsafe { core::mem::zeroed() },
        }
    }

    /// Fold one frame's raw events into the aggregated state. Call once per
    /// frame, before anything reads the state.
    pub fn ingest(&mut self, events: &[RawInputEvent]) {
        // Per-frame fields reset; *_down and cursor x/y persist.
        self.keyboard.keys_pressed = [0; 32];
        self.keyboard.keys_released = [0; 32];
        self.mouse.buttons_pressed = 0;
        self.mouse.buttons_released = 0;
        self.mouse.delta_x = 0;
        self.mouse.delta_y = 0;
        self.mouse.scroll_x = 0.0;
        self.mouse.scroll_y = 0.0;

        for ev in events {
            // Safety:
            match ev.kind {
                InputEventKind::KeyDown => {
                    let key = unsafe { ev.data.key };
                    if let Some(sc) = scan_bit(key.scan_code) {
                        if !bit_get(&self.keyboard.keys_down, sc) {
                            bit_set(&mut self.keyboard.keys_pressed, sc); // edge only on transition
                        }
                        bit_set(&mut self.keyboard.keys_down, sc);
                    }
                }
                InputEventKind::KeyUp => {
                    let key = unsafe { ev.data.key };
                    if let Some(sc) = scan_bit(key.scan_code) {
                        if bit_get(&self.keyboard.keys_down, sc) {
                            bit_set(&mut self.keyboard.keys_released, sc);
                        }
                        bit_clear(&mut self.keyboard.keys_down, sc);
                    }
                }
                InputEventKind::MouseDelta => {
                    let d = unsafe { ev.data.mouse_delta };
                    self.mouse.delta_x += d.dx;
                    self.mouse.delta_y += d.dy;
                }
                InputEventKind::MouseMove => {
                    let m = unsafe { ev.data.mouse_move };
                    self.mouse.x = m.x;
                    self.mouse.y = m.y;
                }
                InputEventKind::MouseButtonDown => {
                    let b = unsafe { ev.data.mouse_button };
                    if let Some(mask) = button_mask(b.button) {
                        if self.mouse.buttons_down & mask == 0 {
                            self.mouse.buttons_pressed |= mask;
                        }
                        self.mouse.buttons_down |= mask;
                    }
                }
                InputEventKind::MouseButtonUp => {
                    let b = unsafe { ev.data.mouse_button };
                    if let Some(mask) = button_mask(b.button) {
                        if self.mouse.buttons_down & mask != 0 {
                            self.mouse.buttons_released |= mask;
                        }
                        self.mouse.buttons_down &= !mask;
                    }
                }
                InputEventKind::MouseScroll => {
                    let s = unsafe { ev.data.mouse_scroll };
                    self.mouse.scroll_x += s.delta_x;
                    self.mouse.scroll_y += s.delta_y;
                }
                InputEventKind::FocusLost => {
                    // OS stops sending key-up while unfocused, drop held state
                    // so nothing sticks down across an alt-tab.
                    self.keyboard.keys_down = [0; 32];
                    self.mouse.buttons_down = 0;
                }
                InputEventKind::FocusGained => {}
            }
        }
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

/// Scan code → bit index in the 256-bit set, or None if out of range.
/// Extended keys above 0xFF (arrows, numpad) aren't mapped yet — fine for
/// WASD-era camera control.
#[inline]
fn scan_bit(scan_code: u32) -> Option<u8> {
    (scan_code < 256).then_some(scan_code as u8)
}

/// Button index → mask (0=L,1=R,2=M,3/4=X1/X2). buttons_* are u8.
#[inline]
fn button_mask(button: u32) -> Option<u8> {
    (button < 5).then(|| 1u8 << button)
}

#[inline]
fn bit_get(bits: &[u8; 32], i: u8) -> bool {
    bits[(i >> 3) as usize] & (1 << (i & 7)) != 0
}
#[inline]
fn bit_set(bits: &mut [u8; 32], i: u8) {
    bits[(i >> 3) as usize] |= 1 << (i & 7);
}
#[inline]
fn bit_clear(bits: &mut [u8; 32], i: u8) {
    bits[(i >> 3) as usize] &= !(1 << (i & 7));
}
