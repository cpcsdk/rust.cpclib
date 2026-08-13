//! Execution of a bndbuild rule from the editor (the "▶ Run" code lens),
//! streaming its output back to the client as it runs, and mapping a build
//! failure back onto the source line that caused it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cpclib_bndbuild::event::{
    BndBuilderEvent, BndBuilderObserved, BndBuilderObserver, BndBuilderObserverRc
};
use cpclib_common::event::EventObserver;
use tokio::sync::mpsc::UnboundedSender;
use tower_lsp::lsp_types::*;

use super::BuildFileAnalyzer;
use crate::common::document::Document;

/// Result of running a rule on behalf of the editor.
pub struct RuleRunOutcome {
    /// Human-readable summary for a client notification.
    pub message: String,
    /// Error diagnostic anchored on the failing line; empty on success.
    pub diagnostics: Vec<Diagnostic>,
    /// A diagnostic for a *different* file the failing tool's own output
    /// referenced (e.g. `basm`'s own `┌─ sna.asm:24:5` syntax-error locus) —
    /// `None` when the failure carries no such reference, or it couldn't be
    /// resolved to a real file. Unlike `diagnostics` (always for the build
    /// file itself, republished fresh on every run), the caller is expected
    /// to store this and keep it visible on the target file until the next
    /// successful build, not just until that file's own next edit.
    pub build_error: Option<(Url, Diagnostic)>,
    pub success: bool
}

/// A line of build output. `true` marks stderr (surfaced as a louder log
/// level so it stands out in the client's output channel).
pub type OutputLine = (bool, String);

/// Forwards build progress and task stdout/stderr to a channel in real time,
/// so the editor can show it as the build runs (mirrors what a terminal used
/// to show before rules were run in-process). `pub(crate)` so other LSP
/// features that stream task output (e.g. `locomotive::run`'s "run BASIC in
/// emulator") can reuse it instead of writing a second observer type.
#[derive(Clone)]
pub(crate) struct StreamingObserver {
    tx: UnboundedSender<OutputLine>
}

impl StreamingObserver {
    pub(crate) fn new(tx: UnboundedSender<OutputLine>) -> Self {
        Self { tx }
    }
}

impl std::fmt::Debug for StreamingObserver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamingObserver").finish()
    }
}

impl EventObserver for StreamingObserver {
    fn emit_stdout(&self, s: &str) {
        // ANSI-stripped before ever reaching the channel: every consumer
        // (the CodeLens run path's `log_message` forwarding in
        // `backend.rs`, and `locomotive::run`'s BASIC-emulator-launch
        // streaming, which reuses this same observer) would otherwise
        // forward raw escape sequences into VS Code's Output channel, which
        // doesn't render them as color - they'd just show up as garbled
        // control characters, actively hurting readability rather than
        // helping it.
        let _ = self.tx.send((false, strip_ansi(s)));
    }

    fn emit_stderr(&self, s: &str) {
        let _ = self.tx.send((true, strip_ansi(s)));
    }
}

impl BndBuilderObserver for StreamingObserver {
    fn update(&self, event: BndBuilderEvent) {
        match event {
            BndBuilderEvent::Stdout(s) | BndBuilderEvent::TaskStdout(_, _, s) => {
                self.emit_stdout(s)
            },
            BndBuilderEvent::Stderr(s) | BndBuilderEvent::TaskStderr(_, _, s) => {
                self.emit_stderr(s)
            },
            BndBuilderEvent::StartRule { rule, nb, out_of } => {
                self.emit_stdout(&format!("→ [{nb}/{out_of}] {rule}"));
            },
            BndBuilderEvent::SkippedRule(p) => {
                self.emit_stdout(&format!("  {p} is already up to date"));
            },
            BndBuilderEvent::StopRule(p) => {
                self.emit_stdout(&format!("✓ {p}"));
            },
            BndBuilderEvent::FailedRule(p) => {
                self.emit_stderr(&format!("✗ {p} failed"));
            },
            _ => {}
        }
    }
}

/// Tracks, per rule: the 0-based index of the most recently *started* task,
/// the combined stdout+stderr output produced by that task so far, and every
/// task whose failure was ignored (command prefixed with `-`).
///
/// The executor runs a rule's tasks strictly in source order and stops at
/// the first one that fails (it never starts another task for that rule
/// afterwards - see `BndBuilder::execute_rule`), so after a rule fails, the
/// most-recently-started task is exactly the one that failed, and its
/// buffered output (reset on every `StartTask`) is exactly that task's own
/// output. This gives a precise line-highlight and full failure context
/// instead of guessing from the (often generic, e.g. "Error while launching
/// the command.") error string alone.
///
/// Note: on the PTY-based runner path (the default outside macOS's legacy
/// pipe path), the child's stdout and stderr are merged by the OS before
/// reaching us, so *all* of a task's output - including what would be stderr
/// on a real terminal - arrives as `TaskStdout`. Buffering both event kinds
/// together is what makes this robust across platforms.
#[derive(Clone, Default)]
struct TaskTracker {
    index_by_rule: Arc<Mutex<HashMap<String, usize>>>,
    output_by_rule: Arc<Mutex<HashMap<String, String>>>,
    ignored_errors: Arc<Mutex<Vec<IgnoredTaskError>>>
}

/// One task whose failure was ignored (`-command` in the build file).
struct IgnoredTaskError {
    rule: String,
    task_index: usize,
    message: String
}

impl TaskTracker {
    fn failed_index_for(&self, rule: &str) -> Option<usize> {
        self.index_by_rule.lock().unwrap().get(rule).copied()
    }

    fn output_for(&self, rule: &str) -> String {
        self.output_by_rule
            .lock()
            .unwrap()
            .get(rule)
            .cloned()
            .unwrap_or_default()
    }

    fn take_ignored_errors(&self) -> Vec<IgnoredTaskError> {
        std::mem::take(&mut *self.ignored_errors.lock().unwrap())
    }
}

impl std::fmt::Debug for TaskTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskTracker").finish()
    }
}

impl EventObserver for TaskTracker {
    fn emit_stdout(&self, _s: &str) {}

    fn emit_stderr(&self, _s: &str) {}
}

impl BndBuilderObserver for TaskTracker {
    fn update(&self, event: BndBuilderEvent) {
        match event {
            BndBuilderEvent::StartTask(Some(rule), _) => {
                let key = rule.to_string();
                let mut idx_map = self.index_by_rule.lock().unwrap();
                let next = idx_map.get(&key).map_or(0, |i| i + 1);
                idx_map.insert(key.clone(), next);
                self.output_by_rule
                    .lock()
                    .unwrap()
                    .insert(key, String::new());
            },
            BndBuilderEvent::TaskStdout(rule, _, s) | BndBuilderEvent::TaskStderr(rule, _, s) => {
                let mut out = self.output_by_rule.lock().unwrap();
                out.entry(rule.to_string()).or_default().push_str(s);
            },
            BndBuilderEvent::TaskIgnoredError(rule, _, message) => {
                let key = rule.to_string();
                let task_index = self
                    .index_by_rule
                    .lock()
                    .unwrap()
                    .get(&key)
                    .copied()
                    .unwrap_or(0);
                self.ignored_errors.lock().unwrap().push(IgnoredTaskError {
                    rule: key,
                    task_index,
                    message: message.to_string()
                });
            },
            _ => {}
        }
    }
}

impl BuildFileAnalyzer {
    /// Build `rule` of the build file behind `document` (blocking: run this
    /// on a worker thread). On failure, returns a diagnostic highlighting the
    /// failing rule — refined to the specific failing task line, identified
    /// from the executor's own task-start events rather than guessed, and
    /// carrying that task's full captured output for context. Also surfaces
    /// every task whose failure was *ignored* (`-command`) as a warning
    /// diagnostic on its own line, whether or not the rule ultimately failed.
    ///
    /// When `output` is provided, every line of build progress/stdout/stderr
    /// is forwarded to it as the build runs.
    pub fn run_rule(
        &self,
        document: &Document,
        rule: &str,
        output: Option<UnboundedSender<OutputLine>>
    ) -> RuleRunOutcome {
        let Ok(path) = document.uri.to_file_path()
        else {
            return failure_outcome(
                document,
                rule,
                rule,
                "invalid build file path".to_string(),
                None,
                ""
            );
        };
        let Some(utf8_path) = path.to_str().map(camino::Utf8PathBuf::from)
        else {
            return failure_outcome(
                document,
                rule,
                rule,
                "non-UTF8 build file path".to_string(),
                None,
                ""
            );
        };

        // `force_serial = false`: same parallel scheduling as the CLI.
        let builder = cpclib_bndbuild::BndBuilder::from_path(&utf8_path, false);
        let mut builder = match builder {
            Ok((_, builder)) => builder,
            Err(e) => {
                return failure_outcome(document, rule, rule, strip_ansi(&e.to_string()), None, "");
            }
        };

        let task_tracker = TaskTracker::default();
        builder.add_observer(BndBuilderObserverRc::new(task_tracker.clone()));

        if let Some(tx) = output {
            builder.add_observer(BndBuilderObserverRc::new(StreamingObserver { tx }));
        }

        let mut outcome = match builder.execute(rule) {
            Ok(()) => {
                RuleRunOutcome {
                    message: format!("Rule '{rule}' built successfully"),
                    diagnostics: Vec::new(),
                    build_error: None,
                    success: true
                }
            },
            Err(e) => {
                // The failing target may be a *dependency* of the clicked
                // rule: anchor the diagnostic on the rule that actually failed.
                let (failing_target, msg) = match &e {
                    cpclib_bndbuild::BndBuilderError::ExecuteError { fname, msg } => {
                        (fname.clone(), msg.clone())
                    },
                    cpclib_bndbuild::BndBuilderError::DefaultTargetError { source } => {
                        match source.as_ref() {
                            cpclib_bndbuild::BndBuilderError::ExecuteError { fname, msg } => {
                                (fname.clone(), msg.clone())
                            },
                            other => (rule.to_string(), other.to_string())
                        }
                    },
                    other => (rule.to_string(), other.to_string())
                };
                let failed_task_index = task_tracker.failed_index_for(&failing_target);
                let full_output = strip_ansi(&task_tracker.output_for(&failing_target));
                failure_outcome(
                    document,
                    rule,
                    &failing_target,
                    strip_ansi(&msg),
                    failed_task_index,
                    &full_output
                )
            }
        };

        outcome.diagnostics.extend(ignored_error_diagnostics(
            document,
            task_tracker.take_ignored_errors()
        ));
        outcome
    }

    /// Run a single command (`task_index`, 0-based, in source order) from
    /// `rule`, bypassing the normal target-run path entirely: no dependency
    /// resolution, no up-to-date check, and no other task in the rule is
    /// touched. Directly calls the one `Task`'s own public `Task::execute`
    /// - the exact function a full dependency-aware run
    /// (`run_rule`/`BndBuilder::execute`) eventually calls per task anyway,
    /// so `ignore_errors()` ("-command") semantics are honored identically.
    pub fn run_single_task(
        &self,
        document: &Document,
        rule: &str,
        task_index: usize,
        output: Option<UnboundedSender<OutputLine>>
    ) -> RuleRunOutcome {
        let Ok(path) = document.uri.to_file_path()
        else {
            return failure_outcome(
                document,
                rule,
                rule,
                "invalid build file path".to_string(),
                None,
                ""
            );
        };
        let Some(utf8_path) = path.to_str().map(camino::Utf8PathBuf::from)
        else {
            return failure_outcome(
                document,
                rule,
                rule,
                "non-UTF8 build file path".to_string(),
                None,
                ""
            );
        };

        let builder = match cpclib_bndbuild::BndBuilder::from_path(&utf8_path, false) {
            Ok((_, b)) => b,
            Err(e) => {
                return failure_outcome(document, rule, rule, strip_ansi(&e.to_string()), None, "");
            }
        };

        let tx = output.unwrap_or_else(|| {
            let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
            tx
        });
        let observer = Arc::new(StreamingObserver::new(tx));

        // Shared with the `bndbuild --only-task RULE:INDEX` CLI flag
        // (`cpclib_bndbuild::builder::BndBuilder::execute_task`'s own doc
        // comment) - one canonical "run a single task with full rule
        // context" implementation, not two independently-maintained copies
        // of the same rule/task lookup.
        match builder.execute_task(rule, task_index, &observer) {
            Ok(()) => {
                RuleRunOutcome {
                    message: format!(
                        "Task #{} of rule '{rule}' executed successfully",
                        task_index + 1
                    ),
                    diagnostics: Vec::new(),
                    build_error: None,
                    success: true
                }
            },
            Err(e) => {
                let msg = strip_ansi(&e.to_string());
                failure_outcome(document, rule, rule, msg.clone(), Some(task_index), &msg)
            }
        }
    }

    /// Like `run_rule`, but for a rule embedded in a `.asm` file's own
    /// comments (`basm::embedded_bndbuild`) rather than a real standalone
    /// build file — `yaml_text`/`yaml_start_line` come from
    /// `basm::embedded_bndbuild::EmbeddedBndbuildBlock`; `host_document` is
    /// the *.asm* file itself, since there is no separate on-disk build
    /// file to load.
    ///
    /// Builds the `BndBuilder` from the extracted text directly
    /// (`BndBuilder::decode_from_reader` + `from_string`) instead of
    /// `BndBuilder::from_path`, with `working_directory` set to the `.asm`
    /// file's own parent directory so `{% include %}`/relative task paths
    /// resolve the same way a real build file's would.
    ///
    /// Reuses `failure_outcome`/`ignored_error_diagnostics` unmodified
    /// against a synthetic in-memory `Document` whose text is just the
    /// block's own YAML (so their internal task-line-identification logic
    /// runs against precise, block-relative line numbers) but whose `uri`
    /// stays the real `.asm` file's own (so a cross-file `build_error`
    /// reference still resolves correctly) — the resulting block-relative
    /// diagnostic line numbers are then shifted by `yaml_start_line` to
    /// land back on the right line in the real `.asm` file. `build_error`'s
    /// diagnostic, if any, already carries real, absolute coordinates for a
    /// *different* file and must not be shifted.
    pub(crate) fn run_embedded_rule(
        &self,
        host_document: &Document,
        yaml_text: &str,
        yaml_start_line: u32,
        rule: &str,
        output: Option<UnboundedSender<OutputLine>>
    ) -> RuleRunOutcome {
        let Ok(path) = host_document.uri.to_file_path()
        else {
            return failure_outcome(
                host_document,
                rule,
                rule,
                "invalid .asm file path".to_string(),
                None,
                ""
            );
        };
        let Some(utf8_path) = path.to_str().map(camino::Utf8PathBuf::from)
        else {
            return failure_outcome(
                host_document,
                rule,
                rule,
                "non-UTF8 .asm file path".to_string(),
                None,
                ""
            );
        };
        let working_directory = utf8_path.parent().map(|d| d.to_owned());
        let synthetic_name = camino::Utf8PathBuf::from(format!("{utf8_path}#{rule} (embedded)"));

        let rendered = cpclib_bndbuild::BndBuilder::decode_from_reader(
            std::io::Cursor::new(yaml_text.as_bytes()),
            working_directory.as_ref(),
            &Vec::<(String, String)>::new(),
            &synthetic_name
        );
        let rendered = match rendered {
            Ok(r) => r,
            Err(e) => {
                return failure_outcome(
                    host_document,
                    rule,
                    rule,
                    strip_ansi(&e.to_string()),
                    None,
                    ""
                );
            }
        };

        let builder =
            cpclib_bndbuild::BndBuilder::from_string(rendered, Some(&synthetic_name), false);
        let mut builder = match builder {
            Ok(b) => b,
            Err(e) => {
                return failure_outcome(
                    host_document,
                    rule,
                    rule,
                    strip_ansi(&e.to_string()),
                    None,
                    ""
                );
            }
        };

        let task_tracker = TaskTracker::default();
        builder.add_observer(BndBuilderObserverRc::new(task_tracker.clone()));

        if let Some(tx) = output {
            builder.add_observer(BndBuilderObserverRc::new(StreamingObserver { tx }));
        }

        let block_document = Document::new(host_document.uri.clone(), yaml_text.to_string(), 0);

        let mut outcome = match builder.execute(rule) {
            Ok(()) => {
                RuleRunOutcome {
                    message: format!("Rule '{rule}' built successfully"),
                    diagnostics: Vec::new(),
                    build_error: None,
                    success: true
                }
            },
            Err(e) => {
                let (failing_target, msg) = match &e {
                    cpclib_bndbuild::BndBuilderError::ExecuteError { fname, msg } => {
                        (fname.clone(), msg.clone())
                    },
                    cpclib_bndbuild::BndBuilderError::DefaultTargetError { source } => {
                        match source.as_ref() {
                            cpclib_bndbuild::BndBuilderError::ExecuteError { fname, msg } => {
                                (fname.clone(), msg.clone())
                            },
                            other => (rule.to_string(), other.to_string())
                        }
                    },
                    other => (rule.to_string(), other.to_string())
                };
                let failed_task_index = task_tracker.failed_index_for(&failing_target);
                let full_output = strip_ansi(&task_tracker.output_for(&failing_target));
                let mut o = failure_outcome(
                    &block_document,
                    rule,
                    &failing_target,
                    strip_ansi(&msg),
                    failed_task_index,
                    &full_output
                );
                for d in o.diagnostics.iter_mut() {
                    d.range.start.line += yaml_start_line;
                    d.range.end.line += yaml_start_line;
                }
                o
            }
        };

        let mut ignored =
            ignored_error_diagnostics(&block_document, task_tracker.take_ignored_errors());
        for d in ignored.iter_mut() {
            d.range.start.line += yaml_start_line;
            d.range.end.line += yaml_start_line;
        }
        outcome.diagnostics.extend(ignored);
        outcome
    }
}

/// Build the failure outcome: locate the best line to highlight and produce
/// the diagnostic. `full_output` is the failing task's own captured
/// stdout+stderr (may be empty, e.g. for failures not tied to a task at
/// all) and is appended to the diagnostic so hovering it shows everything
/// the command actually printed, not just the (often generic) error string.
fn failure_outcome(
    document: &Document,
    requested_rule: &str,
    failing_target: &str,
    msg: String,
    failed_task_index: Option<usize>,
    full_output: &str
) -> RuleRunOutcome {
    let text = document.text();

    let rule_line =
        BuildFileAnalyzer::find_target_line(&text, failing_target, &super::token::TGT_KEY_NAMES)
            .or_else(|| {
                BuildFileAnalyzer::find_target_line(
                    &text,
                    requested_rule,
                    &super::token::TGT_KEY_NAMES
                )
            })
            .unwrap_or(0);

    // Prefer the task line identified precisely from the executor's own
    // task-start events; fall back to a text-based guess (e.g. for failures
    // that aren't tied to a specific task, like a missing dependency rule),
    // and finally to the rule's own `tgt:` line.
    let line_idx = failed_task_index
        .and_then(|idx| nth_task_line(&text, rule_line as usize, idx))
        .or_else(|| best_failing_line(&text, rule_line as usize, &msg))
        .unwrap_or(rule_line as usize);

    let line_text = text.lines().nth(line_idx).unwrap_or_default();
    let start_char = line_text.len() - line_text.trim_start().len();

    let output = full_output.trim();
    let message = if output.is_empty() || output == msg.trim() {
        format!("Rule '{failing_target}' failed: {msg}")
    }
    else {
        format!("Rule '{failing_target}' failed: {msg}\n\n{output}")
    };

    let diagnostic = Diagnostic {
        range: Range {
            start: Position {
                line: line_idx as u32,
                character: start_char as u32
            },
            end: Position {
                line: line_idx as u32,
                character: line_text.len() as u32
            }
        },
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("bndbuild".to_string()),
        message,
        ..Default::default()
    };

    // In addition to the bnd-file-anchored diagnostic above, the failing
    // tool's own output may reference a specific *other* source file/line
    // (e.g. a `basm` syntax error) - surface that as a second diagnostic on
    // that file too, if it can be resolved to a real one on disk.
    //
    // Tried against `full_output` first (a real subprocess's captured
    // stdout+stderr), falling back to `msg` itself - `msg` is where the
    // locus line actually lives for an *in-process* task (e.g. `basm` run
    // as bndbuild's own `Assembler::Basm`/`TaskKind::Embedded`, not spawned
    // as a subprocess: no `TaskStdout`/`TaskStderr` events ever fire for it,
    // so `TaskTracker` never captures anything and `full_output` stays
    // empty). This also covers a rule whose failure got wrapped in
    // `BndBuilderError::AnyError("Error N:\n...")` by `BndBuilder::execute`'s
    // own multi-task aggregation (falls through to the generic `other` match
    // arm in `run_rule`/`run_embedded_rule`, so `msg` is that whole
    // aggregated string) - real case that surfaced this gap: clicking a
    // "test" rule whose `basm` task failed produced a correct-looking
    // `showMessage` (built from `msg`) but no clickable cross-file
    // diagnostic, because only `full_output` (empty here) was ever scanned.
    let build_error = extract_referenced_location(full_output)
        .or_else(|| extract_referenced_location(&msg))
        .and_then(|(path, ref_line, ref_col)| {
            let target_uri = resolve_referenced_path(&path, &document.uri)?;
            let line = ref_line.saturating_sub(1);
            let character = ref_col.saturating_sub(1);
            let diagnostic = Diagnostic {
                range: Range {
                    start: Position { line, character },
                    end: Position {
                        line,
                        character: character + 1
                    }
                },
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("bndbuild".to_string()),
                message: format!("Build error (from rule '{failing_target}'): {msg}"),
                ..Default::default()
            };
            Some((target_uri, diagnostic))
        });

    RuleRunOutcome {
        message: format!("Rule '{requested_rule}' failed: {msg}"),
        diagnostics: vec![diagnostic],
        build_error,
        success: false
    }
}

/// Extract a `path:line:col` location referenced in a failing tool's own
/// captured output (e.g. a `codespan-reporting`-rendered `basm` syntax
/// error) - `line`/`col` both 1-based, exactly as reported. Scans line by
/// line for one containing the `"┌─ "` locus marker `codespan-reporting`
/// actually renders by default in this workspace (`colored_errors` is a
/// default Cargo feature on `cpclib-asm`, empirically confirmed by running
/// the real `basm` binary against a broken file) or the ASCII `"--> "` form
/// (a differently-configured build, or a different tool). `path` may itself
/// contain `:` (rare on Linux, but not impossible), so only the *last two*
/// colon-separated segments are required to be plain digits.
fn extract_referenced_location(text: &str) -> Option<(String, u32, u32)> {
    for line in text.lines() {
        let after_marker = line
            .split_once("┌─ ")
            .or_else(|| line.split_once("--> "))
            .map(|(_, rest)| rest.trim());
        let Some(rest) = after_marker
        else {
            continue;
        };

        let mut parts = rest.rsplitn(3, ':');
        let (Some(col_str), Some(line_str), Some(path)) =
            (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let (Ok(col), Ok(line_no)) = (col_str.parse::<u32>(), line_str.parse::<u32>())
        else {
            continue;
        };
        if path.is_empty() {
            continue;
        }
        return Some((path.to_string(), line_no, col));
    }
    None
}

/// Resolve `path_str` (as reported by a failing tool, possibly absolute,
/// possibly relative to wherever it was invoked from) to a real file's
/// `Url`. Tries it directly first (covers an absolute path, the common case
/// for an in-process/embedded task); falls back to resolving it relative to
/// the build file's own directory (walking up to the nearest project-root
/// marker) via the same `resolve_include_path` basm's own include
/// navigation already uses. `None` when neither resolves to a real file.
fn resolve_referenced_path(path_str: &str, doc_uri: &Url) -> Option<Url> {
    // `canonicalize` (not a bare `.exists()` check) so a *relative*
    // `path_str` (the common case for an in-process/embedded task, which
    // reports paths relative to wherever `BndBuilder` last `set_current_dir`ed
    // to) resolves to a real absolute path before being handed to
    // `Url::from_file_path`, which silently rejects anything not already
    // absolute. The previous version's early `return` on a bare
    // `.exists()` check meant a relative path always failed to become a
    // URL and returned `None` for the whole function immediately - never
    // even trying the fallback strategies below.
    if let Ok(canonical) = std::fs::canonicalize(path_str)
        && let Ok(url) = Url::from_file_path(&canonical)
    {
        return Some(url);
    }
    if let Some(resolved) = crate::basm::definition::resolve_include_path(path_str, doc_uri) {
        return Url::from_file_path(resolved).ok();
    }
    // Last resort: `resolve_include_path` only walks *up* the ancestor
    // chain (one `dir.join(filename)` check per level) - it never looks
    // inside a sibling or child directory. A referenced file living
    // elsewhere in the same project (a common real layout, e.g.
    // `src/palettes.asm` referenced from a rule embedded in
    // `src/effects/sna.asm`) needs an actual recursive search, scoped to
    // the nearest project root (or the document's own directory if no
    // marker is found) so this doesn't walk arbitrarily far up the
    // filesystem. Matches by basename only, since `path_str` is usually a
    // bare filename with no directory component of its own.
    let search_root = crate::basm::definition::project_root_for(doc_uri).or_else(|| {
        doc_uri
            .to_file_path()
            .ok()?
            .parent()
            .map(|p| p.to_path_buf())
    })?;
    let basename = std::path::Path::new(path_str).file_name()?;
    crate::common::walk::files_under(&search_root)
        .into_iter()
        .find(|e| e.file_name() == basename)
        .and_then(|e| Url::from_file_path(e.path()).ok())
}

/// Enumerate the task lines declared under the rule starting at `rule_line`,
/// in source order, as `(line_index, task_content)`. A task line is either a
/// `- item` in a task list, or the value of a scalar `cmd:`/`tasks:`/... key
/// (the whole rule then has exactly that one task).
pub(super) fn task_lines_in_rule(text: &str, rule_line: usize) -> Vec<(usize, &str)> {
    let lines: Vec<&str> = text.lines().collect();
    let rule_indent = lines
        .get(rule_line)
        .map(|l| l.len() - l.trim_start().len())
        .unwrap_or(0);
    let mut out = Vec::new();
    let mut idx = rule_line + 1;
    while idx < lines.len() {
        let line = lines[idx];
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            idx += 1;
            continue;
        }
        let indent = line.len() - trimmed.len();
        if indent <= rule_indent {
            break; // left the rule block
        }

        let content = if let Some(item) = trimmed.strip_prefix("- ") {
            item
        }
        else if let Some((key, value)) = trimmed.split_once(':') {
            let value = value.trim_start();
            if super::token::TASK_KEY_NAMES.contains(&key.trim()) && !value.is_empty() {
                value
            }
            else {
                idx += 1;
                continue;
            }
        }
        else {
            idx += 1;
            continue;
        };

        out.push((idx, content));

        if is_block_scalar_header(content) {
            // `content` (`|`/`>`, optionally with a chomping/indentation
            // indicator) is one task whose real text is every following
            // line indented more than *this* line - not further tasks of
            // their own. Skip past all of them so they aren't mistaken for
            // separate `- item`/`key: value` tasks (which would otherwise
            // desync this function's task count from the real YAML
            // deserializer's `Rule::commands().len()`, which treats the
            // whole block as a single `Task`).
            idx += 1;
            while idx < lines.len() {
                let cont = lines[idx];
                let cont_trimmed = cont.trim_start();
                if cont_trimmed.is_empty() {
                    idx += 1;
                    continue;
                }
                let cont_indent = cont.len() - cont_trimmed.len();
                if cont_indent <= indent {
                    break;
                }
                idx += 1;
            }
        }
        else {
            idx += 1;
        }
    }
    out
}

/// Whether `content` (a `- `/`key:`-stripped value) is a YAML block-scalar
/// header (`|`, `>`, optionally followed by chomping (`+`/`-`) and/or an
/// explicit indentation-indicator digit) rather than a real one-line value.
fn is_block_scalar_header(content: &str) -> bool {
    let c = content.trim();
    matches!(c.as_bytes().first(), Some(b'|') | Some(b'>'))
        && c[1..]
            .bytes()
            .all(|b| b == b'+' || b == b'-' || b.is_ascii_digit())
}

/// Line of the `task_index`-th task (0-based, in source order) declared
/// under the rule starting at `rule_line`.
fn nth_task_line(text: &str, rule_line: usize, task_index: usize) -> Option<usize> {
    task_lines_in_rule(text, rule_line)
        .get(task_index)
        .map(|(idx, _)| *idx)
}

/// One WARNING diagnostic per task whose failure was ignored (`-command`),
/// anchored on that task's own line. Produced regardless of whether the
/// overall rule ultimately succeeded or failed.
fn ignored_error_diagnostics(
    document: &Document,
    ignored: Vec<IgnoredTaskError>
) -> Vec<Diagnostic> {
    if ignored.is_empty() {
        return Vec::new();
    }
    let text = document.text();

    ignored
        .into_iter()
        .filter_map(|entry| {
            let rule_line = BuildFileAnalyzer::find_target_line(
                &text,
                &entry.rule,
                &super::token::TGT_KEY_NAMES
            )?;
            let line_idx = nth_task_line(&text, rule_line as usize, entry.task_index)
                .unwrap_or(rule_line as usize);
            let line_text = text.lines().nth(line_idx).unwrap_or_default();
            let start_char = line_text.len() - line_text.trim_start().len();
            Some(Diagnostic {
                range: Range {
                    start: Position {
                        line: line_idx as u32,
                        character: start_char as u32
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: line_text.len() as u32
                    }
                },
                severity: Some(DiagnosticSeverity::WARNING),
                source: Some("bndbuild".to_string()),
                message: format!(
                    "Task error ignored (command prefixed with `-`) in rule '{}': {}",
                    entry.rule,
                    strip_ansi(&entry.message)
                ),
                ..Default::default()
            })
        })
        .collect()
}

/// Within the rule block starting at `rule_line`, find the task line whose
/// command shares the most distinctive tokens with the error message.
fn best_failing_line(text: &str, rule_line: usize, msg: &str) -> Option<usize> {
    let mut best: Option<(usize, usize)> = None; // (score, line)
    for (idx, content) in task_lines_in_rule(text, rule_line) {
        let score = content
            .split_whitespace()
            .filter(|tok| tok.len() > 3 && !tok.starts_with('-'))
            .filter(|tok| msg.contains(*tok))
            .count();
        if score > 0 && best.is_none_or(|(s, _)| score > s) {
            best = Some((score, idx));
        }
    }
    best.map(|(_, line)| line)
}

/// Remove ANSI escape sequences from tool output.
pub(crate) fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for n in chars.by_ref() {
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        }
        else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    fn doc(dir: &std::path::Path, content: &str) -> Document {
        let path = dir.join("bndbuild.yml");
        std::fs::write(&path, content).unwrap();
        let uri = Url::from_file_path(&path).unwrap();
        Document::new(uri, content.to_string(), 1)
    }

    /// Regression test: `StreamingObserver` feeds VS Code's Output channel
    /// (via `window/logMessage`, which doesn't render ANSI) - raw escape
    /// codes must never reach the channel, or they show up as garbled
    /// control characters instead of the color they'd have had in a real
    /// terminal.
    #[test]
    fn streaming_observer_strips_ansi_from_stdout_and_stderr() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let observer = StreamingObserver::new(tx);
        observer.emit_stdout("\u{1b}[32mok\u{1b}[0m");
        observer.emit_stderr("\u{1b}[31merror\u{1b}[0m");
        drop(observer);

        let (is_err1, line1) = rx.try_recv().unwrap();
        assert!(!is_err1);
        assert_eq!(line1, "ok");
        let (is_err2, line2) = rx.try_recv().unwrap();
        assert!(is_err2);
        assert_eq!(line2, "error");
    }

    /// Writes `content` to `filename` (typically a `.asm` name) and returns
    /// a `Document` for it — the host document `run_embedded_rule` needs,
    /// standing in for the real `.asm` file whose comments the rule was
    /// extracted from.
    fn doc_asm(dir: &std::path::Path, filename: &str, content: &str) -> Document {
        let path = dir.join(filename);
        std::fs::write(&path, content).unwrap();
        let uri = Url::from_file_path(&path).unwrap();
        Document::new(uri, content.to_string(), 1)
    }

    #[serial]
    #[test]
    fn failing_rule_yields_diagnostic_on_its_line() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let content = "- tgt: ok.txt\n  phony: true\n  cmd: echo fine\n\n- tgt: broken\n  phony: true\n  cmd: cp does_not_exist_anywhere.src dst.bin\n";
        let document = doc(tmp.path().as_std_path(), content);

        let outcome = BuildFileAnalyzer::new().run_rule(&document, "broken", None);
        assert!(!outcome.success);
        assert_eq!(outcome.diagnostics.len(), 1);
        let diag = &outcome.diagnostics[0];
        // Highlighted on the failing task line (6) or at least the rule line (4).
        assert!(
            diag.range.start.line == 6 || diag.range.start.line == 4,
            "unexpected line: {:?}",
            diag.range
        );
        assert!(diag.message.contains("broken"), "{}", diag.message);
    }

    #[serial]
    #[test]
    fn successful_rule_yields_no_diagnostics() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let content = "- tgt: fine\n  phony: true\n  cmd: echo all good\n";
        let document = doc(tmp.path().as_std_path(), content);

        let outcome = BuildFileAnalyzer::new().run_rule(&document, "fine", None);
        assert!(outcome.success, "{}", outcome.message);
        assert!(outcome.diagnostics.is_empty());
    }

    #[serial]
    #[test]
    fn run_single_task_executes_only_the_requested_command() {
        let tmp = camino_tempfile::tempdir().unwrap();
        // Two tasks - only the first must run.
        let content = "- tgt: multi\n  phony: true\n  cmd:\n   - echo first\n   - cp does_not_exist_anywhere.src dst.bin\n";
        let document = doc(tmp.path().as_std_path(), content);

        let outcome = BuildFileAnalyzer::new().run_single_task(&document, "multi", 0, None);
        assert!(outcome.success, "{}", outcome.message);
    }

    #[serial]
    #[test]
    fn run_single_task_handles_a_multiline_block_scalar_command() {
        let tmp = camino_tempfile::tempdir().unwrap();
        // A single task whose command spans several lines via `|` (the
        // real shape used by birthtro/src/build.bnd) - must still resolve
        // as task index 0, not fail to be found or run the wrong task.
        let content = "- tgt: out.txt\n  phony: true\n  cmd: |\n    echo hello \\\n    world\n";
        let document = doc(tmp.path().as_std_path(), content);

        let outcome = BuildFileAnalyzer::new().run_single_task(&document, "out.txt", 0, None);
        assert!(outcome.success, "{}", outcome.message);
    }

    #[test]
    fn task_lines_in_rule_treats_a_block_scalar_as_one_task() {
        let text = "- tgt: out.sna\n  cmd: |\n    basm --snapshot sna.asm -o out.sna\n        -DFOO=1\n\nunrelated: true\n";
        let tasks = task_lines_in_rule(text, 0);
        assert_eq!(tasks.len(), 1, "{tasks:?}");
        assert_eq!(tasks[0].0, 1);
    }

    #[test]
    fn task_lines_in_rule_handles_a_block_scalar_list_item_among_others() {
        let text = "- tgt: out\n  cmd:\n    - |\n      multi\n      line\n    - echo done\n";
        let tasks = task_lines_in_rule(text, 0);
        assert_eq!(tasks.len(), 2, "{tasks:?}");
        assert_eq!(tasks[0].0, 2); // "- |" line
        assert_eq!(tasks[1].0, 5); // "- echo done" line
    }

    #[serial]
    #[test]
    fn run_single_task_reports_failure_of_just_that_task() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let content = "- tgt: multi\n  phony: true\n  cmd:\n   - echo first\n   - cp does_not_exist_anywhere.src dst.bin\n";
        let document = doc(tmp.path().as_std_path(), content);

        let outcome = BuildFileAnalyzer::new().run_single_task(&document, "multi", 1, None);
        assert!(!outcome.success);
    }

    #[serial]
    #[test]
    fn run_single_task_out_of_range_index_fails_cleanly() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let content = "- tgt: one\n  phony: true\n  cmd: echo only\n";
        let document = doc(tmp.path().as_std_path(), content);

        let outcome = BuildFileAnalyzer::new().run_single_task(&document, "one", 5, None);
        assert!(!outcome.success);
    }

    #[serial]
    #[test]
    fn output_is_streamed_as_the_build_runs() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let content = "- tgt: fine\n  phony: true\n  cmd: echo hello from the build\n";
        let document = doc(tmp.path().as_std_path(), content);

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let outcome = BuildFileAnalyzer::new().run_rule(&document, "fine", Some(tx));
        assert!(outcome.success, "{}", outcome.message);

        let mut lines = Vec::new();
        while let Ok(line) = rx.try_recv() {
            lines.push(line);
        }
        assert!(
            lines
                .iter()
                .any(|(is_err, text)| !is_err && text.contains("hello from the build")),
            "expected the echoed text to be streamed, got: {lines:?}"
        );
    }

    #[serial]
    #[test]
    fn failure_output_is_streamed_too() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let content =
            "- tgt: broken\n  phony: true\n  cmd: cp does_not_exist_anywhere.src dst.bin\n";
        let document = doc(tmp.path().as_std_path(), content);

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let outcome = BuildFileAnalyzer::new().run_rule(&document, "broken", Some(tx));
        assert!(!outcome.success);

        let mut lines = Vec::new();
        while let Ok(line) = rx.try_recv() {
            lines.push(line);
        }
        assert!(
            !lines.is_empty(),
            "expected some output to be streamed on failure"
        );
    }

    #[serial]
    #[test]
    fn highlights_the_specific_failing_task_not_the_first_one() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let content = "- tgt: multi\n  phony: true\n  cmd:\n    - echo first task ok\n    - cp does_not_exist_anywhere.src dst.bin\n    - echo third task never runs\n";
        let document = doc(tmp.path().as_std_path(), content);

        let outcome = BuildFileAnalyzer::new().run_rule(&document, "multi", None);
        assert!(!outcome.success);
        assert_eq!(outcome.diagnostics.len(), 1);
        let diag = &outcome.diagnostics[0];
        // Line 4 is the second `- cp ...` task (0-based task index 1), not
        // the first task (line 3) nor the rule's `tgt:` line (line 0).
        assert_eq!(
            diag.range.start.line, 4,
            "expected the failing task's own line, got: {:?}",
            diag.range
        );
    }

    #[test]
    fn task_tracker_records_the_last_started_task_per_rule() {
        let tracker = TaskTracker::default();
        let rule = camino::Utf8Path::new("some/rule");
        let task = cpclib_bndbuild::task::Task::new_echo("hello");

        tracker.update(BndBuilderEvent::StartTask(Some(rule), &task));
        assert_eq!(tracker.failed_index_for("some/rule"), Some(0));
        tracker.update(BndBuilderEvent::StartTask(Some(rule), &task));
        assert_eq!(tracker.failed_index_for("some/rule"), Some(1));

        assert_eq!(tracker.failed_index_for("other/rule"), None);
    }

    #[test]
    fn task_tracker_buffers_output_per_current_task() {
        let tracker = TaskTracker::default();
        let rule = camino::Utf8Path::new("some/rule");
        let task = cpclib_bndbuild::task::Task::new_echo("hello");

        tracker.update(BndBuilderEvent::StartTask(Some(rule), &task));
        tracker.update(BndBuilderEvent::TaskStdout(
            rule,
            &task,
            "first task output\n"
        ));
        assert_eq!(tracker.output_for("some/rule"), "first task output\n");

        // A new task starting resets the buffer to just its own output.
        tracker.update(BndBuilderEvent::StartTask(Some(rule), &task));
        tracker.update(BndBuilderEvent::TaskStderr(
            rule,
            &task,
            "second task output\n"
        ));
        assert_eq!(tracker.output_for("some/rule"), "second task output\n");
    }

    #[test]
    fn task_tracker_records_ignored_errors_with_task_index() {
        let tracker = TaskTracker::default();
        let rule = camino::Utf8Path::new("some/rule");
        let task = cpclib_bndbuild::task::Task::new_echo("hello");

        tracker.update(BndBuilderEvent::StartTask(Some(rule), &task)); // index 0
        tracker.update(BndBuilderEvent::StartTask(Some(rule), &task)); // index 1
        tracker.update(BndBuilderEvent::TaskIgnoredError(rule, &task, "boom"));

        let ignored = tracker.take_ignored_errors();
        assert_eq!(ignored.len(), 1);
        assert_eq!(ignored[0].rule, "some/rule");
        assert_eq!(ignored[0].task_index, 1);
        assert_eq!(ignored[0].message, "boom");

        // Draining clears the list.
        assert!(tracker.take_ignored_errors().is_empty());
    }

    #[serial]
    #[test]
    fn ignored_task_error_is_reported_as_a_warning_not_a_failure() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let content = "- tgt: tolerant\n  phony: true\n  cmd:\n    - echo first task ok\n    - -cp does_not_exist_anywhere.src dst.bin\n    - echo third task still runs\n";
        let document = doc(tmp.path().as_std_path(), content);

        let outcome = BuildFileAnalyzer::new().run_rule(&document, "tolerant", None);
        assert!(
            outcome.success,
            "an ignored error must not fail the rule: {}",
            outcome.message
        );
        assert_eq!(outcome.diagnostics.len(), 1);
        let diag = &outcome.diagnostics[0];
        assert_eq!(diag.severity, Some(DiagnosticSeverity::WARNING));
        // Anchored on the ignored task's own line (index 1), not task 0 or 2.
        assert_eq!(
            diag.range.start.line, 4,
            "unexpected line: {:?}",
            diag.range
        );
        assert!(diag.message.contains("ignored"), "{}", diag.message);
    }

    #[serial]
    #[test]
    fn failure_diagnostic_includes_the_command_s_full_output() {
        // `extern` shells out via ExternRunner (the real PTY-based path used
        // for every delegated/external command), whose own returned error
        // string is just a generic "Error while launching the command." -
        // all the actually useful detail only ever existed in the process's
        // own output. `cat` on a missing file reliably writes a clear error
        // to stderr and exits non-zero on any Unix system.
        let tmp = camino_tempfile::tempdir().unwrap();
        let content =
            "- tgt: broken\n  phony: true\n  cmd: extern cat /this/path/does/not/exist12345\n";
        let document = doc(tmp.path().as_std_path(), content);

        let outcome = BuildFileAnalyzer::new().run_rule(&document, "broken", None);
        assert!(!outcome.success);
        assert_eq!(outcome.diagnostics.len(), 1);
        let diag = &outcome.diagnostics[0];
        assert!(
            diag.message.contains("exist12345"),
            "diagnostic should include the command's actual output, not just \
             the generic error string, got: {}",
            diag.message
        );
    }

    #[test]
    fn extract_referenced_location_finds_the_unicode_locus_line() {
        // The real, empirically-confirmed shape `codespan-reporting`
        // renders by default in this workspace (`colored_errors` is a
        // default Cargo feature on `cpclib-asm`).
        let text = "error: Syntax error\n  ┌─ /tmp/broken.asm:2:7\n  │\n2 │ ld a, ,\n  │       ^ invalid LD: wrong source\n";
        assert_eq!(
            extract_referenced_location(text),
            Some(("/tmp/broken.asm".to_string(), 2, 7))
        );
    }

    #[test]
    fn extract_referenced_location_finds_the_ascii_locus_line_too() {
        let text = "error: Syntax error\n  --> /tmp/broken.asm:2:7\n";
        assert_eq!(
            extract_referenced_location(text),
            Some(("/tmp/broken.asm".to_string(), 2, 7))
        );
    }

    /// Regression test for a real repro (`birthtro/src/build.bnd`'s `test`
    /// rule, which depends on a rule whose own `basm` sub-assembly fails):
    /// the failing tool's own error message, aggregated by
    /// `BndBuilderError`'s generic `Display` for a nested
    /// dependency-of-a-dependency failure (not the more specific
    /// `ExecuteError`/`DefaultTargetError` variants `run_rule` special-cases),
    /// still carries the real locus line and must still be found.
    #[test]
    fn extract_referenced_location_finds_it_inside_an_aggregated_nested_dependency_error() {
        let text = "Error 1:\nUnable to build birthtro.sna: Error while assembling.\nAssembling error:\nerror: FAIL: \n  --> sna.asm:4:1\n  |\n4 | fail\n  | ^^^^\n\n.";
        assert_eq!(
            extract_referenced_location(text),
            Some(("sna.asm".to_string(), 4, 1))
        );
    }

    #[test]
    fn extract_referenced_location_handles_a_path_containing_a_colon() {
        let text = "  ┌─ C:/weird/path.asm:10:3\n";
        assert_eq!(
            extract_referenced_location(text),
            Some(("C:/weird/path.asm".to_string(), 10, 3))
        );
    }

    #[test]
    fn extract_referenced_location_returns_none_without_a_locus_line() {
        let text = "Error while launching the command.\nsome generic message\n";
        assert_eq!(extract_referenced_location(text), None);
    }

    #[test]
    fn extract_referenced_location_returns_none_for_a_malformed_locus_line() {
        let text = "  ┌─ not_a_real_location\n";
        assert_eq!(extract_referenced_location(text), None);
    }

    #[test]
    fn failure_outcome_surfaces_a_cross_file_diagnostic_for_a_referenced_source_file() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let bnd_content = "- tgt: broken\n  phony: true\n  cmd: basm broken.asm\n";
        let document = doc(tmp.path().as_std_path(), bnd_content);
        let asm_path = tmp.path().as_std_path().join("broken.asm");
        std::fs::write(&asm_path, "ld a, ,\n").unwrap();

        let full_output = format!(
            "error: Syntax error\n  ┌─ {}:1:7\n  │\n1 │ ld a, ,\n  │       ^ invalid LD: wrong source\n",
            asm_path.display()
        );
        let outcome = failure_outcome(
            &document,
            "broken",
            "broken",
            "invalid LD: wrong source".to_string(),
            None,
            &full_output
        );

        let (target_uri, diag) = outcome
            .build_error
            .expect("expected a cross-file diagnostic");
        assert_eq!(target_uri, Url::from_file_path(&asm_path).unwrap());
        assert_eq!(
            diag.range.start,
            Position {
                line: 0,
                character: 6
            }
        );
        assert!(diag.message.contains("Build error"), "{}", diag.message);
        assert!(diag.message.contains("invalid LD"), "{}", diag.message);
    }

    #[test]
    fn failure_outcome_finds_the_locus_line_inside_msg_when_full_output_is_empty() {
        // Regression test for a real failure reported against the shipped
        // embedded-rule feature: a `basm` task run as bndbuild's own
        // in-process `Assembler::Basm` (`TaskKind::Embedded`, not a spawned
        // subprocess) never emits `TaskStdout`/`TaskStderr` events, so
        // `TaskTracker` never captures anything and `full_output` stays
        // empty - the locus line only ever exists inside `msg` itself (and,
        // in the real case that triggered this, `msg` was actually the
        // whole "Error 1:\n..." string `BndBuilder::execute`'s own
        // multi-task-failure aggregation produces). Without falling back to
        // scanning `msg`, no cross-file diagnostic was ever built, so the
        // referenced source file's error line was never highlighted/
        // clickable in the editor.
        let tmp = camino_tempfile::tempdir().unwrap();
        let bnd_content = "- tgt: test\n  phony: true\n  cmd: basm demo_code.asm\n";
        let document = doc(tmp.path().as_std_path(), bnd_content);
        let asm_path = tmp.path().as_std_path().join("demo_code.asm");
        std::fs::write(&asm_path, "    jp demo_run\n").unwrap();

        let msg = format!(
            "Error 1:\nUnable to build birthtro.sna: Error while assembling.\n\
             Assembling error:\nerror: Unknown symbol: demo_run\n   --> {}:1:8\n   |\n\
             1 |     jp demo_run\n   |        ^^^^^^^^\n   |\n   = Closest one is: demo_init\n",
            asm_path.display()
        );
        let outcome = failure_outcome(&document, "test", "test", msg, None, "");

        let (target_uri, diag) = outcome
            .build_error
            .expect("expected a cross-file diagnostic even though full_output is empty");
        assert_eq!(target_uri, Url::from_file_path(&asm_path).unwrap());
        assert_eq!(diag.range.start.line, 0);
        assert!(diag.message.contains("Unknown symbol"), "{}", diag.message);
    }

    #[test]
    fn resolve_referenced_path_finds_a_file_in_a_different_subdirectory() {
        // Regression test for a real report: `resolve_include_path` only
        // walks *up* the ancestor chain (one `dir.join(filename)` check per
        // level) - a referenced file living in a *different* subdirectory
        // of the same project (e.g. the build file at the project root,
        // the referenced source under `src/`) was never found, so no
        // cross-file diagnostic - and no clickable/highlighted error - was
        // ever produced for it.
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("bndbuild.yml"), "marker").unwrap();
        let bnd_document = doc(
            tmp.path().as_std_path(),
            "- tgt: test\n  phony: true\n  cmd: basm palettes.asm\n"
        );
        let src_dir = tmp.path().join("src");
        std::fs::create_dir(&src_dir).unwrap();
        let palettes_path = src_dir.join("palettes.asm");
        std::fs::write(&palettes_path, "    jp demo_run\n").unwrap();

        let msg = format!(
            "error: Unknown symbol: demo_run\n   --> {}:1:8\n",
            "palettes.asm"
        );
        let outcome = failure_outcome(&bnd_document, "test", "test", msg, None, "");

        let (target_uri, _diag) = outcome
            .build_error
            .expect("expected the recursive subdirectory search to find palettes.asm");
        assert_eq!(target_uri, Url::from_file_path(&palettes_path).unwrap());
    }

    /// Regression test for a real repro (`birthtro/src/build.bnd`): when a
    /// referenced path is relative *and happens to exist relative to the
    /// process's current directory* (the real, common case for an
    /// in-process/embedded task, since `BndBuilder::from_path` itself
    /// `set_current_dir`s to the build file's own directory as a side
    /// effect of loading it) - `resolve_referenced_path` used to convert
    /// that relative path to a `Url` directly, which always fails
    /// (`Url::from_file_path` requires an absolute path) and, because that
    /// branch returned unconditionally, the whole function gave up right
    /// there instead of falling through to a strategy that actually works.
    #[serial]
    #[test]
    fn resolve_referenced_path_resolves_a_relative_path_that_exists_at_the_current_directory() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let asm_path = tmp.path().join("sna.asm");
        std::fs::write(&asm_path, "fail\n").unwrap();
        let bnd_document = doc(
            tmp.path().as_std_path(),
            "- tgt: test\n  cmd: basm sna.asm\n"
        );

        // Not saving/restoring the previous CWD: several other `#[serial]`
        // tests in this file (via `BndBuilder::from_path`'s own
        // `set_current_dir` side effect) already leave it pointing at their
        // own now-dropped tempdir by the time a later test runs, so
        // `std::env::current_dir()` itself isn't reliably callable here -
        // matches this file's existing convention of just accepting CWD
        // drift between serial tests.
        std::env::set_current_dir(tmp.path()).unwrap();
        let msg = "error: FAIL: \n  --> sna.asm:4:1\n  |\n4 | fail\n  | ^^^^\n".to_string();
        let outcome = failure_outcome(&bnd_document, "test", "test", msg, None, "");

        let (target_uri, diag) = outcome
            .build_error
            .expect("expected a cross-file diagnostic for the relative sna.asm reference");
        assert_eq!(target_uri, Url::from_file_path(&asm_path).unwrap());
        assert_eq!(diag.range.start.line, 3);
    }

    #[test]
    fn failure_outcome_has_no_cross_file_diagnostic_when_output_references_nothing() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let content =
            "- tgt: broken\n  phony: true\n  cmd: cp does_not_exist_anywhere.src dst.bin\n";
        let document = doc(tmp.path().as_std_path(), content);

        let outcome = failure_outcome(
            &document,
            "broken",
            "broken",
            "generic failure".to_string(),
            None,
            "cp: cannot stat 'does_not_exist_anywhere.src': No such file or directory\n"
        );
        assert!(outcome.build_error.is_none(), "{:?}", outcome.message);
    }

    // ── run_embedded_rule ────────────────────────────────────────────────

    #[serial]
    #[test]
    fn run_embedded_rule_executes_successfully_and_streams_output() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let host_document = doc_asm(
            tmp.path().as_std_path(),
            "foo.asm",
            "; #!bndbuild\n; - tgt: fine\n;   cmd: echo hello from embedded\nORG 0x8000\n"
        );
        let yaml_text = "- tgt: fine\n  cmd: echo hello from embedded";

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let outcome = BuildFileAnalyzer::new().run_embedded_rule(
            &host_document,
            yaml_text,
            1,
            "fine",
            Some(tx)
        );
        assert!(outcome.success, "{}", outcome.message);
        assert!(outcome.diagnostics.is_empty());

        let mut lines = Vec::new();
        while let Ok(line) = rx.try_recv() {
            lines.push(line);
        }
        assert!(
            lines
                .iter()
                .any(|(is_err, text)| !is_err && text.contains("hello from embedded")),
            "expected the echoed text to be streamed, got: {lines:?}"
        );
    }

    #[serial]
    #[test]
    fn run_embedded_rule_failure_anchors_the_diagnostic_on_the_correct_shifted_line() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let host_document = doc_asm(
            tmp.path().as_std_path(),
            "foo.asm",
            "ORG 0\n; #!bndbuild\n; - tgt: multi\n;   phony: true\n;   cmd:\n;    - echo first task ok\n;    - cp does_not_exist_anywhere.src dst.bin\n;    - echo third task never runs\n"
        );
        // Mirrors `highlights_the_specific_failing_task_not_the_first_one`'s
        // fixture: the failing task is at block-relative (0-based) line 4.
        let yaml_text = "- tgt: multi\n  phony: true\n  cmd:\n    - echo first task ok\n    - cp does_not_exist_anywhere.src dst.bin\n    - echo third task never runs";

        let outcome =
            BuildFileAnalyzer::new().run_embedded_rule(&host_document, yaml_text, 2, "multi", None);
        assert!(!outcome.success);
        assert_eq!(outcome.diagnostics.len(), 1);
        let diag = &outcome.diagnostics[0];
        // Block-relative line 4 shifted by yaml_start_line=2 -> absolute 6.
        assert_eq!(
            diag.range.start.line, 6,
            "unexpected line: {:?}",
            diag.range
        );
        assert!(diag.message.contains("multi"), "{}", diag.message);
    }

    #[serial]
    #[test]
    fn run_embedded_rule_resolves_relative_paths_against_the_asm_files_own_directory() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("data.txt"), "hello\n").unwrap();
        // `wc` (not `cat` - that's one of bndbuild's own internal command
        // aliases, `task::CATALOG_CMDS`, and would be dispatched to its
        // catalog tool instead of a real subprocess) so this genuinely
        // exercises the external-command path where `working_directory`
        // actually matters.
        let host_document = doc_asm(
            tmp.path().as_std_path(),
            "foo.asm",
            "; #!bndbuild\n; - tgt: count\n;   phony: true\n;   cmd: extern wc -l data.txt\n"
        );
        let yaml_text = "- tgt: count\n  phony: true\n  cmd: extern wc -l data.txt";

        let outcome =
            BuildFileAnalyzer::new().run_embedded_rule(&host_document, yaml_text, 1, "count", None);
        assert!(outcome.success, "{}", outcome.message);
    }

    #[serial]
    #[test]
    fn run_embedded_rule_still_surfaces_a_cross_file_diagnostic_for_a_referenced_source_file() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let asm_path = tmp.path().join("broken.asm");
        std::fs::write(&asm_path, "ld a, ,\n").unwrap();

        // A tiny failing script standing in for a real failing tool (e.g.
        // `basm`), whose own stderr references `broken.asm`'s locus in the
        // exact shape `codespan-reporting` renders - avoids depending on a
        // real `basm` binary being on PATH for this test.
        let script_path = tmp.path().join("fail.sh");
        std::fs::write(
            &script_path,
            format!(
                "#!/bin/sh\necho 'error: Syntax error' >&2\necho '  \u{250c}\u{2500} {}:1:7' >&2\nexit 1\n",
                asm_path
            )
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        // Referenced by its absolute path (not just "fail.sh", relative to
        // `working_directory`) - this test's own concern is cross-file
        // diagnostic surfacing, not CWD wiring (that's
        // `run_embedded_rule_resolves_relative_paths_against_the_asm_files_own_directory`'s
        // job), and other tests in this suite that run *concurrently* also
        // mutate the process-wide CWD via `decode_from_reader` (a known,
        // pre-existing hazard - see its own `XXX` comment), so avoiding a
        // relative reference here keeps this test robust regardless of
        // scheduling.
        let host_document = doc_asm(
            tmp.path().as_std_path(),
            "foo.asm",
            &format!("; #!bndbuild\n; - tgt: broken\n;   cmd: extern sh {script_path}\n")
        );
        let yaml_text = format!("- tgt: broken\n  cmd: extern sh {script_path}");

        let outcome = BuildFileAnalyzer::new().run_embedded_rule(
            &host_document,
            &yaml_text,
            1,
            "broken",
            None
        );
        assert!(!outcome.success);
        let (target_uri, diag) = outcome.build_error.unwrap_or_else(|| {
            panic!(
                "expected a cross-file diagnostic; message was: {}",
                outcome.message
            )
        });
        assert_eq!(target_uri, Url::from_file_path(&asm_path).unwrap());
        assert_eq!(diag.range.start.line, 0);
        assert!(diag.message.contains("Build error"), "{}", diag.message);
    }

    #[serial]
    #[test]
    fn run_embedded_rule_ignored_error_diagnostic_is_shifted_too() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let host_document = doc_asm(
            tmp.path().as_std_path(),
            "foo.asm",
            "ORG 0\n; #!bndbuild\n; - tgt: lax\n;   phony: true\n;   cmd:\n;    - -cp does_not_exist_anywhere.src dst.bin\n;    - echo still runs\n"
        );
        let yaml_text = "- tgt: lax\n  phony: true\n  cmd:\n    - -cp does_not_exist_anywhere.src dst.bin\n    - echo still runs";

        let outcome =
            BuildFileAnalyzer::new().run_embedded_rule(&host_document, yaml_text, 2, "lax", None);
        assert!(outcome.success, "{}", outcome.message);
        assert_eq!(outcome.diagnostics.len(), 1);
        let diag = &outcome.diagnostics[0];
        assert_eq!(diag.severity, Some(DiagnosticSeverity::WARNING));
        // Block-relative line 3 (the ignored `-cp ...` task) shifted by
        // yaml_start_line=2 -> absolute 5.
        assert_eq!(
            diag.range.start.line, 5,
            "unexpected line: {:?}",
            diag.range
        );
    }
}
