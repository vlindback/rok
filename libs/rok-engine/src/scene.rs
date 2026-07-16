// scene.rs
//

use std::default;

use rok_math::vec3::Vec3;
use rok_renderer::mesh_handle::MeshHandle;

use crate::instance::Instance;

pub struct Scene {
    pub(crate) instances: Vec<Instance>,
}

impl Scene {
    pub(crate) fn new() -> Self {
        Self {
            instances: Vec::new(),
        }
    }

    pub(crate) fn add_instance(&mut self, instance: Instance) {
        self.instances.push(instance);
    }

    pub(crate) fn test_scene(test_model: MeshHandle) -> Self {
        let mut scene = Self::default();
        scene.add_instance(Instance::new(test_model, Vec3::zero()));
        scene
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}
