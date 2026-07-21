// material.rs
//
// A material: the surface's texture maps grouped into one descriptor set.
//

use ash::vk;

use crate::backend::buffer::{Buffer, upload_via_staging};
use crate::backend::texture::Texture;
use crate::error::{RendererResult, check};

#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct MaterialFactors {
    pub base_color_factor: [f32; 4], // offset 0
    pub emissive_factor: [f32; 4],   // offset 16  (rgb + pad in [3])
    pub metallic_factor: f32,        // offset 32
    pub roughness_factor: f32,       // offset 36
    pub normal_scale: f32,           // offset 40
    pub occlusion_strength: f32,     // offset 44
}

// MaterialFactors must match byte for byte inside the forwarding shader.
const _: () = assert!(std::mem::size_of::<MaterialFactors>() == 48);

impl MaterialFactors {
    fn new(
        base_color_factor: [f32; 4],
        emissive_factor: [f32; 3],
        metallic_factor: f32,
        roughness_factor: f32,
        normal_scale: f32,
        occlusion_strength: f32,
    ) -> Self {
        Self {
            base_color_factor,
            emissive_factor: [
                emissive_factor[0],
                emissive_factor[1],
                emissive_factor[2],
                0.0,
            ],
            metallic_factor,
            roughness_factor,
            normal_scale,
            occlusion_strength,
        }
    }
}

/// The descriptor-set-0 layout shared by every material.
pub(crate) struct MaterialLayout {
    layout: vk::DescriptorSetLayout,
}

impl MaterialLayout {
    pub(crate) fn new(device: &ash::Device) -> RendererResult<Self> {
        let bindings = [
            // Samplers
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(3)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            // Uniform buffer
            vk::DescriptorSetLayoutBinding::default()
                .binding(4)
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
            "material set layout"
        )?;

        Ok(Self { layout })
    }

    #[inline]
    pub(crate) fn handle(&self) -> vk::DescriptorSetLayout {
        self.layout
    }

    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        unsafe { device.destroy_descriptor_set_layout(self.layout, None) };
    }
}

/// Decoded RGBA8 image for one material slot.
pub struct MapImage<'a> {
    pub width: u32,
    pub height: u32,
    pub pixels: &'a [u8],
}

pub struct Material {
    albedo: Texture,
    normal: Texture,
    roughness: Texture,
    emissive: Texture,
    pool: vk::DescriptorPool,
    factors_buffer: Buffer,
    pub(crate) set: vk::DescriptorSet,
}

impl Material {
    pub(crate) fn new(
        device: &ash::Device,
        mem_props: &vk::PhysicalDeviceMemoryProperties,
        queue: vk::Queue,
        queue_family: u32,
        layout: &MaterialLayout,
        albedo: Option<MapImage>,
        normal: Option<MapImage>,
        roughness: Option<MapImage>,
        emissive: Option<MapImage>,
        base_color_factor: [f32; 4],
        emissive_factor: [f32; 3],
        metallic_factor: f32,
        roughness_factor: f32,
        normal_scale: f32,
        occlusion_strength: f32,
    ) -> RendererResult<Self> {
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
        )?;
        let roughness = slot(
            device,
            mem_props,
            queue,
            queue_family,
            roughness,
            vk::Format::R8G8B8A8_UNORM,
            [128, 128, 128, 255],
        )?;

        let emissive = slot(
            device,
            mem_props,
            queue,
            queue_family,
            emissive,
            vk::Format::R8G8B8A8_SRGB, // emissive is COLOR (light output) -> sRGB, like albedo
            [0, 0, 0, 255],            // fallback: BLACK = no emission (identity for addition)
        )?;

        let factors = MaterialFactors::new(
            base_color_factor,
            emissive_factor,
            metallic_factor,
            roughness_factor,
            normal_scale,
            occlusion_strength,
        );

        let factors_buffer = upload_via_staging(
            device,
            mem_props,
            queue,
            queue_family,
            std::slice::from_ref(&factors),
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        )?;

        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(4),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1),
        ];

        let pool = check!(
            unsafe {
                device.create_descriptor_pool(
                    &vk::DescriptorPoolCreateInfo::default()
                        .max_sets(1)
                        .pool_sizes(&pool_sizes),
                    None,
                )
            },
            "material pool"
        )?;

        // Allocate the set against the SHARED layout. Bind the handle to a local
        // so the &-ref handed to set_layouts points at a named value, not a temporary.
        let set_layout = layout.handle();
        let set = check!(
            unsafe {
                device.allocate_descriptor_sets(
                    &vk::DescriptorSetAllocateInfo::default()
                        .descriptor_pool(pool)
                        .set_layouts(std::slice::from_ref(&set_layout)),
                )
            },
            "material set"
        )?[0];

        let infos = [
            vk::DescriptorImageInfo::default()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(albedo.view)
                .sampler(albedo.sampler),
            vk::DescriptorImageInfo::default()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(normal.view)
                .sampler(normal.sampler),
            vk::DescriptorImageInfo::default()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(roughness.view)
                .sampler(roughness.sampler),
            vk::DescriptorImageInfo::default()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(emissive.view)
                .sampler(emissive.sampler),
        ];

        let buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(factors_buffer.buffer)
            .offset(0)
            .range(std::mem::size_of::<MaterialFactors>() as u64);

        let writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&infos[0])),
            vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&infos[1])),
            vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(2)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&infos[2])),
            vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(3)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&infos[3])),
            vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(4)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(std::slice::from_ref(&buffer_info)),
        ];

        unsafe { device.update_descriptor_sets(&writes, &[]) };

        Ok(Self {
            albedo,
            normal,
            roughness,
            emissive,
            pool,
            factors_buffer,
            set,
        })
    }

    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            self.factors_buffer.destroy(device);
            self.albedo.destroy(device);
            self.normal.destroy(device);
            self.roughness.destroy(device);
            self.emissive.destroy(device);
            device.destroy_descriptor_pool(self.pool, None);
        }
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
