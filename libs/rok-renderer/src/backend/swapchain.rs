// vk/swapchain.rs
//
// Swapchain creation and recreation.
//
// The swapchain is the bridge between Vulkan and the window system.
// It manages a ring of presentable images that we render into and
// hand to the compositor.
//
// The swapchain must be recreated when:
//   - The window is resized
//   - The surface becomes suboptimal or out of date
//   - Display settings change (HDR toggle, vsync change)

use ash::{khr, vk};

use rok_log::{log_info, log_warn};

use crate::backend::device::VulkanDevice;
use crate::backend::physical_device::QueueFamilyIndices;
use crate::error::{RendererResult, check};

// ---------------------------------------------------------------------------
// SwapchainConfig
// ---------------------------------------------------------------------------

/// Resolved swapchain parameters. Stored so we can log them and use them
/// during recreation.
#[derive(Debug, Clone)]
pub(crate) struct SwapchainConfig {
    pub format: vk::SurfaceFormatKHR,
    pub present_mode: vk::PresentModeKHR,
    pub extent: vk::Extent2D,
    pub image_count: u32,
}

// ---------------------------------------------------------------------------
// VulkanSwapchain
// ---------------------------------------------------------------------------

pub(crate) struct VulkanSwapchain {
    pub(crate) swapchain: vk::SwapchainKHR,
    pub(crate) images: Vec<vk::Image>,
    pub(crate) image_views: Vec<vk::ImageView>,
    pub(crate) config: SwapchainConfig,
}

impl VulkanSwapchain {
    /// Create a new swapchain. `old_swapchain` is passed during recreation
    /// so the driver can reuse resources.
    pub(crate) fn new(
        device: &VulkanDevice,
        surface_loader: &khr::surface::Instance,
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        queue_families: &QueueFamilyIndices,
        requested_extent: vk::Extent2D,
        old_swapchain: Option<vk::SwapchainKHR>,
        vsync: bool,
    ) -> RendererResult<Self> {
        let capabilities = unsafe {
            check!(
                surface_loader.get_physical_device_surface_capabilities(physical_device, surface),
                "query surface capabilities"
            )?
        };

        let formats = unsafe {
            check!(
                surface_loader.get_physical_device_surface_formats(physical_device, surface),
                "query surface formats"
            )?
        };

        let present_modes = unsafe {
            check!(
                surface_loader.get_physical_device_surface_present_modes(physical_device, surface),
                "query present modes"
            )?
        };

        // Choose format, present mode & extent
        let format = choose_surface_format(&formats);
        let present_mode = choose_present_mode(&present_modes, vsync);
        let extent = choose_extent(&capabilities, requested_extent);

        // Image count
        // We want one more than the minimum to avoid stalling on the driver.
        // Cap at max (0 means unlimited).
        let mut image_count = capabilities.min_image_count + 1;
        if capabilities.max_image_count > 0 && image_count > capabilities.max_image_count {
            image_count = capabilities.max_image_count;
        }

        let config = SwapchainConfig {
            format,
            present_mode,
            extent,
            image_count,
        };

        log_info!(
            "Swapchain: {}x{}, {:?}, {:?}, {} images",
            extent.width,
            extent.height,
            format.format,
            present_mode,
            image_count,
        );

        // --- Queue family sharing ---
        // If graphics and present are different families, use CONCURRENT.
        // Otherwise EXCLUSIVE (faster, no ownership transfer needed).
        let graphics_family = queue_families.graphics;
        let present_family = queue_families.present;
        let concurrent_families = [graphics_family, present_family];
        let (sharing_mode, family_indices) = if graphics_family != present_family {
            (vk::SharingMode::CONCURRENT, &concurrent_families[..])
        } else {
            (vk::SharingMode::EXCLUSIVE, &[][..])
        };

        // --- Create swapchain ---
        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(sharing_mode)
            .queue_family_indices(family_indices)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(old_swapchain.unwrap_or(vk::SwapchainKHR::null()));

        let swapchain = unsafe {
            check!(
                device.swapchain_loader.create_swapchain(&create_info, None),
                "create swapchain"
            )?
        };

        // --- Get images ---
        let images = unsafe {
            check!(
                device.swapchain_loader.get_swapchain_images(swapchain),
                "get swapchain images"
            )?
        };

        // --- Create image views ---
        let image_views = create_image_views(device.handle(), &images, format.format)?;

        Ok(Self {
            swapchain,
            images,
            image_views,
            config,
        })
    }

    /// Destroy swapchain resources. Called before recreation or on shutdown.
    ///
    /// # Safety
    /// The caller must ensure no GPU work is referencing these resources.
    pub(crate) unsafe fn destroy(&mut self, device: &VulkanDevice) {
        for &view in &self.image_views {
            // Safety: images can still have active references, safety must be verified by caller.
            unsafe { device.handle().destroy_image_view(view, None) };
        }
        self.image_views.clear();

        // Safety: swapchain lifetime safety must be verified by caller.
        unsafe {
            device
                .swapchain_loader
                .destroy_swapchain(self.swapchain, None);
        }
    }
}

// ---------------------------------------------------------------------------
// Selection helpers
// ---------------------------------------------------------------------------

fn choose_surface_format(available: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
    // Prefer SRGB with B8G8R8A8 — standard for desktop rendering.
    for &format in available {
        if format.format == vk::Format::B8G8R8A8_SRGB
            && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        {
            return format;
        }
    }

    // Fallback: B8G8R8A8_UNORM (still very common).
    for &format in available {
        if format.format == vk::Format::B8G8R8A8_UNORM {
            return format;
        }
    }

    // Last resort: whatever the driver gives us first.
    if !available.is_empty() {
        log_warn!(
            "Preferred surface format not found, using {:?}",
            available[0].format
        );
    }
    available[0]
}

fn choose_present_mode(available: &[vk::PresentModeKHR], vsync: bool) -> vk::PresentModeKHR {
    if vsync {
        // If VSync is requested, prioritize MAILBOX for low-latency tear-free rendering.
        if available.contains(&vk::PresentModeKHR::MAILBOX) {
            return vk::PresentModeKHR::MAILBOX;
        }

        // Fallback to FIFO, which the Vulkan spec guarantees is ALWAYS available.
        return vk::PresentModeKHR::FIFO;
    } else {
        // If VSync is OFF, prioritize IMMEDIATE for absolute lowest latency.
        if available.contains(&vk::PresentModeKHR::IMMEDIATE) {
            return vk::PresentModeKHR::IMMEDIATE;
        }

        // Fallback to FIFO_RELAXED if immediate isn't there (tears only when lagging).
        if available.contains(&vk::PresentModeKHR::FIFO_RELAXED) {
            return vk::PresentModeKHR::FIFO_RELAXED;
        }

        // Ultimate fallback. This will force VSync on, but it's better than crashing!
        vk::PresentModeKHR::FIFO
    }
}

fn choose_extent(
    capabilities: &vk::SurfaceCapabilitiesKHR,
    requested: vk::Extent2D,
) -> vk::Extent2D {
    // If currentExtent is 0xFFFFFFFF, the surface size is determined by the
    // extent of a swapchain targeting it. Otherwise use the reported size.
    if capabilities.current_extent.width != u32::MAX {
        return capabilities.current_extent;
    }

    vk::Extent2D {
        width: requested.width.clamp(
            capabilities.min_image_extent.width,
            capabilities.max_image_extent.width,
        ),
        height: requested.height.clamp(
            capabilities.min_image_extent.height,
            capabilities.max_image_extent.height,
        ),
    }
}

fn create_image_views(
    device: &ash::Device,
    images: &[vk::Image],
    format: vk::Format,
) -> RendererResult<Vec<vk::ImageView>> {
    images
        .iter()
        .map(|&image| {
            let info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            unsafe {
                check!(
                    device.create_image_view(&info, None),
                    "create swapchain image view"
                )
                .map_err(Into::into)
            }
        })
        .collect()
}
