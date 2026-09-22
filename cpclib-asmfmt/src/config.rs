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
    let content = fs_err::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    toml::from_str(&content).map_err(|e| format!("invalid config in {}: {e}", path.display()))
}

pub fn load_config() -> AsmFormatOptions {
    find_config_file()
        .and_then(|p| load_config_from(&p).ok())
        .unwrap_or_default()
}

// A top-level `ignore = ["path/glob", ...]` array in `basm-fmt.toml`, read
// alongside (not as part of) `AsmFormatOptions` - the same convention
// rustfmt.toml itself uses. Patterns are relative to the config file's own
// directory, matching rustfmt's own behavior, and are matched against each
// input path given on the command line (this formatter has no directory-
// walking mode of its own to filter). Silently empty when the file has no
// such key, isn't valid TOML, or can't be read - `load_config_from` already
// surfaces a warning for a genuinely broken config file, so this doesn't
// duplicate that.
pub fn load_ignore_patterns_from(path: &Path) -> Vec<String> {
    match fs_err::read_to_string(path) {
        Ok(content) => parse_ignore_patterns(&content),
        Err(_) => Vec::new()
    }
}

// The pure, filesystem-free half of `load_ignore_patterns_from` - split out
// so it can be unit tested directly against a string.
fn parse_ignore_patterns(content: &str) -> Vec<String> {
    // `toml::Value`'s `FromStr` parses a single bare value expression, not a
    // whole document - `toml::from_str` (serde-based) is what actually
    // parses `key = value` document syntax into a `Value`.
    let Ok(value) = toml::from_str::<toml::Value>(content)
    else {
        return Vec::new();
    };
    value
        .get("ignore")
        .and_then(toml::Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_ignore_list_out_of_a_config_document() {
        let content = "ignore = [\"sub/*.asm\", \"vendor/**\"]\nindent_size = 2\n";
        assert_eq!(parse_ignore_patterns(content), vec!["sub/*.asm", "vendor/**"]);
        // The same document's real options must still parse fine through the
        // normal path - the `ignore` key is simply unknown to
        // `AsmFormatOptions` and dropped by serde, not a hard error.
        assert_eq!(toml::from_str::<AsmFormatOptions>(content).unwrap().indent_size, 2);
    }

    #[test]
    fn returns_empty_when_there_is_no_ignore_key() {
        assert!(parse_ignore_patterns("indent_size = 4\n").is_empty());
    }

    #[test]
    fn returns_empty_for_unparseable_content_rather_than_panicking() {
        assert!(parse_ignore_patterns("not valid toml [[[").is_empty());
    }
}
