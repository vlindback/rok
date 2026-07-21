// lib.rs
//
// rok-mesh - library for loading complex geometry
//

pub(crate) mod mesh;

// shared
mod image;

// loaders
mod gltf_loader;
mod gltf_schema;
mod obj_loader;

// re-export
pub use gltf_loader::{GltfLoader, MaterialDesc};
pub use image::ImageData;
pub use mesh::{IndexType, MeshData, MeshVertex};
pub use obj_loader::ObjLoader;
