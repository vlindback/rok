// config.rs

use std::{collections::HashMap, fs};

use crate::host_error::HostError;

pub(crate) struct HostConfig {
    pub engine: String,
    pub target: String,
}

impl HostConfig {
    pub(crate) fn load(config_name: &str) -> Result<Self, HostError> {
        let path = format!("config/targets/{}/{}.cfg", config_name, config_name);
        let contents = fs::read_to_string(&path)?;

        let map: HashMap<&str, &str> = contents
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| {
                let mut parts = line.splitn(2, '=');
                Some((parts.next()?.trim(), parts.next()?.trim()))
            })
            .collect();

        Ok(Self {
            engine: map
                .get("engine")
                .ok_or(HostError::ConfigMissingKey("engine"))?
                .to_string(),
            target: map
                .get("target")
                .ok_or(HostError::ConfigMissingKey("target"))?
                .to_string(),
        })
    }
}
