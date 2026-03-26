// engine.rs

// rok engine

use std::sync::atomic::AtomicBool;

use rok_abi::NativeSurfaceHandle;

use crate::{error::EngineError, frame::FrameInput, target::Target};

pub struct EngineConfig {
    pub target_path: String,
    pub surface: Option<NativeSurfaceHandle>,
}

pub struct Engine {
    target: Target,
    should_quit: AtomicBool,
}

impl Engine {
    pub fn from_config(config: &EngineConfig) -> Result<Self, EngineError> {
        let target = Target::from_filepath(&config.target_path)?;
        Ok(Self {
            target,
            should_quit: AtomicBool::new(false),
        })
    }

    pub fn update(&self, frame_input: &FrameInput) {}

    pub fn render(&self) {}

    pub fn should_quit(&self) -> bool {
        self.should_quit.load(std::sync::atomic::Ordering::Relaxed)
    }
}
