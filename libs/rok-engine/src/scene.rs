// scene.rs
//
// v0 scene: a flat list of instance transforms. Deliberately minimal.
//

use crate::transform::Transform;

pub struct Scene {
    pub instances: Vec<Transform>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
        }
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}
