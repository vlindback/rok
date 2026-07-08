// lib.rs
// rok-renderer
//

mod backend;
mod command;
mod error;
mod renderer;

pub use command::RenderCommand;
pub use error::{RendererError, RendererResult};
pub use renderer::{Renderer, RendererConfig};

use crate::backend::light::LightUbo;
