//! `suggest_optimizations` / `apply_optimizations` - `cpclib-basmopt`'s
//! peephole-optimization advisor, structured (not a text diff): each
//! `Suggestion` carries the matched rule, why it's believed safe
//! (`SuggestionReason`s with their own source positions), and whether it's
//! safe to bulk-apply.

use camino::{Utf8Path, Utf8PathBuf};
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

/// The `Options` construction every one of this module's tools shares -
/// only `defines`/`disabled_rules`/`include_dirs` differ per input struct,
/// which the caller still owns (so it can move them without cloning).
fn build_options(
    goal: Option<&str>,
    disabled_rules: Vec<String>,
    include_dirs: Vec<String>,
    defines: Vec<String>
) -> Result<Options, ToolError> {
    Ok(Options {
        goal: parse_goal(goal)?,
        disabled_rules,
        include_dirs: include_dirs.into_iter().map(Utf8PathBuf::from).collect(),
        defines,
        ..Default::default()
    })
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
    pub include_dirs: Option<Vec<String>>,
    /// Symbols to define before assembling, `NAME` (= 1) or `NAME=VALUE`,
    /// like `basm -D` - for code conditional on a symbol the real build
    /// passes on its command line (e.g. `LINKED_VERSION=1`).
    pub defines: Option<Vec<String>>,
    /// Also analyze every file `path` (transitively) `INCLUDE`s, each
    /// against `path`'s own real, whole-project address space - not just
    /// `path`'s own top-level lines. A project that keeps its real code in
    /// included files (a thin top-level file that mostly `INCLUDE`s
    /// everything else - the common shape, and how e.g. a project like
    /// etchy's `main.asm` is built) has almost nothing to find without
    /// this: `path`'s own `INCLUDE` lines are opaque to a single-file
    /// analysis. Read-only either way - only `apply_optimizations` writes,
    /// and only ever to `path` itself, never to an included file this
    /// finds. Default false.
    pub include_project: Option<bool>
}

/// Peephole-optimization suggestions for a source file - each with the
/// matched rule, why it's believed safe, and whether it's bulk-safe to
/// apply automatically. Read-only.
pub(crate) fn suggest_optimizations(input: SuggestInput) -> ToolResult {
    let path = Utf8Path::new(&input.path);
    let options = build_options(
        input.goal.as_deref(),
        input.disabled_rules.unwrap_or_default(),
        input.include_dirs.unwrap_or_default(),
        input.defines.unwrap_or_default()
    )?;

    if input.include_project.unwrap_or(false) {
        let outcomes = cpclib_basmopt::analyze_project(path, &options).map_err(|e| {
            ToolError::with_details(
                ToolErrorKind::Assembler,
                e.to_string(),
                json!({ "path": input.path })
            )
        })?;
        return Ok(project_outcomes_to_json(&input.path, &outcomes));
    }

    let outcome = cpclib_basmopt::analyze_file(path, &options).map_err(|e| {
        ToolError::with_details(
            ToolErrorKind::Assembler,
            e.to_string(),
            json!({ "path": input.path })
        )
    })?;
    Ok(outcome_to_json(&outcome))
}

/// `analyze_project`'s per-file outcomes into one JSON report: a `files`
/// breakdown (only files with something to say - a clean file with no
/// suggestions and no warning is just noise in an already long list),
/// plus the totals a caller checking "is there anything at all" wants
/// without walking `files` itself.
fn project_outcomes_to_json(entry_path: &str, outcomes: &[(Utf8PathBuf, AnalyzeOutcome)]) -> Value {
    let files: Vec<Value> = outcomes
        .iter()
        .filter(|(_, o)| !o.suggestions.is_empty() || o.assemble_warning.is_some())
        .map(|(path, o)| {
            json!({
                "path": path.as_str(),
                "suggestion_count": o.suggestions.len(),
                "suggestions": o.suggestions.iter().map(suggestion_to_json).collect::<Vec<_>>(),
                "assemble_warning": o.assemble_warning
            })
        })
        .collect();
    json!({
        "entry": entry_path,
        "files_analyzed": outcomes.len(),
        "files_with_findings": files.len(),
        "suggestion_count": outcomes.iter().map(|(_, o)| o.suggestions.len()).sum::<usize>(),
        "files": files
    })
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ApplyInput {
    /// Path to the `.asm` file to analyze and rewrite.
    pub path: String,
    pub goal: Option<String>,
    pub disabled_rules: Option<Vec<String>>,
    pub include_dirs: Option<Vec<String>>,
    /// Symbols to define before assembling - see `suggest_optimizations`.
    pub defines: Option<Vec<String>>,
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
    let options = build_options(
        input.goal.as_deref(),
        input.disabled_rules.unwrap_or_default(),
        input.include_dirs.unwrap_or_default(),
        input.defines.unwrap_or_default()
    )?;
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

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ApplyProjectInput {
    /// Directory to rewrite every `.asm` file under (recursively,
    /// respecting `.gitignore`) - not one file.
    pub project_dir: String,
    pub goal: Option<String>,
    pub disabled_rules: Option<Vec<String>>,
    pub include_dirs: Option<Vec<String>>,
    /// Symbols to define before assembling - see `suggest_optimizations`.
    pub defines: Option<Vec<String>>,
    /// Must be `true` - this tool rewrites every matching file under
    /// `project_dir` in place. There is no sandboxed/dry-run variant:
    /// every fix it applies already went through the same bulk-safety
    /// check `apply_optimizations` trusts for a single file - call
    /// `suggest_optimizations` with `include_project: true` first if you
    /// want to see what it would find before committing to a rewrite.
    pub in_place: bool
}

/// Rewrites every `.asm` file under `project_dir` in place with its own
/// bulk-safe, non-address-aware peephole suggestions applied (up to 2
/// passes per file). **Mutating** - requires `in_place: true`.
///
/// Address-aware rules (`jp2jr`) are never applied this way, regardless of
/// `goal`: rewriting one file mid-run can shift another's real addresses
/// via a shared `INCLUDE`, which could invalidate an address-aware match
/// found before that rewrite happened - see
/// `cpclib_basmopt::apply_fixes_in_place_project`'s own doc comment. Ask
/// `suggest_optimizations` with `include_project: true` for those; apply
/// them individually with `apply_optimizations` on the file that actually
/// needs them.
pub(crate) fn apply_optimizations_project(input: ApplyProjectInput) -> ToolResult {
    if !input.in_place {
        return Err(ToolError::invalid_input(
            "apply_optimizations_project is a mutating tool - pass in_place: true to confirm              you want to rewrite every matching file under project_dir, or call              suggest_optimizations with include_project: true for a read-only preview"
        ));
    }
    let path = Utf8Path::new(&input.project_dir);
    let options = build_options(
        input.goal.as_deref(),
        input.disabled_rules.unwrap_or_default(),
        input.include_dirs.unwrap_or_default(),
        input.defines.unwrap_or_default()
    )?;
    let outcome = cpclib_basmopt::apply_fixes_in_place_project(path, &options);
    Ok(json!({
        "project_dir": input.project_dir,
        "files_touched": outcome.files_touched,
        "files_with_errors": outcome.files_with_errors,
        "total_applied": outcome.total_applied,
        "total_skipped_for_review": outcome.total_skipped_for_review,
        "total_address_aware_skipped": outcome.total_address_aware_skipped
    }))
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Value>, Json<Value>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = basmopt_router, vis = "pub(crate)")]
impl McpServer {
    #[tool(description = "Peephole-optimization suggestions for a basm source file, with the \
                           matched rule and why each is believed safe. Read-only. By default \
                           only looks at path's own top-level lines - pass include_project: \
                           true to also analyze every file it INCLUDEs (transitively), each \
                           against the real whole-project address space, which most projects \
                           need: a thin top-level file that mostly INCLUDEs everything else has \
                           almost nothing to find otherwise.")]
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

    #[tool(description = "MUTATING: rewrites every .asm file under project_dir in place with \
                           its own bulk-safe, non-address-aware peephole suggestions applied \
                           (respects .gitignore). Requires in_place: true. Never applies an \
                           address-aware rule (e.g. jp2jr) regardless of goal - rewriting one \
                           file can shift another's real addresses via a shared INCLUDE, which \
                           could invalidate an address-aware match found earlier in the same \
                           run; total_address_aware_skipped reports how many were left for \
                           apply_optimizations on the individual file instead.")]
    async fn apply_optimizations_project(
        &self,
        Parameters(input): Parameters<ApplyProjectInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(apply_optimizations_project(input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(path: &str) -> SuggestInput {
        SuggestInput {
            path: path.to_string(),
            goal: None,
            disabled_rules: None,
            include_dirs: None,
            defines: None,
            include_project: None
        }
    }

    /// Without `include_project`, a thin entry file that only `INCLUDE`s the
    /// real code finds nothing - the gap this option exists to close.
    #[test]
    fn without_include_project_an_include_only_entry_finds_nothing() {
        let dir = camino_tempfile::tempdir().unwrap();
        fs_err::write(dir.path().join("code.asm"), "\tld a,0\n\tld a,0\n\tret\n").unwrap();
        fs_err::write(dir.path().join("entry.asm"), "\torg 0x4000\n\tinclude \"code.asm\"\n").unwrap();
        let path = dir.path().join("entry.asm").to_string();

        let out = suggest_optimizations(base(&path)).unwrap();
        assert_eq!(out["suggestion_count"], 0, "{out}");
    }

    /// With it, the same project finds the suggestion inside the included
    /// file, reported under its own path in `files`.
    #[test]
    fn include_project_finds_suggestions_in_an_included_file() {
        let dir = camino_tempfile::tempdir().unwrap();
        fs_err::write(dir.path().join("code.asm"), "\tld a,0\n\tld a,0\n\tret\n").unwrap();
        fs_err::write(dir.path().join("entry.asm"), "\torg 0x4000\n\tinclude \"code.asm\"\n").unwrap();
        let path = dir.path().join("entry.asm").to_string();

        let out = suggest_optimizations(SuggestInput { include_project: Some(true), ..base(&path) }).unwrap();
        assert_eq!(out["files_analyzed"], 2, "{out}");
        assert!(out["suggestion_count"].as_u64().unwrap() > 0, "{out}");
        let files = out["files"].as_array().unwrap();
        assert!(
            files.iter().any(|f| f["path"].as_str().unwrap().ends_with("code.asm")),
            "the included file must be listed: {out}"
        );
    }

    fn project_base(project_dir: &str) -> ApplyProjectInput {
        ApplyProjectInput {
            project_dir: project_dir.to_string(),
            goal: None,
            disabled_rules: None,
            include_dirs: None,
            defines: None,
            in_place: false
        }
    }

    #[test]
    fn apply_optimizations_project_requires_in_place() {
        let err = apply_optimizations_project(project_base(".")).unwrap_err();
        assert_eq!(err.kind, "invalid_input");
    }

    /// Real rewrite across two files under a directory - the actual point
    /// of this tool over calling `apply_optimizations` file by file.
    #[test]
    fn apply_optimizations_project_rewrites_every_matching_file() {
        let dir = camino_tempfile::tempdir().unwrap();
        fs_err::write(dir.path().join("a.asm"), "\tld a,0\n\tld a,0\n\tret\n").unwrap();
        fs_err::write(dir.path().join("b.asm"), "\tld b,1\n\tld b,1\n\tret\n").unwrap();
        let project_dir = dir.path().to_string();

        let out = apply_optimizations_project(ApplyProjectInput { in_place: true, ..project_base(&project_dir) }).unwrap();
        assert_eq!(out["files_touched"], 2, "{out}");
        assert_eq!(out["files_with_errors"], 0, "{out}");
        assert!(out["total_applied"].as_u64().unwrap() >= 2, "{out}");

        assert_eq!(fs_err::read_to_string(dir.path().join("a.asm")).unwrap(), "\tld a, 0\n\tret\n");
        assert_eq!(fs_err::read_to_string(dir.path().join("b.asm")).unwrap(), "\tld b, 1\n\tret\n");
    }
}
