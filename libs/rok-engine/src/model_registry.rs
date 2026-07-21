use rok_renderer::{MaterialHandle, MeshHandle};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ModelHandle(pub(crate) u32);

/// A registered model: its parts as resolved GPU handles. No CPU data.
pub(crate) struct RegisteredModel {
    pub(crate) parts: Vec<(MeshHandle, MaterialHandle)>,
}

pub(crate) struct ModelRegistry {
    models: Vec<RegisteredModel>,
}

impl ModelRegistry {
    pub(crate) fn new() -> Self {
        Self { models: Vec::new() }
    }

    pub(crate) fn add(&mut self, parts: Vec<(MeshHandle, MaterialHandle)>) -> ModelHandle {
        let h = ModelHandle(self.models.len() as u32);
        self.models.push(RegisteredModel { parts });
        h
    }

    pub(crate) fn get(&self, h: ModelHandle) -> &RegisteredModel {
        &self.models[h.0 as usize]
    }
}
