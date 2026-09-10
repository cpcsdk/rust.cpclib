//! Patching 1984js so this crate's debug session can reach its in-page debug
//! engine, and installing a distribution carrying that patch.
//!
//! `cpclib_runner::web::js1984` downloads and serves the plain upstream
//! distribution for anyone who just wants to run it (its own `cpc run`
//! command, for instance) - it knows nothing about DAP. Everything here is
//! specific to this crate's own protocol: the bridge script builds real DAP
//! request/response envelopes and answers three protocol extensions of ours
//! (`cpclib/autotype`, `cpclib/setWatches`, `cpclib/machineState`), so it
//! belongs here rather than in the generic runner. This module downloads the
//! same upstream files (reusing `cpclib_runner::web::js1984::download_dist`)
//! into a cache folder of its own, then applies the patch on top - kept
//! separate from the runner's plain install so the two never collide and
//! bumping the bridge never forces a redownload for someone who only runs
//! `cpc run`.

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};

/// What the patch adds to an upstream 1984js `web/dist`.
pub const BRIDGE_SCRIPT: &str = include_str!("../assets/1984js/cpclib-bridge.js");
pub const BRIDGE_FILENAME: &str = "cpclib-bridge.js";

/// Bumped whenever the bridge changes.
///
/// It is part of the cache folder name because the installed-or-not check
/// below is folder existence alone: without this, a changed bridge would
/// silently never be reinstalled.
pub const PATCH_REVISION: u32 = 1;

/// The one line appended to `app.js`, handing the bridge the objects it cannot
/// otherwise reach.
///
/// `loadSnapshotFile` is among them deliberately rather than re-implemented:
/// loading a snapshot is not just `poc_load_snapshot`, it also resets audio,
/// re-reads the memory size, and - the part that matters here - calls
/// `adoptCoreMlBreakpoints()`, which arms the emulator's breakpoint channels
/// from the chunks *inside* the snapshot. Reusing upstream's function is what
/// makes a `BREAKPOINT` directive written into the source actually stop the
/// program.
///
/// Upstream's own session-creation call keeps the emscripten module and the
/// debug session as function locals and `app.js` exposes no global at all,
/// so *some* edit is unavoidable. This is the smallest one that works: it
/// publishes what already exists and changes no behaviour.
const APP_HOOK: &str = "\n// added by cpclib: hand the debug session to the bridge, if one is loaded\n\
                        if (typeof globalThis.__cpclib_attach === 'function') {\n\
                        \x20 globalThis.__cpclib_attach({ module: m, session: mlDap, connection: new JS1984DAP.Connection(mlDap), loadSnapshot: loadSnapshotFile, startAudio: startAudio, audioContext: () => audioCtx });\n\
                        }\n";

/// The one line appended right after `frame()`'s own `lastFrame` counter is
/// declared, exposing the same catch-up step upstream's `requestAnimationFrame`
/// loop runs.
///
/// `frame()` is the only thing that ever calls `m._poc_step()`, and it is only
/// ever invoked by `requestAnimationFrame` - which browsers suspend once the
/// tab is not the visible one. That freezes CPU execution itself, not just
/// rendering: a breakpoint ahead of the current PC is never reached until the
/// tab is looked at again, no matter how the debugger tries to detect it.
///
/// This closes over the very same `lastFrame` binding `frame()` uses (it is
/// inserted into the same enclosing scope, right after the `let`), so calling
/// it from the bridge's own poll while the tab is hidden and calling it from
/// `frame()` while visible can never double-count the same wall-clock gap -
/// there is exactly one counter, advanced by whichever caller is active.
const STEP_HOOK: &str = "\n// added by cpclib: let the bridge keep stepping the CPU while \
                         requestAnimationFrame is suspended (a backgrounded tab), sharing this \
                         same lastFrame counter so nothing is ever caught up twice\n\
                         globalThis.__cpclib_step_catchup = function (time) {\n\
                         \x20 while (time - lastFrame >= 20) {\n\
                         \x20\x20 m._poc_step();\n\
                         \x20\x20 lastFrame += 20;\n\
                         \x20\x20 scheduleAudio();\n\
                         \x20\x20 pollGamepad();\n\
                         \x20\x20 updateLed();\n\
                         \x20\x20 updateTapeDeck();\n\
                         \x20 }\n\
                         };\n";

/// The `<script>` added to `index.html`, immediately before `</body>` so the
/// bridge loads only after every one of the emulator's own scripts has
/// already run.
const INDEX_HOOK: &str = "<script src=\"cpclib-bridge.js\"></script>\n</body>";

#[derive(Debug)]
pub enum PatchError {
    Missing(Utf8PathBuf),
    Io(String),
    /// The anchor we insert at was not found exactly once, which means upstream
    /// changed under us. Naming the file is the whole point: a silently
    /// half-applied patch produces an emulator that loads and then does
    /// nothing, which is far harder to diagnose than a refused install.
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
pub fn apply_bridge_patch(root: &Utf8Path) -> Result<(), PatchError> {
    let index = root.join("index.html");
    let app = root.join("app.js");
    for required in [&index, &app] {
        if !required.exists() {
            return Err(PatchError::Missing(required.clone()));
        }
    }

    fs_err::write(root.join(BRIDGE_FILENAME), BRIDGE_SCRIPT)
        .map_err(|e| PatchError::Io(e.to_string()))?;

    // index.html: load the bridge last.
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

    // app.js: publish the session the bridge needs.
    let app_text = read(&app)?;
    if !app_text.contains("__cpclib_attach") {
        let anchor = "createMlDapSession();";
        let found = app_text.matches(anchor).count();
        if found == 0 {
            return Err(PatchError::AnchorNotUnique {
                file: app.clone(),
                anchor,
                found
            });
        }
        // The call appears several times (startup, and on each machine reset);
        // hooking the *definition* instead would need brace matching, so the
        // hook is appended after every call and is written to be idempotent on
        // the bridge side.
        write(
            &app,
            &app_text.replace(anchor, &format!("{anchor}{APP_HOOK}"))
        )?;
    }

    // app.js: expose frame()'s catch-up step so the bridge can drive it while
    // the tab is hidden and requestAnimationFrame is not calling frame() at all.
    let app_text = read(&app)?;
    if !app_text.contains("__cpclib_step_catchup") {
        let anchor = "let lastFrame = 0;";
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
            &app_text.replace(anchor, &format!("{anchor}{STEP_HOOK}"))
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

/// Where the patched distribution lives - a name of its own, distinct from
/// `cpclib_runner::web::js1984`'s plain-install cache folder, so the two
/// never collide.
fn cache_folder() -> Utf8PathBuf {
    let short = &cpclib_runner::web::js1984::PINNED_COMMIT[..7];
    cpclib_runner::delegated::base_cache_folder()
        .join(format!("1984js_{short}_dapbridge_p{PATCH_REVISION}"))
}

/// Whether the patched distribution is already installed.
fn is_installed() -> bool {
    let root = cache_folder();
    root.join("index.html").exists() && root.join(BRIDGE_FILENAME).exists()
}

/// Download the pinned distribution and apply the bridge patch.
///
/// A no-op when it is already there, so callers can call it unconditionally.
pub fn install() -> Result<Utf8PathBuf, String> {
    let root = cache_folder();
    if is_installed() {
        return Ok(root);
    }

    fs_err::create_dir_all(&root).map_err(|e| format!("cannot create {root}: {e}"))?;
    cpclib_runner::web::js1984::download_dist(&root)?;

    apply_bridge_patch(&root).map_err(|e| {
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

    /// A directory shaped like the parts of `web/dist` the patch touches.
    fn fake_dist() -> camino_tempfile::Utf8TempDir {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("index.html"),
            "<html><body><script src=\"app.js\"></script></body></html>"
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("app.js"),
            "function boot() {\n  createMlDapSession();\n}\n\
             let lastFrame = 0;\n\
             function frame(time) {\n  m._poc_step();\n}\n"
        )
        .unwrap();
        tmp
    }

    #[test]
    fn the_patch_adds_the_bridge_and_both_hooks() {
        let tmp = fake_dist();
        apply_bridge_patch(tmp.path()).expect("applies");

        assert!(tmp.path().join(BRIDGE_FILENAME).exists());
        let index = std::fs::read_to_string(tmp.path().join("index.html")).unwrap();
        assert!(index.contains("cpclib-bridge.js"));
        assert!(
            index.find("cpclib-bridge.js") < index.find("</body>"),
            "the bridge loads before the body closes"
        );
        let app = std::fs::read_to_string(tmp.path().join("app.js")).unwrap();
        assert!(app.contains("__cpclib_attach"));
        assert!(app.contains("__cpclib_step_catchup"));
        assert!(
            app.find("let lastFrame = 0;").unwrap() < app.find("__cpclib_step_catchup").unwrap(),
            "the hook must close over lastFrame, so it has to come after the declaration"
        );
    }

    /// Upstream drifting on the stepping anchor specifically must also fail
    /// loudly - losing this one silently would mean a backgrounded tab quietly
    /// stops running the machine again, with no error anywhere to explain why.
    #[test]
    fn a_missing_step_anchor_is_refused_by_name() {
        let tmp = fake_dist();
        std::fs::write(
            tmp.path().join("app.js"),
            "function boot() {\n  createMlDapSession();\n}\n"
        )
        .unwrap();
        let error = apply_bridge_patch(tmp.path()).unwrap_err();
        let text = error.to_string();
        assert!(text.contains("app.js"), "{text}");
        assert!(text.contains("1984js has changed"), "{text}");
    }

    /// Re-installing must not double-apply.
    #[test]
    fn the_patch_is_idempotent() {
        let tmp = fake_dist();
        apply_bridge_patch(tmp.path()).unwrap();
        let once = std::fs::read_to_string(tmp.path().join("app.js")).unwrap();
        apply_bridge_patch(tmp.path()).unwrap();
        let twice = std::fs::read_to_string(tmp.path().join("app.js")).unwrap();
        assert_eq!(once, twice);
    }

    /// Upstream drifting must fail loudly, naming the file.
    #[test]
    fn a_missing_anchor_is_refused_by_name() {
        let tmp = fake_dist();
        std::fs::write(tmp.path().join("app.js"), "function boot() {}\n").unwrap();
        let error = apply_bridge_patch(tmp.path()).unwrap_err();
        let text = error.to_string();
        assert!(text.contains("app.js"), "{text}");
        assert!(text.contains("1984js has changed"), "{text}");
    }

    #[test]
    fn a_missing_file_is_refused_by_name() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let error = apply_bridge_patch(tmp.path()).unwrap_err();
        assert!(error.to_string().contains("index.html"), "{error}");
    }

    /// The bridge has a few pieces it cannot work without, and losing one is
    /// silent: the emulator still loads, still runs, and simply never tells the
    /// debugger anything.
    ///
    /// This exists because exactly that happened - an edit to the transport
    /// replaced a block that happened to contain the event poll, and the
    /// symptom was "breakpoints do nothing", four layers away from the cause.
    #[test]
    fn the_bridge_keeps_the_pieces_it_cannot_work_without() {
        for (needle, why) in [
            (
                "setInterval",
                "without the poll, queued events are never flushed"
            ),
            ("sync()", "sync() is what flushes them"),
            (
                "__cpclib_attach",
                "the emulator hands us its session through this"
            ),
            (
                "__cpclib_session",
                "the token is injected into the page, not the URL"
            ),
            ("loadSnapshot", "the program under test has to be loaded"),
            ("/session/events", "the downstream half of the debug-message channel"),
            ("/session/upstream", "the upstream half"),
            (
                "cpclib/setWatches",
                "watches are armed through the module, not through the unmodified upstream script"
            ),
            (
                "cpclib/autotype",
                "auto-running a launched BASIC program depends on this"
            ),
            (
                "_poc_key",
                "autotype presses real keys, the same call app.js's own keydown handler makes"
            ),
            (
                "_poc_debug_watch_serial",
                "without the write-event poll, watched labels report nothing"
            ),
            ("notifyWrite", "which is how a write reaches the editor"),
            (
                "_poc_debug_breakpoint_clear",
                "channels the page armed from the snapshot are ones we cannot clear"
            ),
            (
                "startAudio",
                "a debug session never clicks the page, so nothing else starts the audio"
            ),
            (
                "poc_save_snapshot",
                "the CRTC and Gate Array are readable only through a snapshot"
            ),
            (
                "__cpclib_step_catchup",
                "without it a backgrounded tab never reaches a breakpoint at all, not just never reports one"
            ),
            (
                "document.hidden",
                "the fallback must stay off while requestAnimationFrame is already stepping, or the two would race"
            )
        ] {
            assert!(
                BRIDGE_SCRIPT.contains(needle),
                "the bridge lost `{needle}`: {why}"
            );
        }
    }

    /// Naming the cache folder after the patch revision, not just the pinned
    /// commit, forces a reinstall when the bridge changes.
    #[test]
    fn the_cache_folder_names_both_revisions() {
        let folder = cache_folder();
        let name = folder.file_name().unwrap();
        assert!(name.starts_with("1984js_3c3044b"), "{name}");
        assert!(name.ends_with(&format!("_p{PATCH_REVISION}")), "{name}");
    }
}
