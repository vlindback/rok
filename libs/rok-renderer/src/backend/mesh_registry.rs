// mesh_registry.rs

use crate::{
    RendererResult,
    backend::buffer::{self, Buffer},
};

use ash::vk;

use rok_mesh::{MeshData, MeshVertex};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct MeshHandle(pub u32); // index into the mesh registry

pub struct GpuMesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
    pub index_type: vk::IndexType,
}

pub(crate) struct MeshRegistry {
    meshes: Vec<GpuMesh>,
}

impl MeshRegistry {
    pub fn new() -> Self {
        Self { meshes: Vec::new() }
    }

    pub fn upload(
        &mut self,
        device: &ash::Device,
        mem_props: &vk::PhysicalDeviceMemoryProperties,
        queue: vk::Queue,
        family: u32,
        data: &MeshData,
    ) -> RendererResult<MeshHandle> {
        // Vertex type layouts need to match. Make sure.

        let vb = buffer::upload_via_staging(
            device,
            mem_props,
            queue,
            family,
            &data.vertices,
            vk::BufferUsageFlags::VERTEX_BUFFER,
        )?;
        let ib = buffer::upload_via_staging(
            device,
            mem_props,
            queue,
            family,
            &data.indices,
            vk::BufferUsageFlags::INDEX_BUFFER,
        )?;
        let handle = MeshHandle(self.meshes.len() as u32);

        let index_type = match data.index_type {
            rok_mesh::IndexType::U16 => vk::IndexType::UINT16,
            rok_mesh::IndexType::U32 => vk::IndexType::UINT32,
        };

        let index_byte_width = match data.index_type {
            rok_mesh::IndexType::U16 => size_of::<u16>(),
            rok_mesh::IndexType::U32 => size_of::<u32>(),
        };

        let index_count = (data.indices.len() / index_byte_width) as u32;

        self.meshes.push(GpuMesh {
            vertex_buffer: vb,
            index_buffer: ib,
            index_count,
            index_type,
        });
        Ok(handle)
    }

    pub fn get(&self, h: MeshHandle) -> &GpuMesh {
        &self.meshes[h.0 as usize]
    }

    pub unsafe fn destroy(&mut self, device: &ash::Device) {
        for m in &mut self.meshes {
            m.vertex_buffer.destroy(device);
            m.index_buffer.destroy(device);
        }
    }
}
