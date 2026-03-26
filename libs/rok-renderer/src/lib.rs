// rok-renderer/src/lib.rs

// Public API for the rok renderer.

pub mod error;

mod vk;

use std::num::NonZeroU32;

use error::{RendererError, RendererResult};
use rok_abi::surface::{NativeSurfaceHandle, SurfaceType};
use rok_log::log_info;
use vk::instance::VulkanInstance;

// ---------------------------------------------------------------------------
// RendererConfig
// ---------------------------------------------------------------------------

/// Configuration for renderer creation. The Engine decides these values
/// and they are fixed for the renderer's lifetime.
pub struct RendererConfig {
    /// Application name shown in debug tools (RenderDoc, validation layers).
    pub app_name: String,

    /// Number of frames that can be in flight simultaneously.
    /// Typically 2 (double buffering) or 3 (triple buffering).
    /// The Engine decides this at startup; it cannot change at runtime.
    pub frames_in_flight: NonZeroU32,

    /// The native surface to present to (if presentation is needed).
    /// None for headless / compute-only mode.
    pub surface: Option<NativeSurfaceHandle>,
}

// ---------------------------------------------------------------------------
// Renderer
// ---------------------------------------------------------------------------

/// The renderer. Owns all Vulkan state.
pub struct Renderer {
    instance: VulkanInstance,
}

impl Renderer {
    /// Create and initialise the renderer.
    ///
    /// This loads the Vulkan library, creates an instance with validation
    /// layers (debug builds), selects a physical device, creates a logical
    /// device, and sets up the swapchain and frame infrastructure.
    pub fn new(config: &RendererConfig) -> RendererResult<Self> {
        // Determine which platform surface extensions we need.
        let required_extensions = surface_extensions(config.surface.as_ref())?;
        let ext_refs: Vec<&std::ffi::CStr> = required_extensions.iter().map(|s| *s).collect();

        let instance = VulkanInstance::new(&config.app_name, &ext_refs)?;

        log_info!("rok-renderer: Vulkan instance created");

        Ok(Self { instance })
    }
}

// ---------------------------------------------------------------------------
// Platform surface extensions
// ---------------------------------------------------------------------------

fn surface_extensions(
    surface: Option<&NativeSurfaceHandle>,
) -> RendererResult<Vec<&'static std::ffi::CStr>> {
    let Some(surface) = surface else {
        // Headless / compute-only — no surface extensions needed.
        return Ok(Vec::new());
    };

    match surface.type_ {
        SurfaceType::Win32 => Ok(vec![ash::khr::win32_surface::NAME]),
        SurfaceType::Wayland => Ok(vec![ash::khr::wayland_surface::NAME]),
        SurfaceType::Android => Ok(vec![ash::khr::android_surface::NAME]),
    }
}
