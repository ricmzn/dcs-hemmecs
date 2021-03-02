use anyhow::{anyhow, Result};
use std::fs::File;
use std::{
    io::Read,
    path::{Path, PathBuf},
};

const HEMMECS_EXPORT_SNIPPET: &str = include_str!("../lua/Scripts/Export.lua.snippet");
const HEMMECS_EXPORT_SCRIPT: &str = include_str!("../lua/Scripts/HemmecsExport.lua");
const HEMMECS_EXPORT_SCRIPT_PATH: &str = "Scripts/HemmecsExport.lua";
const DCS_EXPORT_LUA_PATH: &str = "Scripts/Export.lua";
const HEMMECS_VARS: &[&str] = &["hemmecsStatus", "hemmecsErr"];

/// Gets the user's Saved Games folder path through native Windows APIs
pub fn saved_games() -> Result<PathBuf> {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt, ptr::null_mut, slice};
    use winapi::{
        shared::{ntdef::PWSTR, winerror::S_OK},
        um::{
            combaseapi::CoTaskMemFree,
            knownfolders::FOLDERID_SavedGames,
            shlobj::{SHGetKnownFolderPath, KF_FLAG_DEFAULT},
        },
    };
    unsafe {
        let mut saved_games_str_ptr: PWSTR = null_mut();
        let hresult = SHGetKnownFolderPath(
            &FOLDERID_SavedGames as *const _,
            KF_FLAG_DEFAULT,
            null_mut(),
            &mut saved_games_str_ptr as *mut _,
        );
        if hresult != S_OK {
            return Err(anyhow!(
                "Unspecified error while looking for Saved Games folder"
            ));
        }
        let saved_games_str_len = {
            let mut ptr = saved_games_str_ptr;
            let mut len = 0usize;
            while *ptr != 0 {
                ptr = ptr.offset(1);
                len += 1;
            }
            len
        };
        let saved_games_slice = slice::from_raw_parts(saved_games_str_ptr, saved_games_str_len);
        let saved_games_path_str = OsString::from_wide(saved_games_slice);
        CoTaskMemFree(saved_games_str_ptr as *mut _);
        Ok(PathBuf::from(saved_games_path_str))
    }
}

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
        Ok(saved_games()?.join(folder_name))
    }

    pub fn install_status(&self) -> Result<InstallStatus> {
        let user_folder = self.user_folder()?;
        match Path::is_dir(&user_folder) {
            false => Ok(InstallStatus::DCSNotFound),
            true => {
                let mut export_script_path = user_folder.clone();
                export_script_path.push(HEMMECS_EXPORT_SCRIPT_PATH);
                if !Path::exists(&export_script_path) {
                    Ok(InstallStatus::NotInstalled)
                } else {
                    if File::open(&export_script_path)?
                        .bytes()
                        .map(Result::unwrap)
                        .eq(HEMMECS_EXPORT_SCRIPT.bytes())
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
