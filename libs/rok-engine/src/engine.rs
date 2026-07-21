// engine.rs

// rok engine

use std::{num::NonZeroU32, sync::atomic::AtomicBool};

use crate::instance::Instance;
use crate::model::{Model, default_material_info, material_create_info_from_desc};
use crate::model_registry::{ModelHandle, ModelRegistry};
use crate::scene::Scene;
use crate::{
    api::build, camera::OrbitCamera, error::EngineError, frame::FrameInput, input::InputState,
    target::Target, transform::Transform,
};
use rok_abi::{EngineApi, NativeSurfaceHandle, engine_api::EngineHandle, input::ScanCode};
use rok_log::log_info;
use rok_math::{quaternion::Quat, vec3::Vec3};
use rok_mesh::{GltfLoader, ImageData, MaterialDesc, MeshData, MeshVertex, ObjLoader};
use rok_renderer::{MapImage, Renderer, RendererError};
use rok_renderer::{MaterialCreateInfo, MeshHandle};
use rok_renderer::{RenderCommand, cube};
use rok_renderer::{RendererConfig, RendererResult};

const SUZANNE_OBJ: &[u8] = include_bytes!("../assets/suzanne.obj");
const DAMAGED_HELMET_GLB: &[u8] = include_bytes!("../assets/DamagedHelmet.glb");

pub struct EngineConfig {
    pub target_path: String,
    pub surface: Option<NativeSurfaceHandle>,
}

pub struct Engine {
    api: Option<EngineApi>,
    target: Option<Target>,
    renderer: Renderer,
    input_state: InputState,
    should_quit: AtomicBool,
    camera: OrbitCamera,
    pub scene: Scene,
    render_commands: Vec<RenderCommand>,
    model_registry: ModelRegistry,
}

impl Engine {
    pub fn from_config(config: &EngineConfig) -> Result<Box<Self>, EngineError> {
        let renderer_config = RendererConfig {
            app_name: "rok".into(),
            frames_in_flight: unsafe { NonZeroU32::new_unchecked(2) },
            surface: config.surface,
            vsync: false, // TODO: load from config
        };

        let mut renderer =
            Renderer::from_config(&renderer_config).map_err(EngineError::Renderer)?;

        let helmet_model = load_gltf_model(DAMAGED_HELMET_GLB)?;
        let suzanne_model = load_obj_model(SUZANNE_OBJ)?;

        let mut engine = Box::new(Engine {
            renderer,
            api: None,
            target: None,
            input_state: InputState::new(),
            should_quit: AtomicBool::new(false),
            camera: OrbitCamera::new(),
            scene: Scene::new(),
            render_commands: Vec::new(),
            model_registry: ModelRegistry::new(),
        });

        let suzanne = engine.register_model(&suzanne_model)?;
        let helmet = engine.register_model(&helmet_model)?;

        let helmet_transform = Transform::identity();
        let suzanne_transform = Transform::from_position(Vec3::new(3.0, 0.0, 0.0));

        engine.scene.spawn(suzanne, suzanne_transform);
        engine.scene.spawn(helmet, helmet_transform);

        let handle = (&mut *engine as *mut Engine).cast::<EngineHandle>();

        engine.api = Some(build(handle));

        let target = Target::load(&config.target_path, engine.api.as_ref().unwrap(), None)?;

        engine.target = Some(target);

        Ok(engine)
    }

    pub fn update(&mut self, input: &FrameInput) {
        // Handle resize updates.
        if input.lifecycle.surface_changed {
            self.renderer.on_resize(
                input.lifecycle.surface_width,
                input.lifecycle.surface_height,
            );
            if let Some(target) = self.target.as_mut() {
                target.on_resize(
                    input.lifecycle.surface_width,
                    input.lifecycle.surface_height,
                );
            }
        }

        self.input_state.ingest(input.events);
        self.camera.update(&self.input_state, input.delta_time);

        if let Some(target) = self.target.as_mut() {
            target.update(input.delta_time);
        }

        // Run the targets update.
        if let Some(target) = self.target.as_mut() {
            target.update(input.delta_time);
        }
    }

    pub fn render(&mut self) {
        let commands = &mut self.render_commands;

        commands.clear();
        for instance in &self.scene.instances {
            let registered = self.model_registry.get(instance.model);
            for &(mesh, material) in &registered.parts {
                commands.push(RenderCommand::DrawMesh {
                    mesh,
                    material,
                    model: instance.transform.to_matrix(),
                });
            }
        }

        self.renderer
            .render(self.camera.view(), self.camera.eye(), &self.render_commands);

        if let Some(target) = self.target.as_mut() {
            target.render();
        }
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn register_model(&mut self, model: &Model) -> RendererResult<ModelHandle> {
        let mut parts = Vec::with_capacity(model.meshes.len());
        for mesh in &model.meshes {
            let mesh_handle = self.renderer.register_mesh(mesh)?;
            let material_handle = match mesh.material_index.and_then(|i| model.materials.get(i)) {
                Some(desc) => {
                    let info = material_create_info_from_desc(desc); // borrows `desc` (from model)
                    self.renderer.create_material(info)? // uploads, borrow ends here
                }
                None => {
                    let info = default_material_info(); // 'static, no borrow
                    self.renderer.create_material(info)?
                }
            };

            parts.push((mesh_handle, material_handle));
        }
        Ok(self.model_registry.add(parts))
    }
}

// TODO: engine shouldnt do this
fn load_obj_model(data: &[u8]) -> Result<Model, EngineError> {
    let meshes = ObjLoader::new()
        .parse_data(data)
        .ok_or(EngineError::EngineInitFailure)?
        .to_mesh_data();
    // OBJ has no materials yet → empty. Each mesh's material_index is None,
    // so register_model gives it the fallback. No material built here.
    Ok(Model::from_data(meshes, Vec::new()))
}

fn load_gltf_model(data: &[u8]) -> Result<Model, EngineError> {
    let loaded = GltfLoader::new()
        .load_glb(data)
        .map_err(|err| EngineError::EngineInitFailure)?;

    // load_glb gives LoadedGltfModel { meshes: Vec<LoadedGltfMesh>, materials: Vec<MaterialDesc> }.
    // Model wants Vec<MeshData> — unwrap each LoadedGltfMesh to its baked MeshData.
    let meshes = loaded.meshes.into_iter().map(|m| m.data).collect();

    Ok(Model::from_data(meshes, loaded.materials))
}
// TODO: move later
fn image_data_to_map_image(image_data: &Option<ImageData>) -> Option<MapImage<'_>> {
    image_data.as_ref().map(|data| MapImage {
        width: data.width,
        height: data.height,
        pixels: &data.rgba8,
    })
}
