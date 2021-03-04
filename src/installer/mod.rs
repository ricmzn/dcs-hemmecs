mod constants;
mod utils;

use std::{
    fs::{create_dir_all, remove_file, File},
    io::{Read, Write},
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

pub fn install(dcs_version: &DCSVersion) -> Result<()> {
    // Ensure the scripts folder exists
    create_dir_all(dcs_version.user_folder()?.join("Scripts"))
        .context("Could not ensure DCS Scripts folder exists")?;

    // Overwrite the Hemmecs export script
    let mut export_script = utils::open_rw(
        &dcs_version
            .user_folder()?
            .join(constants::HEMMECS_EXPORT_SCRIPT_PATH),
    )?;
    write!(export_script, "{}", constants::HEMMECS_EXPORT_SCRIPT)?;

    // Rewrite the Export.lua snippet
    let mut export_lua = utils::open_rw(
        &dcs_version
            .user_folder()?
            .join(constants::DCS_EXPORT_LUA_PATH),
    )?;
    utils::rewrite_export_lua_snippet(&mut export_lua, true)?;

    Ok(())
}

pub fn uninstall(dcs_version: &DCSVersion) -> Result<()> {
    // Give up if there's no scripts folder
    if !Path::is_dir(&dcs_version.user_folder()?.join("Scripts")) {
        return Ok(());
    }

    // Remove the Export.lua snippet
    let mut export_lua = utils::open_rw(
        &dcs_version
            .user_folder()?
            .join(constants::DCS_EXPORT_LUA_PATH),
    )?;
    utils::rewrite_export_lua_snippet(&mut export_lua, false)?;

    // Delete the export script
    remove_file(
        &dcs_version
            .user_folder()?
            .join(constants::HEMMECS_EXPORT_SCRIPT_PATH),
    )?;

    Ok(())
}
