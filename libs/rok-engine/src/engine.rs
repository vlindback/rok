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
use rok_renderer::RenderCommand;
use rok_renderer::Renderer;
use rok_renderer::RendererConfig;

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

        let renderer = Renderer::new(&renderer_config).map_err(EngineError::Renderer)?;

        let mut scene = Scene::new();
        let n = 4; // 4x4x4 = 64 cubes
        let spacing = 2.0;
        let offset = (n as f32 - 1.0) * spacing * 0.5; // center the grid on the origin
        for x in 0..n {
            for y in 0..n {
                for z in 0..n {
                    scene.instances.push(Transform::from_position(Vec3::new(
                        x as f32 * spacing - offset,
                        y as f32 * spacing - offset,
                        z as f32 * spacing - offset,
                    )));
                }
            }
        }

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
                model: instance.to_matrix(),
            });
        }

        self.renderer
            .render(self.camera.view(), &self.render_commands);

        if let Some(target) = self.target.as_mut() {
            target.render();
        }
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit.load(std::sync::atomic::Ordering::Relaxed)
    }
}
