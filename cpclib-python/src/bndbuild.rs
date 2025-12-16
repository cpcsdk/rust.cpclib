#![allow(unsafe_op_in_unsafe_fn)]

use std::fmt;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use cpclib_bndbuild::event::{BndBuilderEvent, BndBuilderObserver};
use cpclib_bndbuild::task::{InnerTask, StandardTaskArguments, Task};
use cpclib_common::event::EventObserver;
use pyo3::Py;
use pyo3::prelude::*;
use pyo3::types::PyAny;

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
            },
            StartRule { rule, nb, out_of } => {
                self.emit_stdout(&format!("StartRule {} {}/{}", rule, nb, out_of));
            },
            StopRule(p) => {
                self.emit_stdout(&format!("StopRule {}", p));
            },
            FailedRule(p) => {
                self.emit_stdout(&format!("FailedRule {}", p));
            },
            StartTask(_r, t) => {
                self.emit_stdout(&format!("StartTask {}", t));
            },
            StopTask(_r, t, d) => {
                self.emit_stdout(&format!("StopTask {} {}ms", t, d.as_millis()));
            },
            TaskStdout(tgt, _task, txt) => {
                println!("[{}] {}", tgt, txt);
            },
            TaskStderr(tgt, _task, txt) => {
                eprintln!("[{}] {}", tgt, txt);
            },
            Stdout(s) => {
                println!("{}", s);
            },
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

/// Python-visible task object that stores a parsed `Task` and exposes `execute`.
#[pyclass(name = "Task")]
pub struct PyBndTask {
    inner: Mutex<Task>
}

#[pymethods]
impl PyBndTask {
    /// Constructor: parse a task string and return a new PyBndTask.
    ///
    /// Two modes are supported from Python:
    ///  - `PyBndTask("basm toto.asm -o toto.o")` (single string, YAML-like parse)
    ///  - `PyBndTask("basm", ["toto.asm", "-o", "toto.o"])` (command + args list)
    #[new]
    pub fn new(task: &PyAny, args: Option<&PyAny>) -> PyResult<Self> {
        match args {
            None => {
                let task_str: &str = task
                    .extract()
                    .map_err(pyo3::exceptions::PyValueError::new_err)?;
                let t = InnerTask::from_str(task_str)
                    .map_err(pyo3::exceptions::PyValueError::new_err)?;
                let t: Task = t.into();
                Ok(PyBndTask {
                    inner: Mutex::new(t)
                })
            },
            Some(py_args) => {
                // First argument is the command token, second is a sequence of strings
                let code: &str = task
                    .extract()
                    .map_err(pyo3::exceptions::PyValueError::new_err)?;
                let vec: Vec<String> = py_args
                    .extract()
                    .map_err(pyo3::exceptions::PyValueError::new_err)?;
                // Surround each argument with quotes and escape internal quotes.
                let quoted: Vec<String> = vec
                    .into_iter()
                    .map(|s| format!("\"{}\"", s.replace('"', "\\\"")))
                    .collect();
                let joined = quoted.join(" ");
                let std = StandardTaskArguments::new(joined);
                let inner = InnerTask::from_command_and_arguments(code, std)
                    .map_err(pyo3::exceptions::PyValueError::new_err)?;
                Ok(PyBndTask {
                    inner: Mutex::new(inner.into())
                })
            }
        }
    }

    /// Execute the stored task synchronously and return a result dict.
    pub fn execute(&self, py: Python) -> PyResult<()> {
        let observer = Arc::new(PyConsoleObserver);
        self.execute_with_observer(py, observer)
    }
}

impl PyBndTask {
    /// Execute the stored task using the provided observer.
    /// This is a Rust-level helper where the observer is an argument.
    pub(crate) fn execute_with_observer(
        &self,
        py: Python,
        observer: Arc<PyConsoleObserver>
    ) -> PyResult<()> {
        // Execute the task without holding the GIL.
        py.allow_threads(|| {
            let guard = self.inner.lock().unwrap();
            guard.execute(&observer)
        })
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)
    }
}

/// Factory function to create a `PyBndTask` from a task string.
#[pyfunction]
pub fn create_bndbuild_task(task: &PyAny, py: Python) -> PyResult<PyObject> {
    let task_str: &str = task
        .extract()
        .map_err(pyo3::exceptions::PyValueError::new_err)?;
    let t = Task::from_str(task_str).map_err(pyo3::exceptions::PyValueError::new_err)?;
    let obj = Py::new(
        py,
        PyBndTask {
            inner: Mutex::new(t)
        }
    )?;
    Ok(obj.into_py(py))
}
