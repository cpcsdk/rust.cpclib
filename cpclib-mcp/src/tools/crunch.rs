//! `compare_crunchers` - tries every supported crunch format on one byte
//! block and reports the resulting size for each, via
//! `cpclib_crunchers::CompressMethod::compress` (a clean, in-process,
//! standalone library call - no subprocess, no basm rebuild needed).
//!
//! Deliberately scoped to a single block in isolation: a project's *final
//! linked size* also depends on each format's own decruncher stub size,
//! and - confirmed against a real project (`skyline` uses one
//! `SELECTED_CRUNCHER` symbol, `birthtro` uses two different crunchers at
//! once in the same build) - there is no single, general way to rebuild a
//! whole project under a caller-chosen cruncher without already knowing
//! that project's own crunch-site conventions. That's a basm/cpclib-asm
//! capability gap (a standardized way to override a crunched section's
//! cruncher independent of a project's own selection convention), not
//! something this tool can safely guess at - so it only ever answers "what
//! does each format do to this exact block", not "what would my final ROM
//! look like".

use std::time::Duration;

use cpclib_crunchers::{CompressMethod, CrunchersError};
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::{Value, json};

use crate::McpServer;
use crate::error::{ToolError, ToolResult};

/// How long a single cruncher gets before this tool gives up on it and
/// reports a timeout for that one format rather than hanging forever.
/// Confirmed live: `pucrunch` (a vendored native tool) hangs indefinitely -
/// "Optimizing LZ77 and RLE lengths..." then nothing - on a degenerate,
/// maximally-repetitive input (256 zero bytes). Since `compress()` is a
/// synchronous, unbounded call with no timeout of its own, and this tool's
/// business logic runs directly on a `tool_router` async handler (not
/// `spawn_blocking`), a hang here would stall this whole MCP server, not
/// just this one call - the same class of bug already found and fixed
/// twice in `cpclib-runner`'s emulator automation this session. Run on its
/// own thread with a bounded `recv_timeout` instead, so a native cruncher
/// that hangs only costs one leaked thread (it cannot be force-killed from
/// safe Rust) and a clean per-format timeout error, never the whole call.
///
/// 15s was the original value, calibrated only against that tiny
/// degenerate-input hang case. Confirmed live to be too aggressive for
/// real data: `upkr` alone hit this timeout on a real, non-degenerate 16KB
/// CPC screen asset while still making genuine progress (unlike
/// `pucrunch`, which never budges at all) - a false "appears to hang"
/// report for a format that was simply still working. Since
/// `compare_crunchers` now runs every format concurrently rather than
/// sequentially (see its own comment), a longer bound here costs nothing
/// in the typical case - total wall time is bounded by the slowest format,
/// not the sum - so it's raised generously to give real slow-but-finishing
/// crunchers room, while still catching genuine hangs like `pucrunch`
/// eventually.
pub(crate) const CRUNCHER_TIMEOUT: Duration = Duration::from_secs(90);

/// Every format name tried when `formats` is omitted. Matches this crate's
/// `CompressMethod` variants one-to-one, except `lzsa1`/`lzsa2` (the single
/// `Lzsa(version, _)` variant, named for its two real on-wire formats the
/// way the asm-facing `CrunchType` directive already does) and
/// `zx0`/`zx0_backward`. No `aplib` - the crate has no separate
/// implementation, only `apultra` (LZAPU), confirmed against the real enum.
///
/// **No `pucrunch` or `shrinkler` here, deliberately** - both confirmed
/// live to write straight to this process's real stdout, which would
/// corrupt the MCP protocol stream (`main.rs`'s own stdout-is-reserved
/// contract):
/// - `pucrunch`'s vendored C (`extra/pucrunch.c`) writes a verbose per-byte
///   trace via bare `printf` (lines ~2509-2693), completely unconditional -
///   no verbosity flag exists anywhere in that file to disable it. It also
///   hangs indefinitely on a degenerate, maximally-repetitive input (see
///   [`CRUNCHER_TIMEOUT`]), a second, independent problem.
/// - `shrinkler`'s vendored C++ (`extra/Shrinkler4.6NoParityContext/
///   basm_bridge.cpp`) looked fixable at first - its `compress_for_basm`
///   takes a `log` parameter, and setting it `false` does silence two
///   `printf` lines in that bridge file - but live-tested against a real
///   project's real-sized data (not just this crate's own small test
///   input, which never actually exercised the progress path), the
///   multi-pass progress bar ("After 1st pass ... After 9th pass") prints
///   regardless: it comes from inside the underlying compression engine
///   itself, gated by nothing this bridge exposes (there's even a dead,
///   hardcoded `bool show_progress = true; // TODO set to false ASAP` in
///   that same file - unused by the compiler's own account, i.e. not
///   actually wired to anything). Properly fixing this needs either
///   patching the vendored engine's own progress-printing calls (real,
///   separate C++ surgery) or running the call in a subprocess with its
///   own stdout (the only way to redirect output that's actually safe
///   under concurrency - see `resolve_cruncher`'s own note on why a
///   process-wide fd redirect here would risk corrupting a *different*,
///   concurrent tool call's real response) - both deliberately out of
///   scope for this pass.
///
/// `resolve_cruncher` still accepts both names by explicit request (the
/// tool's own description warns about this), since bounding the hang and
/// trimming what output there is are still real, if partial, improvements
/// - but shipping either silently in the default list is not safe.
const ALL_FORMATS: &[&str] = &[
    "apultra", "exomizer", "lz4", "lz48", "lz49", "lzsa1", "lzsa2", "upkr", "zx0", "zx0_backward",
    "zx7"
];

pub(crate) fn resolve_cruncher(name: &str) -> Result<CompressMethod, ToolError> {
    match name {
        "none" => Ok(CompressMethod::None),
        "apultra" => Ok(CompressMethod::Apultra),
        "exomizer" => Ok(CompressMethod::Exomizer),
        "lz4" => Ok(CompressMethod::Lz4),
        "lz48" => Ok(CompressMethod::Lz48),
        "lz49" => Ok(CompressMethod::Lz49),
        "lzsa1" => {
            let version = cpclib_crunchers::lzsa::LzsaVersion::V1;
            Ok(CompressMethod::Lzsa(version, Some(version.default_minmatch())))
        },
        "lzsa2" => {
            let version = cpclib_crunchers::lzsa::LzsaVersion::V2;
            Ok(CompressMethod::Lzsa(version, Some(version.default_minmatch())))
        },
        // `log: false` still trims two `printf` lines even though it
        // doesn't stop the real problem (the engine's own progress bar -
        // see `ALL_FORMATS`'s own doc comment for the full story), so it's
        // worth keeping regardless of `Default::default()`'s `log: true`.
        "shrinkler" => {
            Ok(CompressMethod::Shrinkler(cpclib_crunchers::shrinkler::ShrinklerConfiguration {
                iterations: 9,
                log: false
            }))
        },
        "pucrunch" => Ok(CompressMethod::Pucrunch),
        "upkr" => Ok(CompressMethod::Upkr),
        "zx0" => Ok(CompressMethod::Zx0),
        "zx0_backward" => Ok(CompressMethod::BackwardZx0),
        "zx7" => Ok(CompressMethod::Zx7),
        other => {
            Err(ToolError::invalid_input(format!(
                "unknown cruncher '{other}' - expected one of: none, pucrunch/shrinkler \
                 (excluded by default, see tool description), {}",
                ALL_FORMATS.join(", ")
            )))
        }
    }
}

/// Runs `resolve_cruncher(name).compress(&data)` on a dedicated thread and
/// waits up to `timeout` for it - see [`CRUNCHER_TIMEOUT`]'s own doc
/// comment for why this exists. `name`/`data` are owned (not borrowed) so
/// they can move into the spawned thread; a hung cruncher leaks that one
/// thread (unavoidable - safe Rust has no way to force-kill a thread) but
/// this call still returns on time, with a clear per-format error instead
/// of hanging the whole tool call.
pub(crate) fn compress_with_timeout(name: String, data: Vec<u8>, timeout: Duration) -> Result<cpclib_crunchers::CompressionResult, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = match resolve_cruncher(&name) {
            Ok(method) => {
                method
                    .compress(&data)
                    .map_err(|CrunchersError::CompressionFailed| "compression failed".to_string())
            },
            Err(e) => Err(e.message)
        };
        // The receiver may already be gone if `recv_timeout` below already
        // gave up - that's fine, nothing left to deliver to.
        let _ = tx.send(result);
    });
    rx.recv_timeout(timeout).unwrap_or_else(|_| {
        Err(format!(
            "timed out after {timeout:?} - this cruncher appears to hang on this input \
             (confirmed live for pucrunch on a maximally-repetitive block)"
        ))
    })
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CompareCrunchersInput {
    /// Path to a file whose raw bytes to compare across crunchers.
    pub path: Option<String>,
    /// Inline basm source to assemble first, then compare the resulting
    /// bytes - alternative to `path`. Exactly one of `path`/`code` is
    /// required.
    pub code: Option<String>,
    /// Formats to try; omit to try every supported format. See this tool's
    /// description for the full list.
    pub formats: Option<Vec<String>>
}

/// Compresses one byte block with every requested cruncher and reports the
/// resulting size for each, sorted smallest-first. Read-only, in-process,
/// no rebuild - see this module's own doc comment for why it only ever
/// answers "what does each format do to this block", not a project's real
/// final linked size.
pub(crate) fn compare_crunchers(input: CompareCrunchersInput) -> ToolResult {
    let bytes = match (&input.path, &input.code) {
        (Some(_), Some(_)) => {
            return Err(ToolError::invalid_input(
                "give only one of `path`/`code`, not both"
            ));
        },
        (Some(p), None) => {
            fs_err::read(p).map_err(|e| ToolError::io(format!("cannot read {p}: {e}")))?
        },
        (None, Some(c)) => {
            cpclib_asm::assemble(c)
                .map_err(|e| ToolError::invalid_input(format!("could not assemble `code`: {e}")))?
        },
        (None, None) => {
            return Err(ToolError::invalid_input(
                "either `path` or `code` must be provided"
            ));
        }
    };

    let format_names: Vec<String> = input
        .formats
        .unwrap_or_else(|| ALL_FORMATS.iter().map(|s| s.to_string()).collect());

    // Validate every requested format name up front - fail the whole call
    // on a typo instead of silently reporting a not-attempted format's
    // "error" in the results, and it's cheap since `resolve_cruncher`
    // never runs a cruncher itself.
    for name in &format_names {
        resolve_cruncher(name)?;
    }

    // Run every format concurrently, each on its own OS thread (which is
    // what `compress_with_timeout` already spawns internally). There is no
    // shared mutable state between formats - each gets its own clone of
    // `bytes` - so this is safe, and it turns total wall-clock time from
    // the *sum* of every format's time into roughly the *slowest single
    // format's* time. Confirmed live this made a real difference: an
    // 11-format sweep over a real 16KB asset took 34.6s sequentially.
    let handles: Vec<(String, std::thread::JoinHandle<Result<cpclib_crunchers::CompressionResult, String>>)> =
        format_names
            .iter()
            .map(|name| {
                let name = name.clone();
                let data = bytes.clone();
                let handle_name = name.clone();
                (handle_name, std::thread::spawn(move || compress_with_timeout(name, data, CRUNCHER_TIMEOUT)))
            })
            .collect();

    let mut results: Vec<Value> = Vec::with_capacity(handles.len());
    for (name, handle) in handles {
        match handle
            .join()
            .unwrap_or_else(|_| Err(format!("cruncher thread for '{name}' panicked")))
        {
            Ok(compressed) => {
                results.push(json!({
                    "format": name,
                    "ok": true,
                    "size": compressed.stream.len(),
                    "delta": compressed.delta
                }));
            },
            Err(message) => {
                results.push(json!({
                    "format": name,
                    "ok": false,
                    "error": message
                }));
            }
        }
    }
    results.sort_by_key(|r| r["size"].as_u64().unwrap_or(u64::MAX));

    Ok(json!({
        "uncrunched_size": bytes.len(),
        "results": results
    }))
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Value>, Json<Value>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = crunch_router, vis = "pub(crate)")]
impl McpServer {
    #[tool(description = "Compress one byte block (from a file, or basm source assembled first) \
                           with every supported cruncher format and report the resulting size \
                           for each, sorted smallest-first. Read-only, in-process, no rebuild. \
                           All formats run concurrently on their own threads, so total wall time \
                           is roughly the slowest single format, not the sum - real crunchers can \
                           still take tens of seconds on real-sized data. Compares this block in \
                           isolation only - does not account for a project's real final linked \
                           size when a decruncher stub's own size differs per format, or when \
                           multiple crunch sites are selected independently. `pucrunch`/`shrinkler` \
                           are excluded from the default format list: both write debug/progress \
                           output straight to this server's real stdout on every call, confirmed \
                           live to corrupt the MCP protocol stream on real-sized data (small test \
                           inputs can misleadingly look clean) - pucrunch also hangs indefinitely \
                           on degenerate input. Request either explicitly via `formats` only if \
                           you understand the risk. A per-format 90s timeout applies to every \
                           format, to give legitimately slow (but finishing) crunchers like \
                           `upkr` room on larger real assets without waiting forever on a genuine \
                           hang.")]
    async fn compare_crunchers(
        &self,
        Parameters(input): Parameters<CompareCrunchersInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(compare_crunchers(input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_crunchers_requires_path_or_code() {
        let err = compare_crunchers(CompareCrunchersInput {
            path: None,
            code: None,
            formats: None
        })
        .expect_err("neither path nor code given should be an error");
        assert_eq!(err.kind, "invalid_input");
    }

    #[test]
    fn compare_crunchers_rejects_an_unknown_format() {
        let err = compare_crunchers(CompareCrunchersInput {
            path: None,
            code: Some("org 0x4000\n defs 64, 0\n".to_string()),
            formats: Some(vec!["not_a_real_cruncher".to_string()])
        })
        .expect_err("an unknown format name should be rejected");
        assert_eq!(err.kind, "invalid_input");
    }

    #[test]
    fn compare_crunchers_compresses_repetitive_assembled_bytes() {
        // A repeated real instruction sequence (real byte variety, not a
        // single repeated byte) - compresses well under any real format
        // without hitting the all-same-byte degenerate case confirmed to
        // hang `pucrunch` (see `compress_with_timeout_gives_up_instead_of_hanging`
        // below, and `CRUNCHER_TIMEOUT`'s own doc comment).
        let code = "org 0x4000\n REPEAT 32\n  ld a, 1\n  add a, 2\n  nop\n ENDR\n";
        let result = compare_crunchers(CompareCrunchersInput {
            path: None,
            code: Some(code.to_string()),
            formats: None
        })
        .expect("compressing a repeated real instruction sequence should succeed for every format");
        assert_eq!(result["uncrunched_size"], 160, "{result:#}");
        let results = result["results"].as_array().unwrap();
        assert_eq!(results.len(), ALL_FORMATS.len(), "{results:#?}");
        for r in results {
            assert_eq!(r["ok"], true, "{r:#}");
            let size = r["size"].as_u64().expect("size should be present on success");
            assert!(size > 0, "{r:#}");
        }
        // Sorted smallest-first.
        let sizes: Vec<u64> = results.iter().map(|r| r["size"].as_u64().unwrap()).collect();
        let mut sorted = sizes.clone();
        sorted.sort_unstable();
        assert_eq!(sizes, sorted, "{results:#?}");
    }

    #[test]
    fn compress_with_timeout_gives_up_instead_of_hanging() {
        // Regression test for a real, live-confirmed bug: `pucrunch`
        // hangs indefinitely ("Optimizing LZ77 and RLE lengths..." then
        // never returns) on a maximally-repetitive input (256 zero
        // bytes) - this used to hang `cargo test` itself for the better
        // part of two hours before being tracked down. A short timeout
        // here (not `CRUNCHER_TIMEOUT`'s real 90s) keeps this test fast
        // while still proving the bound actually fires.
        let start = std::time::Instant::now();
        let result = compress_with_timeout(
            "pucrunch".to_string(),
            vec![0u8; 256],
            Duration::from_millis(200)
        );
        assert!(start.elapsed() < Duration::from_secs(2), "{:?}", start.elapsed());
        let err = result.expect_err("pucrunch on 256 zero bytes should time out, not succeed");
        assert!(err.contains("timed out"), "{err}");
    }
}
