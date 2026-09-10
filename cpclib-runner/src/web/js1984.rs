//! Installing 1984js: the emscripten build of the 1984 emulator, plain and
//! unmodified.
//!
//! This crate serves it as-is - nothing here knows how to reach the debug
//! engine it contains, that is a separate concern living wherever an actual
//! debugger integration is built. [`download_dist`] is exposed precisely so
//! such an integration can fetch the same pinned files into a cache folder
//! of its own and patch them there, without this module needing to know
//! anything about what that patch does.
//!
//! Pinned to a commit rather than tracking `main`, so "which upstream" is
//! part of the contract: moving the pin is a deliberate action.

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};

/// The upstream revision this crate is pinned to.
pub const PINNED_COMMIT: &str = "3c3044ba239ea81b87c4fd0b86264622543e45e0";

/// The files that make up the distribution.
///
/// Listed rather than discovered because a directory listing is not available
/// over `raw.githubusercontent.com`, and because an unexpected new file
/// upstream should be a deliberate decision, not something silently pulled in.
pub const DIST_FILES: &[&str] = &[
    "index.html",
    "app.js",
    "dap.js",
    "6128.js",
    "6128.wasm",
    "gamepad.js",
    "media-url.js",
    "ml-monitor.js",
    "styles.css",
    "theme-cpc464.css",
    "theme-retro-crt.css",
    "theme-sapporo.css",
    "theme-sapporo-dark.css",
    "brand-1984.png"
];

/// Where the plain distribution lives.
pub fn cache_folder() -> Utf8PathBuf {
    let short = &PINNED_COMMIT[..7];
    crate::delegated::base_cache_folder().join(format!("1984js_{short}"))
}

/// Whether it is already installed.
pub fn is_installed() -> bool {
    cache_folder().join("index.html").exists()
}

/// The directory to serve.
pub fn web_root() -> Utf8PathBuf {
    cache_folder()
}

fn file_url(name: &str) -> String {
    format!("https://raw.githubusercontent.com/salvogendut/1984/{PINNED_COMMIT}/web/dist/{name}")
}

/// Download every file in [`DIST_FILES`] into `root`, which must already
/// exist. Exposed so a caller wanting a *patched* distribution (in a cache
/// folder of its own) can fetch the same pinned files without duplicating
/// this list or the download logic.
pub fn download_dist(root: &Utf8Path) -> Result<(), String> {
    for name in DIST_FILES {
        let url = file_url(name);
        let mut reader = cpclib_common::network::download(&url)
            .map_err(|e| format!("cannot download {url}: {e}"))?;
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut bytes)
            .map_err(|e| format!("cannot read {url}: {e}"))?;
        fs_err::write(root.join(name), &bytes).map_err(|e| format!("cannot write {name}: {e}"))?;
    }
    Ok(())
}

/// Download the pinned distribution, plain and unmodified.
///
/// A no-op when it is already there, so callers can call it unconditionally.
pub fn install() -> Result<Utf8PathBuf, String> {
    let root = cache_folder();
    if is_installed() {
        return Ok(root);
    }

    fs_err::create_dir_all(&root).map_err(|e| format!("cannot create {root}: {e}"))?;
    download_dist(&root)?;

    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cache folder carries the upstream pin, so bumping it forces a
    /// reinstall.
    #[test]
    fn the_cache_folder_names_the_pinned_commit() {
        let folder = cache_folder();
        let name = folder.file_name().unwrap();
        assert!(name.starts_with("1984js_3c3044b"), "{name}");
    }

    /// Every file the emulator needs is listed - a missing one produces a page
    /// that half-loads.
    #[test]
    fn the_distribution_lists_the_essentials() {
        for essential in ["index.html", "app.js", "dap.js", "6128.js", "6128.wasm"] {
            assert!(DIST_FILES.contains(&essential), "{essential} is missing");
        }
    }
}
