#![allow(unsafe_op_in_unsafe_fn)]

use pyo3::prelude::*;
use std::sync::{Arc, Mutex};
use std::fmt;
use std::str::FromStr;

use cpclib_bndbuild::task::Task;
use cpclib_bndbuild::task::StandardTaskArguments;
use cpclib_bndbuild::event::{BndBuilderEvent, BndBuilderObserver};
use cpclib_common::event::EventObserver;

use pyo3::types::PyAny;

/// Python-visible task object that stores a parsed `Task` and exposes `execute`.
#[pyclass]
pub struct PyBndTask {
    inner: Mutex<Task>,
}

/// Python wrapper around `StandardTaskArguments`.
///
/// This wrapper directly owns a shared (`Arc`) `StandardTaskArguments`.
/// It intentionally does not keep separate Python-visible caches for
/// `args`/`ignore_error` and instead exposes the underlying Rust value
/// directly via `Deref`.
#[pyclass]
pub struct PyStandardTaskArguments {
    /// Shared, immutable Rust `StandardTaskArguments` value.
    inner: Arc<StandardTaskArguments>,
}

#[pymethods]
impl PyStandardTaskArguments {
    #[new]
    pub fn new(task_args: &PyAny) -> PyResult<Self> {
        let s: &str = task_args.extract().map_err(|e| pyo3::exceptions::PyValueError::new_err(e))?;
        let args_s = s.to_owned();
        Ok(PyStandardTaskArguments {
            inner: Arc::new(StandardTaskArguments::new(args_s)),
        })
    }
}


// Rust-only helper conversion: not exposed to Python via `#[pymethods]`.
impl PyStandardTaskArguments {
    /// Create a Rust `StandardTaskArguments` from this wrapper.
    /// Returns a cloned value of the inner `StandardTaskArguments`.
    pub fn into_rust(&self) -> StandardTaskArguments {
        (*self.inner).clone()
    }

    /// Replace automatic variables on a cloned copy and return the resulting
    /// args string. This keeps the wrapper immutable while allowing callers
    /// to obtain the string with automatic variables substituted.
    pub fn replace_automatic_variables(&self) -> Result<String, String> {
        let mut cloned = (*self.inner).clone();
        cloned
            .replace_automatic_variables(None, None)
            .map_err(|e| e)?;
        Ok(cloned.args().to_string())
    }
}

// Allow transparent access to the underlying `StandardTaskArguments` via
// deref, e.g. `&*py_args` yields a `&StandardTaskArguments`.
impl std::ops::Deref for PyStandardTaskArguments {
    type Target = StandardTaskArguments;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl fmt::Display for PyStandardTaskArguments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.replace_automatic_variables() {
            Ok(s) => write!(f, "{}", s),
            Err(_) => write!(f, "{}", self.inner.args()),
        }
    }
}

// Observer that forwards outputs to process streams (no internal storage)
#[derive(Default)]
struct PyConsoleObserver;

impl fmt::Debug for PyConsoleObserver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PyConsoleObserver").finish()
    }
}

impl EventObserver for PyConsoleObserver {
    fn emit_stdout(&self, s: &str) {
        println!("{}", s);
    }

    fn emit_stderr(&self, s: &str) {
        eprintln!("{}", s);
    }
}

impl BndBuilderObserver for PyConsoleObserver {
    fn update(&mut self, event: BndBuilderEvent) {
        use cpclib_bndbuild::event::BndBuilderEvent::*;
        match event {
            ChangeState(_) => {
                self.emit_stdout("ChangeState");
            }
            StartRule { rule, nb, out_of } => {
                self.emit_stdout(&format!("StartRule {} {}/{}", rule, nb, out_of));
            }
            StopRule(p) => {
                self.emit_stdout(&format!("StopRule {}", p));
            }
            FailedRule(p) => {
                self.emit_stdout(&format!("FailedRule {}", p));
            }
            StartTask(_r, t) => {
                self.emit_stdout(&format!("StartTask {}", t));
            }
            StopTask(_r, t, d) => {
                self.emit_stdout(&format!("StopTask {} {}ms", t, d.as_millis()));
            }
            TaskStdout(tgt, _task, txt) => {
                println!("[{}] {}", tgt, txt);
            }
            TaskStderr(tgt, _task, txt) => {
                eprintln!("[{}] {}", tgt, txt);
            }
            Stdout(s) => {
                println!("{}", s);
            }
            Stderr(s) => {
                eprintln!("{}", s);
            }
        }
    }
}

impl fmt::Debug for PyBndTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PyBndTask").finish()
    }
}

#[pymethods]
impl PyBndTask {
    /// Constructor: parse a task string and return a new PyBndTask.
    #[new]
    pub fn new(task: &PyAny) -> PyResult<Self> {
        let task_str: &str = task.extract().map_err(|e| pyo3::exceptions::PyValueError::new_err(e))?;
        let t = Task::from_str(task_str).map_err(|e| pyo3::exceptions::PyValueError::new_err(e))?;
        Ok(PyBndTask { inner: Mutex::new(t) })
    }

    /// Execute the stored task synchronously and return a result dict.
    pub fn execute(&self, py: Python) -> PyResult<()> {
        let observer = Arc::new(PyConsoleObserver::default());
        self.execute_with_observer(py, observer)
    }
}

impl PyBndTask {
    /// Execute the stored task using the provided observer.
    /// This is a Rust-level helper where the observer is an argument.
    pub(crate) fn execute_with_observer(&self, py: Python, observer: Arc<PyConsoleObserver>) -> PyResult<()> {
        // Execute the task without holding the GIL.
        py
            .allow_threads(|| {
                let guard = self.inner.lock().unwrap();
                guard.execute(&observer)
            })
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
    }
}

// Factory removed: use `PyBndTask(...)` constructor from Python instead.
