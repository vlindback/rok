// command.rs
//

// The renderer's command vocabulary.

use rok_math::mat4x4::Mat4x4;

use crate::backend::material_registry::MaterialHandle;
use crate::backend::mesh_registry::MeshHandle;

#[derive(Copy, Clone)]
pub enum RenderCommand {
    DrawMesh {
        mesh: MeshHandle,
        material: MaterialHandle,
        model: Mat4x4,
    },
}
