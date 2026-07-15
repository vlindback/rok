// frame_descriptor.rs
//

use ash::vk;

use crate::backend::frame_ubo::FrameUboBuffer;
use crate::error::{RendererResult, check};

pub(crate) struct FrameDescriptor {
    layout: vk::DescriptorSetLayout,
    pub pool: vk::DescriptorPool,
    pub sets: Vec<vk::DescriptorSet>, // one per frame-in-flight copy
}

impl FrameDescriptor {
    pub(crate) fn new(
        device: &ash::Device,
        frame_ubos: &[FrameUboBuffer],
        light_buffer: vk::Buffer,
    ) -> RendererResult<Self> {
        let count = frame_ubos.len() as u32;

        let bindings = [
            // binding 0: per-frame data (view_proj + camera), vertex + fragment
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT),
            // binding 1: lights (static for now), fragment only
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
        ];
        let layout = check!(
            unsafe {
                device.create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings),
                    None,
                )
            },
            "frame set layout"
        )?;

        // Pool: N frame UBOs + N light-binding descriptors.
        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(count),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(count),
        ];
        let pool = check!(
            unsafe {
                device.create_descriptor_pool(
                    &vk::DescriptorPoolCreateInfo::default()
                        .max_sets(count)
                        .pool_sizes(&pool_sizes),
                    None,
                )
            },
            "frame pool"
        )?;

        let layouts = vec![layout; count as usize];
        let sets = check!(
            unsafe {
                device.allocate_descriptor_sets(
                    &vk::DescriptorSetAllocateInfo::default()
                        .descriptor_pool(pool)
                        .set_layouts(&layouts),
                )
            },
            "frame sets"
        )?;

        for (i, ubo) in frame_ubos.iter().enumerate() {
            let frame_info = vk::DescriptorBufferInfo::default()
                .buffer(ubo.buffer)
                .offset(0)
                .range(vk::WHOLE_SIZE);
            let light_info = vk::DescriptorBufferInfo::default()
                .buffer(light_buffer)
                .offset(0)
                .range(vk::WHOLE_SIZE);
            let writes = [
                vk::WriteDescriptorSet::default()
                    .dst_set(sets[i])
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(std::slice::from_ref(&frame_info)),
                vk::WriteDescriptorSet::default()
                    .dst_set(sets[i])
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(std::slice::from_ref(&light_info)),
            ];
            unsafe { device.update_descriptor_sets(&writes, &[]) };
        }

        Ok(Self { layout, pool, sets })
    }

    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        device.destroy_descriptor_pool(self.pool, None);
        device.destroy_descriptor_set_layout(self.layout, None);
    }

    #[inline]
    pub(crate) fn layout(&self) -> vk::DescriptorSetLayout {
        self.layout
    }
}
