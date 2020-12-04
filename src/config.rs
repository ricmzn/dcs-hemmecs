use anyhow::{Context, Result};
use notify::Watcher;
use serde::{Deserialize, Serialize};
use std::fs::{rename, File};
use std::io::{ErrorKind, Read, Write};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use crate::consts::CONFIG_FILE;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OcclusionConfig {
    pub enable: bool,
    pub allow_hud_overlap: bool,
    pub hud_overlap_horizontal_angle: f32,
    pub hud_overlap_vertical_angle: f32,
}

impl Default for OcclusionConfig {
    fn default() -> Self {
        OcclusionConfig {
            enable: true,
            allow_hud_overlap: false,
            hud_overlap_horizontal_angle: 10.0,
            hud_overlap_vertical_angle: 5.0,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub occlusion: OcclusionConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            occlusion: OcclusionConfig::default(),
        }
    }
}

fn watch(path: &str) -> Result<(impl notify::Watcher, Receiver<notify::DebouncedEvent>)> {
    let (sender, receiver) = channel();
    let mut watcher = notify::watcher(sender, Duration::from_millis(100))?;
    watcher.watch(path, notify::RecursiveMode::NonRecursive)?;
    Ok((watcher, receiver))
}

/// Reads config file if it exists, or returns an error if it cannot be opened
pub fn read_existing_config() -> Result<Config> {
    let mut file = File::open(CONFIG_FILE).context("failed to open config file")?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .context("failed to read from config file")?;
    Ok(toml::from_slice(&buf)?)
}

/// If successful, returns the application config, a modification notification channel, and a boolean indicating if the config was newly created on this call
pub fn load_or_create_config() -> Result<(
    Config,
    impl notify::Watcher,
    Receiver<notify::DebouncedEvent>,
    bool,
)> {
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

            let (watcher, notifier) = watch(CONFIG_FILE)?;
            Ok((config, watcher, notifier, false))
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

                let (watcher, notifier) = watch(CONFIG_FILE)?;
                Ok((default_config, watcher, notifier, true))
            } else {
                Err(error).context("failed to open config file")?
            }
        }
    }
}
