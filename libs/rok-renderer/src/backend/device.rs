// vk/device.rs
//
// Logical device creation and queue handle storage.
//
// We create a single logical device with queues for graphics, present,
// compute, and optionally dedicated transfer. Queue priorities are
// hardcoded: graphics and present get 1.0, compute 0.8, transfer 0.5.
//
// The Device wrapper also stores the ash extension loaders that need
// a device reference (e.g. khr::swapchain::Device).

use std::ffi::CStr;

use ash::{Instance, khr, vk};

use rok_log::log_info;

use crate::backend::physical_device::{PhysicalDeviceSelection, QueueFamilyIndices};
use crate::error::{RendererResult, check};

// ---------------------------------------------------------------------------
// Required device extensions
// ---------------------------------------------------------------------------

const REQUIRED_DEVICE_EXTENSIONS: &[&CStr] = &[khr::swapchain::NAME];

// ---------------------------------------------------------------------------
// Queues
// ---------------------------------------------------------------------------

/// Holds the Vulkan queue handles we use.
pub(crate) struct Queues {
    pub graphics: vk::Queue,
    pub present: vk::Queue,
    pub compute: vk::Queue,
    pub transfer: Option<vk::Queue>,
}

// ---------------------------------------------------------------------------
// VulkanDevice
// ---------------------------------------------------------------------------

/// Owns the logical device and provides access to queues and extension loaders.
///
/// Drop: `ash::Device::destroy_device` is called automatically.
pub(crate) struct VulkanDevice {
    device: ash::Device,
    pub(crate) queues: Queues,
    pub(crate) queue_families: QueueFamilyIndices,
    pub(crate) swapchain_loader: khr::swapchain::Device,
}

impl VulkanDevice {
    /// Create a logical device from the selected physical device.
    pub(crate) fn new(
        instance: &Instance,
        selection: &PhysicalDeviceSelection,
    ) -> RendererResult<Self> {
        let families = &selection.queue_families;
        let unique = families.unique_families();

        // --- Queue create infos ---
        // One DeviceQueueCreateInfo per unique family. Each gets a single queue.
        let queue_priority_high: [f32; 1] = [1.0];
        let queue_priority_compute: [f32; 1] = [0.8];
        let queue_priority_transfer: [f32; 1] = [0.5];

        let queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = unique
            .iter()
            .map(|&family_index| {
                let priorities = if Some(family_index) == families.transfer {
                    &queue_priority_transfer[..]
                } else if family_index == families.compute && family_index != families.graphics {
                    &queue_priority_compute[..]
                } else {
                    &queue_priority_high[..]
                };

                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(family_index)
                    .queue_priorities(priorities)
            })
            .collect();

        // --- Vulkan 1.3 features we require ---
        let mut vk13_features = vk::PhysicalDeviceVulkan13Features::default()
            .dynamic_rendering(true)
            .synchronization2(true)
            .maintenance4(true);

        let mut vk12_features = vk::PhysicalDeviceVulkan12Features::default()
            .descriptor_indexing(true)
            .buffer_device_address(true)
            .timeline_semaphore(true);

        let features = vk::PhysicalDeviceFeatures::default()
            .sampler_anisotropy(true)
            .fill_mode_non_solid(true) // wireframe
            .multi_draw_indirect(true);

        let mut features2 = vk::PhysicalDeviceFeatures2::default()
            .features(features)
            .push_next(&mut vk13_features)
            .push_next(&mut vk12_features);

        // --- Extension names ---
        let extension_names: Vec<*const i8> = REQUIRED_DEVICE_EXTENSIONS
            .iter()
            .map(|ext| ext.as_ptr())
            .collect();

        // --- Device creation ---
        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&extension_names)
            .push_next(&mut features2);

        let device = unsafe {
            check!(
                instance.create_device(selection.device, &device_create_info, None),
                "create logical device"
            )?
        };

        // --- Retrieve queue handles ---
        let graphics = unsafe { device.get_device_queue(families.graphics, 0) };
        let present = unsafe { device.get_device_queue(families.present, 0) };
        let compute = unsafe { device.get_device_queue(families.compute, 0) };
        let transfer = families
            .transfer
            .map(|idx| unsafe { device.get_device_queue(idx, 0) });

        log_info!(
            "Logical device created — queues: graphics={}, present={}, compute={}, transfer={}",
            families.graphics,
            families.present,
            families.compute,
            families.transfer.map_or("none".into(), |t| t.to_string()),
        );

        // --- Extension loaders ---
        let swapchain_loader = khr::swapchain::Device::new(instance, &device);

        Ok(Self {
            device,
            queues: Queues {
                graphics,
                present,
                compute,
                transfer,
            },
            queue_families: *families,
            swapchain_loader,
        })
    }

    /// Borrow the raw ash Device.
    #[inline]
    pub(crate) fn handle(&self) -> &ash::Device {
        &self.device
    }

    /// Wait for all GPU work to finish. Call before cleanup / shutdown.
    pub(crate) fn wait_idle(&self) {
        unsafe {
            let _ = self.device.device_wait_idle();
        }
    }
}

impl Drop for VulkanDevice {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
        }
    }
}
