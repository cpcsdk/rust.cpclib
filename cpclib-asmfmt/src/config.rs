use std::path::{Path, PathBuf};

use crate::options::AsmFormatOptions;

pub const CONFIG_FILE_NAME: &str = "basm-fmt.toml";

pub fn find_config_file() -> Option<PathBuf> {
    if let Ok(mut dir) = std::env::current_dir() {
        loop {
            let path = dir.join(CONFIG_FILE_NAME);
            if path.is_file() {
                return Some(path);
            }
            if !dir.pop() {
                break;
            }
        }
    }
    let config_base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
        .or_else(|_| std::env::var("APPDATA").map(PathBuf::from))
        .ok()?;
    let path = config_base.join("basm-fmt").join(CONFIG_FILE_NAME);
    if path.is_file() { Some(path) } else { None }
}

pub fn load_config_from(path: &Path) -> Result<AsmFormatOptions, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    toml::from_str(&content).map_err(|e| format!("invalid config in {}: {e}", path.display()))
}

pub fn load_config() -> AsmFormatOptions {
    find_config_file()
        .and_then(|p| load_config_from(&p).ok())
        .unwrap_or_default()
}
