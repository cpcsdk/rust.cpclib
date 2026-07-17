//! Execution of a bndbuild rule from the editor (the "▶ Run" code lens),
//! streaming its output back to the client as it runs, and mapping a build
//! failure back onto the source line that caused it.

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

impl BuildFileAnalyzer {
    /// Build `rule` of the build file behind `document` (blocking: run this
    /// on a worker thread). On failure, returns a diagnostic highlighting the
    /// failing rule — refined to the responsible task line when it can be
    /// identified from the error message.
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
            return failure_outcome(document, rule, rule, "invalid build file path".to_string());
        };
        let Some(utf8_path) = path.to_str().map(camino::Utf8PathBuf::from)
        else {
            return failure_outcome(document, rule, rule, "non-UTF8 build file path".to_string());
        };

        // `force_serial = false`: same parallel scheduling as the CLI.
        let builder = cpclib_bndbuild::BndBuilder::from_path(&utf8_path, false);
        let mut builder = match builder {
            Ok((_, builder)) => builder,
            Err(e) => {
                return failure_outcome(document, rule, rule, strip_ansi(&e.to_string()));
            }
        };

        if let Some(tx) = output {
            builder.add_observer(BndBuilderObserverRc::new(StreamingObserver { tx }));
        }

        match builder.execute(rule) {
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
                failure_outcome(document, rule, &failing_target, strip_ansi(&msg))
            }
        }
    }
}

/// Build the failure outcome: locate the best line to highlight and produce
/// the diagnostic.
fn failure_outcome(
    document: &Document,
    requested_rule: &str,
    failing_target: &str,
    msg: String
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

    // Try to refine to the task line responsible for the failure: within the
    // failing rule's block, prefer the task line sharing the most tokens with
    // the error message.
    let line_idx = best_failing_line(&text, rule_line as usize, &msg).unwrap_or(rule_line as usize);

    let line_text = text.lines().nth(line_idx).unwrap_or_default();
    let start_char = line_text.len() - line_text.trim_start().len();

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
        message: format!("Rule '{failing_target}' failed: {msg}"),
        ..Default::default()
    };

    RuleRunOutcome {
        message: format!("Rule '{requested_rule}' failed: {msg}"),
        diagnostics: vec![diagnostic],
        success: false
    }
}

/// Within the rule block starting at `rule_line`, find the task line whose
/// command shares the most distinctive tokens with the error message.
fn best_failing_line(text: &str, rule_line: usize, msg: &str) -> Option<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let rule_indent = lines
        .get(rule_line)
        .map(|l| l.len() - l.trim_start().len())
        .unwrap_or(0);

    let mut best: Option<(usize, usize)> = None; // (score, line)
    for (idx, line) in lines.iter().enumerate().skip(rule_line + 1) {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        let indent = line.len() - trimmed.len();
        if indent <= rule_indent {
            break; // left the rule block
        }

        // Only task-looking content: `cmd: xxx` values or `- xxx` items whose
        // first word is not a rule key.
        let content = if let Some(item) = trimmed.strip_prefix("- ") {
            item
        }
        else if let Some((key, value)) = trimmed.split_once(':') {
            if cpclib_bndbuild::lsp::RULE_KEYS
                .iter()
                .find(|k| k.names.contains(&"tasks"))
                .is_some_and(|k| k.names.contains(&key.trim()))
            {
                value.trim_start()
            }
            else {
                continue;
            }
        }
        else {
            continue;
        };

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
}
