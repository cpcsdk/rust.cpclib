//! `run_build`/`list_build_targets`/`outdated_targets` - thin wrappers over
//! `cpclib_bndbuild::builder::BndBuilder`.

use std::sync::{Arc, Mutex};

use camino::{Utf8Path, Utf8PathBuf};
use cpclib_bndbuild::BndBuilderError;
use cpclib_bndbuild::app::WatchState;
use cpclib_bndbuild::builder::BndBuilder;
use cpclib_bndbuild::event::{BndBuilderEvent, BndBuilderObserved, BndBuilderObserver, BndBuilderObserverRc};
use cpclib_common::event::EventObserver;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::{Value, json};

use crate::McpServer;
use crate::error::{ToolError, ToolErrorKind, ToolResult};

/// Captures every `BndBuilderEvent`'s text into an ordered log, rather than
/// letting a build write to this process's own stdout (reserved for MCP
/// protocol frames). Cheap-clone: the observer instance handed to
/// `BndBuilder::add_observer` is a separate clone of the one this module
/// keeps, sharing the same underlying `Vec` so the log is still readable
/// after the observer itself has been moved into the builder.
#[derive(Debug, Clone, Default)]
struct LogObserver {
    lines: Arc<Mutex<Vec<String>>>
}

impl LogObserver {
    fn push(&self, line: impl Into<String>) {
        self.lines.lock().unwrap().push(line.into());
    }

    fn into_lines(self) -> Vec<String> {
        Arc::try_unwrap(self.lines)
            .map(|m| m.into_inner().unwrap())
            .unwrap_or_else(|arc| arc.lock().unwrap().clone())
    }
}

impl EventObserver for LogObserver {
    fn emit_stdout(&self, s: &str) {
        self.push(s.trim_end());
    }

    fn emit_stderr(&self, s: &str) {
        self.push(format!("[stderr] {}", s.trim_end()));
    }
}

impl BndBuilderObserver for LogObserver {
    fn update(&self, event: BndBuilderEvent) {
        match event {
            BndBuilderEvent::StartRule { rule, nb, out_of } => {
                self.push(format!("[{nb}/{out_of}] building {rule}"));
            },
            BndBuilderEvent::StopRule(rule) => self.push(format!("done: {rule}")),
            BndBuilderEvent::SkippedRule(rule) => self.push(format!("up to date: {rule}")),
            BndBuilderEvent::FailedRule(rule) => self.push(format!("FAILED: {rule}")),
            BndBuilderEvent::TaskStdout(_, _, s) => self.push(s),
            BndBuilderEvent::TaskStderr(_, _, s) => self.push(format!("[stderr] {s}")),
            BndBuilderEvent::TaskIgnoredError(_, _, s) => {
                self.push(format!("[ignored error] {s}"));
            },
            BndBuilderEvent::Stdout(s) => self.push(s),
            BndBuilderEvent::Stderr(s) => self.push(format!("[stderr] {s}")),
            _ => {}
        }
    }
}

fn build_error(e: BndBuilderError) -> ToolError {
    ToolError::new(ToolErrorKind::Build, e.to_string())
}

fn open_builder(bnd_path: &str) -> Result<(Utf8PathBuf, BndBuilder), ToolError> {
    // `false` = do not force every nested `bndbuild` task to add `--serial`
    // (this crate's own "rayon" feature is enabled - see Cargo.toml - so
    // `from_path` takes this extra argument).
    BndBuilder::from_path(Utf8Path::new(bnd_path), false).map_err(build_error)
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RunBuildInput {
    /// Path to the build file (e.g. `build.bnd`) or a directory containing
    /// one.
    pub bnd_path: String,
    /// Target to build; defaults to the build file's own default target.
    pub target: Option<String>
}

/// **MUTATING**: runs a build target (and its dependencies) via
/// `BndBuilder::execute`. Captures build output into a returned log rather
/// than writing to this process's own stdout.
pub(crate) fn run_build(input: RunBuildInput) -> ToolResult {
    let (resolved_path, mut builder) = open_builder(&input.bnd_path)?;
    let target = match &input.target {
        Some(t) => Utf8PathBuf::from(t),
        None => {
            builder
                .default_target()
                .map(|p| p.to_path_buf())
                .ok_or_else(|| {
                    ToolError::new(
                        ToolErrorKind::Build,
                        "no target given and the build file declares no default target"
                    )
                })?
        }
    };

    let observer = LogObserver::default();
    builder.add_observer(BndBuilderObserverRc::new(observer.clone()));

    let start = std::time::Instant::now();
    let result = builder.execute(&target);
    let duration_ms = start.elapsed().as_millis();
    let log = observer.into_lines();

    match result {
        Ok(()) => {
            Ok(json!({
                "bnd_path": resolved_path.as_str(),
                "target": target.as_str(),
                "success": true,
                "duration_ms": duration_ms,
                "log": log
            }))
        },
        Err(e) => {
            Err(ToolError::with_details(
                ToolErrorKind::Build,
                e.to_string(),
                json!({
                    "bnd_path": resolved_path.as_str(),
                    "target": target.as_str(),
                    "duration_ms": duration_ms,
                    "log": log
                })
            ))
        }
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BndPathInput {
    pub bnd_path: String
}

/// Every target the build file declares, with its direct dependencies.
/// Read-only.
pub(crate) fn list_build_targets(input: BndPathInput) -> ToolResult {
    let (resolved_path, builder) = open_builder(&input.bnd_path)?;
    let targets: Vec<Value> = builder
        .targets()
        .into_iter()
        .map(|t| {
            let dependencies: Vec<String> = builder
                .get_rule(t)
                .map(|r| r.dependencies().iter().map(|d| d.to_string()).collect())
                .unwrap_or_default();
            json!({ "target": t.as_str(), "dependencies": dependencies })
        })
        .collect();
    Ok(json!({
        "bnd_path": resolved_path.as_str(),
        "default_target": builder.default_target().map(|p| p.as_str()),
        "targets": targets
    }))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct OutdatedInput {
    pub bnd_path: String,
    /// Target to check; when omitted, every declared target is checked.
    pub target: Option<String>
}

/// Whether target(s) are outdated relative to their dependencies (mtime-
/// based), without building anything. Read-only.
pub(crate) fn outdated_targets(input: OutdatedInput) -> ToolResult {
    let (resolved_path, builder) = open_builder(&input.bnd_path)?;
    let targets_to_check: Vec<Utf8PathBuf> = match &input.target {
        Some(t) => vec![Utf8PathBuf::from(t)],
        None => builder.targets().into_iter().map(|p| p.to_path_buf()).collect()
    };

    let mut results = Vec::new();
    for target in &targets_to_check {
        let outdated = builder
            .outdated(&WatchState::NoWatch, target)
            .map_err(build_error)?;
        results.push(json!({ "target": target.as_str(), "outdated": outdated }));
    }
    Ok(json!({ "bnd_path": resolved_path.as_str(), "results": results }))
}

/// One `<label> equ #<hex>` line from a basm-produced `.sym` file, parsed
/// into a plain `name -> value` map. `.sym` files are always hex (`#XXXX`,
/// `basm`'s own `--sym` output convention) - no other numeric base is
/// handled, matching what's actually emitted.
fn parse_sym_file(text: &str) -> std::collections::HashMap<String, u32> {
    let mut symbols = std::collections::HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        let Some((name, rest)) = line.split_once(" equ ") else {
            continue;
        };
        let Some(hex) = rest.trim().strip_prefix('#') else {
            continue;
        };
        if let Ok(value) = u32::from_str_radix(hex.trim(), 16) {
            symbols.insert(name.to_string(), value);
        }
    }
    symbols
}

/// Every `<base>.length`-suffixed symbol names one crunched section (the
/// convention `LOAD_N_CRUNCH_WITH`-style project macros already write into
/// the `.sym` file, confirmed against a real project - see `report_build`'s
/// own doc comment). Pulls out each section's sizes, tolerating a missing
/// sibling field (`.uncrunched_length`/`.delta`) rather than dropping the
/// whole section over one absent symbol.
fn crunched_sections_from_sym(symbols: &std::collections::HashMap<String, u32>) -> Vec<Value> {
    let mut bases: Vec<&str> = symbols
        .keys()
        .filter_map(|name| name.strip_suffix(".length"))
        .collect();
    bases.sort_unstable();
    bases
        .into_iter()
        .map(|base| {
            json!({
                "label": base,
                "crunched_size": symbols.get(&format!("{base}.length")),
                "uncrunched_size": symbols.get(&format!("{base}.uncrunched_length")),
                "delta": symbols.get(&format!("{base}.delta"))
            })
        })
        .collect()
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ReportBuildInput {
    /// Path to the build file (e.g. `build.bnd`) or a directory containing
    /// one.
    pub bnd_path: String,
    /// Target to build; defaults to the build file's own default target.
    pub target: Option<String>,
    /// Path to the `.sym` file the build produces (basm's `--sym` output) -
    /// read back after a successful build for structured sizes.
    pub sym_path: String,
    /// Optional size budget in bytes (e.g. 2048 for a 2K demo). When given,
    /// the report includes `free_bytes` = this minus `overhead_bytes` minus
    /// the total linked size (negative when over budget).
    pub target_size: Option<i64>,
    /// Bytes `target_size` must also cover besides the linked payload
    /// itself (e.g. 128 for an AMSDOS header) - subtracted from
    /// `target_size` before comparing against the linked size. Defaults to
    /// 0, meaning `target_size` already excludes any such overhead.
    pub overhead_bytes: Option<i64>,
    /// When `false` (the default), `log` drops basm's own per-line
    /// assembler warnings (e.g. "explicit 'A,' prefix" repeated once per
    /// occurrence, which can run to thousands of characters on a real
    /// project) and keeps everything else - build progress, errors,
    /// PRINT-directive output. Set `true` for the full, unfiltered log.
    pub verbose: Option<bool>
}

/// Whether a captured log line is basm's own per-instruction assembler
/// warning noise - the thing `verbose: false` exists to drop. basm's
/// warnings are plain stdout lines (no distinguishing prefix this
/// observer adds itself), so this is a text heuristic, not a structured
/// check - deliberately narrow (matches only an explicit "warning" marker)
/// so it can never eat a real error or a project's own `PRINT` output.
fn is_basm_warning_noise(line: &str) -> bool {
    line.to_ascii_lowercase().contains("warning")
}

/// `log`, filtered for `verbose: false` - see [`is_basm_warning_noise`].
fn filtered_log(log: &[String], verbose: bool) -> Vec<String> {
    if verbose {
        return log.to_vec();
    }
    log.iter().filter(|line| !is_basm_warning_noise(line)).cloned().collect()
}

/// **MUTATING**: runs a build exactly like `run_build`, then reads back the
/// `.sym` file it produced for structured per-section crunch sizes and the
/// total linked size - instead of a caller grepping colored build-log text
/// for lines like `CRUNCHED MAIN.B000 FROM 3456 BYTES TO 1769 BYTES` (no
/// cpclib crate prints a fixed line like that; the real structured data is
/// the `.sym` file's own `<label>.length`/`.uncrunched_length`/`.delta`
/// symbols, written by a project's own crunch macro convention). Total
/// linked size is `last - first` when a project defines both labels (the
/// `link_sky.asm`-style convention this was grounded against does).
pub(crate) fn report_build(input: ReportBuildInput) -> ToolResult {
    let (resolved_path, mut builder) = open_builder(&input.bnd_path)?;
    let target = match &input.target {
        Some(t) => Utf8PathBuf::from(t),
        None => {
            builder
                .default_target()
                .map(|p| p.to_path_buf())
                .ok_or_else(|| {
                    ToolError::new(
                        ToolErrorKind::Build,
                        "no target given and the build file declares no default target"
                    )
                })?
        }
    };

    let verbose = input.verbose.unwrap_or(false);
    let observer = LogObserver::default();
    builder.add_observer(BndBuilderObserverRc::new(observer.clone()));

    let start = std::time::Instant::now();
    let result = builder.execute(&target);
    let duration_ms = start.elapsed().as_millis();
    let log = filtered_log(&observer.into_lines(), verbose);

    if let Err(e) = result {
        return Err(ToolError::with_details(
            ToolErrorKind::Build,
            e.to_string(),
            json!({
                "bnd_path": resolved_path.as_str(),
                "target": target.as_str(),
                "duration_ms": duration_ms,
                "log": log
            })
        ));
    }

    let sym_text = fs_err::read_to_string(&input.sym_path)
        .map_err(|e| ToolError::io(format!("build succeeded but cannot read {}: {e}", input.sym_path)))?;
    let symbols = parse_sym_file(&sym_text);
    let sections = crunched_sections_from_sym(&symbols);
    let total_linked_size = match (symbols.get("first"), symbols.get("last")) {
        (Some(&first), Some(&last)) => Some(last.saturating_sub(first)),
        _ => None
    };
    let overhead_bytes = input.overhead_bytes.unwrap_or(0);
    let free_bytes = match (input.target_size, total_linked_size) {
        (Some(budget), Some(total)) => Some(budget - overhead_bytes - total as i64),
        _ => None
    };

    Ok(json!({
        "bnd_path": resolved_path.as_str(),
        "target": target.as_str(),
        "success": true,
        "duration_ms": duration_ms,
        "sections": sections,
        "total_linked_size": total_linked_size,
        "free_bytes": free_bytes,
        "log": log
    }))
}

/// Restores `path`'s original content on drop, regardless of how the scope
/// exits (an early `?`, a panic during the build, ...) - `compare_link_sizes`
/// temporarily overwrites a project source file once per candidate cruncher
/// and must never leave it modified, even on failure. Best-effort: a
/// restore that itself fails (disk full, permissions) has nowhere left to
/// report to from a `Drop` impl, so it is silently swallowed rather than
/// panicking during unwind - the alternative is losing the original
/// content entirely, which is strictly worse.
struct RestoreFileOnDrop<'a> {
    path: &'a str,
    original: String
}

impl Drop for RestoreFileOnDrop<'_> {
    fn drop(&mut self) {
        let _ = fs_err::write(self.path, &self.original);
    }
}

/// Rewrites the value assigned to `variable_name` on its one *active*
/// (non-comment) `name = value` line in `source` - preserving everything
/// else about that line (leading whitespace, a trailing `; comment`, as
/// real project files like skyline's `link_sky.asm` have: `SELECTED_CRUNCHER
/// = CRUNCHER_ZX0_BACKWARD ; 2041` alongside several commented-out
/// alternatives). Returns `None` when no such line is found, rather than
/// guessing at one - a caller must treat that as "this file does not use
/// this convention", never silently build the file unmodified.
fn find_active_assignment(lines: &[&str], variable_name: &str) -> Option<usize> {
    lines.iter().position(|line| {
        let trimmed = line.trim_start();
        !trimmed.starts_with(';')
            && trimmed.strip_prefix(variable_name).is_some_and(|rest| rest.trim_start().starts_with('='))
    })
}

/// The value currently assigned on the active line (comment stripped), so a
/// comparison table can mark which candidate is what the project uses today.
fn current_assignment_value(source: &str, variable_name: &str) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    let line = lines[find_active_assignment(&lines, variable_name)?];
    let rest = &line[line.find('=')? + 1..];
    Some(rest.split(';').next().unwrap_or("").trim().to_string())
}

fn rewrite_variable_assignment(source: &str, variable_name: &str, new_value: &str) -> Option<String> {
    let mut lines: Vec<&str> = source.lines().collect();
    let target = find_active_assignment(&lines, variable_name)?;

    let line = lines[target];
    let eq = line.find('=')?;
    let (prefix, rest) = line.split_at(eq + 1);
    let trailing_comment = rest.find(';').map(|i| &rest[i..]).unwrap_or("");
    let rewritten = format!("{prefix} {new_value} {trailing_comment}");
    let rewritten_trimmed = rewritten.trim_end().to_string();
    lines[target] = &rewritten_trimmed;
    Some(lines.join("\n"))
}

/// One candidate's measured outcome, before rendering.
#[derive(Debug, Clone)]
struct LinkRow {
    cruncher: String,
    is_current: bool,
    linked: Option<i64>,
    /// Sum of every crunched section's compressed size (from the `.sym`).
    payload: Option<i64>,
    free_bytes: Option<i64>,
    duration_ms: Option<u64>,
    error: Option<String>
}

/// One-line, markdown-cell-safe summary of a build error: ANSI colour codes
/// stripped, whitespace collapsed, `|` neutralised, and - since the useful
/// part of a build failure is usually the last `error:` in it, not the
/// wrapper text - starting from there when one exists.
fn one_line_error(message: &str) -> String {
    let mut plain = String::with_capacity(message.len());
    let mut chars = message.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for n in chars.by_ref() {
                if n.is_ascii_alphabetic() {
                    break;
                }
            }
        }
        else {
            plain.push(c);
        }
    }
    let collapsed = plain.split_whitespace().collect::<Vec<_>>().join(" ").replace('|', "/");
    let tail = collapsed.rfind("error:").map_or(collapsed.as_str(), |i| &collapsed[i..]);
    tail.chars().take(100).collect()
}

/// A markdown table a human can read at a glance, smallest linked size
/// first: rank, cruncher, linked size, delta vs the best, compressed
/// payload, everything else in the link (`linked - payload`: decruncher
/// stub, loader, ... - the part `compare_crunchers` cannot see), free
/// bytes when a budget was given, and build time. Failed candidates are
/// listed last with their error rather than dropped. `rows` must already
/// be sorted.
fn render_comparison_table(rows: &[LinkRow], with_budget: bool) -> String {
    let best = rows.iter().filter_map(|r| r.linked).min();
    let mut header = String::from("| # | Cruncher | Linked | vs best | Payload | Stub + rest |");
    let mut rule = String::from("|--:|:--|--:|--:|--:|--:|");
    if with_budget {
        header.push_str(" Free |");
        rule.push_str("--:|");
    }
    header.push_str(" Build (s) |");
    rule.push_str("--:|");

    let mut out = format!("{header}\n{rule}\n");
    let num = |v: Option<i64>| v.map_or("-".to_string(), |v| v.to_string());
    let mut rank = 0;
    for r in rows {
        let name = if r.is_current {
            format!("{} (current)", r.cruncher)
        }
        else {
            r.cruncher.clone()
        };
        let secs = r.duration_ms.map_or("-".to_string(), |ms| format!("{:.1}", ms as f64 / 1000.0));
        match r.linked {
            Some(linked) => {
                rank += 1;
                let vs_best = match best {
                    Some(b) if linked == b => "best".to_string(),
                    Some(b) => format!("+{}", linked - b),
                    None => "-".to_string()
                };
                let stub = r.payload.map(|p| linked - p);
                out.push_str(&format!(
                    "| {rank} | {name} | {linked} | {vs_best} | {} | {} |",
                    num(r.payload),
                    num(stub)
                ));
                if with_budget {
                    out.push_str(&format!(" {} |", num(r.free_bytes)));
                }
                out.push_str(&format!(" {secs} |\n"));
            },
            None => {
                let reason = r.error.as_deref().map_or("no linked size".to_string(), one_line_error);
                let cols = 6 + usize::from(with_budget);
                out.push_str(&format!(
                    "| - | {name} | FAILED: {} |{} {secs} |\n",
                    reason,
                    " |".repeat(cols - 3)
                ));
            }
        }
    }
    out
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CompareLinkSizesInput {
    /// Path to the build file (e.g. `build.bnd`) or a directory containing
    /// one.
    pub bnd_path: String,
    /// Target to build; defaults to the build file's own default target.
    pub target: Option<String>,
    /// Path to the `.sym` file the build produces - same convention as
    /// `report_build`.
    pub sym_path: String,
    /// Path to the source file containing the cruncher-selection line to
    /// rewrite (e.g. `link_sky.asm`).
    pub cruncher_source_path: String,
    /// The variable name whose assignment gets rewritten (e.g.
    /// `SELECTED_CRUNCHER`). Must appear on exactly one *active*
    /// (non-`;`-commented) `name = value` line in `cruncher_source_path` -
    /// this tool refuses to guess which line to change otherwise.
    pub variable_name: String,
    /// The exact right-hand-side values to try in turn, verbatim as the
    /// project's own source expects them (e.g. `CRUNCHER_ZX0_BACKWARD`,
    /// `CRUNCHER_UPKR`) - not normalized against `compare_crunchers`' own
    /// format names, since that convention is this tool's own, not every
    /// project's.
    pub crunchers: Vec<String>,
    /// Optional extra source lines to prepend to `cruncher_source_path`
    /// for specific candidates only, keyed by the exact value in
    /// `crunchers` (e.g. `{"CRUNCHER_UPKR": ["UPKR_PROBS_ORIGIN = &4000"]}`).
    /// This tool has no per-format knowledge of its own. Real example:
    /// `upkr.asm` itself only computes addresses with `EQU` and emits no
    /// bytes, but a project's own install macro may `org` past the probs
    /// array (skyline's `INSTALL_UPKR` did), which inflates the linked size
    /// unless the macro skips that `org` when `UPKR_PROBS_ORIGIN` is
    /// defined - and the origin must then be a free `0x??00` address with
    /// 320 bytes available. Which symbol to define, and where, is
    /// project-specific.
    pub extra_lines: Option<std::collections::HashMap<String, Vec<String>>>,
    /// Optional size budget in bytes; when given the table gains a `Free`
    /// column (`target_size - overhead_bytes - linked size`).
    pub target_size: Option<i64>,
    /// Bytes `target_size` also has to cover (e.g. 128 for an AMSDOS
    /// header). Default 0.
    pub overhead_bytes: Option<i64>
}

/// **MUTATING (transiently only - always restored)**: re-links a project
/// once per candidate value of a single source-level cruncher-selection
/// variable, rewriting and rebuilding for each, and reports each attempt's
/// real total linked size (the same `.sym`-derived `last - first` computed
/// by `report_build`) - answering the question `compare_crunchers` cannot:
/// a smaller compressed payload does not always mean a smaller final ROM,
/// since each format's own decruncher stub has its own size (real example
/// that prompted this: UPKR won on payload, 1709 vs zx0_backward's 1781,
/// but lost by 31 bytes once zx0_backward's smaller decruncher stub was
/// counted).
///
/// Scoped exactly to the convention this was reported against - a single
/// source line assigning one variable that some other part of the project
/// reads to pick a cruncher at assemble time. This is a real, project-
/// specific convention, not a general capability: a project using more
/// than one cruncher at once for different sections (confirmed real,
/// `birthtro` does) has no single variable this could rewrite, and this
/// tool does not attempt to guess at one - `rewrite_variable_assignment`
/// returning `None` for any candidate fails the whole call rather than
/// silently building some candidates unmodified.
pub(crate) fn compare_link_sizes(input: CompareLinkSizesInput) -> ToolResult {
    if input.crunchers.is_empty() {
        return Err(ToolError::invalid_input("`crunchers` must not be empty"));
    }

    let original = fs_err::read_to_string(&input.cruncher_source_path)
        .map_err(|e| ToolError::io(format!("cannot read {}: {e}", input.cruncher_source_path)))?;
    // Restored on every exit path, including an early `?` below - see
    // `RestoreFileOnDrop`'s own doc comment.
    let _restore = RestoreFileOnDrop {
        path: &input.cruncher_source_path,
        original: original.clone()
    };

    let current = current_assignment_value(&original, &input.variable_name);
    let mut rows: Vec<LinkRow> = Vec::with_capacity(input.crunchers.len());
    for candidate in &input.crunchers {
        let Some(rewritten) = rewrite_variable_assignment(&original, &input.variable_name, candidate)
        else {
            return Err(ToolError::invalid_input(format!(
                "no active (non-commented) `{} = ...` line found in {} - refusing to guess which \
                 line selects the cruncher",
                input.variable_name, input.cruncher_source_path
            )));
        };
        let rewritten = match input.extra_lines.as_ref().and_then(|m| m.get(candidate)) {
            Some(extra) => format!("{}\n{rewritten}", extra.join("\n")),
            None => rewritten
        };
        fs_err::write(&input.cruncher_source_path, &rewritten)
            .map_err(|e| ToolError::io(format!("cannot write {}: {e}", input.cruncher_source_path)))?;

        let is_current = current.as_deref() == Some(candidate.as_str());
        let outcome = report_build(ReportBuildInput {
            bnd_path: input.bnd_path.clone(),
            target: input.target.clone(),
            sym_path: input.sym_path.clone(),
            target_size: input.target_size,
            overhead_bytes: input.overhead_bytes,
            verbose: Some(false)
        });
        rows.push(match outcome {
            Ok(report) => {
                LinkRow {
                    cruncher: candidate.clone(),
                    is_current,
                    linked: report["total_linked_size"].as_i64(),
                    payload: report["sections"].as_array().map(|secs| {
                        secs.iter().filter_map(|s| s["crunched_size"].as_i64()).sum()
                    }),
                    free_bytes: report["free_bytes"].as_i64(),
                    duration_ms: report["duration_ms"].as_u64(),
                    error: None
                }
            },
            Err(e) => {
                LinkRow {
                    cruncher: candidate.clone(),
                    is_current,
                    linked: None,
                    payload: None,
                    free_bytes: None,
                    duration_ms: None,
                    error: Some(e.message.clone())
                }
            }
        });
    }

    // Smallest real linked size first; failed attempts (no comparable size)
    // sort last rather than being dropped, so a caller can see *which*
    // candidates failed and why.
    rows.sort_by_key(|r| r.linked.unwrap_or(i64::MAX));
    let table = render_comparison_table(&rows, input.target_size.is_some());
    let best = rows.iter().filter_map(|r| r.linked).min();
    let results: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "cruncher": r.cruncher,
                "current": r.is_current,
                "ok": r.linked.is_some(),
                "total_linked_size": r.linked,
                "vs_best": r.linked.zip(best).map(|(l, b)| l - b),
                "payload_size": r.payload,
                "stub_and_rest": r.linked.zip(r.payload).map(|(l, p)| l - p),
                "free_bytes": r.free_bytes,
                "duration_ms": r.duration_ms,
                "error": r.error
            })
        })
        .collect();

    Ok(json!({
        "bnd_path": input.bnd_path,
        "cruncher_source_path": input.cruncher_source_path,
        "variable_name": input.variable_name,
        "table": table,
        "results": results
    }))
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Value>, Json<Value>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = build_router, vis = "pub(crate)")]
impl McpServer {
    #[tool(description = "MUTATING: runs a bndbuild target (and its dependencies). Returns a \
                           captured build log.")]
    async fn run_build(
        &self,
        Parameters(input): Parameters<RunBuildInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(run_build(input))
    }

    #[tool(description = "List every target a bndbuild file declares, with its direct \
                           dependencies. Read-only.")]
    async fn list_build_targets(
        &self,
        Parameters(input): Parameters<BndPathInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(list_build_targets(input))
    }

    #[tool(description = "Check whether bndbuild target(s) are outdated relative to their \
                           dependencies, without building anything. Read-only.")]
    async fn outdated_targets(
        &self,
        Parameters(input): Parameters<OutdatedInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(outdated_targets(input))
    }

    #[tool(description = "MUTATING: runs a bndbuild target like run_build, then reads back the \
                           .sym file it produced for structured per-section crunch sizes \
                           (uncrunched/crunched/delta) and the total linked size (last-first), \
                           instead of grepping build-log text. Requires the project to write a \
                           .sym file and follow the <label>.length/.uncrunched_length/.delta \
                           symbol convention - not every project does. `log` defaults to a \
                           filtered view (basm's own per-line assembler warnings dropped, \
                           everything else kept) - pass `verbose: true` for the raw log. \
                           `free_bytes` (when `target_size` is given) is target_size minus \
                           `overhead_bytes` (default 0 - e.g. pass 128 for an AMSDOS header) \
                           minus the linked size.")]
    async fn report_build(
        &self,
        Parameters(input): Parameters<ReportBuildInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(report_build(input))
    }

    #[tool(description = "MUTATING (transiently only - the rewritten source file is always \
                           restored to its original content before this call returns, even on \
                           error): answers 'which cruncher gives the smallest real linked \
                           output', not just the smallest payload - compare_crunchers compares \
                           raw bytes in isolation and ignores each format's own decruncher stub \
                           size, which compare_link_sizes does not: it rewrites a single \
                           project-defined variable (e.g. SELECTED_CRUNCHER) to each candidate \
                           value in turn, rebuilds the real target, and reports each attempt's \
                           real total linked size, smallest first, plus a ready-to-display markdown `table` \
                           (rank, linked size, delta vs best, compressed payload, stub+rest, \
                           optional free bytes, build time, the project's current choice marked) \
                           for a human to compare - show it as-is. Optional `extra_lines` injects \
                           per-candidate source lines (e.g. UPKR_PROBS_ORIGIN). Scoped to projects using \
                           exactly one such variable to pick a cruncher at assemble time - fails \
                           clearly (does not guess) when the variable's line cannot be found; \
                           does not support projects selecting different crunchers for different \
                           sections independently (e.g. birthtro).")]
    async fn compare_link_sizes(
        &self,
        Parameters(input): Parameters<CompareLinkSizesInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(compare_link_sizes(input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real lines from a real project's `.sym` file (`skyline/main/sky.sym`)
    /// - grounds this parser against actual `basm` output, not a guessed
    /// format. `#06E9` = 1769, `#0D80` = 3456, matching the feedback's own
    /// worked example ("CRUNCHED MAIN.B000 FROM 3456 BYTES TO 1769 BYTES").
    const REAL_SYM_EXCERPT: &str = "\
first equ #0100
MAIN_B000 equ #2092
MAIN_B000.delta equ #0002
MAIN_B000.last equ #277A
MAIN_B000.length equ #06E9
MAIN_B000.next equ #277B
MAIN_B000.start equ #2092
MAIN_B000.uncrunched_length equ #0D80
last equ #2900
";

    #[test]
    fn parse_sym_file_reads_real_project_lines() {
        let symbols = parse_sym_file(REAL_SYM_EXCERPT);
        assert_eq!(symbols["MAIN_B000.length"], 0x06E9);
        assert_eq!(symbols["MAIN_B000.uncrunched_length"], 0x0D80);
        assert_eq!(symbols["MAIN_B000.delta"], 0x0002);
        assert_eq!(symbols["first"], 0x0100);
        assert_eq!(symbols["last"], 0x2900);
    }

    #[test]
    fn crunched_sections_reports_the_real_projects_known_sizes() {
        let symbols = parse_sym_file(REAL_SYM_EXCERPT);
        let sections = crunched_sections_from_sym(&symbols);
        assert_eq!(sections.len(), 1, "{sections:#?}");
        let section = &sections[0];
        assert_eq!(section["label"], "MAIN_B000");
        assert_eq!(section["crunched_size"], 0x06E9);
        assert_eq!(section["uncrunched_size"], 0x0D80);
        assert_eq!(section["delta"], 0x0002);
    }

    #[test]
    fn crunched_sections_tolerates_a_missing_sibling_field() {
        let symbols = parse_sym_file("ONLY.length equ #0010\n");
        let sections = crunched_sections_from_sym(&symbols);
        assert_eq!(sections.len(), 1, "{sections:#?}");
        assert_eq!(sections[0]["crunched_size"], 0x0010);
        assert!(sections[0]["uncrunched_size"].is_null());
    }

    /// Real feedback: "the log field contains every basm warning (thousands
    /// of chars of 'explicit A, prefix' noise)". The default (non-verbose)
    /// view must drop lines like this...
    #[test]
    fn filtered_log_drops_basm_warning_noise_by_default() {
        let log = vec![
            "[1/3] building main.o".to_string(),
            "[WARNING] explicit 'A,' prefix in ADD A,B".to_string(),
            "done: main.o".to_string(),
        ];
        let filtered = filtered_log(&log, false);
        assert_eq!(filtered, vec!["[1/3] building main.o".to_string(), "done: main.o".to_string()]);
    }

    /// ...but must never drop a real error or a project's own `PRINT`
    /// output, which share the same plain-stdout shape as a warning and
    /// carry no special prefix this observer adds.
    #[test]
    fn filtered_log_keeps_errors_and_print_output_by_default() {
        let log = vec![
            "Unknown symbol: zzz".to_string(),
            "[stderr] fatal error".to_string(),
            "FAILED: main.o".to_string(),
            "my print statement".to_string(),
        ];
        assert_eq!(filtered_log(&log, false), log);
    }

    #[test]
    fn filtered_log_returns_everything_unfiltered_when_verbose() {
        let log = vec!["[WARNING] explicit 'A,' prefix in ADD A,B".to_string()];
        assert_eq!(filtered_log(&log, true), log);
    }

    /// Real feedback: "Total budget here is 2048 = 128-byte AMSDOS header +
    /// linked size, but the tool computes target_size - linked." -
    /// `overhead_bytes` must be subtracted too.
    #[test]
    fn free_bytes_subtracts_overhead_bytes_from_the_budget() {
        let symbols = parse_sym_file(REAL_SYM_EXCERPT);
        let total_linked_size = symbols["last"] - symbols["first"];
        assert_eq!(total_linked_size, 0x2900 - 0x0100);
        let target_size = 2048i64;
        let overhead_bytes = 128i64;
        let free_bytes = target_size - overhead_bytes - total_linked_size as i64;
        assert_eq!(free_bytes, 2048 - 128 - (0x2900 - 0x0100));
    }

    /// The exact real convention `compare_link_sizes` was built against -
    /// skyline's own `link_sky.asm`: one active line plus several
    /// commented-out alternatives, with a trailing comment on the active
    /// one that must survive the rewrite untouched.
    // The commented-out, leading-whitespace line is deliberately not first:
    // Rust's own `"\` line-continuation (used here only to avoid a leading
    // blank line in the literal) strips leading whitespace from exactly the
    // line right after it, which would make this fixture lie about what
    // "leading whitespace preserved" is even testing.
    const REAL_LINK_SKY_EXCERPT: &str = "\
 SELECTED_CRUNCHER = CRUNCHER_ZX0_BACKWARD ; 2041
  ;SELECTED_CRUNCHER = CRUNCHER_UPKR  ;
;SELECTED_CRUNCHER = CRUNCHER_ZX0 ;
;SELECTED_CRUNCHER = CRUNCHER_EXOMIZER ;
; SELECTED_CRUNCHER = CRUNCHER_SHRINKLER
;SELECTED_CRUNCHER = CRUNCHER_TRANSPARENT
";

    #[test]
    fn rewrite_variable_assignment_only_touches_the_one_active_line() {
        let rewritten =
            rewrite_variable_assignment(REAL_LINK_SKY_EXCERPT, "SELECTED_CRUNCHER", "CRUNCHER_UPKR")
                .expect("the real skyline convention must be recognized");
        let lines: Vec<&str> = rewritten.lines().collect();
        // Every commented-out line is untouched, including the one whose
        // value also happens to be CRUNCHER_UPKR (proving this matches by
        // position/comment-state, not by the old value's text) and its own
        // leading whitespace.
        assert_eq!(lines[1], "  ;SELECTED_CRUNCHER = CRUNCHER_UPKR  ;");
        assert_eq!(lines[2], ";SELECTED_CRUNCHER = CRUNCHER_ZX0 ;");
        assert_eq!(lines[3], ";SELECTED_CRUNCHER = CRUNCHER_EXOMIZER ;");
        assert_eq!(lines[4], "; SELECTED_CRUNCHER = CRUNCHER_SHRINKLER");
        assert_eq!(lines[5], ";SELECTED_CRUNCHER = CRUNCHER_TRANSPARENT");
        // The one active line (now first, so also subject to the same
        // continuation-stripped leading space as the fixture's own comment
        // above) changed value but kept its trailing comment.
        assert_eq!(lines[0], "SELECTED_CRUNCHER = CRUNCHER_UPKR ; 2041");
    }

    #[test]
    fn rewrite_variable_assignment_returns_none_when_the_variable_is_never_active() {
        let all_commented = "\
;SELECTED_CRUNCHER = CRUNCHER_UPKR
;SELECTED_CRUNCHER = CRUNCHER_ZX0
";
        assert_eq!(
            rewrite_variable_assignment(all_commented, "SELECTED_CRUNCHER", "CRUNCHER_ZX7"),
            None,
            "must refuse to guess which commented-out line to activate"
        );
    }

    #[test]
    fn rewrite_variable_assignment_returns_none_for_an_unrelated_file() {
        assert_eq!(
            rewrite_variable_assignment("org 0x4000\n ret\n", "SELECTED_CRUNCHER", "CRUNCHER_ZX7"),
            None
        );
    }

    /// `compare_link_sizes` must restore the source file to its exact
    /// original content once every candidate has been tried - simulated
    /// here directly against `RestoreFileOnDrop` (the mechanism
    /// `compare_link_sizes` itself relies on), rather than through a full
    /// build, which the plain-Rust-function unit tests in this module
    /// otherwise avoid needing (see this crate's own "business logic is a
    /// plain testable function" convention).
    #[test]
    fn restore_file_on_drop_puts_the_original_content_back() {
        let dir = camino_tempfile::tempdir().unwrap();
        let path = dir.path().join("link_sky.asm");
        fs_err::write(&path, REAL_LINK_SKY_EXCERPT).unwrap();

        {
            let _restore = RestoreFileOnDrop {
                path: path.as_str(),
                original: REAL_LINK_SKY_EXCERPT.to_string()
            };
            fs_err::write(&path, "corrupted by a failed candidate build\n").unwrap();
        }

        assert_eq!(fs_err::read_to_string(&path).unwrap(), REAL_LINK_SKY_EXCERPT);
    }

    fn row(name: &str, linked: Option<i64>, payload: Option<i64>, current: bool) -> LinkRow {
        LinkRow {
            cruncher: name.to_string(),
            is_current: current,
            linked,
            payload,
            free_bytes: linked.map(|l| 1920 - l),
            duration_ms: Some(1500),
            error: linked.is_none().then(|| "wrapper\n\u{1b}[31merror:\u{1b}[0m boom | pipe\nmore".to_string())
        }
    }

    /// Real skyline figures: UPKR wins on payload (1709 vs 1781) yet loses
    /// on linked size once the decruncher stub is counted - the table has
    /// to make exactly that visible.
    #[test]
    fn comparison_table_shows_payload_versus_stub_and_marks_the_current_one() {
        let rows = vec![
            row("CRUNCHER_ZX0_BACKWARD", Some(1870), Some(1781), true),
            row("CRUNCHER_UPKR", Some(1901), Some(1709), false),
            row("CRUNCHER_BAD", None, None, false),
        ];
        let table = render_comparison_table(&rows, true);
        let lines: Vec<&str> = table.lines().collect();
        assert!(lines[0].contains("Free"), "{table}");
        assert!(lines[2].contains("| 1 | CRUNCHER_ZX0_BACKWARD (current) | 1870 | best | 1781 | 89 | 50 |"), "{table}");
        assert!(lines[3].contains("| 2 | CRUNCHER_UPKR | 1901 | +31 | 1709 | 192 | 19 |"), "{table}");
        assert!(lines[4].contains("FAILED: error: boom / pipe more"), "{table}");
        assert_eq!(lines[2].matches('|').count(), lines[0].matches('|').count(), "{table}");
        assert_eq!(lines[4].matches('|').count(), lines[0].matches('|').count(), "{table}");
    }

    #[test]
    fn comparison_table_omits_the_free_column_without_a_budget() {
        let table = render_comparison_table(&[row("A", Some(10), Some(8), false)], false);
        assert!(!table.contains("Free"), "{table}");
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines[0].matches('|').count(), lines[2].matches('|').count(), "{table}");
    }

    #[test]
    fn current_assignment_value_strips_the_trailing_comment() {
        assert_eq!(
            current_assignment_value(REAL_LINK_SKY_EXCERPT, "SELECTED_CRUNCHER").as_deref(),
            Some("CRUNCHER_ZX0_BACKWARD")
        );
    }

    #[test]
    fn compare_link_sizes_rejects_an_empty_cruncher_list() {
        let err = compare_link_sizes(CompareLinkSizesInput {
            bnd_path: "build.bnd".to_string(),
            target: None,
            sym_path: "out.sym".to_string(),
            cruncher_source_path: "link_sky.asm".to_string(),
            variable_name: "SELECTED_CRUNCHER".to_string(),
            crunchers: vec![],
            extra_lines: None,
            target_size: None,
            overhead_bytes: None
        })
        .expect_err("an empty candidate list should be rejected up front");
        assert_eq!(err.kind, "invalid_input");
    }

    /// A file that does not follow the expected convention must fail the
    /// whole call (and must not have been modified) rather than silently
    /// building some or all candidates unmodified.
    #[test]
    fn compare_link_sizes_fails_closed_when_the_variable_is_not_found() {
        let dir = camino_tempfile::tempdir().unwrap();
        let source_path = dir.path().join("link_sky.asm");
        let original = "org 0x4000\n ret\n";
        fs_err::write(&source_path, original).unwrap();

        let err = compare_link_sizes(CompareLinkSizesInput {
            bnd_path: dir.path().join("build.bnd").to_string(),
            target: None,
            sym_path: dir.path().join("out.sym").to_string(),
            cruncher_source_path: source_path.to_string(),
            variable_name: "SELECTED_CRUNCHER".to_string(),
            crunchers: vec!["CRUNCHER_ZX7".to_string()],
            extra_lines: None,
            target_size: None,
            overhead_bytes: None
        })
        .expect_err("a file without the convention must be rejected, not guessed at");
        assert_eq!(err.kind, "invalid_input");
        assert_eq!(
            fs_err::read_to_string(&source_path).unwrap(),
            original,
            "the source file must be untouched after a failed call"
        );
    }
}
