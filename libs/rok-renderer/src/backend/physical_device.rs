// vk/physical_device.rs
//
// Physical device enumeration and selection.
//
// The selection algorithm scores every physical device and picks the best one.
// Devices that fail hard requirements (missing extensions, no suitable queue
// families, insufficient API version) are rejected outright. Remaining
// candidates are ranked by a weighted score.

use std::ffi::CStr;

use ash::{Instance, khr, vk};

use rok_log::{log_info, log_trace, log_warn};

use crate::error::{RendererError, RendererResult};

// ---------------------------------------------------------------------------
// Queue family indices
// ---------------------------------------------------------------------------

/// Indices of the queue families we need.
///
/// Graphics and present may be the same family (common on desktop GPUs).
/// Compute may also overlap. We track them separately so the caller can
/// decide whether to create one queue or many.
#[derive(Debug, Clone, Copy)]
pub(crate) struct QueueFamilyIndices {
    pub graphics: u32,
    pub present: u32,
    pub compute: u32,
    pub transfer: Option<u32>, // dedicated transfer queue, if available
}

impl QueueFamilyIndices {
    /// Returns the set of unique family indices (for device queue creation).
    pub(crate) fn unique_families(&self) -> Vec<u32> {
        let mut families = vec![self.graphics, self.present, self.compute];
        if let Some(t) = self.transfer {
            families.push(t);
        }
        families.sort();
        families.dedup();
        families
    }
}

// ---------------------------------------------------------------------------
// Selection result
// ---------------------------------------------------------------------------

/// Everything we learned about the chosen physical device during selection.
pub(crate) struct PhysicalDeviceSelection {
    pub device: vk::PhysicalDevice,
    pub properties: vk::PhysicalDeviceProperties,
    pub queue_families: QueueFamilyIndices,
    pub score: u32,
}

// ---------------------------------------------------------------------------
// Required device extensions
// ---------------------------------------------------------------------------

/// Extensions we require on the device. No fallback — if the device doesn't
/// have them, it's rejected.
const REQUIRED_DEVICE_EXTENSIONS: &[&CStr] = &[khr::swapchain::NAME];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Enumerate all physical devices, score them, and return the best candidate.
///
/// `surface_loader` and `surface` are needed to check present support.
/// For headless mode, pass `None` for both and presentation will not be scored.
pub(crate) fn select_physical_device(
    instance: &Instance,
    surface_loader: Option<&khr::surface::Instance>,
    surface: Option<vk::SurfaceKHR>,
) -> RendererResult<PhysicalDeviceSelection> {
    let devices = unsafe {
        instance
            .enumerate_physical_devices()
            .map_err(|e| crate::error::VkError::new("enumerate physical devices", e))?
    };

    if devices.is_empty() {
        return Err(RendererError::NoSuitableDevice);
    }

    log_info!("Found {} physical device(s)", devices.len());

    let mut best: Option<PhysicalDeviceSelection> = None;

    for device in &devices {
        match evaluate_device(instance, *device, surface_loader, surface) {
            Some(candidate) => {
                let name = unsafe {
                    CStr::from_ptr(candidate.properties.device_name.as_ptr())
                        .to_str()
                        .unwrap_or("<unknown>")
                };
                log_info!(
                    "  [{}] {} — score {}",
                    device_type_str(candidate.properties.device_type),
                    name,
                    candidate.score,
                );

                // If the score is higher update the best candidate.
                if best.as_ref().map_or(true, |b| candidate.score > b.score) {
                    best = Some(candidate);
                }
            }
            None => {
                let props = unsafe { instance.get_physical_device_properties(*device) };
                let name = unsafe {
                    CStr::from_ptr(props.device_name.as_ptr())
                        .to_str()
                        .unwrap_or("<unknown>")
                };
                log_warn!(
                    "  [{}] {} - rejected",
                    device_type_str(props.device_type),
                    name
                );
            }
        }
    }

    let selection = best.ok_or(RendererError::NoSuitableDevice)?;

    let name = unsafe {
        CStr::from_ptr(selection.properties.device_name.as_ptr())
            .to_str()
            .unwrap_or("<unknown>")
    };
    log_info!("Selected GPU: {} (score {})", name, selection.score,);

    Ok(selection)
}

// ---------------------------------------------------------------------------
// Device evaluation
// ---------------------------------------------------------------------------

/// Evaluate a single physical device. Returns `None` if it fails hard
/// requirements, or `Some(selection)` with a score if it passes.
fn evaluate_device(
    instance: &Instance,
    device: vk::PhysicalDevice,
    surface_loader: Option<&khr::surface::Instance>,
    surface: Option<vk::SurfaceKHR>,
) -> Option<PhysicalDeviceSelection> {
    let properties = unsafe { instance.get_physical_device_properties(device) };

    // --- Hard requirement: Vulkan 1.3+ ---
    let api_version = properties.api_version;
    if vk::api_version_major(api_version) < 1
        || (vk::api_version_major(api_version) == 1 && vk::api_version_minor(api_version) < 3)
    {
        log_trace!("  rejected: Vulkan version too low");
        return None;
    }

    // --- Hard requirement: required extensions ---
    if !has_required_extensions(instance, device) {
        log_trace!("  rejected: missing required device extensions");
        return None;
    }

    // --- Hard requirement: queue families ---
    let queue_families = find_queue_families(instance, device, surface_loader, surface)?;

    // --- Scoring ---
    let mut score: u32 = 0;

    // Device type (most important signal).
    score += match properties.device_type {
        vk::PhysicalDeviceType::DISCRETE_GPU => 10_000,
        vk::PhysicalDeviceType::INTEGRATED_GPU => 1_000,
        vk::PhysicalDeviceType::VIRTUAL_GPU => 500,
        vk::PhysicalDeviceType::CPU => 100,
        _ => 0,
    };

    // VRAM size (discrete GPUs report this in device-local heaps).
    let memory_properties = unsafe { instance.get_physical_device_memory_properties(device) };

    let device_local_bytes: u64 = (0..memory_properties.memory_heap_count as usize)
        .filter(|&i| {
            memory_properties.memory_heaps[i]
                .flags
                .contains(vk::MemoryHeapFlags::DEVICE_LOCAL)
        })
        .map(|i| memory_properties.memory_heaps[i].size)
        .sum();

    // 1 point per 64 MB of VRAM (capped at 4096 points = 256 GB).
    let vram_score = (device_local_bytes / (64 * 1024 * 1024)).min(4096) as u32;
    score += vram_score;

    // Bonus for having a dedicated transfer queue (async DMA).
    if queue_families.transfer.is_some() {
        score += 500;
    }

    // Bonus for higher max image dimension (proxy for tier).
    let max_dim = properties.limits.max_image_dimension2_d;
    score += (max_dim / 1024).min(16); // typically 16384 → 16 points

    Some(PhysicalDeviceSelection {
        device,
        properties,
        queue_families,
        score,
    })
}

// ---------------------------------------------------------------------------
// Extension check
// ---------------------------------------------------------------------------

fn has_required_extensions(instance: &Instance, device: vk::PhysicalDevice) -> bool {
    let available = unsafe {
        instance
            .enumerate_device_extension_properties(device)
            .unwrap_or_default()
    };

    REQUIRED_DEVICE_EXTENSIONS.iter().all(|required| {
        available
            .iter()
            .any(|ext| unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) == *required })
    })
}

// ---------------------------------------------------------------------------
// Queue family discovery
// ---------------------------------------------------------------------------

fn find_queue_families(
    instance: &Instance,
    device: vk::PhysicalDevice,
    surface_loader: Option<&khr::surface::Instance>,
    surface: Option<vk::SurfaceKHR>,
) -> Option<QueueFamilyIndices> {
    let families = unsafe { instance.get_physical_device_queue_family_properties(device) };

    let mut graphics: Option<u32> = None;
    let mut present: Option<u32> = None;
    let mut compute: Option<u32> = None;
    let mut transfer: Option<u32> = None;

    for (i, family) in families.iter().enumerate() {
        let idx = i as u32;

        if family.queue_count == 0 {
            continue;
        }

        let has_graphics = family.queue_flags.contains(vk::QueueFlags::GRAPHICS);
        let has_compute = family.queue_flags.contains(vk::QueueFlags::COMPUTE);
        let has_transfer = family.queue_flags.contains(vk::QueueFlags::TRANSFER);

        // Graphics queue (prefer first one found).
        if has_graphics && graphics.is_none() {
            graphics = Some(idx);
        }

        // Present support.
        if let (Some(loader), Some(surf)) = (surface_loader, surface) {
            let supports_present = unsafe {
                loader
                    .get_physical_device_surface_support(device, idx, surf)
                    .unwrap_or(false)
            };
            if supports_present && present.is_none() {
                present = Some(idx);
            }
        }

        // Dedicated compute (no graphics).
        if has_compute && !has_graphics && compute.is_none() {
            compute = Some(idx);
        }

        // Dedicated transfer (no graphics, no compute) — ideal for async DMA.
        if has_transfer && !has_graphics && !has_compute && transfer.is_none() {
            transfer = Some(idx);
        }
    }

    // Fallback: if we didn't find a dedicated compute queue, the graphics
    // queue always supports compute (Vulkan spec guarantee).
    if compute.is_none() {
        compute = graphics;
    }

    // Headless mode: no surface means no present queue needed.
    if surface.is_none() {
        // TODO: this is logically dangerous.
        present = graphics; // doesn't matter, won't be used
    }

    // All three core families must be present.
    Some(QueueFamilyIndices {
        graphics: graphics?,
        present: present?,
        compute: compute?,
        transfer,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn device_type_str(ty: vk::PhysicalDeviceType) -> &'static str {
    match ty {
        vk::PhysicalDeviceType::DISCRETE_GPU => "Discrete",
        vk::PhysicalDeviceType::INTEGRATED_GPU => "Integrated",
        vk::PhysicalDeviceType::VIRTUAL_GPU => "Virtual",
        vk::PhysicalDeviceType::CPU => "CPU",
        _ => "Other",
    }
}
