use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{ErrorKind, Read, Write};

use crate::consts::CONFIG_FILE;

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub enable_hud_occlusion: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            enable_hud_occlusion: true,
        }
    }
}

/// Returns a result of the config (loaded or created) and a boolean indicating if the config was newly created with this call
pub fn load_or_create_config() -> Result<(Config, bool)> {
    let mut file = match File::open(CONFIG_FILE) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            let default_config = Config::default();
            let mut file = File::create(CONFIG_FILE)?;
            file.write_all(toml::to_string(&default_config)?.as_bytes())?;
            return Ok((default_config, true));
        }
        error => error?,
    };
    let mut string = String::new();
    file.read_to_string(&mut string)?;
    Ok((toml::from_str(&string)?, false))
}
