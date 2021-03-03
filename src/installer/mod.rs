mod constants;
mod utils;

use anyhow::Result;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

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
        let user_folder = self.user_folder()?;
        match Path::is_dir(&user_folder) {
            false => Ok(InstallStatus::DCSNotFound),
            true => {
                let mut export_script_path = user_folder.clone();
                export_script_path.push(constants::HEMMECS_EXPORT_SCRIPT_PATH);
                if !Path::exists(&export_script_path) {
                    Ok(InstallStatus::NotInstalled)
                } else {
                    if File::open(&export_script_path)?
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
    unimplemented!()
}

pub fn uninstall(dcs_version: &DCSVersion) -> Result<()> {
    unimplemented!()
}
