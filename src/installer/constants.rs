use once_cell::sync::Lazy;
use regex::Regex;

pub static HEMMECS_EXPORT_SNIPPET: &str = include_str!("../../lua/Scripts/Export.lua.snippet");
pub static HEMMECS_EXPORT_SCRIPT: &str = include_str!("../../lua/Scripts/HemmecsExport.lua");
pub static HEMMECS_EXPORT_SCRIPT_PATH: &str = "Scripts/HemmecsExport.lua";
pub static DCS_EXPORT_LUA_PATH: &str = "Scripts/Export.lua";

/// Any lines containing both `hemmecsStatus` and `hemmecsErr`, in any order
pub static HEMMECS_VARS_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r".*hemmecsStatus.*hemmecsErr.*\n|.*:hemmecsErr.*hemmecsStatus.*\n").unwrap()
});
