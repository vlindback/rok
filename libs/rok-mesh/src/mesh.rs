// mesh.rs

use rok_math::{vec2::Vec2, vec3::Vec3, vec4::Vec4};

#[derive(Copy, Clone, PartialEq)]
pub enum IndexType {
    U16,
    U32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MeshVertex {
    pub position: Vec3,
    pub uv: Vec2,
    pub normal: Vec3,
    pub tangent: Vec4,
}

pub struct MeshData {
    pub vertices: Vec<MeshVertex>,
    /// Either u16 or u32 depending on the index_type field.
    pub indices: Vec<u8>,
    /// Which material this sub-mesh referenced (by name, resolved later).
    pub material_index: Option<usize>,
    pub material_name: Option<String>,
    pub index_type: IndexType,
}
