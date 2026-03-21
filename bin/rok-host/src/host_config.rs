// config.rs

use std::{collections::HashMap, fs};

use rok_log::log_warn;

use crate::host_error::HostError;

pub(crate) struct HostConfig {
    // Required
    pub engine: String,
    pub target: String,
    // I/O
    pub io_max_open_files: u32,
}

/// Construct the full DLL path from a stem name.
///
/// The profile (debug/release) is resolved at compile time from debug_assertions,
/// which matches how Cargo names its output directories.
///
///   Windows -> .build/{profile}/{stem}.dll
///   Linux   -> .build/{profile}/lib{stem}.so
///   macOS   -> .build/{profile}/lib{stem}.dylib
fn make_dll_path(stem: &str) -> String {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };

    // TODO: this is acceptable for now but when we later bake the game
    // we are going to have to handle this differently. The problem is .build

    #[cfg(target_os = "windows")]
    let (prefix, ext) = ("", "dll");
    #[cfg(target_os = "linux")]
    let (prefix, ext) = ("lib", "so");
    #[cfg(target_os = "macos")]
    let (prefix, ext) = ("lib", "dylib");
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    compile_error!("rok-host: unsupported platform — add DLL naming convention here");

    format!(".build/{}/{}{}.{}", profile, prefix, stem, ext)
}

impl HostConfig {
    pub(crate) fn load(config_name: &str) -> Result<Self, HostError> {
        // Load the config file contents.

        let path = format!("config/targets/{}/{}.cfg", config_name, config_name);
        let contents = fs::read_to_string(&path)?;

        // Bake it into a key, value string reference map.

        let map: HashMap<&str, &str> = contents
            .lines()
            .filter(|line| {
                let t = line.trim();
                !t.is_empty() && !t.starts_with('#')
            })
            .filter_map(|line| {
                let mut parts = line.splitn(2, '=');
                Some((parts.next()?.trim(), parts.next()?.trim()))
            })
            .collect();

        // Fetch required properties:

        // Where is the engine.dll
        let engine_path = map
            .get("engine")
            .ok_or(HostError::ConfigMissingKey("engine"))?;

        // Where is the target.dll
        let target_path = map
            .get("target")
            .ok_or(HostError::ConfigMissingKey("target"))?;

        // Fetch all other options and warn if they are missing.

        let io_max_open_files = HostConfig::get_u32_or_default(&map, "io_max_open_files", 128, 32);

        //

        Ok(Self {
            engine: make_dll_path(engine_path),
            target: make_dll_path(target_path),
            io_max_open_files,
        })
    }

    fn get_u32_or_default(map: &HashMap<&str, &str>, key: &str, default: u32, floor: u32) -> u32 {
        match map.get(key) {
            None => {
                log_warn!(
                    "host config: '{}' not set, using default ({})",
                    key,
                    default
                );
                default
            }
            Some(val) => match val.parse::<u32>() {
                Err(_) => {
                    log_warn!(
                        "host config: '{}' could not be parsed (got '{}'), using default ({})",
                        key,
                        val,
                        default
                    );
                    default
                }
                Ok(parsed) if parsed < floor => {
                    log_warn!(
                        "host config: '{}' is below minimum ({} < {}), using floor",
                        key,
                        parsed,
                        floor,
                    );
                    floor
                }
                Ok(parsed) => parsed,
            },
        }
    }
}
