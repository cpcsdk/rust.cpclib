//! Patching 1984js so Robot automation can drive it directly, instead of
//! falling back to window capture and host keystrokes the way it does for
//! every other emulator with no native API of its own.
//!
//! 1984js's emscripten module exports raw debug primitives
//! (`_poc_debug_mem_read`, `_poc_debug_mem_write_byte`, `_poc_debug_pause`,
//! `_poc_debug_continue`, `_poc_key`, ...) directly - independent of the
//! bundled `dap.js`/`JS1984DAP` engine, which is only ever how upstream's
//! *own* on-page monitor happens to be built. The bridge here calls those
//! primitives straight, and answers a small ad hoc JSON request/response
//! protocol of its own (`{"id", "cmd", ...}` -> `{"id", "ok", ...}`) - not
//! the Debug Adapter Protocol - so there is nothing here for `cpclib-dap`'s
//! own, separately patched and separately cached, bridge to share.

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};

/// What the patch adds to an upstream 1984js `web/dist`.
pub const BRIDGE_SCRIPT: &str = include_str!("../../assets/1984js-robot/robot-bridge.js");
pub const BRIDGE_FILENAME: &str = "cpclib-robot-bridge.js";

/// Bumped whenever the bridge changes.
pub const PATCH_REVISION: u32 = 1;

/// The one line appended right after the emscripten module is done
/// initialising, handing the bridge the module object it needs to call the
/// raw `_poc_debug_*`/`_poc_key` primitives - nothing here needs the DAP
/// session object `app.js` builds a few lines later, so unlike
/// `cpclib-dap`'s own patch this hooks *before* that point.
const APP_HOOK: &str = "\n// added by cpclib: hand the emscripten module to the robot bridge, if one is loaded\n\
                        if (typeof globalThis.__cpclib_robot_attach === 'function') {\n\
                        \x20 globalThis.__cpclib_robot_attach(m);\n\
                        }\n";

/// The `<script>` added to `index.html`, immediately before `</body>` so the
/// bridge loads only after every one of the emulator's own scripts has
/// already run.
const INDEX_HOOK: &str = "<script src=\"cpclib-robot-bridge.js\"></script>\n</body>";

#[derive(Debug)]
pub enum PatchError {
    Missing(Utf8PathBuf),
    Io(String),
    /// The anchor we insert at was not found exactly once, which means
    /// upstream changed under us. Naming the file is the whole point: a
    /// silently half-applied patch produces an emulator that loads and then
    /// does nothing, which is far harder to diagnose than a refused install.
    AnchorNotUnique {
        file: Utf8PathBuf,
        anchor: &'static str,
        found: usize
    }
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchError::Missing(p) => write!(f, "{p} is missing from the 1984js distribution"),
            PatchError::Io(e) => write!(f, "{e}"),
            PatchError::AnchorNotUnique {
                file,
                anchor,
                found
            } => {
                write!(
                    f,
                    "1984js has changed: expected exactly one `{anchor}` in {file}, found \
                     {found}. The pinned commit and the patch need updating together."
                )
            }
        }
    }
}

/// Add the bridge to an unpacked `web/dist`.
///
/// Idempotent: running it twice on the same directory is a no-op, so a
/// re-install after a partial failure does not double-apply.
fn apply_patch(root: &Utf8Path) -> Result<(), PatchError> {
    let index = root.join("index.html");
    let app = root.join("app.js");
    for required in [&index, &app] {
        if !required.exists() {
            return Err(PatchError::Missing(required.clone()));
        }
    }

    fs_err::write(root.join(BRIDGE_FILENAME), BRIDGE_SCRIPT)
        .map_err(|e| PatchError::Io(e.to_string()))?;

    let index_text = read(&index)?;
    if !index_text.contains(BRIDGE_FILENAME) {
        let found = index_text.matches("</body>").count();
        if found != 1 {
            return Err(PatchError::AnchorNotUnique {
                file: index.clone(),
                anchor: "</body>",
                found
            });
        }
        write(&index, &index_text.replace("</body>", INDEX_HOOK))?;
    }

    let app_text = read(&app)?;
    if !app_text.contains("__cpclib_robot_attach") {
        let anchor = "const framebuffer = m._poc_pixels();";
        let found = app_text.matches(anchor).count();
        if found != 1 {
            return Err(PatchError::AnchorNotUnique {
                file: app.clone(),
                anchor,
                found
            });
        }
        write(
            &app,
            &app_text.replace(anchor, &format!("{anchor}{APP_HOOK}"))
        )?;
    }

    Ok(())
}

fn read(path: &Utf8Path) -> Result<String, PatchError> {
    fs_err::read_to_string(path).map_err(|e| PatchError::Io(format!("{path}: {e}")))
}

fn write(path: &Utf8Path, text: &str) -> Result<(), PatchError> {
    fs_err::write(path, text).map_err(|e| PatchError::Io(format!("{path}: {e}")))
}

/// Where the robot-patched distribution lives - a name of its own, distinct
/// from both the plain install and `cpclib-dap`'s own DAP-bridge install, so
/// none of the three ever collide.
fn cache_folder() -> Utf8PathBuf {
    let short = &super::js1984::PINNED_COMMIT[..7];
    crate::delegated::base_cache_folder()
        .join(format!("1984js_{short}_robotbridge_p{PATCH_REVISION}"))
}

/// Whether the robot-patched distribution is already installed.
fn is_installed() -> bool {
    let root = cache_folder();
    root.join("index.html").exists() && root.join(BRIDGE_FILENAME).exists()
}

/// Download the pinned distribution and apply the robot bridge patch.
///
/// A no-op when it is already there, so callers can call it unconditionally.
pub fn install() -> Result<Utf8PathBuf, String> {
    let root = cache_folder();
    if is_installed() {
        return Ok(root);
    }

    fs_err::create_dir_all(&root).map_err(|e| format!("cannot create {root}: {e}"))?;
    super::js1984::download_dist(&root)?;

    apply_patch(&root).map_err(|e| {
        // Leave nothing half-patched behind: a partially installed emulator
        // loads and then quietly does nothing, which is far harder to
        // diagnose than a missing one.
        let _ = fs_err::remove_dir_all(&root);
        e.to_string()
    })?;

    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_dist() -> camino_tempfile::Utf8TempDir {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("index.html"),
            "<html><body><script src=\"app.js\"></script></body></html>"
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("app.js"),
            "if (m._poc_init() !== 0) { return; }\n\
             const framebuffer = m._poc_pixels();\n\
             const modelEl = 1;\n"
        )
        .unwrap();
        tmp
    }

    #[test]
    fn the_patch_adds_the_bridge_and_its_hook() {
        let tmp = fake_dist();
        apply_patch(tmp.path()).expect("applies");

        assert!(tmp.path().join(BRIDGE_FILENAME).exists());
        let index = std::fs::read_to_string(tmp.path().join("index.html")).unwrap();
        assert!(index.contains(BRIDGE_FILENAME));
        assert!(
            index.find(BRIDGE_FILENAME) < index.find("</body>"),
            "the bridge loads before the body closes"
        );
        let app = std::fs::read_to_string(tmp.path().join("app.js")).unwrap();
        assert!(app.contains("__cpclib_robot_attach"));
    }

    #[test]
    fn the_patch_is_idempotent() {
        let tmp = fake_dist();
        apply_patch(tmp.path()).unwrap();
        let once = std::fs::read_to_string(tmp.path().join("app.js")).unwrap();
        apply_patch(tmp.path()).unwrap();
        let twice = std::fs::read_to_string(tmp.path().join("app.js")).unwrap();
        assert_eq!(once, twice);
    }

    #[test]
    fn a_missing_anchor_is_refused_by_name() {
        let tmp = fake_dist();
        std::fs::write(tmp.path().join("app.js"), "function boot() {}\n").unwrap();
        let error = apply_patch(tmp.path()).unwrap_err();
        let text = error.to_string();
        assert!(text.contains("app.js"), "{text}");
        assert!(text.contains("1984js has changed"), "{text}");
    }

    #[test]
    fn a_missing_file_is_refused_by_name() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let error = apply_patch(tmp.path()).unwrap_err();
        assert!(error.to_string().contains("index.html"), "{error}");
    }

    /// The bridge has a few pieces it cannot work without, and losing one is
    /// silent.
    #[test]
    fn the_bridge_keeps_the_pieces_it_cannot_work_without() {
        for (needle, why) in [
            ("__cpclib_robot_attach", "the emulator hands us its module through this"),
            ("__cpclib_session", "the token is injected into the page, not the URL"),
            ("_poc_debug_mem_read", "memory reads go through this"),
            ("_poc_debug_mem_write_byte", "memory writes go through this"),
            ("_poc_debug_pause", "a consistent read/write needs the CPU stopped first"),
            ("_poc_debug_continue", "and running again afterwards"),
            ("_poc_key", "autotype presses real keys"),
            ("toDataURL", "screenshots are read straight off the canvas"),
            ("/session/events", "the downstream half of the channel"),
            ("/session/upstream", "the upstream half")
        ] {
            assert!(
                BRIDGE_SCRIPT.contains(needle),
                "the bridge lost `{needle}`: {why}"
            );
        }
    }

    #[test]
    fn the_cache_folder_is_distinct_from_the_plain_and_dap_installs() {
        let folder = cache_folder();
        let name = folder.file_name().unwrap();
        assert!(name.starts_with("1984js_3c3044b"), "{name}");
        assert!(name.contains("robotbridge"), "{name}");
    }
}
