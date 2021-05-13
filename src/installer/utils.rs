use std::{
    convert::TryInto,
    ffi::OsString,
    fs::{File, OpenOptions},
    io::{self, Error, ErrorKind, Read, Seek, SeekFrom, Write},
    ops::Range,
    os::windows::ffi::OsStringExt,
    path::{Path, PathBuf},
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

use anyhow::{Context, Result};

use super::constants;

/// Gets the user's Saved Games folder path through native Windows APIs
pub fn saved_games() -> io::Result<PathBuf> {
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

/// Fuzzy-finds all lines which match the export.lua snippet variables
pub fn export_snippet_ranges(text: &str) -> Result<Vec<Range<usize>>> {
    let ranges = constants::HEMMECS_VARS_REGEX
        .captures_iter(&text)
        .filter_map(|captures| captures.get(0).map(|capture| capture.range()))
        .collect();
    Ok(ranges)
}

/// Returns a copy of a string with all specified byte ranges removed
pub fn remove_ranges(string: &str, ranges: &[Range<usize>]) -> Result<String> {
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

/// Rewrite the Export.lua snippet, optionally re-installing the Hemmecs snippet
pub fn rewrite_export_lua_snippet(file: &mut File, reinstall: bool) -> Result<()> {
    // Read the original contents
    let mut export_lua_text = String::new();
    file.seek(SeekFrom::Start(0))?;
    file.read_to_string(&mut export_lua_text)?;

    // Find and remove the snippet
    let snippet_ranges = export_snippet_ranges(&export_lua_text)?;
    export_lua_text = remove_ranges(&export_lua_text, &snippet_ranges)?;

    // Be nice and trim the end of the file
    export_lua_text = String::from(export_lua_text.trim_end());

    // Install
    if reinstall {
        export_lua_text.push_str("\n\n");
        export_lua_text.push_str(constants::HEMMECS_EXPORT_SNIPPET);
    }

    // Also be nice and put a line break at the end
    export_lua_text.push('\n');

    // Write the changes
    file.seek(SeekFrom::Start(0))?;
    write!(file, "{}", export_lua_text)?;
    file.set_len(
        export_lua_text
            .len()
            .try_into()
            .context("new Export.lua contents are too long")?,
    )?;

    Ok(())
}

/// Open a file for reading and writing, creating it if it does not exist
pub fn open_rw(path: &Path) -> Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .context(format!("cannot open {:?}", path))
}
