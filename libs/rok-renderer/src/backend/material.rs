// material.rs
//
// A material: the surface's texture maps grouped into one descriptor set.
//

use ash::vk;

use crate::backend::texture::Texture;
use crate::error::{RendererResult, check};

/// Decoded RGBA8 image for one material slot.
pub(crate) struct MapImage<'a> {
    pub width: u32,
    pub height: u32,
    pub pixels: &'a [u8],
}

pub(crate) struct Material {
    albedo: Texture,
    normal: Texture,
    roughness: Texture,
    layout: vk::DescriptorSetLayout,
    pool: vk::DescriptorPool,
    pub set: vk::DescriptorSet,
}

impl Material {
    pub(crate) fn new(
        device: &ash::Device,
        mem_props: &vk::PhysicalDeviceMemoryProperties,
        queue: vk::Queue,
        queue_family: u32,
        albedo: Option<MapImage>,
        normal: Option<MapImage>,
        roughness: Option<MapImage>,
    ) -> RendererResult<Self> {
        //

        let albedo = slot(
            device,
            mem_props,
            queue,
            queue_family,
            albedo,
            vk::Format::R8G8B8A8_SRGB,
            [255, 255, 255, 255],
        )?;
        let normal = slot(
            device,
            mem_props,
            queue,
            queue_family,
            normal,
            vk::Format::R8G8B8A8_UNORM,
            [128, 128, 255, 255],
        )?; // flat normal
        let roughness = slot(
            device,
            mem_props,
            queue,
            queue_family,
            roughness,
            vk::Format::R8G8B8A8_UNORM,
            [128, 128, 128, 255],
        )?; // mid roughness

        let bindings: Vec<_> = (0..3u32)
            .map(|i| {
                vk::DescriptorSetLayoutBinding::default()
                    .binding(i)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            })
            .collect();

        let layout = check!(
            unsafe {
                device.create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings),
                    None,
                )
            },
            "material set layout"
        )?;

        let pool_size = vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(3);
        let pool = check!(
            unsafe {
                device.create_descriptor_pool(
                    &vk::DescriptorPoolCreateInfo::default()
                        .max_sets(1)
                        .pool_sizes(std::slice::from_ref(&pool_size)),
                    None,
                )
            },
            "material pool"
        )?;

        let set = check!(
            unsafe {
                device.allocate_descriptor_sets(
                    &vk::DescriptorSetAllocateInfo::default()
                        .descriptor_pool(pool)
                        .set_layouts(std::slice::from_ref(&layout)),
                )
            },
            "material set"
        )?[0];

        let infos = [info(&albedo), info(&normal), info(&roughness)];
        let writes: Vec<_> = (0..3usize)
            .map(|i| {
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(i as u32)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(std::slice::from_ref(&infos[i]))
            })
            .collect();
        unsafe { device.update_descriptor_sets(&writes, &[]) };

        Ok(Self {
            albedo,
            normal,
            roughness,
            layout,
            pool,
            set,
        })
    }

    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            self.albedo.destroy(device);
            self.normal.destroy(device);
            self.roughness.destroy(device);
            device.destroy_descriptor_set_layout(self.layout, None);
            device.destroy_descriptor_pool(self.pool, None);
        }
    }

    #[inline]
    pub(crate) fn layout(&self) -> vk::DescriptorSetLayout {
        self.layout
    }
}

// Private functions

fn slot(
    device: &ash::Device,
    mem_props: &vk::PhysicalDeviceMemoryProperties,
    queue: vk::Queue,
    queue_family: u32,
    image: Option<MapImage>,
    format: vk::Format,
    default_rgba: [u8; 4],
) -> RendererResult<Texture> {
    match image {
        Some(img) => Texture::from_rgba8(
            device,
            mem_props,
            queue,
            queue_family,
            img.width,
            img.height,
            img.pixels,
            format,
        ),
        None => Texture::from_rgba8(
            device,
            mem_props,
            queue,
            queue_family,
            1,
            1,
            &default_rgba,
            format,
        ),
    }
}

fn info(tex: &Texture) -> vk::DescriptorImageInfo {
    vk::DescriptorImageInfo::default()
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .image_view(tex.view)
        .sampler(tex.sampler)
}
