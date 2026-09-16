//! `suggest_optimizations` / `apply_optimizations` - `cpclib-basmopt`'s
//! peephole-optimization advisor, structured (not a text diff): each
//! `Suggestion` carries the matched rule, why it's believed safe
//! (`SuggestionReason`s with their own source positions), and whether it's
//! safe to bulk-apply.

use camino::Utf8Path;
use cpclib_basmopt::{AnalyzeOutcome, OptimizationGoal, Options, Suggestion, SuggestionReason};
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::{Value, json};

use crate::McpServer;
use crate::error::{ToolError, ToolErrorKind, ToolResult};

fn parse_goal(goal: Option<&str>) -> Result<OptimizationGoal, ToolError> {
    match goal.map(str::to_ascii_lowercase).as_deref() {
        None => Ok(OptimizationGoal::default()),
        Some("neutral") => Ok(OptimizationGoal::Neutral),
        Some("size") => Ok(OptimizationGoal::Size),
        Some("speed") => Ok(OptimizationGoal::Speed),
        Some(other) => {
            Err(ToolError::invalid_input(format!(
                "unknown goal '{other}' - expected one of: neutral, size, speed"
            )))
        }
    }
}

fn reason_to_json(r: &SuggestionReason) -> Value {
    json!({ "text": r.text, "line": r.line, "column": r.column })
}

fn suggestion_to_json(s: &Suggestion) -> Value {
    json!({
        "line": s.line,
        "column": s.column,
        "rule_name": s.rule_name,
        "bulk_unsafe": s.bulk_unsafe,
        "message": s.message,
        "replacement": s.replacement,
        "reasons": s.reasons.iter().map(reason_to_json).collect::<Vec<_>>()
    })
}

fn outcome_to_json(o: &AnalyzeOutcome) -> Value {
    json!({
        "suggestion_count": o.suggestions.len(),
        "suggestions": o.suggestions.iter().map(suggestion_to_json).collect::<Vec<_>>(),
        "assemble_warning": o.assemble_warning
    })
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SuggestInput {
    /// Path to the `.asm` file to analyze.
    pub path: String,
    /// Optimization goal: "neutral" (default, only unconditional wins),
    /// "size", or "speed".
    pub goal: Option<String>,
    /// Rule names (a rule's `name:` line) to skip.
    pub disabled_rules: Option<Vec<String>>,
    /// Extra `INCLUDE` search directories, for rules that need a real
    /// assemble (currently only `jp2jr`) to resolve addresses.
    pub include_dirs: Option<Vec<String>>
}

/// Peephole-optimization suggestions for a source file - each with the
/// matched rule, why it's believed safe, and whether it's bulk-safe to
/// apply automatically. Read-only.
pub(crate) fn suggest_optimizations(input: SuggestInput) -> ToolResult {
    let path = Utf8Path::new(&input.path);
    let options = Options {
        goal: parse_goal(input.goal.as_deref())?,
        disabled_rules: input.disabled_rules.unwrap_or_default(),
        include_dirs: input
            .include_dirs
            .unwrap_or_default()
            .into_iter()
            .map(camino::Utf8PathBuf::from)
            .collect(),
        ..Default::default()
    };
    let outcome = cpclib_basmopt::analyze_file(path, &options).map_err(|e| {
        ToolError::with_details(
            ToolErrorKind::Assembler,
            e.to_string(),
            json!({ "path": input.path })
        )
    })?;
    Ok(outcome_to_json(&outcome))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ApplyInput {
    /// Path to the `.asm` file to analyze and rewrite.
    pub path: String,
    pub goal: Option<String>,
    pub disabled_rules: Option<Vec<String>>,
    pub include_dirs: Option<Vec<String>>,
    /// Must be `true` - this tool rewrites `path` in place (up to 2 passes,
    /// bulk-safe suggestions only). There is no dry-run output distinct
    /// from `suggest_optimizations`; ask that first if you only want to see
    /// what would change.
    pub in_place: bool
}

/// Rewrites `path` in place with every bulk-safe suggestion applied
/// (up to 2 passes). **Mutating** - requires `in_place: true`; never
/// overwrites otherwise.
pub(crate) fn apply_optimizations(input: ApplyInput) -> ToolResult {
    if !input.in_place {
        return Err(ToolError::invalid_input(
            "apply_optimizations is a mutating tool - pass in_place: true to confirm you want \
             to rewrite the file, or call suggest_optimizations for a read-only preview"
        ));
    }
    let path = Utf8Path::new(&input.path);
    let options = Options {
        goal: parse_goal(input.goal.as_deref())?,
        disabled_rules: input.disabled_rules.unwrap_or_default(),
        include_dirs: input
            .include_dirs
            .unwrap_or_default()
            .into_iter()
            .map(camino::Utf8PathBuf::from)
            .collect(),
        ..Default::default()
    };
    let outcome = cpclib_basmopt::apply_fixes_in_place(path, &options).map_err(|e| {
        ToolError::with_details(
            ToolErrorKind::Assembler,
            e.to_string(),
            json!({ "path": input.path })
        )
    })?;
    fs_err::write(path, &outcome.final_source)
        .map_err(|e| ToolError::io(format!("cannot write {}: {e}", input.path)))?;
    Ok(json!({
        "path": input.path,
        "passes_run": outcome.passes_run,
        "total_applied": outcome.total_applied,
        "remaining_skipped": outcome.remaining_skipped,
        "assemble_warning": outcome.assemble_warning
    }))
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Value>, Json<Value>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = basmopt_router, vis = "pub(crate)")]
impl McpServer {
    #[tool(description = "Peephole-optimization suggestions for a basm source file, with the \
                           matched rule and why each is believed safe. Read-only.")]
    async fn suggest_optimizations(
        &self,
        Parameters(input): Parameters<SuggestInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(suggest_optimizations(input))
    }

    #[tool(description = "MUTATING: rewrites the file in place with every bulk-safe peephole \
                           suggestion applied. Requires in_place: true.")]
    async fn apply_optimizations(
        &self,
        Parameters(input): Parameters<ApplyInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(apply_optimizations(input))
    }
}
