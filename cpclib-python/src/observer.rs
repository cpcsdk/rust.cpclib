//! How a running task or build talks to Python: its output is echoed to the
//! console, handed to a Python callback, and/or captured - never lost, never a
//! panic (every build event has a meaning here).

use std::fmt;
use std::sync::Mutex;

use cpclib_bndbuild::event::{BndBuilderEvent, BndBuilderObserver};
use cpclib_common::event::EventObserver;
use pyo3::prelude::*;

/// The kinds of event a callback receives, as `callback(kind, text)`:
///
/// - `stdout` / `stderr`: a tool's output;
/// - `rule-start`, `rule-stop`, `rule-skipped` (already up to date),
///   `rule-failed`: the target of a build rule;
/// - `task-start`, `task-stop` (text: the task, then the duration in ms),
///   `task-ignored-error` (a failure the build file said to ignore).
pub(crate) struct PyObserver {
    callback: Option<Py<PyAny>>,
    echo: bool,
    stdout: Mutex<String>,
    stderr: Mutex<String>
}

impl fmt::Debug for PyObserver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PyObserver")
            .field("callback", &self.callback.is_some())
            .field("echo", &self.echo)
            .finish()
    }
}

impl PyObserver {
    /// Output goes to `callback` when there is one; to the console otherwise.
    /// Either way it is also captured, see [`Self::captured`].
    pub(crate) fn new(callback: Option<Py<PyAny>>) -> Self {
        Self {
            echo: callback.is_none(),
            callback,
            stdout: Mutex::default(),
            stderr: Mutex::default()
        }
    }

    /// Everything the task wrote, as `(stdout, stderr)`.
    pub(crate) fn captured(&self) -> (String, String) {
        (
            self.stdout.lock().map(|s| s.clone()).unwrap_or_default(),
            self.stderr.lock().map(|s| s.clone()).unwrap_or_default()
        )
    }

    fn notify(&self, kind: &str, text: &str) {
        if let Some(callback) = &self.callback {
            Python::attach(|py| {
                // a failing callback must not take the build down with it
                if let Err(error) = callback.call1(py, (kind, text)) {
                    error.print(py);
                }
            });
        }
    }

    fn out(&self, text: &str) {
        if let Ok(mut stdout) = self.stdout.lock() {
            stdout.push_str(text);
            stdout.push('\n');
        }
        if self.echo {
            println!("{text}");
        }
        self.notify("stdout", text);
    }

    fn err(&self, text: &str) {
        if let Ok(mut stderr) = self.stderr.lock() {
            stderr.push_str(text);
            stderr.push('\n');
        }
        if self.echo {
            eprintln!("{text}");
        }
        self.notify("stderr", text);
    }
}

impl EventObserver for PyObserver {
    fn emit_stdout(&self, s: &str) {
        self.out(s);
    }

    fn emit_stderr(&self, s: &str) {
        self.err(s);
    }
}

impl BndBuilderObserver for PyObserver {
    fn update(&self, event: BndBuilderEvent) {
        use BndBuilderEvent::*;
        match event {
            Stdout(s) | TaskStdout(_, _, s) => self.out(s),
            Stderr(s) | TaskStderr(_, _, s) => self.err(s),
            StartRule { rule, .. } | StartRuleAlias { alias: rule, .. } => {
                self.notify("rule-start", rule.as_str())
            },
            StopRule(rule) => self.notify("rule-stop", rule.as_str()),
            SkippedRule(rule) => self.notify("rule-skipped", rule.as_str()),
            FailedRule(rule) => self.notify("rule-failed", rule.as_str()),
            StartTask(_, task) => self.notify("task-start", &task.to_string()),
            StopTask(_, task, duration) => {
                self.notify("task-stop", &format!("{task} {}ms", duration.as_millis()))
            },
            TaskIgnoredError(rule, _, error) => {
                self.err(&format!("[{rule}] Error ignored: {error}"));
                self.notify("task-ignored-error", error)
            },
            // state and nesting changes: nothing a script needs
            ChangeState(_) | BuildFileContext(_) => {}
        }
    }
}
