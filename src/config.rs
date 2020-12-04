use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{rename, File};
use std::io::{ErrorKind, Read, Write};

use crate::consts::CONFIG_FILE;

#[derive(Serialize, Deserialize)]
pub struct HudConfig {
    pub enable_occlusion: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub hud: HudConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            hud: HudConfig {
                enable_occlusion: true,
            },
        }
    }
}

/// If successful, returns the application config and a boolean indicating if the config was newly created on this call
pub fn load_or_create_config() -> Result<(Config, bool)> {
    // Try to open an existing config
    match File::open(CONFIG_FILE) {
        Ok(mut file) => {
            // Read the data from the config
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .context("failed to read from config file")?;

            drop(file);
            let config = toml::from_slice(&buf)?;

            // Try to write any newly created config entries back to the file
            let tmp_filename = format!("{}.tmp", CONFIG_FILE);
            let mut file = File::create(&tmp_filename)?;
            let mut buf = toml::to_vec(&config)?;
            file.write_all(&mut buf)
                .context("failed to write new config entries to file")?;

            drop(file);
            rename(&tmp_filename, CONFIG_FILE)
                .context("failed to overwrite config file with new values")?;

            Ok((config, false))
        }
        Err(error) => {
            if error.kind() == ErrorKind::NotFound {
                // Create a new config with default values
                let default_config = Config::default();
                let mut file =
                    File::create(CONFIG_FILE).context("failed to create default config file")?;

                file.write_all(
                    toml::to_string(&default_config)
                        .context("failed to write default config file")?
                        .as_bytes(),
                )?;

                Ok((default_config, true))
            } else {
                Err(error).context("failed to open config file")?
            }
        }
    }
}
