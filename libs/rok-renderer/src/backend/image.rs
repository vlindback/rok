// image.rs
//
// Depth image (color images are swapchain-owned). Extent-sized, so it's
// recreated on resize alongside the swapchain — not create-once like the
// pipeline.

use ash::vk;

use crate::backend::buffer::find_memory_type;
use crate::error::{RendererResult, check};

pub(crate) struct DepthImage {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub format: vk::Format,
}

impl DepthImage {
    pub(crate) fn new(
        device: &ash::Device,
        mem_props: &vk::PhysicalDeviceMemoryProperties,
        extent: vk::Extent2D,
        format: vk::Format,
    ) -> RendererResult<Self> {
        let info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        let image = check!(
            unsafe { device.create_image(&info, None) },
            "create depth image"
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
            "allocate depth memory"
        )?;
        check!(
            unsafe { device.bind_image_memory(image, memory, 0) },
            "bind depth memory"
        )?;

        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });
        let view = check!(
            unsafe { device.create_image_view(&view_info, None) },
            "create depth view"
        )?;

        Ok(Self {
            image,
            memory,
            view,
            format,
        })
    }

    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        device.destroy_image_view(self.view, None);
        device.destroy_image(self.image, None);
        device.free_memory(self.memory, None);
    }
}
