
// error.rs

use std::fmt;

#[derive(Debug)]
pub enum WindowError {
    ClassRegistrationFailed(u32),
    WindowCreationFailed(u32),
    RawInputRegistrationFailed(u32),
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WindowError::ClassRegistrationFailed(code) =>
                write!(f, "Failed to register window class (error {})", code),
            WindowError::WindowCreationFailed(code) =>
                write!(f, "Failed to create window (error {})", code),
            WindowError::RawInputRegistrationFailed(code) =>
                write!(f, "Failed to register raw input devices (error {})", code),
        }
    }
}

impl std::error::Error for WindowError {}