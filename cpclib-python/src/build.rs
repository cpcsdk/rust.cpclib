//! bndbuild build files: list their targets, see what is out of date, run them.
//!
//! ```python
//! from cpclib_python.build import Build
//!
//! build = Build("project/build.bnd")          # or the directory holding it
//! build.default_target                        # 'dsk'
//! [t["target"] for t in build.targets()]      # ['dsk', 'a.bin', ...]
//! build.is_outdated("dsk")
//! build.run("dsk", on_output=lambda kind, text: print(kind, text))
//! ```

#![allow(unsafe_op_in_unsafe_fn)]

use std::sync::Arc;
use std::time::Instant;

use cpclib_bndbuild::app::WatchState;
use cpclib_bndbuild::builder::BndBuilder;
use cpclib_bndbuild::event::{BndBuilderObserved, BndBuilderObserverRc};
use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use pyo3::exceptions::{PyKeyError, PyRuntimeError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::observer::PyObserver;

fn open(path: &str) -> Result<(Utf8PathBuf, BndBuilder), String> {
    // `false`: do not force `--serial` on the nested bndbuild tasks
    BndBuilder::from_path(Utf8Path::new(path), false).map_err(|e| e.to_string())
}

/// A bndbuild build file. The file is read again for each call: what is
/// returned always matches what is on disc.
#[pyclass(name = "Build", frozen)]
pub struct PyBuild {
    path: String,
    /// The build file itself, when `path` was the directory holding it.
    #[pyo3(get)]
    resolved_path: String
}

#[pymethods]
impl PyBuild {
    /// Opens the build file at `path` (a `.bnd` file, or the directory with a
    /// `build.bnd` in it). Raises `RuntimeError` if it cannot be read.
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let (resolved, _) = open(path).map_err(PyRuntimeError::new_err)?;
        Ok(Self {
            path: path.to_string(),
            resolved_path: resolved.to_string()
        })
    }

    /// The target built when none is named, if the build file says so.
    #[getter]
    fn default_target(&self) -> PyResult<Option<String>> {
        let (_, builder) = open(&self.path).map_err(PyRuntimeError::new_err)?;
        Ok(builder.default_target().map(|t| t.to_string()))
    }

    /// Every target, as `{"target": ..., "dependencies": [...]}`.
    fn targets<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyDict>>> {
        let (_, builder) = open(&self.path).map_err(PyRuntimeError::new_err)?;
        builder
            .targets()
            .into_iter()
            .map(|target| {
                let entry = PyDict::new(py);
                entry.set_item("target", target.as_str())?;
                let dependencies: Vec<String> = builder
                    .get_rule(target)
                    .map(|r| r.dependencies().iter().map(|d| d.to_string()).collect())
                    .unwrap_or_default();
                entry.set_item("dependencies", dependencies)?;
                Ok(entry)
            })
            .collect()
    }

    /// Whether `target` needs to be rebuilt. Raises `KeyError` for an unknown target.
    fn is_outdated(&self, target: &str) -> PyResult<bool> {
        let (_, builder) = open(&self.path).map_err(PyRuntimeError::new_err)?;
        if builder.get_rule(target).is_none() && !builder.targets().contains(&Utf8Path::new(target))
        {
            return Err(PyKeyError::new_err(format!("no such target: {target}")));
        }
        builder
            .outdated(&WatchState::NoWatch, target)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Builds `target` (default: the default target) and what it depends on.
    ///
    /// The build's output goes to the console, or to `on_output(kind, text)`
    /// (kinds: `stdout`, `stderr`, `rule-start`, `rule-stop`, `rule-skipped`,
    /// `rule-failed`, `task-start`, `task-stop`, `task-ignored-error`).
    /// Returns `{"target", "duration_ms", "stdout", "stderr"}`; raises
    /// `RuntimeError` if the build fails.
    #[pyo3(signature = (target=None, on_output=None))]
    fn run<'py>(
        &self,
        py: Python<'py>,
        target: Option<&str>,
        on_output: Option<Py<PyAny>>
    ) -> PyResult<Bound<'py, PyDict>> {
        let observer = Arc::new(PyObserver::new(on_output));
        let path = self.path.clone();
        let target = target.map(str::to_string);

        let outcome = py.detach(|| -> Result<(String, u128), String> {
            let (_, mut builder) = open(&path)?;
            let target = match target {
                Some(t) => Utf8PathBuf::from(t),
                None => {
                    builder
                        .default_target()
                        .map(Utf8Path::to_path_buf)
                        .ok_or_else(|| {
                            "no target given and the build file declares no default target"
                                .to_string()
                        })?
                },
            };
            builder.add_observer(BndBuilderObserverRc::new(observer.clone()));
            let start = Instant::now();
            builder.execute(&target).map_err(|e| e.to_string())?;
            Ok((target.to_string(), start.elapsed().as_millis()))
        });

        let (stdout, stderr) = observer.captured();
        match outcome {
            Ok((target, duration_ms)) => {
                let result = PyDict::new(py);
                result.set_item("target", target)?;
                result.set_item("duration_ms", duration_ms as u64)?;
                result.set_item("stdout", stdout)?;
                result.set_item("stderr", stderr)?;
                Ok(result)
            },
            Err(e) => {
                Err(PyRuntimeError::new_err(
                    format!("{e}\n{stderr}").trim().to_string()
                ))
            },
        }
    }

    fn __repr__(&self) -> String {
        format!("Build({:?})", self.resolved_path)
    }
}

pub fn build(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyBuild>()?;
    Ok(())
}
