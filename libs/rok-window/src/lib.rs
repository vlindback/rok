// lib.rs

mod error;

#[cfg(target_os = "windows")]
mod win32;

pub use error::WindowError;

#[cfg(target_os = "windows")]
pub use win32::{EventLoop, Window};

use rok_abi::input::RawInputEvent;

pub struct PumpResult {
    pub should_quit: bool,
    pub surface_changed: bool,
    pub new_width: u32,
    pub new_height: u32,
}
