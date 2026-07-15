// lib.rs
// rok-renderer
//

mod backend;
mod command;
mod error;
mod geometry;
mod renderer;

pub use command::RenderCommand;
pub use error::{RendererError, RendererResult};
pub use renderer::{Renderer, RendererConfig};

use crate::backend::light::{GpuLight, LightsUbo};
use crate::backend::light::{LIGHT_DIRECTIONAL, LIGHT_POINT, MAX_LIGHTS};
