//! Live emulator control - `start_emulator`/`load_snapshot`/`load_disc`/
//! `save_disc`/`read_memory`/`write_memory`/`type_text`/`screenshot`/
//! `close_emulator`/`list_sessions`, all riding `RobotHandle`
//! (`cpclib-runner`) through the actor-thread session manager in
//! `crate::session`.
//!
//! Window-capture-backed backends (most emulators except SugarBox/
//! AMSpiriT, which have native APIs) need a real display to launch against
//! - there is no headless/CI support here, and no attempt to add one.

use base64::Engine;
use camino::Utf8PathBuf;
use cpclib_runner::emucontrol::EmulatorConf;
use cpclib_runner::runner::emulator::cadence::CadenceVersion;
use cpclib_runner::runner::emulator::caprice_forever::CapriceForeverVersion;
use cpclib_runner::runner::emulator::cpcemu::CpcEmuVersion;
use cpclib_runner::runner::emulator::cpcemupower::CpcEmuPowerVersion;
use cpclib_runner::runner::emulator::emulator1984::Emulator1984Version;
use cpclib_runner::runner::emulator::retrovm::RetroVmVersion;
use cpclib_runner::runner::emulator::{
    AceVersion, AmspiritLiteVersion, AmspiritVersion, CpcecVersion, Emulator, SugarBoxV2Version,
    WinapeVersion
};
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::{Value, json};

use crate::McpServer;
use crate::error::{ToolError, ToolResult};

fn parse_emulator(name: &str) -> Result<(Emulator, String), ToolError> {
    let label = name.to_ascii_lowercase();
    let emulator = match label.as_str() {
        "ace" => Emulator::Ace(AceVersion::default()),
        "amspirit" => Emulator::Amspirit(AmspiritVersion::default()),
        "amspiritlite" => Emulator::AmspiritLite(AmspiritLiteVersion::default()),
        "cadence" => Emulator::Cadence(CadenceVersion::default()),
        "capriceforever" => Emulator::CapriceForever(CapriceForeverVersion::default()),
        "cpcemu" => Emulator::CpcEmu(CpcEmuVersion::default()),
        "cpcec" => Emulator::Cpcec(CpcecVersion::default()),
        "cpcemupower" => Emulator::CpcEmuPower(CpcEmuPowerVersion::default()),
        "emulator1984" => Emulator::Emulator1984(Emulator1984Version::default()),
        "retrovm" => Emulator::RetroVm(RetroVmVersion::default()),
        "winape" => Emulator::Winape(WinapeVersion::default()),
        "sugarbox" | "sugarboxv2" => Emulator::SugarBoxV2(SugarBoxV2Version::default()),
        other => {
            return Err(ToolError::invalid_input(format!(
                "unknown emulator '{other}' - expected one of: ace, amspirit, amspiritlite, \
                 cadence, capriceforever, cpcemu, cpcec, cpcemupower, emulator1984, retrovm, \
                 winape, sugarbox"
            )));
        }
    };
    Ok((emulator, label))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StartEmulatorInput {
    /// One of: ace, amspirit, amspiritlite, cadence, capriceforever,
    /// cpcemu, cpcec, cpcemupower, emulator1984, retrovm, winape, sugarbox.
    /// Window-capture-backed backends (everything except sugarbox/
    /// amspiritlite/ace, which have native debug/web APIs - plain
    /// `amspirit`, the full wine-based build, does not) need a real
    /// display.
    pub emulator: String,
    /// `.sna` snapshot to launch with.
    pub snapshot: Option<String>,
    /// Disc image for drive A.
    pub drive_a: Option<String>,
    /// Disc image for drive B.
    pub drive_b: Option<String>,
    /// Run under a dedicated, private virtual display instead of this
    /// server's real desktop (Linux only). Because that overrides this
    /// whole server process's display for as long as the session lives, a
    /// headless session requires **no other session of any kind** to be
    /// open, and refuses any other session from starting until it closes -
    /// see this tool's own description. Default false.
    pub headless: Option<bool>
}

/// Backends whose `load_snapshot` is a real API call (`RobotHandle::
/// load_snapshot` succeeds rather than returning the default "no
/// snapshot-load facility" error) - see each's `UsedEmulator` impl in
/// `cpclib-runner`. These are exactly the emulators `start_emulator`'s own
/// `snapshot` launch argument is unreliable for: they're driven through a
/// debug/web API and don't reliably honor a plain CLI positional the way a
/// classic emulator (CPCEC, plain Amspirit, ...) does.
const API_DRIVEN_SNAPSHOT_LOAD: &[&str] = &["sugarbox", "sugarboxv2", "amspiritlite", "ace"];

pub(crate) async fn start_emulator(
    sessions: &crate::session::SessionManager,
    input: StartEmulatorInput
) -> ToolResult {
    let (emulator, label) = parse_emulator(&input.emulator)?;
    let snapshot = input.snapshot.map(Utf8PathBuf::from);
    let conf = EmulatorConf::builder()
        .transparent(false)
        .break_on_bad_vbl(false)
        .break_on_bad_hbl(false)
        .maybe_snapshot(snapshot.clone())
        .maybe_drive_a(input.drive_a.map(Utf8PathBuf::from))
        .maybe_drive_b(input.drive_b.map(Utf8PathBuf::from))
        .build();
    let headless = input.headless.unwrap_or(false);
    let session_id = sessions.start(emulator, label.clone(), conf, headless).await?;

    // The launch argument above is the only mechanism a classic, CLI-only
    // emulator has - already handled by `conf`. For an API-driven backend
    // it is not reliably honored, so also load it for real through the
    // API, which is the mechanism that actually works there. A short
    // extra settle delay first: the debug/web server binds early in the
    // process's own boot sequence, but `start()` only waits for the robot
    // *window* to appear, not specifically for that server to be
    // listening yet - a fresh process can still lose that race.
    let mut snapshot_loaded_via_api = false;
    let mut ace_load_unverifiable = false;
    if let Some(path) = snapshot
        && API_DRIVEN_SNAPSHOT_LOAD.contains(&label.as_str())
    {
        tokio::time::sleep(std::time::Duration::from_millis(750)).await;
        match sessions.load_snapshot(&session_id, path).await {
            Ok(()) => {
                snapshot_loaded_via_api = true;
                // ACE's own `loadFile` API has no failure signal at all -
                // it reports success unconditionally, load or no load. A
                // caller that needs certainty should follow up with
                // `read_memory`/`screenshot` rather than trust this alone.
                ace_load_unverifiable = label == "ace";
            },
            Err(e) => return Err(e)
        }
    }

    Ok(json!({
        "session_id": session_id,
        "emulator": label,
        "snapshot_loaded_via_api": snapshot_loaded_via_api,
        "ace_snapshot_load_unverifiable": ace_load_unverifiable
    }))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SessionIdInput {
    pub session_id: String
}

pub(crate) async fn close_emulator(
    sessions: &crate::session::SessionManager,
    input: SessionIdInput
) -> ToolResult {
    sessions.close(&input.session_id).await?;
    Ok(json!({ "closed": true, "session_id": input.session_id }))
}

pub(crate) fn list_sessions(sessions: &crate::session::SessionManager) -> ToolResult {
    let list: Vec<Value> = sessions
        .list()
        .into_iter()
        .map(|s| {
            json!({
                "session_id": s.id,
                "emulator": s.emulator,
                "idle_seconds": s.idle_seconds,
                "age_seconds": s.age_seconds,
                "headless": s.headless
            })
        })
        .collect();
    Ok(json!({ "sessions": list }))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TypeTextInput {
    pub session_id: String,
    /// Text to type into the emulator window, `\n` becomes Enter.
    pub text: String
}

pub(crate) async fn type_text(
    sessions: &crate::session::SessionManager,
    input: TypeTextInput
) -> ToolResult {
    sessions.type_text(&input.session_id, input.text).await?;
    Ok(json!({ "typed": true }))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LoadSnapshotInput {
    pub session_id: String,
    pub path: String
}

pub(crate) async fn load_snapshot(
    sessions: &crate::session::SessionManager,
    input: LoadSnapshotInput
) -> ToolResult {
    sessions
        .load_snapshot(&input.session_id, Utf8PathBuf::from(input.path))
        .await?;
    Ok(json!({ "loaded": true }))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LoadDiscInput {
    pub session_id: String,
    /// 0 = drive A, 1 = drive B.
    pub drive: u8,
    pub path: String
}

pub(crate) async fn load_disc(
    sessions: &crate::session::SessionManager,
    input: LoadDiscInput
) -> ToolResult {
    sessions
        .load_disc(&input.session_id, input.drive, Utf8PathBuf::from(input.path))
        .await?;
    Ok(json!({ "loaded": true }))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SaveDiscInput {
    pub session_id: String,
    pub drive: u8
}

pub(crate) async fn save_disc(
    sessions: &crate::session::SessionManager,
    input: SaveDiscInput
) -> ToolResult {
    let bytes = sessions.save_disc(&input.session_id, input.drive).await?;
    Ok(json!({
        "bytes_len": bytes.len(),
        "data_base64": base64::engine::general_purpose::STANDARD.encode(bytes)
    }))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ReadMemoryInput {
    pub session_id: String,
    pub address: u16,
    pub count: u16
}

pub(crate) async fn read_memory(
    sessions: &crate::session::SessionManager,
    input: ReadMemoryInput
) -> ToolResult {
    let bytes = sessions
        .read_memory(&input.session_id, input.address, input.count)
        .await?;
    Ok(json!({
        "address": input.address,
        "count": bytes.len(),
        "data_base64": base64::engine::general_purpose::STANDARD.encode(bytes)
    }))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WriteMemoryInput {
    pub session_id: String,
    pub address: u16,
    /// Base64-encoded bytes to write.
    pub data_base64: String
}

pub(crate) async fn write_memory(
    sessions: &crate::session::SessionManager,
    input: WriteMemoryInput
) -> ToolResult {
    let data = base64::engine::general_purpose::STANDARD
        .decode(&input.data_base64)
        .map_err(|e| ToolError::invalid_input(format!("data_base64 is not valid base64: {e}")))?;
    let len = data.len();
    sessions
        .write_memory(&input.session_id, input.address, data)
        .await?;
    Ok(json!({ "address": input.address, "count": len }))
}

pub(crate) async fn screenshot(
    sessions: &crate::session::SessionManager,
    input: SessionIdInput
) -> ToolResult {
    let png = sessions.screenshot(&input.session_id).await?;
    Ok(json!({ "png_base64": base64::engine::general_purpose::STANDARD.encode(png) }))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct HeadlessScreenshotInput {
    /// One of: ace, amspirit, amspiritlite, cadence, capriceforever,
    /// cpcemu, cpcec, cpcemupower, emulator1984, retrovm, winape, sugarbox.
    pub emulator: String,
    /// `.sna` snapshot to launch with.
    pub snapshot: Option<String>,
    /// Disc image for drive A.
    pub drive_a: Option<String>,
    /// Disc image for drive B.
    pub drive_b: Option<String>,
    /// Text to type once the emulator has settled, before the screenshot
    /// (e.g. a BASIC command followed by `\n`).
    pub autorun: Option<String>,
    /// How long to wait after launch (and after `autorun`, if given)
    /// before the screenshot. Default 2.
    pub seconds: Option<f64>
}

/// The whole "verify headlessly" sequence in one call: launch under a
/// private virtual display (Linux only - see `start_emulator`'s own
/// `headless` field), optionally type `autorun`, wait `seconds`,
/// screenshot, close. Always closes the session before returning, success
/// or failure, so a caller can never leave a headless session (and its
/// exclusive hold on this server) open by forgetting `close_emulator`.
pub(crate) async fn headless_screenshot(
    sessions: &crate::session::SessionManager,
    input: HeadlessScreenshotInput
) -> ToolResult {
    let (emulator, label) = parse_emulator(&input.emulator)?;
    let conf = EmulatorConf::builder()
        .transparent(false)
        .break_on_bad_vbl(false)
        .break_on_bad_hbl(false)
        .maybe_snapshot(input.snapshot.map(Utf8PathBuf::from))
        .maybe_drive_a(input.drive_a.map(Utf8PathBuf::from))
        .maybe_drive_b(input.drive_b.map(Utf8PathBuf::from))
        .build();

    let session_id = sessions.start(emulator, label.clone(), conf, true).await?;

    let run = async {
        if let Some(text) = &input.autorun {
            sessions.type_text(&session_id, text.clone()).await?;
        }
        let seconds = input.seconds.unwrap_or(2.0).max(0.0);
        tokio::time::sleep(std::time::Duration::from_secs_f64(seconds)).await;
        sessions.screenshot(&session_id).await
    };
    let result = run.await;

    // Always close, whether the run above succeeded or not - a headless
    // session left open on error would keep its exclusive hold on this
    // server indefinitely (until the idle timeout, minutes away).
    let _ = sessions.close(&session_id).await;

    let png = result?;
    Ok(json!({
        "emulator": label,
        "png_base64": base64::engine::general_purpose::STANDARD.encode(png)
    }))
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Value>, Json<Value>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = emulator_router, vis = "pub(crate)")]
impl McpServer {
    #[tool(description = "Launch a new emulator session (a dedicated background process/window) \
                           and return its session_id. Window-capture-backed backends need a real \
                           display, unless headless is set (Linux only): runs under a private \
                           virtual display, never touching the real desktop, but requires \
                           exclusive access to this whole server - refuses to start while any \
                           other session is open, and blocks any other session from starting \
                           until it closes. For a one-shot 'run and screenshot' need, prefer \
                           headless_screenshot instead - it handles the whole \
                           start/run/capture/close sequence in one call.")]
    async fn start_emulator(
        &self,
        Parameters(input): Parameters<StartEmulatorInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(start_emulator(&self.sessions, input).await)
    }

    #[tool(description = "MUTATING: closes and force-terminates a live emulator session.")]
    async fn close_emulator(
        &self,
        Parameters(input): Parameters<SessionIdInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(close_emulator(&self.sessions, input).await)
    }

    #[tool(description = "List every live emulator session with its id, emulator, and idle time. \
                           Read-only.")]
    async fn list_sessions(&self) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(list_sessions(&self.sessions))
    }

    #[tool(description = "MUTATING: types text into a live emulator session's window (\\n = Enter).")]
    async fn type_text(
        &self,
        Parameters(input): Parameters<TypeTextInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(type_text(&self.sessions, input).await)
    }

    #[tool(description = "MUTATING: loads a .sna snapshot into a live emulator session.")]
    async fn load_snapshot(
        &self,
        Parameters(input): Parameters<LoadSnapshotInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(load_snapshot(&self.sessions, input).await)
    }

    #[tool(description = "MUTATING: loads a disc image into a live emulator session's drive A/B.")]
    async fn load_disc(
        &self,
        Parameters(input): Parameters<LoadDiscInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(load_disc(&self.sessions, input).await)
    }

    #[tool(description = "Reads back a live emulator session's drive A/B disc image as base64. \
                           Read-only.")]
    async fn save_disc(
        &self,
        Parameters(input): Parameters<SaveDiscInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(save_disc(&self.sessions, input).await)
    }

    #[tool(description = "Reads memory from a live emulator session, base64-encoded. Read-only.")]
    async fn read_memory(
        &self,
        Parameters(input): Parameters<ReadMemoryInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(read_memory(&self.sessions, input).await)
    }

    #[tool(description = "MUTATING: writes base64-encoded bytes into a live emulator session's \
                           memory.")]
    async fn write_memory(
        &self,
        Parameters(input): Parameters<WriteMemoryInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(write_memory(&self.sessions, input).await)
    }

    #[tool(description = "Takes a screenshot of a live emulator session's window as base64 PNG. \
                           Read-only.")]
    async fn screenshot(
        &self,
        Parameters(input): Parameters<SessionIdInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(screenshot(&self.sessions, input).await)
    }

    #[tool(description = "MUTATING (of nothing you'll see again - the session is always closed \
                           before this returns): a short headless run ending in one screenshot. \
                           Launches an emulator under a private virtual display (Linux only - no \
                           window ever touches this server's real desktop), optionally types \
                           autorun, waits seconds, takes a screenshot, and closes the session - \
                           the whole start_emulator+type_text+sleep+screenshot+close_emulator \
                           sequence in one call. Requires exclusive access to this server: \
                           refuses to run while any other session (headless or not) is open.")]
    async fn headless_screenshot(
        &self,
        Parameters(input): Parameters<HeadlessScreenshotInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(headless_screenshot(&self.sessions, input).await)
    }
}
