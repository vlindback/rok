/// Directional light, matching the std140 UBO layout in the shader.
/// vec3 aligns to 16 in std140, so each direction/color is padded to a full
/// 16-byte slot. Get this wrong and the shader reads color out of the padding.
#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct LightUbo {
    pub direction: [f32; 3],
    pub _pad0: f32,
    pub color: [f32; 3],
    pub _pad1: f32,
}
