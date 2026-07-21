// model.rs
//

use rok_mesh::{ImageData, MaterialDesc, MeshData};
use rok_renderer::{MapImage, MaterialCreateInfo};
pub(crate) struct Model {
    pub meshes: Vec<MeshData>,        // baked world-local geometry
    pub materials: Vec<MaterialDesc>, // parallel material descriptions
}

impl Model {
    pub fn from_data(meshes: Vec<MeshData>, materials: Vec<MaterialDesc>) -> Self {
        Self { meshes, materials }
    }
}

pub(crate) fn material_create_info_from_desc(d: &MaterialDesc) -> MaterialCreateInfo<'_> {
    MaterialCreateInfo {
        albedo: image_data_to_map_image(&d.base_color),
        normal: image_data_to_map_image(&d.normal),
        metallic_roughness: image_data_to_map_image(&d.metallic_roughness),
        emissive: image_data_to_map_image(&d.emissive),
        base_color_factor: d.base_color_factor,
        emissive_factor: d.emissive_factor,
        metallic_factor: d.metallic_factor,
        roughness_factor: d.roughness_factor,
        normal_scale: d.normal_scale,
        occlusion_strength: d.occlusion_strength,
    }
}

pub(crate) fn default_material_info() -> MaterialCreateInfo<'static> {
    MaterialCreateInfo {
        albedo: None,
        normal: None,
        metallic_roughness: None,
        emissive: None,
        base_color_factor: [0.8, 0.8, 0.8, 1.0],
        emissive_factor: [0.0, 0.0, 0.0],
        metallic_factor: 0.0,
        roughness_factor: 0.7,
        normal_scale: 1.0,
        occlusion_strength: 1.0,
    }
}

// image_data_to_map_image moves here from engine.rs, it belongs with the
// MaterialDesc to MaterialCreateInfo translation.
pub(crate) fn image_data_to_map_image(img: &Option<ImageData>) -> Option<MapImage<'_>> {
    img.as_ref().map(|i| MapImage {
        width: i.width,
        height: i.height,
        pixels: &i.rgba8,
    })
}
