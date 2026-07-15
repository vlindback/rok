// Light.rs
//

pub(crate) const MAX_LIGHTS: usize = 16;

// Light types — must match the shader's int constants.
pub(crate) const LIGHT_DIRECTIONAL: u32 = 0;
pub(crate) const LIGHT_POINT: u32 = 1;

/// One light, std140-aligned: each vec3 padded to a full 16-byte slot,
/// so the element is exactly 32 bytes (matches the shader's array stride).
#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct GpuLight {
    pub direction: [f32; 3],
    pub kind: u32,
    pub color: [f32; 3],
    pub _pad1: f32,
    pub position: [f32; 3],
    pub _pad2: f32,
}

/// The light UBO: a count, padded up to 16 so the array lands on a 16-byte
/// boundary (std140), then a fixed-size array. Total 16 + 16*32 = 528 bytes.
#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct LightsUbo {
    pub count: u32,
    pub _pad: [u32; 3], // push `lights` to offset 16
    pub lights: [GpuLight; MAX_LIGHTS],
}

impl GpuLight {
    pub fn directional(dir: [f32; 3], color: [f32; 3]) -> Self {
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        Self {
            direction: [dir[0] / len, dir[1] / len, dir[2] / len],
            kind: LIGHT_DIRECTIONAL,
            color,
            _pad1: 0.0,
            position: [0.0, 0.0, 0.0],
            _pad2: 0.0,
        }
    }

    pub fn point(position: [f32; 3], color: [f32; 3]) -> Self {
        Self {
            direction: [0.0, 0.0, 0.0],
            kind: LIGHT_POINT,
            color,
            _pad1: 0.0,
            position,
            _pad2: 0.0,
        }
    }

    pub const ZERO: GpuLight = GpuLight {
        direction: [0.0, 0.0, 0.0],
        kind: LIGHT_DIRECTIONAL,
        color: [0.0, 0.0, 0.0],
        _pad1: 0.0,
        position: [0.0, 0.0, 0.0],
        _pad2: 0.0,
    };
}
