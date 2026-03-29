// vk/mod.rs
//
// Thin ownership wrappers around Vulkan objects.
//
// These are NOT general-purpose abstractions. They exist to manage lifetimes
// and drop order for rok-renderer's specific needs. Each wrapper owns its
// Vulkan handle and destroys it on drop.

pub(crate) mod device;
pub(crate) mod frame;
pub(crate) mod instance;
pub(crate) mod physical_device;
pub(crate) mod surface;
pub(crate) mod swapchain;
