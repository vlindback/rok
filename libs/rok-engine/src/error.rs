// error.rs (engine)

use std::fmt;

#[derive(Debug)]
pub enum EngineError {
    Library(libloading::Error),
    Io(std::io::Error),
    Renderer(rok_renderer::RendererError),
    Utf8(std::str::Utf8Error),
    FromUtf8(std::string::FromUtf8Error),
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

impl From<rok_renderer::RendererError> for EngineError {
    fn from(err: rok_renderer::RendererError) -> Self {
        EngineError::Renderer(err)
    }
}

impl From<std::str::Utf8Error> for EngineError {
    fn from(err: std::str::Utf8Error) -> Self {
        EngineError::Utf8(err)
    }
}

impl From<std::string::FromUtf8Error> for EngineError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        EngineError::FromUtf8(err)
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EngineError::Library(e) => write!(f, "Library loading error: {}", e),
            EngineError::Io(e) => write!(f, "I/O error: {}", e),
            EngineError::Renderer(e) => write!(f, "Renderer error: {}", e),
            EngineError::Utf8(e) => write!(f, "UTF-8 decoding error: {}", e),
            EngineError::FromUtf8(e) => write!(f, "UTF-8 conversion error: {}", e),
            EngineError::EngineInitFailure => write!(f, "The engine failed to start."),
            EngineError::TargetInitFailure => write!(f, "Could not find the target file."),
        }
    }
}

impl std::error::Error for EngineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EngineError::Library(e) => Some(e),
            EngineError::Io(e) => Some(e),
            EngineError::Renderer(e) => Some(e),
            EngineError::Utf8(e) => Some(e),
            EngineError::FromUtf8(e) => Some(e),
            _ => None,
        }
    }
}
