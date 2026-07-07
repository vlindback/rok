// command.rs
//

// The renderer's command vocabulary.

use rok_math::mat4x4::Mat4x4;

#[derive(Copy, Clone)]
pub enum RenderCommand {
    DrawMesh { model: Mat4x4 },
}
