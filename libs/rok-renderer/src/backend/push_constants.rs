// push_constants.rs
//

use ash::vk;

/// Per-draw push constants. repr(C) so the layout matches the shader's
/// push_constant block byte-for-byte (std430: mat4=64, vec4=16).
/// Offsets: mvp@0, model@64, camera_pos@128, material@144. Total 64 bytes.
/// NOTE: Vulkan only guarantees a 128 byte floor for devices.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct PushConstants {
    pub model: [f32; 16],
}

impl PushConstants {
    pub fn push_stages() -> vk::ShaderStageFlags {
        vk::ShaderStageFlags::VERTEX
    }
}
