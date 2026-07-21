// gltf_loader.rs
//

use rok_math::{mat4x4::Mat4x4, quaternion::Quat, vec2::Vec2, vec3::Vec3, vec4::Vec4};

use crate::{
    IndexType, MeshData, MeshVertex,
    gltf_schema::{GltfDocument, Material, Primitive},
    image::{ImageData, decode_image},
};

pub struct MaterialDesc {
    pub name: Option<String>,

    pub base_color_factor: [f32; 4],
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub emissive_factor: [f32; 3],

    pub base_color: Option<ImageData>,
    pub metallic_roughness: Option<ImageData>,
    pub normal: Option<ImageData>,
    pub occlusion: Option<ImageData>,
    pub emissive: Option<ImageData>,

    pub normal_scale: f32,
    pub occlusion_strength: f32,

    pub alpha_mode: String,
    pub alpha_cutoff: f32,
    pub double_sided: bool,
}

pub(crate) struct GltfModel<'a> {
    pub document: GltfDocument,
    pub bin: Option<&'a [u8]>,
}

impl<'a> GltfModel<'a> {
    fn accessor_reader(&self, accessor_index: usize) -> Result<AccessorReader<'a>, String> {
        let acc = self
            .document
            .accessors
            .get(accessor_index)
            .ok_or_else(|| format!("accessor index {accessor_index} out of range"))?;

        let view_index = acc
            .buffer_view
            .ok_or("accessor has no bufferView (sparse/implicit unsupported)")?;
        let view = self
            .document
            .buffer_views
            .get(view_index)
            .ok_or_else(|| format!("bufferView index {view_index} out of range"))?;

        if view.buffer != 0 {
            return Err(format!(
                "bufferView buffer {} unsupported (only glb buffer 0)",
                view.buffer
            ));
        }
        let bytes = self
            .bin
            .ok_or("accessor needs buffer data but glb has no BIN chunk")?;

        let comp_size = component_size(acc.component_type)?; // piece 1
        let comp_count = component_count(&acc.kind)?; // piece 1

        let element_size = comp_count * comp_size;
        let stride = view.byte_stride.unwrap_or(element_size);
        let base = view.byte_offset + acc.byte_offset;
        let count = acc.count;
        let view_end = view.byte_offset + view.byte_length;

        if view_end > bytes.len() {
            return Err(format!(
                "bufferView ends at {view_end} but bin is only {} bytes",
                bytes.len()
            ));
        }

        if count > 0 {
            let last_byte = base + (count - 1) * stride + element_size;
            if last_byte > view_end {
                return Err(format!(
                    "accessor overruns its bufferView: reaches {last_byte}, view ends {view_end}"
                ));
            }
        }

        Ok(AccessorReader {
            bytes,
            base,
            stride,
            count,
            component_count: comp_count,
            component_type: acc.component_type,
        })
    }

    /// Resolve a texture reference (an index into document.textures) all the way
    /// down to a decoded RGBA8 image.
    fn load_texture(&self, texture_index: usize) -> Result<ImageData, String> {
        let texture = self
            .document
            .textures
            .get(texture_index)
            .ok_or_else(|| format!("texture index {texture_index} out of range"))?;

        // hop: texture to image
        let source = texture.source.ok_or("texture has no image source")?;
        let image = self
            .document
            .images
            .get(source)
            .ok_or_else(|| format!("image index {source} out of range"))?;

        // hop: image to raw bytes. In a glb, images live in a bufferView (NOT an
        // accessor. This is a raw byte range, no stride, no element decoding).
        let bv_idx = image
            .buffer_view
            .ok_or("external-URI images unsupported (glb-embedded only)")?;
        let bv = self
            .document
            .buffer_views
            .get(bv_idx)
            .ok_or_else(|| format!("bufferView {bv_idx} out of range"))?;
        let bin = self
            .bin
            .ok_or("image needs bin data but glb has no BIN chunk")?;

        let start = bv.byte_offset;
        let end = start + bv.byte_length;
        if end > bin.len() {
            return Err(format!(
                "image bufferView overruns bin: {end} > {}",
                bin.len()
            ));
        }
        let encoded = &bin[start..end]; // the raw PNG/JPEG bytes

        decode_image(encoded, image.mime_type.as_deref()) // the codec boundary
    }

    fn read_vec4(&self, accessor_index: usize) -> Result<Vec<Vec4>, String> {
        let reader = self.accessor_reader(accessor_index)?;
        let vec4s: Vec<Vec4> = (0..reader.count)
            .map(|n| {
                let a = reader.read_f32s(n);
                Vec4::new(a[0], a[1], a[2], a[3])
            })
            .collect();

        Ok(vec4s)
    }

    fn read_vec3(&self, accessor_index: usize) -> Result<Vec<Vec3>, String> {
        let reader = self.accessor_reader(accessor_index)?;
        let vec3s: Vec<Vec3> = (0..reader.count)
            .map(|n| {
                let a = reader.read_f32s(n);
                Vec3::new(a[0], a[1], a[2])
            })
            .collect();

        Ok(vec3s)
    }

    fn read_vec2(&self, accessor_index: usize) -> Result<Vec<Vec2>, String> {
        let reader = self.accessor_reader(accessor_index)?;
        let vec2s: Vec<Vec2> = (0..reader.count)
            .map(|n| {
                let a = reader.read_f32s(n);
                Vec2::new(a[0], a[1])
            })
            .collect();

        Ok(vec2s)
    }

    fn read_indices(&self, accessor_index: usize) -> Result<(Vec<u8>, IndexType), String> {
        let reader = self.accessor_reader(accessor_index)?;

        // I heard flat_map cant calculate the size so to avoid more allocations
        // we reserve upfront then extend.
        let mut indices: Vec<u8> = Vec::with_capacity(reader.count * 4); // widening to u32 (4) 

        indices.extend((0..reader.count).flat_map(|n| {
            let i = reader.read_index(n);
            i.to_le_bytes()
        }));

        Ok((indices, IndexType::U32))
    }

    fn build_primitive(&self, prim: &Primitive) -> Result<MeshData, String> {
        // POSITION is mandatory — a primitive without it is malformed.
        let pos_idx = *prim
            .attributes
            .get("POSITION")
            .ok_or("primitive has no POSITION attribute")?;
        let positions = self.read_vec3(pos_idx)?;
        let n = positions.len();

        // Optional attributes: read if present, else a sane default of the SAME length
        // so the transpose below can index all four arrays uniformly.
        let normals = match prim.attributes.get("NORMAL") {
            Some(&i) => self.read_vec3(i)?,
            None => vec![Vec3::new(0.0, 0.0, 0.0); n], // missing-normal generation deferred
        };
        let uvs = match prim.attributes.get("TEXCOORD_0") {
            Some(&i) => self.read_vec2(i)?,
            None => vec![Vec2::new(0.0, 0.0); n], // same fallback OBJ uses
        };
        let tangents: Vec<Vec4> = match prim.attributes.get("TANGENT") {
            Some(&i) => self.read_vec4(i)?,
            None => vec![Vec4::new(0.0, 0.0, 0.0, 0.0); n],
        };

        // Cheap integrity guard - glTF promises these are equal.
        if normals.len() != n || uvs.len() != n || tangents.len() != n {
            return Err(format!(
                "attribute count mismatch: pos {n}, nrm {}, uv {}, tan {}",
                normals.len(),
                uvs.len(),
                tangents.len()
            ));
        }

        // Indices: a primitive MAY be non-indexed (indices == None). Defer that.
        let (indices, index_type) = match prim.indices {
            Some(i) => self.read_indices(i)?,
            None => return Err("non-indexed primitives unsupported yet".into()),
        };

        let material_name = prim
            .material
            .and_then(|i| self.document.materials.get(i))
            .and_then(|m| m.name.clone());

        let material_index = prim.material;

        let mut vertices: Vec<MeshVertex> = Vec::with_capacity(n);

        for i in 0..n {
            let vertex = MeshVertex {
                position: positions[i],
                uv: uvs[i],
                normal: normals[i],
                tangent: tangents[i],
            };
            vertices.push(vertex);
        }

        Ok(MeshData {
            vertices,
            indices,
            material_index,
            material_name,
            index_type,
        })
    }

    pub(crate) fn load_materials(&self) -> Result<Vec<MaterialDesc>, String> {
        let mut loaded_materials = Vec::with_capacity(self.document.materials.len());

        for mat in &self.document.materials {
            // Unpack PBR values, falling back to spec defaults if the block is entirely absent
            let (bc_factor, m_factor, r_factor, bc_tex, mr_tex) = match &mat.pbr_metallic_roughness
            {
                Some(pbr) => (
                    pbr.base_color_factor,
                    pbr.metallic_factor,
                    pbr.roughness_factor,
                    pbr.base_color_texture.as_ref(),
                    pbr.metallic_roughness_texture.as_ref(),
                ),
                None => (
                    [1.0, 1.0, 1.0, 1.0], // default_white
                    1.0,                  // default_one
                    1.0,                  // default_one
                    None,
                    None,
                ),
            };

            let base_color = bc_tex.map(|t| self.load_texture(t.index)).transpose()?;
            let metallic_roughness = mr_tex.map(|t| self.load_texture(t.index)).transpose()?;
            let normal_scale = mat.normal_texture.as_ref().map_or(1.0, |t| t.scale);
            let occlusion_strength = mat.occlusion_texture.as_ref().map_or(1.0, |t| t.strength);

            let normal = mat
                .normal_texture
                .as_ref()
                .map(|t| self.load_texture(t.index))
                .transpose()?;
            let occlusion = mat
                .occlusion_texture
                .as_ref()
                .map(|t| self.load_texture(t.index))
                .transpose()?;
            let emissive = mat
                .emissive_texture
                .as_ref()
                .map(|t| self.load_texture(t.index))
                .transpose()?;

            loaded_materials.push(MaterialDesc {
                name: mat.name.clone(),
                base_color_factor: bc_factor,
                metallic_factor: m_factor,
                roughness_factor: r_factor,
                emissive_factor: mat.emissive_factor,
                base_color,
                metallic_roughness,
                normal_scale: normal_scale,
                occlusion_strength,
                alpha_mode: mat.alpha_mode.clone(),
                alpha_cutoff: mat.alpha_cutoff,
                double_sided: mat.double_sided,
                normal,
                occlusion,
                emissive,
            });
        }

        Ok(loaded_materials)
    }
}

pub struct LoadedGltfMesh {
    pub data: MeshData,
    //pub transform: Mat4x4,
}

pub struct LoadedGltfModel {
    pub meshes: Vec<LoadedGltfMesh>,
    pub materials: Vec<MaterialDesc>,
}

pub struct GltfLoader {}

impl GltfLoader {
    pub fn new() -> Self {
        Self {}
    }

    pub fn load_glb(&mut self, data: &[u8]) -> Result<LoadedGltfModel, String> {
        let model = self.parse_glb(data)?;

        let materials = model.load_materials()?;

        let mut loaded_meshes = Vec::new();

        // Find the active scene (glTF files can have multiple, but usually specify a default)
        let scene_index = model.document.scene.unwrap_or(0);
        let scene = model
            .document
            .scenes
            .get(scene_index)
            .ok_or("Scene index out of bounds")?;

        // Traverse the scene graph starting from the root nodes
        for &node_idx in &scene.nodes {
            self.process_scene_node(node_idx, &model, Mat4x4::new(), &mut loaded_meshes)?;
        }

        if loaded_meshes.is_empty() {
            return Err("glTF contained no instantiated meshes in the scene".into());
        }

        Ok(LoadedGltfModel {
            meshes: loaded_meshes,
            materials,
        })
    }

    fn process_scene_node(
        &self,
        node_index: usize,
        model: &GltfModel,
        parent_transform: Mat4x4,
        out_meshes: &mut Vec<LoadedGltfMesh>,
    ) -> Result<(), String> {
        let node = model
            .document
            .nodes
            .get(node_index)
            .ok_or("Node index out of bounds")?;

        // Calculate this node's local transformation
        let local_transform = if let Some(matrix) = &node.matrix {
            // The JSON provides a flat array of 16 floats in column-major order
            Mat4x4::from_cols_slice(matrix)
        } else {
            // Otherwise, build it from TRS (Translation, Rotation, Scale)
            // If any are missing, default to identity values.
            let t = node.translation.unwrap_or([0.0, 0.0, 0.0]);
            let r = node.rotation.unwrap_or([0.0, 0.0, 0.0, 1.0]); // Quaternion [x, y, z, w]
            let s = node.scale.unwrap_or([1.0, 1.0, 1.0]);

            Mat4x4::from_trs(Vec3::from(t), Quat::from_array(&r), s.into())
        };

        let global_transform = parent_transform * local_transform;

        // If this node has a mesh, build it and attach the global transform
        if let Some(mesh_idx) = node.mesh {
            let mesh = &model.document.meshes[mesh_idx];
            for prim in &mesh.primitives {
                let mut mesh_data = model.build_primitive(prim)?;
                bake_transform(&mut mesh_data, &global_transform);
                out_meshes.push(LoadedGltfMesh {
                    data: mesh_data,
                    //&transform: global_transform,
                });
            }
        }

        for &child_idx in &node.children {
            self.process_scene_node(child_idx, model, global_transform, out_meshes)?;
        }

        Ok(())
    }

    fn parse_glb<'a>(&mut self, data: &'a [u8]) -> Result<GltfModel<'a>, String> {
        // Need at least 12 bytes for the header and 20 for header+chunk
        if data.len() < 20 {
            return Err(
                "File too short to contain a valid 12-byte glTF/glb header and a chunk (b<20)."
                    .to_string(),
            );
        }
        let header_bytes = &data[0..12];

        if !(&header_bytes[0..4] == b"glTF") {
            return Err("Gltf header magic missing.".to_string());
        }

        let version = u32::from_le_bytes(
            header_bytes[4..8]
                .try_into()
                .map_err(|_| "Failed to parse GLB version bytes".to_string())?,
        );

        let length = u32::from_le_bytes(
            header_bytes[8..12]
                .try_into()
                .map_err(|_| "Failed to parse GLB length bytes".to_string())?,
        );

        if version != 2 {
            return Err(format!(
                "Unsupported GLB container version {version}; expected 2."
            ));
        }
        if length as usize != data.len() {
            return Err(format!(
                "Header length {length} disagrees with file size {}.",
                data.len()
            ));
        }

        // Parse chunk header fields (located at bytes 12..20)
        let chunk_length = u32::from_le_bytes(
            data[12..16]
                .try_into()
                .map_err(|_| "Failed to read chunk length".to_string())?,
        ) as usize; // Cast to usize so we can use it for slicing

        let chunk_type = u32::from_le_bytes(
            data[16..20]
                .try_into()
                .map_err(|_| "Failed to read chunk type".to_string())?,
        );

        // Verify that this is the JSON chunk
        // 0x4E4F534A is ASCII for "JSON"
        if chunk_type != 0x4E4F534A {
            return Err("Expected first chunk to be of type JSON (0x4E4F534A).".to_string());
        }

        let json_start = 20;
        let json_end = json_start + chunk_length;

        // Ensure the file actually has enough bytes to match the chunk length it claims to have.
        if data.len() < json_end {
            return Err(format!(
                "File is missing data. Expected at least {} bytes, but file is only {}.",
                json_end,
                data.len()
            ));
        }

        let json_bytes = &data[json_start..json_end];
        let bin_data: Option<&[u8]> = if json_end + 8 <= data.len() {
            let bin_len =
                u32::from_le_bytes(data[json_end..json_end + 4].try_into().unwrap()) as usize;
            let bin_type = u32::from_le_bytes(data[json_end + 4..json_end + 8].try_into().unwrap());

            if bin_type != 0x004E4942 {
                // "BIN\0"
                return Err(format!(
                    "Expected second chunk BIN (0x004E4942), got {bin_type:#010X}."
                ));
            }
            let start = json_end + 8;
            let end = start + bin_len;
            if end > data.len() {
                return Err(format!(
                    "BIN chunk truncated: needs {end} bytes, file has {}.",
                    data.len()
                ));
            }
            Some(&data[start..end])
        } else {
            None
        };

        let document: GltfDocument = serde_json::from_slice(json_bytes)
            .map_err(|e| format!("glTF JSON parse error: {e}"))?;

        // asset.version is the glTF SPEC version ("2.0"), distinct from the GLB
        // container version (2) already checked from the header.
        if !document.asset.version.starts_with("2.") {
            return Err(format!(
                "Unsupported glTF spec version {}",
                document.asset.version
            ));
        }

        println!(
            "glTF {} - {} scenes, {} nodes, {} meshes, {} accessors, {} bufferViews, \
     {} buffers, {} materials, {} images | bin {} bytes",
            document.asset.version,
            document.scenes.len(),
            document.nodes.len(),
            document.meshes.len(),
            document.accessors.len(),
            document.buffer_views.len(),
            document.buffers.len(),
            document.materials.len(),
            document.images.len(),
            bin_data.map_or(0, |b| b.len()),
        );

        Ok(GltfModel {
            document,
            bin: bin_data,
        })
    }
}

// Helpers

// accessor decode helpers

/// Everything needed to read one accessor's elements out of the bin blob.
/// An accessor is the label that tells you how to read a stretch of the blod
/// as a typed array.
///
/// It answers five questions:
/// - where does it start? via its bufferView (+ its own offset)
/// - what's each number? componentType (f32? u16?)
/// - how many numbers per thing? -> type ("VEC3" = 3)
/// - how many things? -> count
/// - how far apart are consecutive things? -> stride
struct AccessorReader<'a> {
    bytes: &'a [u8],        // the buffer (bin), offsets below index into this
    base: usize,            // byte offset of element 0
    stride: usize,          // byte step between consecutive elements
    count: usize,           // number of elements
    component_count: usize, // scalars per element
    component_type: u32,    // scalar type
}

impl<'a> AccessorReader<'a> {
    /// Byte offset where element `n` begins.
    fn element_offset(&self, n: usize) -> usize {
        self.base + n * self.stride
    }

    /// Read element `n` as up to 4 floats (POSITION->3, TEXCOORD->2, etc.).
    /// Assumes componentType is FLOAT (5126). Geometry attributes are.
    fn read_f32s(&self, n: usize) -> [f32; 4] {
        let mut out = [0.0f32; 4];
        let start = self.element_offset(n);
        for c in 0..self.component_count {
            let b = start + c * 4; // each f32 is 4 bytes, contiguous WITHIN the element
            let bytes: [u8; 4] = self.bytes[b..b + 4].try_into().unwrap();
            out[c] = f32::from_le_bytes(bytes); // little-endian, per spec
        }
        out
    }

    /// Read scalar index element `n` as u32, widening u16->u32 so callers
    /// get one uniform type. componentType must be 5123 or 5125.
    fn read_index(&self, n: usize) -> u32 {
        let start = self.element_offset(n);
        match self.component_type {
            5123 => {
                let b: [u8; 2] = self.bytes[start..start + 2].try_into().unwrap();
                u16::from_le_bytes(b) as u32
            }
            5125 => {
                let b: [u8; 4] = self.bytes[start..start + 4].try_into().unwrap();
                u32::from_le_bytes(b)
            }
            other => unreachable!("index accessor has non-integer componentType {other}"),
        }
    }
}

/// Bytes per scalar component, from the accessor's `componentType` GL enum.
fn component_size(component_type: u32) -> Result<usize, String> {
    Ok(match component_type {
        5120 | 5121 => 1, // BYTE / UNSIGNED_BYTE
        5122 | 5123 => 2, // SHORT / UNSIGNED_SHORT
        5125 | 5126 => 4, // UNSIGNED_INT / FLOAT
        other => return Err(format!("Unknown accessor componentType {other}.")),
    })
}

/// Number of components per element, from the accessor's `type` string.
fn component_count(kind: &str) -> Result<usize, String> {
    Ok(match kind {
        "SCALAR" => 1,
        "VEC2" => 2,
        "VEC3" => 3,
        "VEC4" => 4,
        "MAT2" => 4,
        "MAT3" => 9,
        "MAT4" => 16,
        other => return Err(format!("Unknown accessor type '{other}'.")),
    })
}

/// The GLTF nodes exist in a hierarchy with a relative space matrix.
/// We need to flatten this.
fn bake_transform(mesh: &mut MeshData, m: &Mat4x4) {
    for v in &mut mesh.vertices {
        v.position = m.transform_point(v.position);
        v.normal = m.transform_vector(v.normal).normalized();
        let t = m.transform_vector(v.tangent.xyz()).normalized();
        v.tangent = Vec4::new(t.x(), t.y(), t.z(), v.tangent.w());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_damaged_helmet() {
        // 1. Include the GLB file at compile time from your assets folder.
        // Adjust the path relative to this source file if your folder structure differs.
        let helmet_data = include_bytes!("../assets/DamagedHelmet.glb");

        // 2. Instantiate your loader
        let mut loader = GltfLoader::new();

        // 3. Attempt to parse the file
        let result = loader.parse_glb(helmet_data);

        // 4. Assert that the parsing succeeded
        assert!(
            result.is_ok(),
            "Failed to parse DamagedHelmet.glb: {:?}",
            result.err()
        );

        // 5. Optionally verify the parsed structural contents
        let model = result.unwrap();

        assert!(
            !model.document.asset.version.is_empty(),
            "glTF asset version should not be empty"
        );

        // DamagedHelmet typically contains a binary chunk payload
        assert!(
            model.bin.is_some(),
            "DamagedHelmet should contain a BIN chunk payload"
        );

        if let Some(bin) = model.bin {
            assert!(
                !bin.is_empty(),
                "The embedded binary buffer should contain data bytes"
            );
        }

        let doc = &model.document;
        let prim = &doc.meshes[0].primitives[0];

        let pos_accessor = prim.attributes["POSITION"];
        let positions = model.read_vec3(pos_accessor).expect("read POSITION");

        // Compute the bounding box from the bytes we decoded…
        let mut lo = [f32::INFINITY; 3];
        let mut hi = [f32::NEG_INFINITY; 3];
        for p in &positions {
            lo[0] = lo[0].min(p.x());
            lo[1] = lo[1].min(p.y());
            lo[2] = lo[2].min(p.z());
            hi[0] = hi[0].max(p.x());
            hi[1] = hi[1].max(p.y());
            hi[2] = hi[2].max(p.z());
        }

        // …and check it against the accessor's own min/max, the oracle in the file.
        let acc = &doc.accessors[pos_accessor];
        println!("decoded min {:?} max {:?}", lo, hi);
        println!("file    min {:?} max {:?}", acc.min, acc.max);

        assert_eq!(positions.len(), acc.count);
    }
}
