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
//!
//! The actual cruncher resolution/bounding/caching (`resolve_cruncher`,
//! `compress_with_timeout`, `CRUNCHER_TIMEOUT`) lives in `cpclib_crunch`,
//! shared with that crate's own CLI and with `cpclib-mcp`'s other crunch-
//! using tools (`search_reorderings`, `size_map`); this file only adds the
//! MCP-specific error type and the `compare_crunchers` tool itself.

use cpclib_crunch::resolve::ALL_FORMATS;
pub(crate) use cpclib_crunch::resolve::{CRUNCHER_TIMEOUT, compress_with_timeout};
use cpclib_crunchers::CompressMethod;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::{Value, json};

use crate::McpServer;
use crate::error::{ToolError, ToolResult};

/// [`cpclib_crunch::resolve::resolve_cruncher`], with its `String` error
/// turned into this crate's [`ToolError`] - every other tool in this crate
/// calls this one (not the `cpclib_crunch` function directly) so a bad
/// cruncher name always fails the same way.
pub(crate) fn resolve_cruncher(name: &str) -> Result<CompressMethod, ToolError> {
    cpclib_crunch::resolve::resolve_cruncher(name).map_err(ToolError::invalid_input)
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
        // without hitting the all-same-byte degenerate case that used to
        // hang `pucrunch` (see `cpclib_crunch::resolve`'s own tests, and
        // `CRUNCHER_TIMEOUT`'s doc comment there).
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
}
