// error.rs (engine)

use std::fmt;

#[derive(Debug)]
pub enum EngineError {
    Library(libloading::Error),
    Io(std::io::Error),
    EngineInitFailure,
    TargetInitFailure,
}

impl From<std::io::Error> for EngineError {
    fn from(err: std::io::Error) -> Self {
        EngineError::Io(err)
    }
}

impl From<libloading::Error> for EngineError {
    fn from(err: libloading::Error) -> Self {
        EngineError::Library(err)
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EngineError::Library(e) => write!(f, "Library loading error: {}", e),
            EngineError::Io(e) => write!(f, "I/O error: {}", e),
            EngineError::EngineInitFailure => write!(f, "The engine failed to start."),
            EngineError::TargetInitFailure => write!(f, "Could not find the target file."),
        }
    }
}

impl std::error::Error for EngineError {
    // This allows error reporters to see the nested error
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EngineError::Library(e) => Some(e),
            EngineError::Io(e) => Some(e),
            _ => None,
        }
    }
}
