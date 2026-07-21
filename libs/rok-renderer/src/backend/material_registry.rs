// material_registry.rs

use crate::backend::material::Material;

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct MaterialHandle(pub(crate) u32);

pub(crate) struct MaterialRegistry {
    materials: Vec<Material>,
}

impl MaterialRegistry {
    pub(crate) fn new() -> Self {
        Self {
            materials: Vec::new(),
        }
    }

    pub(crate) fn add(&mut self, material: Material) -> MaterialHandle {
        let handle = MaterialHandle(self.materials.len() as u32);
        self.materials.push(material);
        handle
    }

    pub(crate) fn get(&self, handle: MaterialHandle) -> &Material {
        &self.materials[handle.0 as usize]
    }

    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        for m in &mut self.materials {
            unsafe { m.destroy(device) };
        }
        self.materials.clear();
    }
}
