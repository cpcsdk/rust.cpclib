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
    pub success: bool
}

/// A line of build output. `true` marks stderr (surfaced as a louder log
/// level so it stands out in the client's output channel).
pub type OutputLine = (bool, String);

/// Forwards build progress and task stdout/stderr to a channel in real time,
/// so the editor can show it as the build runs (mirrors what a terminal used
/// to show before rules were run in-process).
#[derive(Clone)]
struct StreamingObserver {
    tx: UnboundedSender<OutputLine>
}

impl std::fmt::Debug for StreamingObserver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamingObserver").finish()
    }
}

impl EventObserver for StreamingObserver {
    fn emit_stdout(&self, s: &str) {
        let _ = self.tx.send((false, s.to_string()));
    }

    fn emit_stderr(&self, s: &str) {
        let _ = self.tx.send((true, s.to_string()));
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
    let tgt_keys: Vec<&str> = cpclib_bndbuild::lsp::RULE_KEYS
        .iter()
        .find(|k| k.names.contains(&"targets"))
        .map(|k| k.names.to_vec())
        .unwrap_or_default();

    let rule_line = BuildFileAnalyzer::find_target_line(&text, failing_target, &tgt_keys)
        .or_else(|| BuildFileAnalyzer::find_target_line(&text, requested_rule, &tgt_keys))
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

    RuleRunOutcome {
        message: format!("Rule '{requested_rule}' failed: {msg}"),
        diagnostics: vec![diagnostic],
        success: false
    }
}

/// Enumerate the task lines declared under the rule starting at `rule_line`,
/// in source order, as `(line_index, task_content)`. A task line is either a
/// `- item` in a task list, or the value of a scalar `cmd:`/`tasks:`/... key
/// (the whole rule then has exactly that one task).
fn task_lines_in_rule(text: &str, rule_line: usize) -> Vec<(usize, &str)> {
    let lines: Vec<&str> = text.lines().collect();
    let rule_indent = lines
        .get(rule_line)
        .map(|l| l.len() - l.trim_start().len())
        .unwrap_or(0);
    let task_keys: Vec<&str> = cpclib_bndbuild::lsp::RULE_KEYS
        .iter()
        .find(|k| k.names.contains(&"tasks"))
        .map(|k| k.names.to_vec())
        .unwrap_or_default();

    let mut out = Vec::new();
    for (idx, line) in lines.iter().enumerate().skip(rule_line + 1) {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
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
            if task_keys.contains(&key.trim())
                && !value.is_empty()
                && !value.starts_with('>')
                && !value.starts_with('|')
            {
                value
            }
            else {
                continue;
            }
        }
        else {
            continue;
        };

        out.push((idx, content));
    }
    out
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
    let tgt_keys: Vec<&str> = cpclib_bndbuild::lsp::RULE_KEYS
        .iter()
        .find(|k| k.names.contains(&"targets"))
        .map(|k| k.names.to_vec())
        .unwrap_or_default();

    ignored
        .into_iter()
        .filter_map(|entry| {
            let rule_line = BuildFileAnalyzer::find_target_line(&text, &entry.rule, &tgt_keys)?;
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
fn strip_ansi(s: &str) -> String {
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
    use super::*;

    fn doc(dir: &std::path::Path, content: &str) -> Document {
        let path = dir.join("bndbuild.yml");
        std::fs::write(&path, content).unwrap();
        let uri = Url::from_file_path(&path).unwrap();
        Document::new(uri, content.to_string(), 1)
    }

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

    #[test]
    fn successful_rule_yields_no_diagnostics() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let content = "- tgt: fine\n  phony: true\n  cmd: echo all good\n";
        let document = doc(tmp.path().as_std_path(), content);

        let outcome = BuildFileAnalyzer::new().run_rule(&document, "fine", None);
        assert!(outcome.success, "{}", outcome.message);
        assert!(outcome.diagnostics.is_empty());
    }

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
}
