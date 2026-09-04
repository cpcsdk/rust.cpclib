//! Installing and patching a web-based emulator.
//!
//! Downloading a tool and *being a web application* are unrelated concerns:
//! the existing `DelegateApplicationDescription` already knows how to fetch,
//! unpack and cache, and none of that cares whether what comes out is an
//! executable or a directory of files to serve. So this adds only the part
//! that is genuinely different - what "launching" means - plus the patch that
//! makes an in-page debugger reachable from outside.

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};

pub mod js1984;
pub mod server;
pub use server::{ServerHandle, serve};

/// How an application is started once it is installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchKind {
    /// Spawn a process, the assumption everything else in this crate makes.
    Native,
    /// Serve a directory and open a page in a browser or an editor tab.
    Web
}

/// An application that is served rather than spawned.
pub trait WebApplication {
    /// The directory whose files are served.
    fn web_root(&self) -> Utf8PathBuf;
    /// The document to open inside it.
    fn entry_document(&self) -> &'static str {
        "index.html"
    }
}

/// What our patch adds to an upstream 1984js `web/dist`.
pub const BRIDGE_SCRIPT: &str = include_str!("../../assets/1984js/cpclib-bridge.js");
pub const BRIDGE_FILENAME: &str = "cpclib-bridge.js";

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
/// `createMlDapSession()` keeps the emscripten module and the DAP session as
/// function locals and `app.js` exposes no global at all, so *some* edit is
/// unavoidable. This is the smallest one that works: it publishes what already
/// exists and changes no behaviour.
const APP_HOOK: &str = "\n// added by cpclib: hand the DAP session to the bridge, if one is loaded\n\
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
/// bridge loads after `dap.js` and `app.js`.
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

/// The MIME type to serve a file with.
///
/// Written out rather than pulled from a crate because exactly one entry here
/// is load-bearing: upstream requires `.wasm` to arrive as `application/wasm`,
/// and getting it wrong produces a blank page with no error worth reading.
pub fn mime_for(path: &Utf8Path) -> &'static str {
    match path.extension() {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("png") => "image/png",
        Some("json") => "application/json",
        Some("sna") => "application/octet-stream",
        _ => "application/octet-stream"
    }
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
            ("/session/events", "the downstream half of the DAP channel"),
            ("/session/dap", "the upstream half"),
            (
                "cpclib/setWatches",
                "watches are armed through the module, not through dap.js"
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

    /// The one MIME type that actually matters.
    #[test]
    fn wasm_is_served_as_wasm() {
        assert_eq!(mime_for(Utf8Path::new("6128.wasm")), "application/wasm");
        assert_eq!(
            mime_for(Utf8Path::new("index.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            mime_for(Utf8Path::new("app.js")),
            "text/javascript; charset=utf-8"
        );
    }
}
