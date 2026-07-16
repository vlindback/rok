// obj_loader.rs
//

use rok_math::{vec2::Vec2, vec3::Vec3};

use crate::mesh::{IndexType, MeshData, MeshVertex};

use std::{collections::HashMap, str::SplitWhitespace};

pub struct ObjMaterialGroup {
    /// The name of the material (from `usemtl`)
    pub material_name: String,

    /// The starting index in the index buffer for this material
    pub index_start: usize,

    /// How many indices belong to this material
    pub index_count: usize,
}

pub struct ObjVertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub norm: [f32; 3],
}

/// A Group of geometry that all shares a single material.
pub struct ObjMesh {
    pub material_name: String,

    /// Indices pointing into the main vertex list.
    /// E.g., [0, 1, 2] makes the first triangle.
    pub indices: Vec<u32>,

    /// Index type for index buffers
    pub index_type: IndexType,
}

pub struct ObjModel {
    pub vertices: Vec<ObjVertex>,

    /// The model split up by material.
    pub sub_meshes: Vec<ObjMesh>,

    /// External libraries for materials referenced.
    pub material_libraries: Vec<String>,
}

impl ObjModel {
    pub(crate) fn from_data(
        vertices: Vec<ObjVertex>,
        sub_meshes: Vec<ObjMesh>,
        material_libraries: Vec<String>,
    ) -> Self {
        Self {
            vertices,
            sub_meshes,
            material_libraries,
        }
    }
}

// Dedup key: (position, uv, normal) source indices. Options because a face
// vertex may omit uv ("v//vn") or normal ("v")
type IndexKey = (usize, Option<usize>, Option<usize>);

// The dopamine rush of making this loader cache last as a
// natural design evolution was magical.

#[derive(Default)]
pub struct ObjLoader {
    pub(crate) positions: Vec<[f32; 3]>,
    pub(crate) normals: Vec<[f32; 3]>,
    pub(crate) uvs: Vec<[f32; 2]>,
    pub(crate) vertex_to_index: HashMap<IndexKey, u32>,
    pub(crate) current_face: Vec<u32>,
}

impl ObjLoader {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    /// Resets the cache for a new OBJ loading.
    pub(crate) fn reset(&mut self) {
        self.vertex_to_index.clear();
        self.positions.clear();
        self.normals.clear();
        self.uvs.clear();
        self.current_face.clear();
    }

    /// Parses an obj file blob.
    ///
    /// NOTE: .obj files are encoded in 7 bit ascii by convention.
    /// This is compatible with UTF8 which is what Rust uses natively.
    pub fn parse_data(&mut self, data: &str) -> Option<ObjModel> {
        // obj has three lists. Positions, normals and uv.
        // We parse these from the file then build a vertex buffer
        // then generate an index buffer from them.
        //

        // clear cache first
        self.reset();

        // This data gets moved into the mesh and cannot therefore be cached.
        let mut vertices: Vec<ObjVertex> = Vec::new();
        let mut sub_meshes: Vec<ObjMesh> = Vec::new();
        let mut material_libraries: Vec<String> = Vec::new();
        let mut current_material = String::from("default");
        let mut current_indices: Vec<u32> = Vec::new();
        let mut index_type = IndexType::U16;

        for line in data.lines() {
            let trimmed = line.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let mut parts = trimmed.split_whitespace();

            match parts.next() {
                Some("v") => {
                    let pos = parse_vec3(&mut parts)?;
                    self.positions.push(pos);
                }
                Some("vn") => {
                    let norm = parse_vec3(&mut parts)?;
                    self.normals.push(norm);
                }
                Some("vt") => {
                    let uv = parse_vec2(&mut parts)?;
                    self.uvs.push(uv);
                }
                Some("f") => {
                    self.current_face.clear();
                    for token in parts {
                        let (p, t, n) = parse_face_vertex(token)?;

                        // According to the 1980s specification, the components of a face cannot
                        // come after it. Therefore we reject that as invalid.

                        let pos_idx = resolve_index(p, self.positions.len())?;

                        let uv_idx = match t {
                            Some(t) => Some(resolve_index(t, self.uvs.len())?),
                            None => None,
                        };
                        let norm_idx = match n {
                            Some(n) => Some(resolve_index(n, self.normals.len())?),
                            None => None,
                        };

                        let key = (pos_idx, uv_idx, norm_idx);

                        let gpu_index = *self.vertex_to_index.entry(key).or_insert_with(|| {
                            let new_index = vertices.len() as u32;
                            vertices.push(ObjVertex {
                                pos: self.positions[pos_idx],
                                uv: uv_idx.map(|i| self.uvs[i]).unwrap_or([0.0, 0.0]),
                                norm: norm_idx.map(|i| self.normals[i]).unwrap_or([0.0, 0.0, 0.0]),
                            });
                            new_index
                        });
                        self.current_face.push(gpu_index);

                        // Check the index type, if it's > u16 max, bump the index type to u32
                        if index_type != IndexType::U32 && gpu_index > u32::MAX {
                            index_type = IndexType::U32;
                        }
                    }

                    if self.current_face.len() < 3 {
                        return None; // degenerate face
                    }

                    // Fan triangulation: (0,1,2), (0,2,3), (0,3,4), ...
                    for i in 1..self.current_face.len() - 1 {
                        current_indices.push(self.current_face[0]);
                        current_indices.push(self.current_face[i]);
                        current_indices.push(self.current_face[i + 1]);
                    }
                }
                Some("mtllib") => {
                    if let Some(filename) = parts.next() {
                        material_libraries.push(filename.to_string());
                    }
                }
                Some("usemtl") => {
                    if let Some(mat_name) = parts.next() {
                        // If we were already building a mesh, save it before swapping materials
                        if !current_indices.is_empty() {
                            sub_meshes.push(ObjMesh {
                                material_name: current_material,
                                // std::mem::take leaves current_indices empty so we can reuse it
                                indices: std::mem::take(&mut current_indices),
                                index_type,
                            });
                        }
                        current_material = mat_name.to_string();

                        // Reset index type in case it changed.
                        index_type = IndexType::U16;
                    }
                }
                _ => {} // Ignore anything else
            }
        }

        // EOF: push the final sub-mesh.
        if !current_indices.is_empty() {
            sub_meshes.push(ObjMesh {
                material_name: current_material,
                indices: current_indices,
                index_type,
            });
        }

        Some(ObjModel::from_data(
            vertices,
            sub_meshes,
            material_libraries,
        ))
    }
}

fn parse_vec3<'a>(parts: &mut impl Iterator<Item = &'a str>) -> Option<[f32; 3]> {
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    let z = parts.next()?.parse().ok()?;
    Some([x, y, z])
}

fn parse_vec2<'a>(parts: &mut impl Iterator<Item = &'a str>) -> Option<[f32; 2]> {
    let u = parts.next()?.parse().ok()?;
    let v = parts.next()?.parse().ok()?;
    Some([u, v])
}

/// One face vertex: `v`, `v/vt`, `v//vn`, or `v/vt/vn`.
/// Returns (position, optional uv, optional normal) as raw signed OBJ indices.
fn parse_face_vertex(token: &str) -> Option<(i64, Option<i64>, Option<i64>)> {
    let mut s = token.split('/');
    let v = s.next()?.parse::<i64>().ok()?; // position always present
    // Empty string (the "//" case) → None; missing entirely → None.
    let vt = s
        .next()
        .filter(|x| !x.is_empty())
        .map(|x| x.parse::<i64>())
        .transpose()
        .ok()?;
    let vn = s
        .next()
        .filter(|x| !x.is_empty())
        .map(|x| x.parse::<i64>())
        .transpose()
        .ok()?;
    Some((v, vt, vn))
}

/// OBJ indices are 1-based; negatives count back from the end (-1 = last).
/// `len` is the current count of that element list. Returns 0-based, or None
/// if out of range.
fn resolve_index(raw: i64, len: usize) -> Option<usize> {
    let idx = if raw > 0 {
        (raw - 1) as usize
    } else if raw < 0 {
        (len as i64 + raw) as usize // -1 -> len-1
    } else {
        return None; // 0 is invalid in OBJ
    };
    (idx < len).then_some(idx)
}

impl ObjModel {
    pub fn to_mesh_data(&self) -> Vec<MeshData> {
        let mut out = Vec::with_capacity(self.sub_meshes.len());
        for sub in &self.sub_meshes {
            let mut vertices: Vec<MeshVertex> = Vec::new();

            let indice_byte_size = match sub.index_type {
                IndexType::U16 => size_of::<u16>(),
                IndexType::U32 => size_of::<u32>(),
            };
            let indice_bytes = indice_byte_size * sub.indices.len();

            let mut indices: Vec<u8> = Vec::with_capacity(indice_bytes);

            let mut remap = std::collections::HashMap::<u32, u32>::new();

            for &global_idx in &sub.indices {
                let local = *remap.entry(global_idx).or_insert_with(|| {
                    let v = &self.vertices[global_idx as usize];
                    let new = vertices.len() as u32;
                    vertices.push(MeshVertex {
                        position: Vec3::new(v.pos[0], v.pos[1], v.pos[2]),
                        uv: Vec2::new(v.uv[0], v.uv[1]),
                        normal: Vec3::new(v.norm[0], v.norm[1], v.norm[2]),
                        tangent: Vec3::new(0.0, 0.0, 0.0), // filled below
                    });
                    new
                });
                match sub.index_type {
                    IndexType::U16 => {
                        let val = local as u16;
                        indices.extend_from_slice(&val.to_ne_bytes());
                    }
                    IndexType::U32 => {
                        let val = local;
                        indices.extend_from_slice(&val.to_ne_bytes());
                    }
                }
            }

            generate_tangents(&mut vertices, &indices, indice_byte_size);

            let index_type = sub.index_type;

            out.push(MeshData {
                vertices,
                indices,
                material_name: sub.material_name.clone(),
                index_type,
            });
        }
        out
    }
}

fn generate_tangents(vertices: &mut [MeshVertex], indices: &[u8], index_byte_width: usize) {
    let tri_stride = 3 * index_byte_width;

    // tangent fields are already zero from construction, accumulate in place.
    for tri in indices.chunks_exact(tri_stride) {
        let mut index_chunks = tri.chunks_exact(index_byte_width);

        let i0 = read_index(index_chunks.next().unwrap(), index_byte_width);
        let i1 = read_index(index_chunks.next().unwrap(), index_byte_width);
        let i2 = read_index(index_chunks.next().unwrap(), index_byte_width);

        // Read the three vertices data before the mutable borrows. Copy the
        // small values out so we're not holding &vertices while we mutate it.
        let (p0, p1, p2) = (
            vertices[i0].position,
            vertices[i1].position,
            vertices[i2].position,
        );
        let (uv0, uv1, uv2) = (vertices[i0].uv, vertices[i1].uv, vertices[i2].uv);

        let e1 = p1 - p0;
        let e2 = p2 - p0;
        let du1 = uv1.x() - uv0.x();
        let dv1 = uv1.y() - uv0.y();
        let du2 = uv2.x() - uv0.x();
        let dv2 = uv2.y() - uv0.y();

        let denom = du1 * dv2 - du2 * dv1;
        if denom.abs() < 1e-8 {
            continue;
        }
        let r = 1.0 / denom;
        let tangent = Vec3::new(
            r * (dv2 * e1.x() - dv1 * e2.x()),
            r * (dv2 * e1.y() - dv1 * e2.y()),
            r * (dv2 * e1.z() - dv1 * e2.z()),
        );

        vertices[i0].tangent = vertices[i0].tangent + tangent;
        vertices[i1].tangent = vertices[i1].tangent + tangent;
        vertices[i2].tangent = vertices[i2].tangent + tangent;
    }

    // Second pass: orthonormalize against the normal.
    for v in vertices.iter_mut() {
        let n = v.normal;
        let t = v.tangent - n * n.dot(v.tangent);
        let len = t.length();
        v.tangent = if len > 1e-6 {
            t * (1.0 / len)
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        };
    }
}

#[inline(always)]
fn read_index(bytes: &[u8], width: usize) -> usize {
    // Create a temporary 8-byte buffer initialized to 0
    let mut buf = [0u8; 8];

    // Copy only the active bytes into the start of our buffer.
    // Since buf is 8 bytes, this is safe for width 1, 2, 4, or 8.
    buf[..width].copy_from_slice(bytes);
    u64::from_le_bytes(buf) as usize
}
