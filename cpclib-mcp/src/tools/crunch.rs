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

use std::collections::HashMap;
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
/// Every format is in the default list. Two used to be excluded and no longer
/// need to be:
///
/// - `shrinkler` `printf`s progress straight to the real stdout, which corrupts
///   the MCP protocol stream. Fixed at the process level: `main.rs` moves the
///   protocol to a private file descriptor and points fd 1 at stderr (Unix; the
///   Windows equivalent is implemented the same way but untested).
/// - `pucrunch` hung forever on any input with a long run of one byte. Root
///   cause was in the vendored C, not the input: the FFI bypasses `main()`,
///   which is what initialised `maxrlelen`/`lrange`/`maxlzlen` and the
///   Elias-gamma length table, so `LenRle` never terminated for a run and LZ
///   packing was silently disabled for everything (output could even expand).
///   Fixed in `cpclib-crunchers/extra/pucrunch.c` (`pucrunch_ffi_init`).
///
/// The per-format [`CRUNCHER_TIMEOUT`] stays as a safety net for any other
/// native cruncher misbehaving.
const ALL_FORMATS: &[&str] = &[
    "apultra", "exomizer", "lz4", "lz48", "lz49", "lzsa1", "lzsa2", "pucrunch", "shrinkler", "upkr",
    "zx0", "zx0_backward", "zx7"
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
        // `log: false` trims two of Shrinkler's `printf` lines; the rest of
        // its progress output is handled process-wide (see `ALL_FORMATS`'s
        // doc comment), so this is only about less noise on stderr.
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
                "unknown cruncher '{other}' - expected one of: none, {}",
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
    let key = CacheKey::new(&name, &data);
    if let Some(hit) = cache_lookup(&key) {
        return Ok(hit);
    }
    let result = run_with_timeout(timeout, move || {
        match resolve_cruncher(&name) {
            Ok(method) => {
                method
                    .compress(&data)
                    .map_err(|CrunchersError::CompressionFailed| "compression failed".to_string())
            },
            Err(e) => Err(e.message)
        }
    });
    if let Ok(done) = &result {
        cache_store(key, done);
    }
    result
}

/// What a crunch was asked to do: cruncher name and content. Searches
/// (`search_reorderings`, `measure_variants`' proxy, `size_map`'s
/// leave-one-out) keep asking for the same block - every candidate that
/// leaves a region alone, every baseline - and a Shrinkler run is seconds.
/// The content is identified by its length plus two independent 64-bit
/// hashes rather than kept, so the cache stays small.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    name: String,
    len: usize,
    hashes: (u64, u64)
}

impl CacheKey {
    fn new(name: &str, data: &[u8]) -> Self {
        use std::hash::{Hash, Hasher};
        let mut a = std::collections::hash_map::DefaultHasher::new();
        data.hash(&mut a);
        let mut b = std::collections::hash_map::DefaultHasher::new();
        0x9e37_79b9_7f4a_7c15u64.hash(&mut b);
        data.hash(&mut b);
        Self { name: name.to_string(), len: data.len(), hashes: (a.finish(), b.finish()) }
    }
}

const CACHE_LIMIT: usize = 512;

fn cache() -> &'static std::sync::Mutex<HashMap<CacheKey, cpclib_crunchers::CompressionResult>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<HashMap<CacheKey, cpclib_crunchers::CompressionResult>>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default)
}

fn cache_lookup(key: &CacheKey) -> Option<cpclib_crunchers::CompressionResult> {
    cache().lock().ok()?.get(key).cloned()
}

fn cache_store(key: CacheKey, result: &cpclib_crunchers::CompressionResult) {
    if let Ok(mut cache) = cache().lock() {
        if cache.len() >= CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, result.clone());
    }
}

/// Runs `work` on its own thread and waits up to `timeout` for its result;
/// on expiry the thread is leaked (unavoidable in safe Rust) and a clear
/// timeout error is returned instead. Generic so the bound itself can be
/// tested without needing a cruncher that really hangs.
fn run_with_timeout<T: Send + 'static>(
    timeout: Duration,
    work: impl FnOnce() -> Result<T, String> + Send + 'static
) -> Result<T, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        // The receiver may already be gone if `recv_timeout` below already
        // gave up - that's fine, nothing left to deliver to.
        let _ = tx.send(work());
    });
    rx.recv_timeout(timeout).unwrap_or_else(|_| {
        Err(format!(
            "timed out after {timeout:?} - this cruncher did not finish in time (a genuine hang, \
             or simply more work than the bound allows for this input)"
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
                           multiple crunch sites are selected independently. A per-format 90s timeout \
                           applies to every format, to give legitimately slow (but finishing) \
                           crunchers like `upkr` room on larger real assets without waiting \
                           forever on a genuine hang.")]
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
    fn an_identical_crunch_is_answered_from_the_cache() {
        let data: Vec<u8> = (0..2000u32).map(|i| (i * 7 % 251) as u8).collect();
        let first = compress_with_timeout("zx0".into(), data.clone(), CRUNCHER_TIMEOUT).unwrap();
        assert!(cache_lookup(&CacheKey::new("zx0", &data)).is_some());
        let second = compress_with_timeout("zx0".into(), data.clone(), CRUNCHER_TIMEOUT).unwrap();
        assert_eq!(first.stream, second.stream);
        // another cruncher or other content is a different entry
        assert!(cache_lookup(&CacheKey::new("zx7", &data)).is_none());
        let mut other = data;
        other[0] ^= 1;
        assert!(cache_lookup(&CacheKey::new("zx0", &other)).is_none());
    }


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

    /// The bound must actually fire. (This used to be tested with pucrunch
    /// on 256 zero bytes, which hung forever until its uninitialised
    /// `maxrlelen` was fixed - see `cpclib-crunchers/extra/pucrunch.c`.)
    #[test]
    fn run_with_timeout_gives_up_instead_of_hanging() {
        let start = std::time::Instant::now();
        let result: Result<u32, String> = run_with_timeout(Duration::from_millis(200), || {
            std::thread::sleep(Duration::from_secs(30));
            Ok(1)
        });
        assert!(start.elapsed() < Duration::from_secs(2), "{:?}", start.elapsed());
        assert!(result.unwrap_err().contains("timed out"));
    }

    #[test]
    fn run_with_timeout_returns_the_result_when_in_time() {
        assert_eq!(run_with_timeout(Duration::from_secs(5), || Ok::<_, String>(7)), Ok(7));
    }

    /// Regression for the real bug: a long run of one byte used to hang
    /// pucrunch forever.
    #[test]
    fn pucrunch_finishes_on_a_maximally_repetitive_block() {
        let result = compress_with_timeout("pucrunch".to_string(), vec![0u8; 256], Duration::from_secs(20))
            .expect("pucrunch must finish, not time out");
        assert!(result.stream.len() < 32, "{}", result.stream.len());
    }
}
