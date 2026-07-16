// vk/mod.rs
//
// Thin ownership wrappers around Vulkan objects.
//
// These are NOT general-purpose abstractions. They exist to manage lifetimes
// and drop order for rok-renderer's specific needs. Each wrapper owns its
// Vulkan handle and destroys it on drop.

pub(crate) mod buffer;
pub(crate) mod descriptor;
pub(crate) mod device;
pub(crate) mod frame;
pub(crate) mod frame_descriptor;
pub(crate) mod frame_ubo;
pub(crate) mod image;
pub(crate) mod instance;
pub(crate) mod light;
pub(crate) mod material;
pub(crate) mod mesh_registry;
pub(crate) mod physical_device;
pub(crate) mod pipeline;
pub(crate) mod push_constants;
pub(crate) mod surface;
pub(crate) mod swapchain;
pub(crate) mod texture;
pub(crate) mod vertex;
