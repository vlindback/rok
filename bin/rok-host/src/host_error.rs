// host_error.rs

use std::fmt;

#[derive(Debug)]
pub(crate) enum HostError {
    Library(libloading::Error),
    Io(std::io::Error),
    EngineInitFailure,
    TargetInitFailure,
    ConfigMissingKey(&'static str),
}

impl From<std::io::Error> for HostError {
    fn from(err: std::io::Error) -> Self {
        HostError::Io(err)
    }
}

impl From<libloading::Error> for HostError {
    fn from(err: libloading::Error) -> Self {
        HostError::Library(err)
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HostError::Library(e) => write!(f, "Library loading error: {}", e),
            HostError::Io(e) => write!(f, "I/O error: {}", e),
            HostError::EngineInitFailure => write!(f, "The engine failed to start."),
            HostError::TargetInitFailure => write!(f, "Could not find the target file."),
            HostError::ConfigMissingKey(k) => write!(f, "Missing key in config: {}", k),
        }
    }
}

impl std::error::Error for HostError {
    // This allows error reporters to see the nested error
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HostError::Library(e) => Some(e),
            HostError::Io(e) => Some(e),
            _ => None,
        }
    }
}
