// error.rs
//
// Renderer error type. Wraps ash::vk::Result with human-readable context
// about what operation failed and why.
//
// Every Vulkan call that can fail goes through this, so the caller always
// gets "failed to create swapchain: VK_ERROR_OUT_OF_DEVICE_MEMORY" rather
// than a raw negative i32.

use ash::vk;
use std::fmt;

// ---------------------------------------------------------------------------
// VkError
// ---------------------------------------------------------------------------

/// A Vulkan error with context about what went wrong.
#[derive(Debug)]
pub struct VkError {
    /// What we were trying to do when it failed.
    pub context: &'static str,
    /// The Vulkan result code.
    pub result: vk::Result,
}

impl VkError {
    #[inline]
    pub fn new(context: &'static str, result: vk::Result) -> Self {
        Self { context, result }
    }
}

impl fmt::Display for VkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} — {}",
            self.context,
            vk_result_name(self.result),
            vk_result_explanation(self.result),
        )
    }
}

impl std::error::Error for VkError {}

// ---------------------------------------------------------------------------
// RendererError
// ---------------------------------------------------------------------------

/// Top-level renderer error. Covers Vulkan failures and non-Vulkan problems
/// like a missing Vulkan loader or invalid configuration.
#[derive(Debug)]
pub enum RendererError {
    /// A Vulkan API call failed.
    Vulkan(VkError),

    /// The Vulkan loader (libvulkan.so / vulkan-1.dll) could not be found.
    LoaderNotFound(ash::LoadingError),

    /// A required instance extension is not available.
    MissingInstanceExtension(String),

    /// A required device extension is not available.
    MissingDeviceExtension(String),

    /// No physical device meets minimum requirements.
    NoSuitableDevice,

    /// Catch-all for configuration / logic errors.
    Config(&'static str),
}

impl fmt::Display for RendererError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RendererError::Vulkan(e) => write!(f, "{}", e),
            RendererError::LoaderNotFound(e) => {
                write!(f, "Vulkan loader not found: {}", e)
            }
            RendererError::MissingInstanceExtension(ext) => {
                write!(f, "Required instance extension not available: {}", ext)
            }
            RendererError::MissingDeviceExtension(ext) => {
                write!(f, "Required device extension not available: {}", ext)
            }
            RendererError::NoSuitableDevice => {
                write!(
                    f,
                    "No physical device meets the renderer's minimum requirements"
                )
            }
            RendererError::Config(msg) => write!(f, "Renderer configuration error: {}", msg),
        }
    }
}

impl std::error::Error for RendererError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RendererError::Vulkan(e) => Some(e),
            RendererError::LoaderNotFound(e) => Some(e),
            _ => None,
        }
    }
}

impl From<VkError> for RendererError {
    fn from(e: VkError) -> Self {
        RendererError::Vulkan(e)
    }
}

// ---------------------------------------------------------------------------
// Result alias
// ---------------------------------------------------------------------------

/// Shorthand for renderer operations.
pub type RendererResult<T> = Result<T, RendererError>;

// ---------------------------------------------------------------------------
// check! macro — the workhorse for Vulkan calls
// ---------------------------------------------------------------------------

/// Converts an `ash::vk::Result` into a `Result<(), VkError>` with context.
///
/// Usage:
/// ```ignore
/// check!(instance.create_device(...), "create logical device")?;
/// ```
macro_rules! check {
    ($expr:expr, $context:expr) => {
        match $expr {
            Ok(value) => Ok(value),
            Err(result) => Err($crate::error::VkError::new($context, result)),
        }
    };
}

pub(crate) use check;

// ---------------------------------------------------------------------------
// Human-readable VkResult names and explanations
// ---------------------------------------------------------------------------

fn vk_result_name(result: vk::Result) -> &'static str {
    match result {
        // Success codes (callers shouldn't normally wrap these in errors,
        // but include them for completeness in diagnostic output).
        vk::Result::SUCCESS => "VK_SUCCESS",
        vk::Result::NOT_READY => "VK_NOT_READY",
        vk::Result::TIMEOUT => "VK_TIMEOUT",
        vk::Result::EVENT_SET => "VK_EVENT_SET",
        vk::Result::EVENT_RESET => "VK_EVENT_RESET",
        vk::Result::INCOMPLETE => "VK_INCOMPLETE",
        vk::Result::SUBOPTIMAL_KHR => "VK_SUBOPTIMAL_KHR",

        // Error codes
        vk::Result::ERROR_OUT_OF_HOST_MEMORY => "VK_ERROR_OUT_OF_HOST_MEMORY",
        vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => "VK_ERROR_OUT_OF_DEVICE_MEMORY",
        vk::Result::ERROR_INITIALIZATION_FAILED => "VK_ERROR_INITIALIZATION_FAILED",
        vk::Result::ERROR_DEVICE_LOST => "VK_ERROR_DEVICE_LOST",
        vk::Result::ERROR_MEMORY_MAP_FAILED => "VK_ERROR_MEMORY_MAP_FAILED",
        vk::Result::ERROR_LAYER_NOT_PRESENT => "VK_ERROR_LAYER_NOT_PRESENT",
        vk::Result::ERROR_EXTENSION_NOT_PRESENT => "VK_ERROR_EXTENSION_NOT_PRESENT",
        vk::Result::ERROR_FEATURE_NOT_PRESENT => "VK_ERROR_FEATURE_NOT_PRESENT",
        vk::Result::ERROR_INCOMPATIBLE_DRIVER => "VK_ERROR_INCOMPATIBLE_DRIVER",
        vk::Result::ERROR_TOO_MANY_OBJECTS => "VK_ERROR_TOO_MANY_OBJECTS",
        vk::Result::ERROR_FORMAT_NOT_SUPPORTED => "VK_ERROR_FORMAT_NOT_SUPPORTED",
        vk::Result::ERROR_FRAGMENTED_POOL => "VK_ERROR_FRAGMENTED_POOL",
        vk::Result::ERROR_UNKNOWN => "VK_ERROR_UNKNOWN",
        vk::Result::ERROR_OUT_OF_POOL_MEMORY => "VK_ERROR_OUT_OF_POOL_MEMORY",
        vk::Result::ERROR_INVALID_EXTERNAL_HANDLE => "VK_ERROR_INVALID_EXTERNAL_HANDLE",
        vk::Result::ERROR_FRAGMENTATION => "VK_ERROR_FRAGMENTATION",
        vk::Result::ERROR_INVALID_OPAQUE_CAPTURE_ADDRESS => {
            "VK_ERROR_INVALID_OPAQUE_CAPTURE_ADDRESS"
        }
        vk::Result::ERROR_SURFACE_LOST_KHR => "VK_ERROR_SURFACE_LOST_KHR",
        vk::Result::ERROR_NATIVE_WINDOW_IN_USE_KHR => "VK_ERROR_NATIVE_WINDOW_IN_USE_KHR",
        vk::Result::ERROR_OUT_OF_DATE_KHR => "VK_ERROR_OUT_OF_DATE_KHR",
        vk::Result::ERROR_INCOMPATIBLE_DISPLAY_KHR => "VK_ERROR_INCOMPATIBLE_DISPLAY_KHR",
        vk::Result::ERROR_VALIDATION_FAILED_EXT => "VK_ERROR_VALIDATION_FAILED_EXT",
        vk::Result::ERROR_FULL_SCREEN_EXCLUSIVE_MODE_LOST_EXT => {
            "VK_ERROR_FULL_SCREEN_EXCLUSIVE_MODE_LOST_EXT"
        }
        _ => "VK_ERROR_UNKNOWN (unrecognised code)",
    }
}

fn vk_result_explanation(result: vk::Result) -> &'static str {
    match result {
        vk::Result::ERROR_OUT_OF_HOST_MEMORY => {
            "The system ran out of CPU-accessible memory. Close other applications or check for leaks."
        }
        vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => {
            "The GPU ran out of memory. Reduce texture resolution, buffer sizes, or render targets."
        }
        vk::Result::ERROR_INITIALIZATION_FAILED => {
            "Vulkan initialisation failed. The driver may be corrupted or too old."
        }
        vk::Result::ERROR_DEVICE_LOST => {
            "The GPU stopped responding. This is usually a driver crash or hardware fault. \
             The application must destroy and recreate the device."
        }
        vk::Result::ERROR_MEMORY_MAP_FAILED => {
            "Failed to map GPU memory to the CPU address space. The allocation may not have \
             HOST_VISIBLE set, or the system is out of virtual address space."
        }
        vk::Result::ERROR_LAYER_NOT_PRESENT => {
            "A requested Vulkan validation/debug layer is not installed. \
             Install the LunarG Vulkan SDK or remove the layer request."
        }
        vk::Result::ERROR_EXTENSION_NOT_PRESENT => {
            "A requested Vulkan extension is not supported by this driver or device."
        }
        vk::Result::ERROR_FEATURE_NOT_PRESENT => {
            "A requested device feature is not supported by the selected GPU."
        }
        vk::Result::ERROR_INCOMPATIBLE_DRIVER => {
            "The Vulkan driver does not support the requested API version. Update your GPU driver."
        }
        vk::Result::ERROR_TOO_MANY_OBJECTS => {
            "Too many Vulkan objects of a particular type have been created."
        }
        vk::Result::ERROR_FORMAT_NOT_SUPPORTED => {
            "The requested image or buffer format is not supported on this device."
        }
        vk::Result::ERROR_FRAGMENTED_POOL => {
            "A descriptor pool allocation failed due to fragmentation, not lack of space. \
             Reset the pool or create a new one."
        }
        vk::Result::ERROR_OUT_OF_POOL_MEMORY => {
            "A descriptor pool ran out of memory. Increase the pool's maxSets or descriptor counts."
        }
        vk::Result::ERROR_SURFACE_LOST_KHR => {
            "The window surface is no longer available. The window may have been destroyed."
        }
        vk::Result::ERROR_NATIVE_WINDOW_IN_USE_KHR => {
            "The native window already has a Vulkan or other API surface bound to it."
        }
        vk::Result::ERROR_OUT_OF_DATE_KHR => {
            "The swapchain is out of date (e.g. after a resize) and must be recreated."
        }
        vk::Result::ERROR_INCOMPATIBLE_DISPLAY_KHR => {
            "The display used is not compatible with the Vulkan instance."
        }
        vk::Result::ERROR_VALIDATION_FAILED_EXT => {
            "A Vulkan validation layer detected invalid API usage. Check the debug messenger output."
        }
        vk::Result::ERROR_FULL_SCREEN_EXCLUSIVE_MODE_LOST_EXT => {
            "Full-screen exclusive mode was lost. Another window may have taken focus."
        }
        vk::Result::ERROR_INVALID_EXTERNAL_HANDLE => {
            "An external handle (fd, HANDLE) is not valid or was already consumed."
        }
        vk::Result::ERROR_FRAGMENTATION => "Memory allocation failed due to heap fragmentation.",
        vk::Result::ERROR_INVALID_OPAQUE_CAPTURE_ADDRESS => {
            "A buffer was created with an opaque capture address that is no longer valid."
        }
        _ => "No additional explanation available for this error code.",
    }
}
