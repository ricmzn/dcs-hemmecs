use std::{
    ffi::OsString,
    io::{Error, ErrorKind, Result},
    os::windows::ffi::OsStringExt,
    path::PathBuf,
    ptr::null_mut,
    slice,
};
use winapi::{
    shared::{ntdef::PWSTR, winerror::S_OK},
    um::{
        combaseapi::CoTaskMemFree,
        knownfolders::FOLDERID_SavedGames,
        shlobj::{SHGetKnownFolderPath, KF_FLAG_DEFAULT},
    },
};

/// Gets the user's Saved Games folder path through native Windows APIs
pub fn saved_games() -> Result<PathBuf> {
    unsafe {
        let mut saved_games_str_ptr: PWSTR = null_mut();
        let hresult = SHGetKnownFolderPath(
            &FOLDERID_SavedGames as *const _,
            KF_FLAG_DEFAULT,
            null_mut(),
            &mut saved_games_str_ptr as *mut _,
        );
        if hresult != S_OK {
            return Err(Error::new(
                ErrorKind::Other,
                "Unspecified error while looking for Saved Games folder",
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
