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
fn assemble_from(
    code: &str,
    source_path: &str,
    include_dirs: &[String],
    defines: &[String]
) -> Result<Vec<u8>, String> {
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
    let mut assemble = AssemblingOptions::default();
    cpclib_basmopt::apply_defines(&mut assemble, defines)?;
    let options = EnvOptions::new(parse, assemble, Arc::new(DiscardObserver));
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
    pub include_dirs: Option<Vec<String>>,
    /// Symbols to define before assembling, `NAME` (= 1) or `NAME=VALUE`,
    /// like `basm -D` (e.g. `LINKED_VERSION=1`).
    pub defines: Option<Vec<String>>
}

/// One instruction and the exact byte range it occupies in the source.
/// Working on byte ranges rather than whole lines is what lets this handle
/// `ld a,1 : ld b,2` (several statements on one line, this project family's
/// usual style) as well as one instruction per line - reordering swaps the
/// *texts* of the slots and leaves every separator, comment and line break
/// exactly where it was.
struct Slot<'t> {
    line: u32,
    start: usize,
    end: usize,
    token: &'t cpclib_asm::parser::obtained::LocatedToken
}

/// Whether `gap` (the source text between two consecutive instructions)
/// holds nothing but statement separators: whitespace, `:`, and `;`
/// comments. Anything else - a label, a directive, an include - means the
/// two instructions are not directly adjacent and no window may span them.
fn gap_is_only_separators(gap: &str) -> bool {
    gap.lines().all(|line| {
        line.split(';').next().unwrap_or("").chars().all(|c| c.is_whitespace() || c == ':')
    })
}

/// Every legal reordering found for one window, plus its scored outcome.
struct Candidate {
    start_line: u32,
    end_line: u32,
    /// The window's source lines in their new order (repeats when several
    /// instructions share a line).
    new_line_order: Vec<u32>,
    /// The window's instruction texts as they are, and as reordered.
    before: Vec<String>,
    after: Vec<String>,
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
    let defines = input.defines.clone().unwrap_or_default();

    let text = fs_err::read_to_string(&input.path)
        .map_err(|e| ToolError::io(format!("cannot read {}: {e}", input.path)))?;
    let listing = parse_z80(text.clone())
        .map_err(|e| ToolError::new(crate::error::ToolErrorKind::Assembler, e.to_string()))?;

    // One slot per instruction in range. A token is dropped (fail closed)
    // when its text cannot be pinned down exactly: it does not line up with
    // this file's own text (it came from an include), it contains a quote,
    // or several expanded tokens share one source position (a macro or
    // REPEAT body).
    let mut slots: Vec<Slot> = Vec::new();
    let mut starts_seen: HashMap<usize, usize> = HashMap::new();
    for token in cpclib_asmoptim::flatten_for_analysis(listing.iter()) {
        if !token.has_span() {
            continue;
        }
        let span = token.span();
        let (line_1, _col) = span.relative_line_and_column();
        let line = line_1 as u32;
        if line < input.start_line || line > input.end_line {
            continue;
        }
        let raw: &str = span.as_ref();
        let cut = raw.find([':', ';', '\n']).unwrap_or(raw.len());
        let trimmed = raw[..cut].trim_end();
        let start = span.offset_from_start();
        *starts_seen.entry(start).or_default() += 1;
        if trimmed.is_empty()
            || trimmed.contains(['"', '\''])
            || text.get(start..start + trimmed.len()) != Some(trimmed)
        {
            continue;
        }
        slots.push(Slot {
            line,
            start,
            end: start + trimmed.len(),
            token
        });
    }
    slots.retain(|s| starts_seen[&s.start] == 1);
    slots.sort_by_key(|s| s.start);

    // Maximal runs of directly adjacent instructions - see
    // `gap_is_only_separators`.
    let mut runs: Vec<Vec<&Slot>> = Vec::new();
    for slot in &slots {
        match runs.last_mut() {
            Some(run)
                if run.last().is_some_and(|prev| {
                    prev.end <= slot.start && gap_is_only_separators(&text[prev.end..slot.start])
                }) =>
            {
                run.push(slot)
            },
            _ => runs.push(vec![slot])
        }
    }

    let mut candidates: Vec<Candidate> = Vec::new();
    for run in &runs {
        for window_len in 2..=max_window.min(run.len()) {
            for start in 0..=(run.len() - window_len) {
                let window = &run[start..start + window_len];
                let token_refs: Vec<&cpclib_asm::parser::obtained::LocatedToken> =
                    window.iter().map(|e| e.token).collect();
                for perm in legal_reorderings(&token_refs) {
                    candidates.push(build_candidate(
                        &text,
                        window,
                        &perm,
                        &input.cruncher,
                        &input.path,
                        &include_dirs,
                        &defines
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
    let baseline_bytes = assemble_from(&text, &input.path, &include_dirs, &defines)
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
                        "before": c.before,
                        "after": c.after,
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
                        "before": c.before,
                        "after": c.after,
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

/// Builds one candidate: `text` with each of `window`'s slots given the text
/// of the instruction `perm` puts there (`perm[slot] = original index`),
/// then assembled and crunched.
fn build_candidate(
    text: &str,
    window: &[&Slot],
    perm: &[usize],
    cruncher: &str,
    source_path: &str,
    include_dirs: &[String],
    defines: &[String]
) -> Candidate {
    let texts: Vec<&str> = window.iter().map(|s| &text[s.start..s.end]).collect();
    let before: Vec<String> = texts.iter().map(|t| t.to_string()).collect();
    let after: Vec<String> = perm.iter().map(|&i| texts[i].to_string()).collect();
    let start_line = window.first().unwrap().line;
    let end_line = window.last().unwrap().line;
    let new_line_order: Vec<u32> = perm.iter().map(|&i| window[i].line).collect();

    // Replace from the last slot backward so earlier offsets stay valid.
    let mut candidate_text = text.to_string();
    for (slot, replacement) in window.iter().zip(&after).rev() {
        candidate_text.replace_range(slot.start..slot.end, replacement);
    }

    let outcome = assemble_from(&candidate_text, source_path, include_dirs, defines).and_then(|bytes| {
        compress_with_timeout(cruncher.to_string(), bytes, CRUNCHER_TIMEOUT)
            .map(|c| c.stream.len() as u64)
    });
    let (crunched_size, error) = match outcome {
        Ok(size) => (Some(size), None),
        Err(e) => (None, Some(e))
    };
    Candidate {
        start_line,
        end_line,
        new_line_order,
        before,
        after,
        crunched_size,
        error
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
                           file). Works at statement level: instructions on one line separated by `:` are \
                           reordered in place (separators and comments stay put), and a window may \
                           span line breaks and comments but never a label, directive or include; \
                           quoted operands and macro/REPEAT-expanded code are skipped, not guessed \
                           at. Each result lists the `before`/`after` instruction texts. `baseline_crunched_size` and every \
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
            include_dirs: None,
            defines: None
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
            include_dirs: None,
            defines: None
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
            include_dirs: None,
            defines: None
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
            include_dirs: None,
            defines: None
        })
        .expect_err("start_line > end_line should be rejected before touching the filesystem");
        assert_eq!(err.kind, "invalid_input");
    }

    /// Real style in etchy: several statements per line (`ld a,1 : ld b,2`).
    /// Line-based reordering used to drop every such line; slots must find
    /// the window, swap only the instruction texts, and keep separators and
    /// comments exactly where they were.
    #[test]
    fn colon_separated_statements_and_comments_between_them_are_reordered() {
        let dir = camino_tempfile::tempdir().unwrap();
        let path = dir.path().join("colon.asm");
        fs_err::write(&path, "org 0x8000\n ld a,1 : ld b,2 ; two\n ld c,3\n ret\n").unwrap();

        let result = search_reorderings(SearchReorderingsInput {
            path: path.to_string(),
            start_line: 2,
            end_line: 3,
            cruncher: "zx0".to_string(),
            window_size: Some(2),
            include_dirs: None,
            defines: None
        })
        .expect("search should succeed");
        let results = result["results"].as_array().unwrap();
        assert!(!results.is_empty(), "{result:#}");
        assert!(
            results.iter().any(|r| {
                r["before"] == json!(["ld a,1", "ld b,2"]) && r["after"] == json!(["ld b,2", "ld a,1"])
            }),
            "the same-line pair must be found and swapped in place: {result:#}"
        );
        assert!(
            results.iter().any(|r| r["before"] == json!(["ld b,2", "ld c,3"])),
            "a window spanning a line break and a comment must be found: {result:#}"
        );
    }

    #[test]
    fn a_directive_between_instructions_is_not_spanned() {
        assert!(gap_is_only_separators(" : \n ; comment\n  "));
        assert!(!gap_is_only_separators("\n label\n"));
        assert!(!gap_is_only_separators(" : db 1 : "));
    }
}
