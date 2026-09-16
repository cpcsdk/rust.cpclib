//! `assemble_check` and `count_cycles` - both ride `cpclib_lsp::basm::AssemblyAnalyzer`
//! rather than calling `cpclib-asm`/`cpclib-z80flow` directly: the analyzer
//! already does a real multi-pass assemble with parse-error recovery
//! (`assemble_check`) and a control-flow-aware min/max NOP count over a
//! line range via `cpclib_z80flow::cost_range` (`count_cycles`) - reusing
//! it means both tools get the LSP's own battle-tested behavior (and its
//! whole-project assemble/include-graph caches) for free, rather than a
//! second, divergent implementation here.

use cpclib_lsp::basm::AssemblyAnalyzer;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::Value;
use tower_lsp::lsp_types::{Position, Range};

use crate::McpServer;
use crate::error::{ToolError, ToolErrorKind, ToolResult};
use crate::tools::common::document_from_input;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AssembleCheckInput {
    /// Path to a `.asm`/`.z80` file on disk. Used both as the file to check
    /// (when `code` is omitted) and, when `code` is given too, purely as
    /// the file identity for `INCLUDE` resolution while checking `code`'s
    /// text instead of the file's on-disk content.
    pub path: Option<String>,
    /// Inline source to check instead of (or in place of) the file at
    /// `path`. At least one of `path`/`code` is required.
    pub code: Option<String>
}

/// Structured diagnostics from a real assemble (parse errors, with
/// resumption so more than just the first syntax error is reported; plus
/// assembler warnings once the file parses cleanly).
pub(crate) fn assemble_check(analyzer: &AssemblyAnalyzer, input: AssembleCheckInput) -> ToolResult {
    let document = document_from_input(input.path.as_deref(), input.code.as_deref())?;
    let diagnostics = analyzer.analyze(&document);
    Ok(serde_json::json!({
        "ok": diagnostics.iter().all(|d| d.severity != Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR)),
        "diagnostic_count": diagnostics.len(),
        "diagnostics": diagnostics
    }))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CountCyclesInput {
    /// Path to a `.asm`/`.z80` file on disk.
    pub path: Option<String>,
    /// Inline source instead of (or alongside, for `INCLUDE` identity) the
    /// file at `path`. At least one of `path`/`code` is required.
    pub code: Option<String>,
    /// 1-based, inclusive first line to count.
    pub start_line: u32,
    /// 1-based, inclusive last line to count.
    pub end_line: u32
}

/// Control-flow-aware NOP-count summary (min/max real-path cost, not a
/// naive per-line sum) for a line range - see
/// `cpclib_lsp::basm::cycles::SelectionCycleCount`'s own field docs for
/// exactly what `min_nops`/`max_nops`/`max_unbounded`/etc. mean.
pub(crate) fn count_cycles(analyzer: &AssemblyAnalyzer, input: CountCyclesInput) -> ToolResult {
    let document = document_from_input(input.path.as_deref(), input.code.as_deref())?;
    if input.start_line == 0 || input.end_line == 0 {
        return Err(ToolError::invalid_input(
            "start_line/end_line are 1-based and must be >= 1"
        ));
    }
    if input.end_line < input.start_line {
        return Err(ToolError::invalid_input("end_line must be >= start_line"));
    }
    // `character: u32::MAX` on the end position (rather than 0) keeps
    // `end_line` inclusive as given - see `line_range_from_selection`'s own
    // doc comment: an end character of exactly 0 on a later line is
    // interpreted as "one past the real last line" (a plain editor
    // selection's own convention), which isn't what a 1-based inclusive
    // `end_line` input means here.
    let range = Range {
        start: Position {
            line: input.start_line - 1,
            character: 0
        },
        end: Position {
            line: input.end_line - 1,
            character: u32::MAX
        }
    };
    let summary = analyzer
        .cycle_count_for_selection(&document, range)
        .ok_or_else(|| {
            ToolError::new(
                ToolErrorKind::Assembler,
                "could not compute a cycle count for this range - the file may not parse, or the \
                 range may contain no recognized instructions"
            )
        })?;
    serde_json::to_value(summary).map_err(|e| ToolError::io(e.to_string()))
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Value>, Json<Value>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = asm_router, vis = "pub(crate)")]
impl McpServer {
    /// Assemble a Z80/basm source (real, full assemble) and return
    /// structured diagnostics - parse errors (with best-effort recovery so
    /// more than the first one is reported) and assembler warnings.
    /// Read-only.
    #[tool(description = "Assemble a basm/Z80 source and return structured diagnostics (errors \
                           and warnings). Read-only.")]
    async fn assemble_check(
        &self,
        Parameters(input): Parameters<AssembleCheckInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(assemble_check(&self.analyzer, input))
    }

    /// Control-flow-aware min/max NOP-count (Z80 timing) for a line range,
    /// via the same engine as the LSP's status-bar cycle counter. Read-only.
    #[tool(description = "Report a control-flow-aware min/max NOP-count (Z80 instruction timing) \
                           for a 1-based inclusive line range of a basm source. Read-only.")]
    async fn count_cycles(
        &self,
        Parameters(input): Parameters<CountCyclesInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(count_cycles(&self.analyzer, input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assemble_check_reports_a_clean_file_as_ok() {
        let analyzer = AssemblyAnalyzer::new();
        let result = assemble_check(&analyzer, AssembleCheckInput {
            path: None,
            code: Some("org 0x4000\n ld a, 1\n ret\n".to_string())
        })
        .expect("assemble_check itself should not fail on valid input");
        assert_eq!(result["ok"], true, "{result:#}");
        assert_eq!(result["diagnostic_count"], 0, "{result:#}");
    }

    #[test]
    fn assemble_check_reports_a_syntax_error() {
        let analyzer = AssemblyAnalyzer::new();
        let result = assemble_check(&analyzer, AssembleCheckInput {
            path: None,
            code: Some("org 0x4000\n@#$ garbage @#$\n ret\n".to_string())
        })
        .expect("assemble_check itself should not fail even on invalid asm source");
        assert_eq!(result["ok"], false, "{result:#}");
        let count = result["diagnostic_count"].as_u64().unwrap();
        assert!(count >= 1, "{result:#}");
        let diagnostics = result["diagnostics"].as_array().unwrap();
        assert_eq!(diagnostics.len(), count as usize);
    }

    #[test]
    fn assemble_check_requires_path_or_code() {
        let analyzer = AssemblyAnalyzer::new();
        let err = assemble_check(&analyzer, AssembleCheckInput {
            path: None,
            code: None
        })
        .expect_err("neither path nor code given should be an error");
        assert_eq!(err.kind, "invalid_input");
    }

    #[test]
    fn count_cycles_reports_a_known_sequence() {
        let analyzer = AssemblyAnalyzer::new();
        let code = "org 0x4000\n ld a, 1\n nop\n nop\n ret\n";
        let result = count_cycles(&analyzer, CountCyclesInput {
            path: None,
            code: Some(code.to_string()),
            start_line: 2,
            end_line: 4
        })
        .expect("count_cycles should succeed on a known-good sequence");
        assert_eq!(result["instruction_count"], 3, "{result:#}");
        let min_nops = result["min_nops"].as_u64().unwrap();
        assert!(min_nops > 0, "{result:#}");
    }

    #[test]
    fn count_cycles_rejects_zero_based_lines() {
        let analyzer = AssemblyAnalyzer::new();
        let err = count_cycles(&analyzer, CountCyclesInput {
            path: None,
            code: Some("nop\n".to_string()),
            start_line: 0,
            end_line: 1
        })
        .expect_err("start_line: 0 should be rejected as invalid input");
        assert_eq!(err.kind, "invalid_input");
    }
}
