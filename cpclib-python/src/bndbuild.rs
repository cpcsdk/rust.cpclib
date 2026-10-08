#![allow(unsafe_op_in_unsafe_fn)]

use std::fmt;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use cpclib_bndbuild::task::{InnerTask, StandardTaskArguments, Task};
use pyo3::Py;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};

use crate::observer::PyObserver;

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
    #[pyo3(signature = (task, args=None))]
    pub fn new(task: &Bound<'_, PyAny>, args: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
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
                let task = task_from_command(code, &vec)
                    .map_err(pyo3::exceptions::PyValueError::new_err)?;
                Ok(PyBndTask {
                    inner: Mutex::new(task)
                })
            }
        }
    }

    /// Execute the stored task synchronously.
    ///
    /// Its output goes to the console, unless `on_output` is given: a callable
    /// `on_output(kind, text)` (kinds: `stdout`, `stderr`, `task-start`, ...).
    /// With `capture=True`, returns `{"stdout": ..., "stderr": ...}`; returns
    /// None otherwise. Raises `RuntimeError` if the task fails.
    #[pyo3(signature = (on_output=None, capture=false))]
    pub fn execute(
        &self,
        py: Python,
        on_output: Option<Py<PyAny>>,
        capture: bool
    ) -> PyResult<Option<Py<PyDict>>> {
        let observer = Arc::new(PyObserver::new(on_output));
        // Execute the task without holding the GIL.
        let result = py.detach(|| {
            let guard = self.inner.lock().unwrap();
            guard.execute(&observer)
        });

        if capture {
            let (stdout, stderr) = observer.captured();
            let captured = PyDict::new(py);
            captured.set_item("stdout", stdout)?;
            captured.set_item("stderr", stderr)?;
            result.map_err(pyo3::exceptions::PyRuntimeError::new_err)?;
            Ok(Some(captured.into()))
        }
        else {
            result.map_err(pyo3::exceptions::PyRuntimeError::new_err)?;
            Ok(None)
        }
    }
}

/// The task `code` (`basm`, `dsk`, ...) with `args` - each argument quoted, so
/// that spaces and quotes in file names survive.
pub(crate) fn task_from_command(code: &str, args: &[String]) -> Result<Task, String> {
    // Surround each argument with quotes and escape internal quotes.
    let joined = args
        .iter()
        .map(|s| format!("\"{}\"", s.replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(" ");
    InnerTask::from_command_and_arguments(code, StandardTaskArguments::new(joined)).map(Into::into)
}

impl PyBndTask {
    /// Helper to create PyBndTask from a Task instance (for builders)
    pub(crate) fn new_from_task(py: Python, task: Task) -> PyResult<Py<PyBndTask>> {
        Py::new(
            py,
            PyBndTask {
                inner: Mutex::new(task)
            }
        )
    }
}
