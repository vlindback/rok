// config.rs

use std::{collections::HashMap, fs};

use crate::host_error::HostError;

pub(crate) struct HostConfig {
    pub engine: String,
    pub target: String,
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
    // we are going to have to handle this differently.

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
        let path = format!("config/targets/{}/{}.cfg", config_name, config_name);
        let contents = fs::read_to_string(&path)?;

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

        let engine_path = map
            .get("engine")
            .ok_or(HostError::ConfigMissingKey("engine"))?;

        let target_path = map
            .get("target")
            .ok_or(HostError::ConfigMissingKey("target"))?;

        Ok(Self {
            engine: make_dll_path(engine_path),
            target: make_dll_path(target_path),
        })
    }
}
