// frame_ubo.rs
//

use ash::vk;

use crate::backend::buffer::find_memory_type;
use crate::error::{RendererResult, check};

/// Per-frame shader data.
#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct FrameUbo {
    pub view_proj: [f32; 16],
    pub camera_pos: [f32; 4], // xyz + pad
}

pub(crate) struct FrameUboBuffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub mapped: *mut FrameUbo, // valid for the buffer's whole lifetime
}

impl FrameUboBuffer {
    pub(crate) fn new(
        device: &ash::Device,
        mem_props: &vk::PhysicalDeviceMemoryProperties,
    ) -> RendererResult<Self> {
        let size = std::mem::size_of::<FrameUbo>() as vk::DeviceSize;

        let info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buffer = check!(
            unsafe { device.create_buffer(&info, None) },
            "create frame ubo"
        )?;

        let reqs = unsafe { device.get_buffer_memory_requirements(buffer) };
        // HOST_VISIBLE so the CPU can write it every frame; COHERENT so writes
        // are visible to the GPU without an explicit flush.
        let mem_type = find_memory_type(
            mem_props,
            reqs.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        let alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(reqs.size)
            .memory_type_index(mem_type);
        let memory = check!(
            unsafe { device.allocate_memory(&alloc, None) },
            "allocate frame ubo mem"
        )?;
        check!(
            unsafe { device.bind_buffer_memory(buffer, memory, 0) },
            "bind frame ubo mem"
        )?;

        // Map once, keep the pointer for the buffer's lifetime.
        let mapped = check!(
            unsafe { device.map_memory(memory, 0, size, vk::MemoryMapFlags::empty()) },
            "map frame ubo"
        )? as *mut FrameUbo;

        Ok(Self {
            buffer,
            memory,
            mapped,
        })
    }

    /// Write this frame's data through the persistent mapping.
    #[inline]
    pub(crate) fn write(&self, data: &FrameUbo) {
        unsafe { std::ptr::write(self.mapped, *data) };
    }

    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        // unmap happens implicitly on free, but be explicit.
        device.unmap_memory(self.memory);
        device.destroy_buffer(self.buffer, None);
        device.free_memory(self.memory, None);
    }
}
