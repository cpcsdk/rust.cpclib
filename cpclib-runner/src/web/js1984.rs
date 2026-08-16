//! Installing 1984js: the emscripten build of the 1984 emulator, patched so an
//! external debugger can reach the DAP engine it already contains.
//!
//! Pinned to a commit rather than tracking `main`. The patch below inserts at
//! two anchors in upstream's files, so "which upstream" is part of the
//! contract: moving the pin and checking the patch still applies are the same
//! action. Upgrading is editing [`PINNED_COMMIT`] and re-running the install.

use cpclib_common::camino::Utf8PathBuf;

use super::apply_bridge_patch;

/// The upstream revision this patch is written against.
pub const PINNED_COMMIT: &str = "3c3044ba239ea81b87c4fd0b86264622543e45e0";

/// Bumped whenever *our* bridge changes.
///
/// It is part of the cache folder name because the installed-or-not check in
/// `DelegateApplicationDescription` is folder existence alone: without this, a
/// changed bridge would silently never be reinstalled.
pub const PATCH_REVISION: u32 = 15;

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

/// Where the patched distribution lives.
pub fn cache_folder() -> Utf8PathBuf {
    let short = &PINNED_COMMIT[..7];
    crate::delegated::base_cache_folder().join(format!("1984js_{short}_p{PATCH_REVISION}"))
}

/// Whether it is already installed.
pub fn is_installed() -> bool {
    let root = cache_folder();
    root.join("index.html").exists() && root.join(super::BRIDGE_FILENAME).exists()
}

/// The directory to serve.
pub fn web_root() -> Utf8PathBuf {
    cache_folder()
}

fn file_url(name: &str) -> String {
    format!("https://raw.githubusercontent.com/salvogendut/1984/{PINNED_COMMIT}/web/dist/{name}")
}

/// Download the pinned distribution and apply the bridge patch.
///
/// A no-op when it is already there, so callers can call it unconditionally.
pub fn install() -> Result<Utf8PathBuf, String> {
    let root = cache_folder();
    if is_installed() {
        return Ok(root);
    }

    std::fs::create_dir_all(&root).map_err(|e| format!("cannot create {root}: {e}"))?;

    for name in DIST_FILES {
        let url = file_url(name);
        let mut reader = cpclib_common::network::download(&url)
            .map_err(|e| format!("cannot download {url}: {e}"))?;
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut bytes)
            .map_err(|e| format!("cannot read {url}: {e}"))?;
        std::fs::write(root.join(name), &bytes).map_err(|e| format!("cannot write {name}: {e}"))?;
    }

    apply_bridge_patch(&root).map_err(|e| {
        // Leave nothing half-patched behind: a partially installed emulator
        // loads and then quietly does nothing, which is far harder to diagnose
        // than a missing one.
        let _ = std::fs::remove_dir_all(&root);
        e.to_string()
    })?;

    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cache folder carries both the upstream pin and our patch revision,
    /// so bumping either forces a reinstall.
    #[test]
    fn the_cache_folder_names_both_revisions() {
        let folder = cache_folder();
        let name = folder.file_name().unwrap();
        assert!(name.starts_with("1984js_3c3044b"), "{name}");
        assert!(name.ends_with(&format!("_p{PATCH_REVISION}")), "{name}");
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
