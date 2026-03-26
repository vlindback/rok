// host.rs

use rok_engine::{
    engine::{Engine, EngineConfig},
    frame::{FrameInput, LifecycleFlags},
};

use rok_window::EventLoop;

use crate::host_error::HostError;

pub(crate) struct Host {
    event_loop: EventLoop,
    engine: Engine,
}

impl Host {
    pub(crate) fn new(target_path: String) -> Result<Box<Self>, HostError> {
        let mut event_loop = EventLoop::new();
        let window = event_loop.create_window("rok", 1280, 720)?;
        let surface = window.surface_handle();

        let engine_config = EngineConfig {
            target_path,
            surface: Some(surface),
        };

        let engine = Engine::from_config(&engine_config)?;

        Ok(Box::new(Host { event_loop, engine }))
    }
    pub(crate) fn main_loop(&mut self) {
        let mut events = Vec::with_capacity(256);
        let mut last_frame = std::time::Instant::now();
        let start = std::time::Instant::now();

        loop {
            events.clear();
            let pump = self.event_loop.pump(&mut events);

            let should_quit = pump.should_quit || self.engine.should_quit();

            let now = std::time::Instant::now();
            let dt = now.duration_since(last_frame).as_secs_f32().min(0.1);
            last_frame = now;

            let frame_input = FrameInput {
                delta_time: dt,
                timestamp_ns: now.duration_since(start).as_nanos() as u64,
                lifecycle: LifecycleFlags {
                    should_quit,
                    surface_changed: pump.surface_changed,
                    surface_width: pump.new_width,
                    surface_height: pump.new_height,
                    surface_valid: true,
                },
            };

            self.engine.update(&frame_input);
            self.engine.render();

            if should_quit {
                break;
            }
        }
    }
}
