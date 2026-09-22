//! `compare_behavior` - does a candidate snapshot produce the same runtime
//! state as a baseline?
//!
//! `search_reorderings`/`measure_variants`/`suggest_optimizations` prove a
//! change safe *statically*: does the dataflow analysis show this swap/edit
//! touching a register, flag or memory location something later still
//! needs. That is not the same as verified - a real run is the only thing
//! that actually confirms two programs behave the same, and it catches
//! anything the static analysis model doesn't cover (a peephole rule with a
//! subtle bug, an edit reaching further than the analysis assumed, real
//! hardware-timing-sensitive code).
//!
//! Scoped deliberately to **caller-given memory ranges over N real
//! seconds**, not a byte-exact whole-RAM/frame comparison, for two honest
//! reasons:
//! - No frame-step primitive exists in this codebase's emulator automation
//!   (`Robot`) - it drives real emulator processes/windows, not a debugger,
//!   and only ever offers a wall-clock sleep. Bridging it with `cpclib-dap`'s
//!   real breakpoint/step machinery for true frame-accurate, fully
//!   deterministic comparison is a materially bigger, separate effort.
//! - Even with perfect frame accuracy, a whole-RAM/screen compare is the
//!   wrong question for most CPC code anyway: VRAM changes every frame *on
//!   purpose* (animation, a moving sprite, a palette cycle) - two genuinely
//!   equivalent programs would still disagree there. What a caller actually
//!   wants confirmed is a specific location the program's own logic
//!   computes (a checksum, a game-state buffer, a trace table) - which only
//!   the caller (who knows what the change touched) can name. This tool
//!   refuses to guess a default range for the same reason
//!   `compare_link_sizes` refuses to guess which source line selects a
//!   cruncher: a wrong guess is worse than an explicit requirement.

use std::time::Duration;

use camino::Utf8PathBuf;
use cpclib_runner::emucontrol::EmulatorConf;
use cpclib_runner::runner::emulator::Emulator;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::{Value, json};

use crate::McpServer;
use crate::error::{ToolError, ToolResult};
use crate::session::SessionManager;
use crate::tools::emulator::parse_emulator;

/// A stable (non-randomly-seeded, unlike `std`'s default hasher) 64-bit
/// hash, so a hash reported by one call can be meaningfully compared
/// against one from a different call or a different process - `std`'s
/// `DefaultHasher` is reseeded per process and would silently defeat that.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RunSpec {
    /// `.sna` snapshot to launch with.
    pub snapshot: String,
    /// Disc image for drive A, if the run needs one.
    pub drive_a: Option<String>,
    /// Text to type once the emulator has settled, before the timing run
    /// begins (e.g. a BASIC command followed by `\n`).
    pub autorun: Option<String>
}

#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct MemoryRange {
    pub address: u16,
    pub count: u16
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CompareBehaviorInput {
    /// Emulator backend - same names as `start_emulator`; Linux-only for a
    /// headless run (see that tool's own description).
    pub emulator: String,
    pub baseline: RunSpec,
    pub candidate: RunSpec,
    /// How long each run gets before sampling, in real seconds - the same
    /// value for both, one run after the other (a headless session needs
    /// exclusive access to this whole server, so the two can never overlap;
    /// total wall time is roughly `2 * seconds` plus launch overhead).
    /// Default 2.
    pub seconds: Option<f64>,
    /// Which memory to compare after the run - required, must not be
    /// empty. See this module's own doc comment for why no default is
    /// guessed.
    pub memory_ranges: Vec<MemoryRange>,
    /// Also screenshot and hash both runs' final frame. Reported as an
    /// extra diagnostic only, never part of `equivalent` - background/
    /// animation state legitimately differs run to run even for identical
    /// code (see this module's own doc comment). Default false.
    pub include_screen: Option<bool>
}

struct RunSample {
    memory: Vec<Vec<u8>>,
    screen_png: Option<Vec<u8>>
}

/// The whole `start` -> `autorun` -> settle `seconds` -> sample -> `close`
/// sequence for one side of the comparison - always closes the session,
/// success or failure, the same discipline `headless_screenshot` already
/// uses (a headless session left open holds this whole server exclusively
/// until the idle timeout).
async fn run_and_sample(
    sessions: &SessionManager,
    emulator: Emulator,
    label: String,
    spec: &RunSpec,
    seconds: f64,
    memory_ranges: &[MemoryRange],
    want_screen: bool
) -> Result<RunSample, ToolError> {
    let conf = EmulatorConf::builder()
        .transparent(false)
        .break_on_bad_vbl(false)
        .break_on_bad_hbl(false)
        .maybe_snapshot(Some(Utf8PathBuf::from(&spec.snapshot)))
        .maybe_drive_a(spec.drive_a.clone().map(Utf8PathBuf::from))
        .build();
    let session_id = sessions.start(emulator, label, conf, true).await?;

    let run = async {
        if let Some(text) = &spec.autorun {
            sessions.type_text(&session_id, text.clone()).await?;
        }
        tokio::time::sleep(Duration::from_secs_f64(seconds)).await;

        let mut memory = Vec::with_capacity(memory_ranges.len());
        for r in memory_ranges {
            memory.push(sessions.read_memory(&session_id, r.address, r.count).await?);
        }
        let screen_png = if want_screen { Some(sessions.screenshot(&session_id).await?) } else { None };
        Ok::<_, ToolError>(RunSample { memory, screen_png })
    };
    let result = run.await;

    let _ = sessions.close(&session_id).await;
    result
}

/// Runs `baseline` and `candidate` under the same emulator, one after the
/// other, for the same real-time duration, and reports whether each
/// requested memory range came out byte-identical - see this module's own
/// doc comment for what that does and doesn't prove.
pub(crate) async fn compare_behavior(sessions: &SessionManager, input: CompareBehaviorInput) -> ToolResult {
    if input.memory_ranges.is_empty() {
        return Err(ToolError::invalid_input(
            "memory_ranges must not be empty - name at least one address range the change could \
             plausibly have touched (see this tool's own description for why no default is \
             guessed)"
        ));
    }
    let (emulator, label) = parse_emulator(&input.emulator)?;
    let seconds = input.seconds.unwrap_or(2.0).max(0.0);
    let want_screen = input.include_screen.unwrap_or(false);

    // Sequential, not concurrent: a headless session holds this whole
    // server exclusively, so a second one cannot start until the first's
    // `run_and_sample` has closed it.
    let baseline = run_and_sample(
        sessions,
        emulator.clone(),
        format!("{label}-baseline"),
        &input.baseline,
        seconds,
        &input.memory_ranges,
        want_screen
    )
    .await?;
    let candidate = run_and_sample(
        sessions,
        emulator,
        format!("{label}-candidate"),
        &input.candidate,
        seconds,
        &input.memory_ranges,
        want_screen
    )
    .await?;

    let mut equivalent = true;
    let ranges: Vec<Value> = input
        .memory_ranges
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let b = &baseline.memory[i];
            let c = &candidate.memory[i];
            let first_difference = b.iter().zip(c.iter()).position(|(x, y)| x != y);
            let equal = first_difference.is_none() && b.len() == c.len();
            if !equal {
                equivalent = false;
            }
            json!({
                "address": r.address,
                "count": r.count,
                "equal": equal,
                "baseline_hash": format!("{:016x}", fnv1a(b)),
                "candidate_hash": format!("{:016x}", fnv1a(c)),
                "first_difference": first_difference.map(|offset| json!({
                    "offset": offset,
                    "address": r.address.wrapping_add(offset as u16),
                    "baseline_byte": b[offset],
                    "candidate_byte": c[offset]
                }))
            })
        })
        .collect();

    let screen = want_screen.then(|| {
        let bh = baseline.screen_png.as_deref().unwrap_or_default();
        let ch = candidate.screen_png.as_deref().unwrap_or_default();
        json!({
            "equal": bh == ch,
            "baseline_hash": format!("{:016x}", fnv1a(bh)),
            "candidate_hash": format!("{:016x}", fnv1a(ch))
        })
    });

    Ok(json!({
        "emulator": label,
        "seconds": seconds,
        "equivalent": equivalent,
        "ranges": ranges,
        "screen": screen,
        "note": "`equivalent` covers only `memory_ranges`; `screen` (when requested) is a \
                 diagnostic only, since normal animation makes two genuinely equivalent runs \
                 disagree there. A `seconds`-long real-time run, not frame-accurate stepping - \
                 see this tool's own description."
    }))
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Value>, Json<Value>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = behavior_router, vis = "pub(crate)")]
impl McpServer {
    #[tool(description = "MUTATING (of nothing you'll see again - both sessions are always \
                           closed before this returns): confirms a candidate .sna behaves the \
                           same as a baseline .sna by actually running both, one after the \
                           other under a private headless display (Linux only), and comparing \
                           caller-named memory ranges after the same real-time settle period. \
                           Requires exclusive access to this server, like headless_screenshot. \
                           Use this to verify a search_reorderings/measure_variants/ \
                           apply_optimizations candidate really is behaviorally identical, not \
                           just statically provably-safe - the static analysis models can miss \
                           real edge cases. memory_ranges must be given explicitly: this tool \
                           never guesses which addresses matter, and does not compare the whole \
                           of RAM or the screen by default, since ordinary animation makes two \
                           genuinely equivalent programs disagree there. Pass include_screen: \
                           true for an extra (diagnostic-only, not part of the equivalent \
                           verdict) screen comparison.")]
    async fn compare_behavior(
        &self,
        Parameters(input): Parameters<CompareBehaviorInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(compare_behavior(&self.sessions, input).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_is_stable_and_sensitive_to_every_byte() {
        assert_eq!(fnv1a(b"hello"), fnv1a(b"hello"), "same input, same hash");
        assert_ne!(fnv1a(b"hello"), fnv1a(b"hellp"), "a single differing byte must change the hash");
        assert_ne!(fnv1a(b""), fnv1a(b"\0"), "empty vs. a single zero byte must differ");
    }

    #[tokio::test]
    async fn compare_behavior_rejects_empty_memory_ranges() {
        let sessions = SessionManager::new(std::time::Duration::from_secs(600));
        let err = compare_behavior(
            &sessions,
            CompareBehaviorInput {
                emulator: "cpcec".to_string(),
                baseline: RunSpec { snapshot: "a.sna".to_string(), drive_a: None, autorun: None },
                candidate: RunSpec { snapshot: "b.sna".to_string(), drive_a: None, autorun: None },
                seconds: None,
                memory_ranges: vec![],
                include_screen: None
            }
        )
        .await
        .expect_err("empty memory_ranges must be rejected before any session is started");
        assert_eq!(err.kind, "invalid_input");
    }

    #[tokio::test]
    async fn compare_behavior_rejects_an_unknown_emulator_before_starting_anything() {
        let sessions = SessionManager::new(std::time::Duration::from_secs(600));
        let err = compare_behavior(
            &sessions,
            CompareBehaviorInput {
                emulator: "not_a_real_emulator".to_string(),
                baseline: RunSpec { snapshot: "a.sna".to_string(), drive_a: None, autorun: None },
                candidate: RunSpec { snapshot: "b.sna".to_string(), drive_a: None, autorun: None },
                seconds: None,
                memory_ranges: vec![MemoryRange { address: 0, count: 1 }],
                include_screen: None
            }
        )
        .await
        .expect_err("an unknown emulator name must be rejected");
        assert_eq!(err.kind, "invalid_input");
    }
}
