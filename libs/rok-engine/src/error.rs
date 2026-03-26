// error.rs (engine)

use std::fmt;

#[derive(Debug)]
pub enum EngineError {
    Library(libloading::Error),
    Io(std::io::Error),
    Renderer(rok_renderer::error::RendererError),
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

impl From<rok_renderer::error::RendererError> for EngineError {
    fn from(err: rok_renderer::error::RendererError) -> Self {
        EngineError::Renderer(err)
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EngineError::Library(e) => write!(f, "Library loading error: {}", e),
            EngineError::Io(e) => write!(f, "I/O error: {}", e),
            EngineError::Renderer(e) => write!(f, "Renderer error: {}", e),
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
            _ => None,
        }
    }
}
