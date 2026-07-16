// instance.rs
//

use rok_math::vec3::Vec3;
use rok_renderer::mesh_handle::MeshHandle;

use crate::transform::Transform;

pub(crate) struct Instance {
    pub(crate) transform: Transform,
    pub(crate) mesh: MeshHandle,
}

impl Instance {
    pub(crate) fn new(mesh: MeshHandle, pos: Vec3) -> Self {
        Self {
            transform: Transform::from_position(pos),
            mesh,
        }
    }
}
