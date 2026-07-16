// engine.rs

// rok engine

use std::{num::NonZeroU32, sync::atomic::AtomicBool};

use crate::scene::Scene;
use crate::{
    api::build, camera::OrbitCamera, error::EngineError, frame::FrameInput, input::InputState,
    target::Target, transform::Transform,
};
use rok_abi::{EngineApi, NativeSurfaceHandle, engine_api::EngineHandle, input::ScanCode};
use rok_log::log_info;
use rok_math::{quaternion::Quat, vec3::Vec3};
use rok_mesh::{MeshData, MeshVertex, ObjLoader};
use rok_renderer::Renderer;
use rok_renderer::RendererConfig;
use rok_renderer::mesh_handle::MeshHandle;
use rok_renderer::{RenderCommand, cube};

const SUZANNE_OBJ: &[u8] = include_bytes!("../assets/suzanne.obj");

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
    scene: Scene,
    render_commands: Vec<RenderCommand>,
}

impl Engine {
    pub fn from_config(config: &EngineConfig) -> Result<Box<Self>, EngineError> {
        let renderer_config = RendererConfig {
            app_name: "rok".into(),
            frames_in_flight: unsafe { NonZeroU32::new_unchecked(2) },
            surface: config.surface,
            vsync: false, // TODO: load from config
        };

        let mut renderer = Renderer::new(&renderer_config).map_err(EngineError::Renderer)?;

        let obj_text = std::str::from_utf8(SUZANNE_OBJ)?;
        let mut loader = ObjLoader::default();
        let model = loader.parse_data(obj_text).expect("Parsing error");
        let meshes = model.to_mesh_data();
        let suzanne_handle = renderer.register_mesh(&meshes[0])?;

        let scene = Scene::test_scene(suzanne_handle);

        let mut engine = Box::new(Engine {
            renderer,
            api: None,
            target: None,
            input_state: InputState::new(),
            should_quit: AtomicBool::new(false),
            camera: OrbitCamera::new(),
            scene,
            render_commands: Vec::new(),
        });

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
        self.render_commands.clear();
        for instance in &self.scene.instances {
            self.render_commands.push(RenderCommand::DrawMesh {
                model: instance.transform.to_matrix(),
                mesh: instance.mesh,
            });
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
}

// fn cube_mesh_data() -> MeshData {
//     let (vertex_data, index_data) = geometry::cube();

//     let vertices: Vec<MeshVertex> = vertex_data
//         .into_iter()
//         .map(|mv| MeshVertex {
//             position: mv.position,
//             uv: mv.uv,
//             normal: mv.normal,
//             tangent: mv.tangent,
//         })
//         .collect();

//     let indices: Vec<u32> = index_data.into_iter().map(|x| x as u32).collect();

//     MeshData {
//         vertices,
//         indices,
//         material_name: String::from("default"),
//         index_type: rok_mesh::IndexType::U16,
//     }
// }
