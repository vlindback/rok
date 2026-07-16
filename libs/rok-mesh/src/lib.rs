// lib.rs
//
// rok-mesh - library for loading complex geometry
//

pub(crate) mod mesh;

// loaders
mod obj_loader;

// re-export
pub use mesh::{IndexType, MeshData, MeshVertex};
pub use obj_loader::ObjLoader;

// debug todo remove
pub use mesh::debug_dump_mesh_data_to_file;
