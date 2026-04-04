use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;

pub trait EventObserver: Debug + Sync + Send {
    fn emit_stdout(&self, s: &str);
    fn emit_stderr(&self, s: &str);
}

impl EventObserver for () {
    fn emit_stdout(&self, s: &str) {
        println!("{s}")
    }

    fn emit_stderr(&self, s: &str) {
        eprintln!("{s}")
    }
}

impl<E: EventObserver> EventObserver for Option<E> {
    fn emit_stdout(&self, s: &str) {
        if let Some(e) = self {
            e.emit_stdout(s)
        }
    }

    fn emit_stderr(&self, s: &str) {
        if let Some(e) = self {
            e.emit_stderr(s)
        }
    }
}

impl<E: EventObserver> EventObserver for &E {
    fn emit_stdout(&self, s: &str) {
        (*self).emit_stdout(s)
    }

    fn emit_stderr(&self, s: &str) {
        (*self).emit_stderr(s)
    }
}

impl<E: EventObserver> EventObserver for Box<E> {
    fn emit_stdout(&self, s: &str) {
        self.as_ref().emit_stdout(s)
    }

    fn emit_stderr(&self, s: &str) {
        self.as_ref().emit_stderr(s)
    }
}

impl<T: EventObserver> EventObserver for Arc<T> {
    fn emit_stdout(&self, s: &str) {
        self.deref().emit_stdout(s)
    }

    fn emit_stderr(&self, s: &str) {
        self.deref().emit_stderr(s)
    }
}

/// An [`EventObserver`] that silently discards all output.
///
/// Use this instead of `()` when you want to suppress output without printing
/// to the process stdout/stderr (which `()` does).
#[derive(Debug, Clone, Copy, Default)]
pub struct DiscardObserver;

impl EventObserver for DiscardObserver {
    fn emit_stdout(&self, _s: &str) {}

    fn emit_stderr(&self, _s: &str) {}
}

/// An [`EventObserver`] that captures all output into in-memory buffers.
///
/// Intended for use in tests to verify that stdout/stderr is properly routed
/// through the observer instead of leaking to the process standard streams.
///
/// # Example
/// ```
/// use cpclib_common::event::{CapturingObserver, EventObserver};
///
/// let obs = CapturingObserver::new();
/// obs.emit_stdout("hello");
/// obs.emit_stderr("oops");
/// assert_eq!(obs.stdout_joined(), "hello");
/// assert_eq!(obs.stderr_joined(), "oops");
/// ```
#[derive(Debug, Default)]
pub struct CapturingObserver {
    pub stdout: std::sync::Mutex<Vec<String>>,
    pub stderr: std::sync::Mutex<Vec<String>>
}

impl Clone for CapturingObserver {
    fn clone(&self) -> Self {
        CapturingObserver {
            stdout: std::sync::Mutex::new(self.stdout.lock().unwrap().clone()),
            stderr: std::sync::Mutex::new(self.stderr.lock().unwrap().clone())
        }
    }
}

impl CapturingObserver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a clone of all captured stdout entries.
    pub fn get_stdout(&self) -> Vec<String> {
        self.stdout.lock().unwrap().clone()
    }

    /// Returns a clone of all captured stderr entries.
    pub fn get_stderr(&self) -> Vec<String> {
        self.stderr.lock().unwrap().clone()
    }

    /// Concatenates all captured stdout entries into a single string.
    pub fn stdout_joined(&self) -> String {
        self.stdout.lock().unwrap().join("")
    }

    /// Concatenates all captured stderr entries into a single string.
    pub fn stderr_joined(&self) -> String {
        self.stderr.lock().unwrap().join("")
    }
}

impl EventObserver for CapturingObserver {
    fn emit_stdout(&self, s: &str) {
        self.stdout.lock().unwrap().push(s.to_owned());
    }

    fn emit_stderr(&self, s: &str) {
        self.stderr.lock().unwrap().push(s.to_owned());
    }
}
