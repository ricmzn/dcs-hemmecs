use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{rename, File};
use std::io::{ErrorKind, Read, Write};
use std::sync::{Arc, Mutex};

use crate::consts::CONFIG_FILE;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
#[doc = "docs"]
pub struct Occlusion {
    pub hide_on_hud: bool,
    pub hud_horizontal_angle: f32,
    pub hud_vertical_angle: f32,
    pub hide_in_cockpit: bool,
}

impl Default for Occlusion {
    fn default() -> Self {
        Occlusion {
            hide_on_hud: true,
            hud_horizontal_angle: 10.0,
            hud_vertical_angle: 5.0,
            hide_in_cockpit: true,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub color: (u8, u8, u8),
    pub brightness: u8,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            color: (0, 255, 0),
            brightness: 128,
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub occlusion: Occlusion,
    pub appearance: Appearance,

    #[serde(skip)]
    pub show_sample_data: bool,
}

pub type ConfigHandle = Arc<Mutex<Config>>;

pub fn load_or_create_config() -> Result<Config> {
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
            let buf = toml::to_vec(&config)?;
            let tmp_filename = format!("{}.tmp", CONFIG_FILE);
            let mut tmp_file = File::create(&tmp_filename)?;
            tmp_file
                .write_all(&buf)
                .context("failed to write new config entries to file")?;

            drop(tmp_file);
            rename(&tmp_filename, CONFIG_FILE)
                .context("failed to overwrite config file with new values")?;

            Ok(config)
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

                Ok(default_config)
            } else {
                Err(error).context("failed to open config file")
            }
        }
    }
}

pub fn save_config(config: &Config) -> Result<()> {
    let buf = toml::to_vec(&config)?;
    let tmp_filename = format!("{}.tmp", CONFIG_FILE);
    let mut tmp_file = File::create(&tmp_filename)?;

    tmp_file
        .write_all(&buf)
        .context("failed to write new config entries to file")?;

    drop(tmp_file);
    rename(&tmp_filename, CONFIG_FILE)
        .context("failed to overwrite config file with new values")?;

    Ok(())
}
