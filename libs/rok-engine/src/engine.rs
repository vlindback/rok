// engine.rs

// rok engine

use std::{num::NonZeroU32, sync::atomic::AtomicBool};

use crate::{error::EngineError, frame::FrameInput, target::Target};
use rok_abi::NativeSurfaceHandle;
use rok_renderer::{Renderer, RendererConfig};

pub struct EngineConfig {
    pub target_path: String,
    pub surface: Option<NativeSurfaceHandle>,
}

pub struct Engine {
    target: Target,
    renderer: Renderer,
    should_quit: AtomicBool,
}

impl Engine {
    pub fn from_config(config: &EngineConfig) -> Result<Self, EngineError> {
        let target = Target::from_filepath(&config.target_path)?;

        let renderer_config = RendererConfig {
            app_name: "rok".into(),
            frames_in_flight: unsafe { NonZeroU32::new_unchecked(2) },
            surface: config.surface,
        };

        let renderer = Renderer::new(&renderer_config).map_err(EngineError::Renderer)?;

        Ok(Self {
            target,
            renderer,
            should_quit: AtomicBool::new(false),
        })
    }

    pub fn update(&self, frame_input: &FrameInput) {}

    pub fn render(&self) {}

    pub fn should_quit(&self) -> bool {
        self.should_quit.load(std::sync::atomic::Ordering::Relaxed)
    }
}
