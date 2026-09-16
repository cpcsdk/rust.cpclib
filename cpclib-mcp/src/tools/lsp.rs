//! `hover` and `goto_definition` - thin wrappers over
//! `cpclib_lsp::basm::AssemblyAnalyzer`'s own LSP-feature implementations
//! (instruction timing / resolved `EQU`/label values on hover; cross-file
//! macro/label resolution over `INCLUDE`s for goto-definition), reusing the
//! shared long-lived analyzer instance so its whole-project caches carry
//! over between calls.

use cpclib_lsp::basm::AssemblyAnalyzer;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::Value;
use tower_lsp::lsp_types::Position;

use crate::McpServer;
use crate::error::ToolResult;
use crate::tools::common::document_from_input;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PositionInput {
    /// Path to a `.asm`/`.z80` file on disk.
    pub path: Option<String>,
    /// Inline source instead of (or alongside, for `INCLUDE` identity/
    /// project-graph resolution) the file at `path`. At least one of
    /// `path`/`code` is required.
    pub code: Option<String>,
    /// 1-based line.
    pub line: u32,
    /// 1-based column.
    pub column: u32
}

fn position_from_input(line: u32, column: u32) -> Result<Position, crate::error::ToolError> {
    if line == 0 || column == 0 {
        return Err(crate::error::ToolError::invalid_input(
            "line/column are 1-based and must be >= 1"
        ));
    }
    Ok(Position {
        line: line - 1,
        character: column - 1
    })
}

/// Hover info (instruction timing, resolved `EQU`/label values, firmware
/// routine docs, ...) at a position. Read-only.
pub(crate) fn hover(analyzer: &AssemblyAnalyzer, input: PositionInput) -> ToolResult {
    let document = document_from_input(input.path.as_deref(), input.code.as_deref())?;
    let position = position_from_input(input.line, input.column)?;
    let hover = analyzer.hover(&document, position);
    Ok(serde_json::json!({ "hover": hover }))
}

/// Cross-file goto-definition (macros/labels, following real `INCLUDE`s) at
/// a position. Read-only.
pub(crate) fn goto_definition(analyzer: &AssemblyAnalyzer, input: PositionInput) -> ToolResult {
    let document = document_from_input(input.path.as_deref(), input.code.as_deref())?;
    let position = position_from_input(input.line, input.column)?;
    let location = analyzer.goto_definition(&document, position);
    Ok(serde_json::json!({ "location": location }))
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Value>, Json<Value>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = lsp_router, vis = "pub(crate)")]
impl McpServer {
    #[tool(description = "Hover info at a 1-based line/column: instruction timing, resolved \
                           EQU/label values, firmware routine docs. Read-only.")]
    async fn hover(
        &self,
        Parameters(input): Parameters<PositionInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(hover(&self.analyzer, input))
    }

    #[tool(description = "Cross-file goto-definition (macros/labels, following real INCLUDEs) at \
                           a 1-based line/column. Read-only.")]
    async fn goto_definition(
        &self,
        Parameters(input): Parameters<PositionInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(goto_definition(&self.analyzer, input))
    }
}
