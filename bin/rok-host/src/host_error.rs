// host_error.rs

use rok_engine::error::EngineError;
use std::fmt;

#[derive(Debug)]
pub(crate) enum HostError {
    Io(std::io::Error),
    EngineError(EngineError),
    ConfigMissingKey(&'static str),
    Window(rok_window::WindowError),
}

impl From<std::io::Error> for HostError {
    fn from(err: std::io::Error) -> Self {
        HostError::Io(err)
    }
}

impl From<rok_window::WindowError> for HostError {
    fn from(err: rok_window::WindowError) -> Self {
        HostError::Window(err)
    }
}

impl From<EngineError> for HostError {
    fn from(err: EngineError) -> Self {
        HostError::EngineError(err)
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HostError::Io(e) => write!(f, "I/O error: {}", e),
            HostError::EngineError(e) => write!(f, "Engine error: {}", e),
            HostError::ConfigMissingKey(k) => write!(f, "Missing key in config: {}", k),
            HostError::Window(e) => write!(f, "Window error: {}", e),
        }
    }
}

impl std::error::Error for HostError {
    // This allows error reporters to see the nested error
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HostError::Io(e) => Some(e),
            HostError::EngineError(e) => Some(e),
            _ => None,
        }
    }
}
