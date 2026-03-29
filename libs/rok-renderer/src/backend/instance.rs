// vk/instance.rs

// Vulkan instance wrapper

use std::ffi::{CStr, CString};

use ash::{Entry, Instance, ext, khr, vk};
use rok_log::{log_error, log_info, log_trace, log_warn};

use crate::error::{RendererError, RendererResult, check};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ENGINE_NAME: &CStr = c"rok";
const ENGINE_VERSION: u32 = vk::make_api_version(0, 0, 1, 0);

/// Minimum Vulkan API version we require.
/// 1.3 gives us dynamic rendering, synchronization2, and maintenance4
/// without needing extensions.
const REQUIRED_API_VERSION: u32 = vk::make_api_version(0, 1, 3, 0);

const VALIDATION_LAYER: &CStr = c"VK_LAYER_KHRONOS_validation";

// ---------------------------------------------------------------------------
// Instance
// ---------------------------------------------------------------------------

pub(crate) struct VulkanInstance {
    debug_loader: Option<ext::debug_utils::Instance>,
    debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
    instance: Instance,
    _entry: Entry, // must outlive Instance
}

impl VulkanInstance {
    /// Create a new Vulkan instance.
    ///
    /// `app_name`              application name shown in debug tools (e.g. RenderDoc).
    /// `required_extensions`   platform surface extensions the caller needs
    ///                        (e.g. VK_KHR_win32_surface, VK_KHR_wayland_surface).
    pub(crate) fn new(app_name: &str, required_extensions: &[&CStr]) -> RendererResult<Self> {
        let entry = unsafe { Entry::load().map_err(RendererError::LoaderNotFound)? };

        let version = unsafe {
            entry
                .try_enumerate_instance_version()
                .unwrap_or(None)
                .unwrap_or(vk::make_api_version(0, 1, 0, 0))
        };

        if version < REQUIRED_API_VERSION {
            log_error!(
                "Vulkan {}.{}.{} found, but {}.{}.{} required",
                vk::api_version_major(version),
                vk::api_version_minor(version),
                vk::api_version_patch(version),
                vk::api_version_major(REQUIRED_API_VERSION),
                vk::api_version_minor(REQUIRED_API_VERSION),
                vk::api_version_patch(REQUIRED_API_VERSION),
            );
            return Err(RendererError::Config(
                "Vulkan 1.3 or later is required. Update your GPU driver.",
            ));
        }

        log_info!(
            "Vulkan instance version: {}.{}.{}",
            vk::api_version_major(version),
            vk::api_version_minor(version),
            vk::api_version_patch(version),
        );

        let available_extensions = unsafe {
            entry
                .enumerate_instance_extension_properties(None)
                .unwrap_or_default()
        };

        let mut extensions: Vec<*const i8> = Vec::new();

        // Surface extensions (caller provides platform-specific ones).
        extensions.push(khr::surface::NAME.as_ptr());

        for ext in required_extensions {
            if !has_extension(&available_extensions, ext) {
                return Err(RendererError::MissingInstanceExtension(
                    ext.to_str().unwrap_or("(invalid UTF-8)").to_owned(),
                ));
            }
            extensions.push(ext.as_ptr());
        }

        let enable_validation = cfg!(debug_assertions) && has_validation_layer(&entry);

        let mut layers: Vec<*const i8> = Vec::new();

        if enable_validation {
            layers.push(VALIDATION_LAYER.as_ptr());
            extensions.push(ext::debug_utils::NAME.as_ptr());
            log_info!("Vulkan validation layers enabled");
        }

        let app_name_c =
            CString::new(app_name).unwrap_or_else(|_| CString::new("rok-app").unwrap());

        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name_c)
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(ENGINE_NAME)
            .engine_version(ENGINE_VERSION)
            .api_version(REQUIRED_API_VERSION);

        let mut debug_create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(debug_callback));

        let mut create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extensions)
            .enabled_layer_names(&layers);

        if enable_validation {
            create_info = create_info.push_next(&mut debug_create_info);
        }

        let instance = unsafe {
            check!(
                entry.create_instance(&create_info, None),
                "create Vulkan instance"
            )?
        };

        let (debug_loader, debug_messenger) = if enable_validation {
            let loader = ext::debug_utils::Instance::new(&entry, &instance);
            let messenger = unsafe {
                check!(
                    loader.create_debug_utils_messenger(&debug_create_info, None),
                    "create debug messenger"
                )?
            };
            (Some(loader), Some(messenger))
        } else {
            (None, None)
        };

        Ok(Self {
            debug_loader,
            debug_messenger,
            instance,
            _entry: entry,
        })
    }

    // Getters

    #[inline]
    pub(crate) fn handle(&self) -> &Instance {
        &self.instance
    }

    #[inline]
    pub(crate) fn entry(&self) -> &Entry {
        &self._entry
    }
}

unsafe extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    if callback_data.is_null() {
        return vk::FALSE;
    }

    let message = unsafe {
        let data = &*callback_data;
        if data.p_message.is_null() {
            "<no message>"
        } else {
            CStr::from_ptr(data.p_message)
                .to_str()
                .unwrap_or("<invalid UTF-8>")
        }
    };

    match severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            log_error!("[Vulkan] {}", message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            log_warn!("[Vulkan] {}", message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            log_info!("[Vulkan] {}", message);
        }
        _ => {
            log_trace!("[Vulkan] {}", message);
        }
    }

    vk::FALSE
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn has_extension(available: &[vk::ExtensionProperties], name: &CStr) -> bool {
    available.iter().any(|ext| {
        // SAFETY: extension_name is a null-terminated c_char array from the driver.
        unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) == name }
    })
}

fn has_validation_layer(entry: &Entry) -> bool {
    let layers = unsafe {
        entry
            .enumerate_instance_layer_properties()
            .unwrap_or_default()
    };
    layers
        .iter()
        .any(|layer| unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) == VALIDATION_LAYER })
}
