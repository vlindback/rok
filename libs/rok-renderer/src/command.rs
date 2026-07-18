// command.rs
//

// The renderer's command vocabulary.

use rok_math::mat4x4::Mat4x4;

use crate::mesh_handle::MeshHandle;

#[derive(Copy, Clone)]
pub enum RenderCommand {
    DrawMesh { mesh: MeshHandle, model: Mat4x4 },
}
