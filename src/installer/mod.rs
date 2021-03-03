mod constants;
mod utils;

use std::{
    fs::{create_dir_all, File, OpenOptions},
    io::{Read, Write},
    ops::Range,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

pub enum DCSVersion {
    Stable,
    Openbeta,
}

pub enum InstallStatus {
    DCSNotFound,
    NotInstalled,
    RequiresUpdate,
    Installed,
}

impl DCSVersion {
    pub fn user_folder(&self) -> Result<PathBuf> {
        let folder_name = match self {
            DCSVersion::Stable => "DCS",
            DCSVersion::Openbeta => "DCS.openbeta",
        };
        Ok(utils::saved_games()?.join(folder_name))
    }

    pub fn install_status(&self) -> Result<InstallStatus> {
        match Path::is_dir(&self.user_folder()?) {
            false => Ok(InstallStatus::DCSNotFound),
            true => {
                let export_script_path = self
                    .user_folder()?
                    .join(constants::HEMMECS_EXPORT_SCRIPT_PATH);
                if !Path::exists(&export_script_path) {
                    Ok(InstallStatus::NotInstalled)
                } else {
                    if File::open(export_script_path)?
                        .bytes()
                        .map(Result::unwrap)
                        .eq(constants::HEMMECS_EXPORT_SCRIPT.bytes())
                    {
                        Ok(InstallStatus::Installed)
                    } else {
                        Ok(InstallStatus::RequiresUpdate)
                    }
                }
            }
        }
    }
}

/// Fuzzy-finds all lines which match the export.lua snippet variables
fn export_snippet_ranges(text: &str) -> Result<Vec<Range<usize>>> {
    let ranges = constants::HEMMECS_VARS_REGEX
        .captures_iter(&text)
        .filter_map(|captures| captures.get(0).map(|capture| capture.range()))
        .collect();
    Ok(ranges)
}

/// Returns a copy of a string with all specified byte ranges removed
fn remove_ranges(string: &str, ranges: &[Range<usize>]) -> Result<String> {
    let filtered_bytes = string
        .bytes()
        .into_iter()
        .enumerate()
        .filter(|(i, _)| {
            // This is pretty inefficient, looping over every range for every character in the lines to be removed...
            for range in ranges {
                if *i >= range.start && *i <= range.end {
                    return false;
                }
            }
            return true;
        })
        .map(|(_, ch)| ch);
    Ok(String::from_utf8(filtered_bytes.collect())?)
}

fn open_rw(path: &PathBuf) -> Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .context(format!("Cannot open {:?}", path))
}

pub fn install(dcs_version: &DCSVersion) -> Result<()> {
    // Ensure the Scripts folder exists
    create_dir_all(dcs_version.user_folder()?.join("Scripts"))
        .context("Could not ensure DCS Scripts folder exists")?;

    // Overwrite export script
    let mut export_script = open_rw(
        &dcs_version
            .user_folder()?
            .join(constants::HEMMECS_EXPORT_SCRIPT_PATH),
    )?;
    write!(export_script, "{}", constants::HEMMECS_EXPORT_SCRIPT)?;

    // Rewrite the Export.lua snippet
    let mut export_lua = open_rw(
        &dcs_version
            .user_folder()?
            .join(constants::DCS_EXPORT_LUA_PATH),
    )?;
    let mut export_lua_text = String::new();
    export_lua.read_to_string(&mut export_lua_text)?;
    let original_export_lua = export_lua_text.clone();
    let snippet_ranges = export_snippet_ranges(&export_lua_text)?;
    export_lua_text = remove_ranges(&export_lua_text, &snippet_ranges)?;
    export_lua_text = String::from(export_lua_text.trim_end());
    export_lua_text.push_str("\n\n");
    export_lua_text.push_str(constants::HEMMECS_EXPORT_SNIPPET);
    println!(
        "ORIGINAL FILE:\n===\n{}\n===\n\nMODIFIED FILE:\n===\n{}\n===",
        original_export_lua, export_lua_text,
    );
    Ok(())
}

pub fn uninstall(_: &DCSVersion) -> Result<()> {
    Ok(())
}
