// scene.rs
//

use std::default;

use rok_math::{mat4x4::Mat4x4, vec3::Vec3};
use rok_renderer::{MaterialHandle, MeshHandle};

use crate::{instance::Instance, model_registry::ModelHandle, transform::Transform};

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

    pub fn spawn(&mut self, model: ModelHandle, world: Transform) {
        let inst = Instance::new(model, world);
        self.instances.push(inst);
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}
