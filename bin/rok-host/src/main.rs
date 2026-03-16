// rok-host/src/main.rs
//
// The Host is the thin, stable exe that owns:
//   - The OS window and native event loop
//   - The input event queue
//   - DLL lifetimes (engine + target)
//
// It knows almost nothing about rendering or game logic — those live in the DLLs.
// Its job is to pump the OS, collect raw input, and drive the engine tick.

mod engine;
mod host;
mod host_api;
mod host_config;
mod host_error;
mod host_state;

use rok_abi::LogLevel;

use rok_log::{StderrSink, log_fatal};

use crate::{host::Host, host_config::HostConfig};

// ---------------------------------------------------------------------------
// Main Setup Methods
// ---------------------------------------------------------------------------

fn init_logging() {
    std::panic::set_hook(Box::new(|info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "<unknown>".into());
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "<non-string panic>"
        };

        log_fatal!("panic at {}: {}", location, message)
    }));

    let initial_sinks: Vec<Box<dyn rok_log::Sink>> = vec![
        #[cfg(debug_assertions)]
        Box::new(StderrSink::new(LogLevel::Trace)),
        #[cfg(not(debug_assertions))]
        Box::new(StderrSink::new(LogLevel::Warning)),
    ];

    rok_log::init(initial_sinks);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up the logging first.
    init_logging();

    let result = run();

    result
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config_name = std::env::args().nth(1).ok_or("Usage: rok-host <config>")?;

    let config = HostConfig::load(&config_name).map_err(|e| {
        log_fatal!("Failed to load config '{}': {}", config_name, e);
        e
    })?;

    let mut host = Host::new(&config.engine, &config.target).map_err(|e| {
        log_fatal!(
            "Failed to start host: {} (engine: {}, target: {})",
            e,
            config.engine,
            config.target
        );
        e
    })?;

    host.main_loop();

    Ok(())
}
