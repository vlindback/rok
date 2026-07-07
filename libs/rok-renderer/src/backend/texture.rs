// texture.rs
//
// A sampled 2D texture: device-local image + view + sampler. The image twin
// of buffer.rs — same staging-upload shape, but images have a LAYOUT, so the
// upload brackets the copy with two barriers (UNDEFINED -> TRANSFER_DST ->
// SHADER_READ_ONLY) instead of a bare buffer copy.

use ash::vk;

use crate::backend::buffer::{self, find_memory_type};
use crate::error::{RendererError, RendererResult, check};

pub(crate) struct Texture {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
}

impl Texture {
    /// Create a device-local sampled texture from tightly-packed RGBA8 pixels
    /// (`width * height * 4` bytes). SRGB format so the GPU linearizes on
    /// sample — correct for albedo/color textures like a photo.
    pub(crate) fn from_rgba8(
        device: &ash::Device,
        mem_props: &vk::PhysicalDeviceMemoryProperties,
        queue: vk::Queue,
        queue_family: u32,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> RendererResult<Self> {
        let expected = (width as usize) * (height as usize) * 4;
        if pixels.len() != expected {
            return Err(RendererError::Config("texture pixel buffer size mismatch"));
        }

        // --- device-local image ---
        let format = vk::Format::R8G8B8A8_SRGB;
        let info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        let image = check!(
            unsafe { device.create_image(&info, None) },
            "create texture image"
        )?;

        let reqs = unsafe { device.get_image_memory_requirements(image) };
        let mem_type = find_memory_type(
            mem_props,
            reqs.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        let alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(reqs.size)
            .memory_type_index(mem_type);
        let memory = check!(
            unsafe { device.allocate_memory(&alloc, None) },
            "allocate texture memory"
        )?;
        check!(
            unsafe { device.bind_image_memory(image, memory, 0) },
            "bind texture memory"
        )?;

        // --- staging buffer with the pixels ---
        // Reuse the buffer module's staging pattern by hand here (it's buffer->
        // image, not buffer->buffer, so upload_via_staging doesn't fit directly).
        let staging = buffer::create_host_buffer(device, mem_props, pixels)?;

        // --- copy staging -> image, bracketed by layout transitions ---
        buffer::immediate_submit(device, queue, queue_family, |device, cmd| unsafe {
            // UNDEFINED -> TRANSFER_DST_OPTIMAL
            let to_dst = vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
                .src_access_mask(vk::AccessFlags2::NONE)
                .dst_stage_mask(vk::PipelineStageFlags2::COPY)
                .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .image(image)
                .subresource_range(color_range());
            device.cmd_pipeline_barrier2(
                cmd,
                &vk::DependencyInfo::default().image_memory_barriers(std::slice::from_ref(&to_dst)),
            );

            let region = vk::BufferImageCopy2::default()
                .buffer_offset(0)
                .image_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image_extent(vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                });
            let copy = vk::CopyBufferToImageInfo2::default()
                .src_buffer(staging.buffer)
                .dst_image(image)
                .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .regions(std::slice::from_ref(&region));
            device.cmd_copy_buffer_to_image2(cmd, &copy);

            // TRANSFER_DST_OPTIMAL -> SHADER_READ_ONLY_OPTIMAL
            let to_read = vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::COPY)
                .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                .dst_access_mask(vk::AccessFlags2::SHADER_SAMPLED_READ)
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image(image)
                .subresource_range(color_range());
            device.cmd_pipeline_barrier2(
                cmd,
                &vk::DependencyInfo::default()
                    .image_memory_barriers(std::slice::from_ref(&to_read)),
            );
        })?;

        // staging no longer needed
        let mut staging = staging;
        unsafe { staging.destroy(device) };

        // --- view + sampler ---
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(color_range());
        let view = check!(
            unsafe { device.create_image_view(&view_info, None) },
            "create texture view"
        )?;

        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .min_lod(0.0)
            .max_lod(0.0); // single mip for now
        let sampler = check!(
            unsafe { device.create_sampler(&sampler_info, None) },
            "create sampler"
        )?;

        Ok(Self {
            image,
            memory,
            view,
            sampler,
        })
    }

    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        device.destroy_sampler(self.sampler, None);
        device.destroy_image_view(self.view, None);
        device.destroy_image(self.image, None);
        device.free_memory(self.memory, None);
    }
}

fn color_range() -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
    }
}
