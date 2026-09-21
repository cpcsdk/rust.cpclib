//! `search_reorderings` - crunch-friendly instruction-order search over a
//! caller-given line range: enumerates every legal reordering of small
//! adjacent-instruction windows (via `cpclib_asmoptim::reorder`, which
//! does the actual safety analysis - see that module's own doc comment for
//! why this is a strictly local question, answerable without a CFG walk),
//! rebuilds each candidate, and reports the crunched-size delta versus the
//! original, best first.
//!
//! Replaces the workflow real feedback described: a hand-rolled Python
//! harness that only understood `ld`-pairs, parsing register read/write
//! sets by hand and re-invoking `basm` as a subprocess per candidate. This
//! is general across arbitrary Z80 instructions (reusing the same
//! register/flag-effects table the shipped peephole optimizer already
//! uses) and stays in-process for every candidate (`cpclib_asm::assemble`
//! + `CompressMethod::compress`, both plain library calls - no subprocess
//! spawn per permutation).

use std::collections::HashMap;
use std::sync::Arc;

use cpclib_asm::assembler::EnvOptions;
use cpclib_asm::parser::context::ParserOptions;
use cpclib_asm::{AssemblingOptions, MayHaveSpan, assemble_with_options, parse_z80};
use cpclib_asmoptim::reorder::{MAX_WINDOW, legal_reorderings};
use cpclib_common::event::DiscardObserver;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::{Value, json};

use crate::McpServer;
use crate::error::{ToolError, ToolResult};
use crate::tools::crunch::{CRUNCHER_TIMEOUT, compress_with_timeout, resolve_cruncher};

/// `cpclib_asm::assemble()` alone has no notion of where `code` came from,
/// so a real project's relative `include`s (`include once
/// "../src/demosystem/common.asm"`, confirmed live against `skyline`) fail
/// to resolve - this adds `source_path`'s own directory as a search path
/// first, exactly like `cpclib-lsp`'s own `dry_run_env` does for the same
/// reason (`ParserOptions::add_search_path_from_file`).
fn assemble_from(code: &str, source_path: &str, include_dirs: &[String]) -> Result<Vec<u8>, String> {
    let mut parse = ParserOptions::default();
    parse
        .add_search_path_from_file(source_path)
        .map_err(|e| format!("cannot resolve includes relative to {source_path}: {e}"))?;
    // Projects often include relative to the build's working directory, not
    // the including file's own (skyline: `include "files/x.cfx"` from
    // `main/src/`, resolved against `main/`), so callers can add more.
    for dir in include_dirs {
        parse
            .add_search_path(dir)
            .map_err(|e| format!("cannot add include dir {dir}: {e}"))?;
    }
    let options = EnvOptions::new(parse, AssemblingOptions::default(), Arc::new(DiscardObserver));
    assemble_with_options(code, options)
        .map(|(bytes, _symbols)| bytes)
        .map_err(|e| e.to_string())
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchReorderingsInput {
    /// Path to the `.asm` file to search.
    pub path: String,
    /// 1-based, inclusive first line of the "safe" range - never touch
    /// anything outside it (no interrupt-timing/cycle-exact code should be
    /// included).
    pub start_line: u32,
    /// 1-based, inclusive last line of the safe range.
    pub end_line: u32,
    /// Cruncher format to score candidates with (any name
    /// `compare_crunchers` accepts, including `pucrunch` - the same
    /// stdout/hang caveats apply, see that tool's own description).
    pub cruncher: String,
    /// Widest adjacent-instruction window to search, 2-4. Defaults to 4.
    /// Every window size from 2 up to this value is tried at every
    /// position in the range.
    pub window_size: Option<usize>,
    /// Extra `INCLUDE` search directories, for projects whose includes
    /// resolve relative to the build's working directory rather than the
    /// file's own (same meaning as `suggest_optimizations`' field).
    pub include_dirs: Option<Vec<String>>
}

/// One `(line, token)` entry only kept when its line held **exactly one**
/// token - a line with a label plus an instruction, several colon-
/// separated instructions, or anything else ambiguous is dropped rather
/// than guessed at, matching every other "fail closed" policy already
/// established across this crate's static-analysis tools.
struct LineEntry<'t> {
    line: u32,
    token: &'t cpclib_asm::parser::obtained::LocatedToken
}

/// Every legal reordering found for one window, plus its scored outcome.
struct Candidate {
    start_line: u32,
    end_line: u32,
    /// The window's original lines, in their new order (e.g. `[3, 2]` for
    /// a 2-line window at lines 2-3 with the order flipped).
    new_line_order: Vec<u32>,
    crunched_size: Option<u64>,
    error: Option<String>
}

pub(crate) fn search_reorderings(input: SearchReorderingsInput) -> ToolResult {
    if input.start_line == 0 || input.end_line == 0 {
        return Err(ToolError::invalid_input("start_line/end_line are 1-based, not 0"));
    }
    if input.start_line > input.end_line {
        return Err(ToolError::invalid_input("start_line must be <= end_line"));
    }
    let max_window = input.window_size.unwrap_or(MAX_WINDOW);
    if !(2..=MAX_WINDOW).contains(&max_window) {
        return Err(ToolError::invalid_input(format!(
            "window_size must be between 2 and {MAX_WINDOW}"
        )));
    }
    // Validated once up front, exactly like `compare_crunchers` - fail the
    // whole call on a typo rather than reporting it per-candidate.
    resolve_cruncher(&input.cruncher)?;
    let include_dirs = input.include_dirs.clone().unwrap_or_default();

    let text = fs_err::read_to_string(&input.path)
        .map_err(|e| ToolError::io(format!("cannot read {}: {e}", input.path)))?;
    let lines: Vec<&str> = text.lines().collect();

    let listing = parse_z80(text.clone())
        .map_err(|e| ToolError::new(crate::error::ToolErrorKind::Assembler, e.to_string()))?;

    // One token per line, within range, dropping any line that isn't
    // exactly one token - see `LineEntry`'s own doc comment.
    let mut per_line: HashMap<u32, Vec<&cpclib_asm::parser::obtained::LocatedToken>> = HashMap::new();
    for token in cpclib_asmoptim::flatten_for_analysis(listing.iter()) {
        if !token.has_span() {
            continue;
        }
        let (line_1, _col) = token.span().relative_line_and_column();
        let line = line_1 as u32;
        if line < input.start_line || line > input.end_line {
            continue;
        }
        per_line.entry(line).or_default().push(token);
    }
    let mut entries: Vec<LineEntry> = per_line
        .into_iter()
        .filter_map(|(line, tokens)| (tokens.len() == 1).then(|| LineEntry { line, token: tokens[0] }))
        .collect();
    entries.sort_by_key(|e| e.line);

    // Maximal runs of consecutive line numbers - a window can't span a
    // dropped or out-of-range line.
    let mut runs: Vec<Vec<&LineEntry>> = Vec::new();
    for entry in &entries {
        match runs.last_mut() {
            Some(run) if run.last().unwrap().line + 1 == entry.line => run.push(entry),
            _ => runs.push(vec![entry])
        }
    }

    let mut candidates: Vec<Candidate> = Vec::new();
    for run in &runs {
        for window_len in 2..=max_window.min(run.len()) {
            for start in 0..=(run.len() - window_len) {
                let window = &run[start..start + window_len];
                let token_refs: Vec<&cpclib_asm::parser::obtained::LocatedToken> =
                    window.iter().map(|e| e.token).collect();
                let permutations = legal_reorderings(&token_refs);
                let window_lines: Vec<u32> = window.iter().map(|e| e.line).collect();
                for perm in permutations {
                    let new_line_order: Vec<u32> = perm.iter().map(|&i| window_lines[i]).collect();
                    candidates.push(build_candidate(
                        &lines,
                        &window_lines,
                        &new_line_order,
                        &input.cruncher,
                        &input.path,
                        &include_dirs
                    ));
                }
            }
        }
    }

    // The baseline: the file exactly as it stands, crunched the same way,
    // for every candidate's delta to be measured against. Payload-only, like
    // every candidate's own size below - real feedback measured a 3-byte gap
    // against a real project's actual linked size (1778 here vs 1781 built),
    // traced to ZX0-backward's own overlap delta bytes, added at link time,
    // which this isolated compress-the-bytes call never sees. The relative
    // ranking between candidates stays valid either way; only the absolute
    // numbers can be off by whatever a project's own link step adds.
    let baseline_bytes = assemble_from(&text, &input.path, &include_dirs)
        .map_err(|e| ToolError::new(crate::error::ToolErrorKind::Assembler, e))?;
    let baseline_size = match compress_with_timeout(input.cruncher.clone(), baseline_bytes, CRUNCHER_TIMEOUT) {
        Ok(compressed) => compressed.stream.len() as i64,
        Err(e) => return Err(ToolError::new(crate::error::ToolErrorKind::InvalidInput, e))
    };

    let mut results: Vec<Value> = candidates
        .iter()
        .map(|c| {
            match (c.crunched_size, &c.error) {
                (Some(size), None) => {
                    json!({
                        "start_line": c.start_line,
                        "end_line": c.end_line,
                        "new_line_order": c.new_line_order,
                        "crunched_size": size,
                        "delta": baseline_size - size as i64,
                        "ok": true
                    })
                },
                _ => {
                    json!({
                        "start_line": c.start_line,
                        "end_line": c.end_line,
                        "new_line_order": c.new_line_order,
                        "ok": false,
                        "error": c.error
                    })
                }
            }
        })
        .collect();
    // Best (largest size reduction) first.
    results.sort_by_key(|r| -(r["delta"].as_i64().unwrap_or(i64::MIN)));

    Ok(json!({
        "path": input.path,
        "baseline_crunched_size": baseline_size,
        "windows_considered": runs.iter().map(|r| r.len().saturating_sub(1)).sum::<usize>(),
        "legal_reorderings_found": results.len(),
        "results": results
    }))
}

/// Builds one candidate: the original source with `window_lines` replaced,
/// in order, by `new_line_order`'s text, then assembled and crunched.
fn build_candidate(
    lines: &[&str],
    window_lines: &[u32],
    new_line_order: &[u32],
    cruncher: &str,
    source_path: &str,
    include_dirs: &[String]
) -> Candidate {
    let start_line = *window_lines.first().unwrap();
    let end_line = *window_lines.last().unwrap();

    let mut rebuilt: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
    for (slot, &source_line) in window_lines.iter().zip(new_line_order) {
        // Both 1-based; `lines` is 0-indexed.
        rebuilt[(*slot - 1) as usize] = lines[(source_line - 1) as usize].to_string();
    }
    let candidate_text = rebuilt.join("\n");

    match assemble_from(&candidate_text, source_path, include_dirs) {
        Ok(bytes) => {
            match compress_with_timeout(cruncher.to_string(), bytes, CRUNCHER_TIMEOUT) {
                Ok(compressed) => {
                    Candidate {
                        start_line,
                        end_line,
                        new_line_order: new_line_order.to_vec(),
                        crunched_size: Some(compressed.stream.len() as u64),
                        error: None
                    }
                },
                Err(e) => {
                    Candidate {
                        start_line,
                        end_line,
                        new_line_order: new_line_order.to_vec(),
                        crunched_size: None,
                        error: Some(e)
                    }
                }
            }
        },
        Err(e) => {
            Candidate {
                start_line,
                end_line,
                new_line_order: new_line_order.to_vec(),
                crunched_size: None,
                error: Some(e)
            }
        }
    }
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Value>, Json<Value>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = reorder_router, vis = "pub(crate)")]
impl McpServer {
    #[tool(description = "Search a line range for crunch-friendly instruction reorderings: \
                           finds every legal permutation of small adjacent-instruction windows \
                           (2-4 instructions, register/flag-dependency-safe - never reorders \
                           across a real data dependency, and never across or including a \
                           branch/call/ret/djnz/rst/halt, a hard barrier regardless of \
                           dependencies), rebuilds each candidate, and reports the crunched-size \
                           delta versus the original, best first. Read-only (never writes the \
                           file). Only considers lines that parse as exactly one plain \
                           instruction - labels, directives, and multi-statement lines are \
                           skipped, not guessed at. `baseline_crunched_size` and every \
                           `crunched_size`/`delta` are payload-only (this file's bytes crunched \
                           in isolation) - they can differ slightly from a real project's final \
                           linked size when the chosen cruncher adds its own extra bytes at link \
                           time (e.g. ZX0-backward's overlap delta bytes); treat the ranking \
                           between candidates as reliable, not the absolute numbers.")]
    async fn search_reorderings(
        &self,
        Parameters(input): Parameters<SearchReorderingsInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(search_reorderings(input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_independent_instructions_yield_legal_reorderings_with_a_measured_delta() {
        let dir = camino_tempfile::tempdir().unwrap();
        let path = dir.path().join("indep.asm");
        // Lines 1-5 (1-based): org, three mutually-independent `ld`s, ret.
        fs_err::write(&path, "org 0x8000\n ld a,1\n ld b,2\n ld c,3\n ret\n").unwrap();

        let result = search_reorderings(SearchReorderingsInput {
            path: path.to_string(),
            start_line: 2,
            end_line: 4,
            cruncher: "lz4".to_string(),
            window_size: None,
            include_dirs: None
        })
        .expect("three independent instructions should search cleanly");

        assert!(result["baseline_crunched_size"].as_i64().unwrap() > 0, "{result:#}");
        let found = result["legal_reorderings_found"].as_u64().unwrap();
        // 2-instruction windows at (2,3) and (3,4), each with 1 legal swap,
        // plus the 3-instruction window (2,3,4) with 5 legal permutations
        // (see cpclib-asmoptim's own equivalent test) = 7.
        assert_eq!(found, 7, "{result:#}");
        let results = result["results"].as_array().unwrap();
        for r in results {
            assert_eq!(r["ok"], true, "{r:#}");
            assert!(r["crunched_size"].as_i64().unwrap() > 0, "{r:#}");
        }
    }

    #[test]
    fn a_true_dependency_in_range_yields_no_reorderings() {
        let dir = camino_tempfile::tempdir().unwrap();
        let path = dir.path().join("dep.asm");
        // `ld b,a` reads what `ld a,1` just wrote - no legal reordering.
        fs_err::write(&path, "org 0x8000\n ld a,1\n ld b,a\n ret\n").unwrap();

        let result = search_reorderings(SearchReorderingsInput {
            path: path.to_string(),
            start_line: 2,
            end_line: 3,
            cruncher: "lz4".to_string(),
            window_size: None,
            include_dirs: None
        })
        .expect("a dependent pair should still search cleanly, just find nothing");
        assert_eq!(result["legal_reorderings_found"], 0, "{result:#}");
    }

    #[test]
    fn an_unknown_cruncher_is_rejected_up_front() {
        let dir = camino_tempfile::tempdir().unwrap();
        let path = dir.path().join("x.asm");
        fs_err::write(&path, "org 0x8000\n ld a,1\n ld b,2\n ret\n").unwrap();

        let err = search_reorderings(SearchReorderingsInput {
            path: path.to_string(),
            start_line: 2,
            end_line: 3,
            cruncher: "not_a_real_cruncher".to_string(),
            window_size: None,
            include_dirs: None
        })
        .expect_err("an unknown cruncher name should be rejected");
        assert_eq!(err.kind, "invalid_input");
    }

    #[test]
    fn start_line_must_not_exceed_end_line() {
        let err = search_reorderings(SearchReorderingsInput {
            path: "/nonexistent".to_string(),
            start_line: 5,
            end_line: 2,
            cruncher: "lz4".to_string(),
            window_size: None,
            include_dirs: None
        })
        .expect_err("start_line > end_line should be rejected before touching the filesystem");
        assert_eq!(err.kind, "invalid_input");
    }
}
