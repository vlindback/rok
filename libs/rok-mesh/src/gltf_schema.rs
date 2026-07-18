// gltf_schema.rs
//
// Faithful (subset) model of the glTF 2.0 JSON. Pure data, no logic.
// We model the static-rendering + scene slice; animation/skins/morph and
// arbitrary extensions/extras are intentionally NOT modeled - serde drops
// unknown keys by default, so those parse fine and simply don't appear here.

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GltfDocument {
    pub asset: Asset,
    #[serde(default)]
    pub scene: Option<usize>, // index of default scene
    #[serde(default)]
    pub scenes: Vec<Scene>,
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub meshes: Vec<Mesh>,
    #[serde(default)]
    pub accessors: Vec<Accessor>,
    #[serde(default)]
    pub buffer_views: Vec<BufferView>,
    #[serde(default)]
    pub buffers: Vec<Buffer>,
    #[serde(default)]
    pub materials: Vec<Material>,
    #[serde(default)]
    pub textures: Vec<Texture>,
    #[serde(default)]
    pub images: Vec<Image>,
    #[serde(default)]
    pub samplers: Vec<Sampler>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Asset {
    pub version: String, // glTF spec version, "2.0"
    #[serde(default)]
    pub min_version: Option<String>,
    #[serde(default)]
    pub generator: Option<String>,
}

// ── buffers / views / accessors ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Buffer {
    // NONE for glb buffer 0 - its bytes ARE the BIN chunk you sliced in parse_glb.
    #[serde(default)]
    pub uri: Option<String>,
    pub byte_length: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BufferView {
    pub buffer: usize,
    #[serde(default)]
    pub byte_offset: usize, // absent ⇒ 0
    pub byte_length: usize,
    #[serde(default)]
    pub byte_stride: Option<usize>, // absent ⇒ tightly packed
    #[serde(default)]
    pub target: Option<u32>, // 34962/34963 - a hint, ignorable
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Accessor {
    #[serde(default)]
    pub buffer_view: Option<usize>, // absent ⇒ implicit/sparse
    #[serde(default)]
    pub byte_offset: usize, // absent ⇒ 0
    pub component_type: u32, // 5126=F32 5123=U16 5125=U32 5121=U8 5122=I16
    #[serde(default)]
    pub normalized: bool, // absent ⇒ false
    pub count: usize,
    #[serde(rename = "type")]
    pub kind: String, // SCALAR|VEC2|VEC3|VEC4|MAT2|MAT3|MAT4
    #[serde(default)]
    pub min: Option<Vec<f32>>,
    #[serde(default)]
    pub max: Option<Vec<f32>>,
    #[serde(default)]
    pub sparse: Option<Sparse>, // modeled, not yet handled
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Sparse {
    pub count: usize,
    pub indices: SparseIndices,
    pub values: SparseValues,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SparseIndices {
    pub buffer_view: usize,
    #[serde(default)]
    pub byte_offset: usize,
    pub component_type: u32,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SparseValues {
    pub buffer_view: usize,
    #[serde(default)]
    pub byte_offset: usize,
}

// ── meshes / primitives ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Mesh {
    pub primitives: Vec<Primitive>, // each primitive ≈ one of your MeshData sub-meshes
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Primitive {
    // keys: "POSITION" "NORMAL" "TANGENT" "TEXCOORD_0" "COLOR_0" … -> accessor index
    pub attributes: HashMap<String, usize>,
    #[serde(default)]
    pub indices: Option<usize>, // absent ⇒ non-indexed draw (legal)
    #[serde(default)]
    pub material: Option<usize>, // absent ⇒ default material
    #[serde(default = "default_mode")]
    pub mode: u32, // absent ⇒ 4 (TRIANGLES)
}

// ── scene graph ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Scene {
    #[serde(default)]
    pub nodes: Vec<usize>, // root node indices
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Node {
    #[serde(default)]
    pub children: Vec<usize>,
    #[serde(default)]
    pub mesh: Option<usize>,
    // A node has EITHER `matrix` OR any of translation/rotation/scale - never both.
    // All optional; resolve to a local transform at extraction (phase 4), not here.
    #[serde(default)]
    pub matrix: Option<[f32; 16]>, // column-major (matches your Mat4x4!)
    #[serde(default)]
    pub translation: Option<[f32; 3]>,
    #[serde(default)]
    pub rotation: Option<[f32; 4]>, // quaternion, [x, y, z, w] order
    #[serde(default)]
    pub scale: Option<[f32; 3]>,
    #[serde(default)]
    pub name: Option<String>,
}

// ── materials / textures / images / samplers ─────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Material {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub pbr_metallic_roughness: Option<PbrMetallicRoughness>,
    #[serde(default)]
    pub normal_texture: Option<NormalTextureInfo>,
    #[serde(default)]
    pub occlusion_texture: Option<OcclusionTextureInfo>,
    #[serde(default)]
    pub emissive_texture: Option<TextureInfo>,
    #[serde(default)]
    pub emissive_factor: [f32; 3], // serde default ⇒ [0,0,0]  ✓ spec default
    #[serde(default = "default_opaque")]
    pub alpha_mode: String, // OPAQUE|MASK|BLEND
    #[serde(default = "default_cutoff")]
    pub alpha_cutoff: f32, // 0.5
    #[serde(default)]
    pub double_sided: bool, // ⇒ false
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PbrMetallicRoughness {
    #[serde(default = "default_white")]
    pub base_color_factor: [f32; 4], // [1,1,1,1]
    #[serde(default)]
    pub base_color_texture: Option<TextureInfo>,
    #[serde(default = "default_one")]
    pub metallic_factor: f32, // 1.0
    #[serde(default = "default_one")]
    pub roughness_factor: f32, // 1.0
    #[serde(default)]
    pub metallic_roughness_texture: Option<TextureInfo>,
}

// A texture reference: which texture, and which UV set feeds it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TextureInfo {
    pub index: usize, // textures[]
    #[serde(default)]
    pub tex_coord: u32, // selects TEXCOORD_<n>, default 0
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NormalTextureInfo {
    pub index: usize,
    #[serde(default)]
    pub tex_coord: u32,
    #[serde(default = "default_one")]
    pub scale: f32, // normal-map strength
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OcclusionTextureInfo {
    pub index: usize,
    #[serde(default)]
    pub tex_coord: u32,
    #[serde(default = "default_one")]
    pub strength: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Texture {
    #[serde(default)]
    pub source: Option<usize>, // -> images[]
    #[serde(default)]
    pub sampler: Option<usize>, // -> samplers[]
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Image {
    // An image is EITHER a uri (external / data-URI) OR a bufferView + mimeType
    // (glb-embedded). For glb it's the latter.
    #[serde(default)]
    pub uri: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>, // "image/png" | "image/jpeg"
    #[serde(default)]
    pub buffer_view: Option<usize>, // -> bufferViews[]
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Sampler {
    #[serde(default)]
    pub mag_filter: Option<u32>, // 9728 NEAREST, 9729 LINEAR
    #[serde(default)]
    pub min_filter: Option<u32>, // + mipmap variants
    #[serde(default)]
    pub wrap_s: Option<u32>, // 10497 REPEAT(default) 33071 CLAMP 33648 MIRROR
    #[serde(default)]
    pub wrap_t: Option<u32>,
}

// ── spec defaults ────────────────────────────────────────────────────────
fn default_mode() -> u32 {
    4
}
fn default_one() -> f32 {
    1.0
}
fn default_white() -> [f32; 4] {
    [1.0, 1.0, 1.0, 1.0]
}
fn default_opaque() -> String {
    "OPAQUE".to_string()
}
fn default_cutoff() -> f32 {
    0.5
}
