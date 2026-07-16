// mesh.rs

use rok_math::{vec2::Vec2, vec3::Vec3};

// DEBUG TODO REMOVE
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

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
    pub tangent: Vec3,
}

pub struct MeshData {
    pub vertices: Vec<MeshVertex>,
    /// Either u16 or u32 depending on the index_type field.
    pub indices: Vec<u8>,
    /// Which material this sub-mesh referenced (by name, resolved later).
    pub material_name: String,
    pub index_type: IndexType,
}

// TODO: remove later
pub fn debug_dump_mesh_data_to_file<P: AsRef<Path>>(
    path: P,
    mesh: &MeshData,
) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    // 1. Write Metadata Header
    writeln!(writer, "# Material: {}", mesh.material_name)?;
    writeln!(writer, "# Vertex Count: {}", mesh.vertices.len())?;
    writeln!(writer, "# Index Count: {}", mesh.indices.len())?;
    writeln!(writer)?;

    // 2. Write Vertices List
    writeln!(writer, "[Vertices]")?;
    writeln!(
        writer,
        "# format: position.x position.y position.z | uv.x uv.y | normal.x normal.y normal.z | tangent.x tangent.y tangent.z"
    )?;
    for v in &mesh.vertices {
        writeln!(
            writer,
            "{} {} {} | {} {} | {} {} {} | {} {} {}",
            v.position.x(),
            v.position.y(),
            v.position.z(),
            v.uv.x(),
            v.uv.y(),
            v.normal.x(),
            v.normal.y(),
            v.normal.z(),
            v.tangent.x(),
            v.tangent.y(),
            v.tangent.z()
        )?;
    }
    writeln!(writer)?;

    // 3. Write Indices List
    writeln!(writer, "[Indices]")?;
    // Writing 3 indices per line (representing triangles) makes it much easier to visually debug
    for chunk in mesh.indices.chunks(3) {
        let line = chunk
            .iter()
            .map(|idx| idx.to_string())
            .collect::<Vec<String>>()
            .join(" ");
        writeln!(writer, "{}", line)?;
    }

    writer.flush()?;
    Ok(())
}
