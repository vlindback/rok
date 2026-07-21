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
pub use geometry::cube;
pub use renderer::{Renderer, RendererConfig};

use crate::backend::light::{GpuLight, LightsUbo};
use crate::backend::light::{LIGHT_DIRECTIONAL, LIGHT_POINT, MAX_LIGHTS};

// exports
pub use backend::material::MapImage;
pub use renderer::MaterialCreateInfo;

// handles
pub use backend::material_registry::MaterialHandle;
pub use backend::mesh_registry::MeshHandle;
