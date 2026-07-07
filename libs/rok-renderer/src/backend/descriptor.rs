// descriptor.rs
//

use ash::vk;

use crate::error::{RendererResult, check};

pub(crate) struct TextureDescriptor {
    pub layout: vk::DescriptorSetLayout,
    pub pool: vk::DescriptorPool,
    pub set: vk::DescriptorSet,
}

impl TextureDescriptor {
    /// Layout = one combined image sampler at binding 0, fragment stage.
    /// Allocates a single set from a one-set pool and points it at
    /// `view` + `sampler`.
    pub(crate) fn new(
        device: &ash::Device,
        view: vk::ImageView,
        sampler: vk::Sampler,
    ) -> RendererResult<Self> {
        // --- layout: the shape a shader expects ---
        let binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);
        let layout_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(std::slice::from_ref(&binding));
        let layout = check!(
            unsafe { device.create_descriptor_set_layout(&layout_info, None) },
            "create descriptor set layout"
        )?;

        // --- pool: capacity for one combined-image-sampler set ---
        let pool_size = vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1);
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(1)
            .pool_sizes(std::slice::from_ref(&pool_size));
        let pool = check!(
            unsafe { device.create_descriptor_pool(&pool_info, None) },
            "create descriptor pool"
        )?;

        // --- allocate one set from the pool ---
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(pool)
            .set_layouts(std::slice::from_ref(&layout));
        let set = check!(
            unsafe { device.allocate_descriptor_sets(&alloc_info) },
            "allocate descriptor set"
        )?[0];

        // --- point the set at the actual texture ---
        let image_info = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(view)
            .sampler(sampler);
        let write = vk::WriteDescriptorSet::default()
            .dst_set(set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&image_info));
        unsafe { device.update_descriptor_sets(std::slice::from_ref(&write), &[]) };

        Ok(Self { layout, pool, set })
    }

    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        // Destroying the pool frees its sets; the layout is separate.
        device.destroy_descriptor_pool(self.pool, None);
        device.destroy_descriptor_set_layout(self.layout, None);
    }
}
