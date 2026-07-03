// engine.rs

// rok engine

use std::{num::NonZeroU32, sync::atomic::AtomicBool};

use crate::{api::build, error::EngineError, frame::FrameInput, target::Target};
use rok_abi::{EngineApi, NativeSurfaceHandle, engine_api::EngineHandle};
use rok_renderer::{Renderer, RendererConfig};

pub struct EngineConfig {
    pub target_path: String,
    pub surface: Option<NativeSurfaceHandle>,
}

pub struct Engine {
    api: Option<EngineApi>,
    target: Option<Target>,
    renderer: Renderer,
    should_quit: AtomicBool,
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

        let mut engine = Box::new(Engine {
            renderer,
            api: None,
            target: None,
            should_quit: AtomicBool::new(false),
        });

        let handle = (&mut *engine as *mut Engine).cast::<EngineHandle>();

        engine.api = Some(build(handle));

        let target = Target::load(&config.target_path, engine.api.as_ref().unwrap(), None)?;

        engine.target = Some(target);

        Ok(engine)
    }

    pub fn update(&mut self, frame_input: &FrameInput) {
        // Handle resize updates.
        if frame_input.lifecycle.surface_changed {
            self.renderer.on_resize(
                frame_input.lifecycle.surface_width,
                frame_input.lifecycle.surface_height,
            );
            if let Some(target) = self.target.as_mut() {
                target.on_resize(
                    frame_input.lifecycle.surface_width,
                    frame_input.lifecycle.surface_height,
                );
            }
        }

        // Run the targets update.
        if let Some(target) = self.target.as_mut() {
            target.update(frame_input.delta_time);
        }
    }

    pub fn render(&mut self) {
        self.renderer.render();

        if let Some(target) = self.target.as_mut() {
            target.render();
        }
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit.load(std::sync::atomic::Ordering::Relaxed)
    }
}
