// instance.rs
//

use crate::{model_registry::ModelHandle, transform::Transform};

pub(crate) struct Instance {
    pub(crate) transform: Transform,
    pub(crate) model: ModelHandle,
}

impl Instance {
    pub(crate) fn new(model: ModelHandle, transform: Transform) -> Self {
        Self { transform, model }
    }
}
