//! Compression with the crunchers the workspace embeds (ZX0, ZX7, Exomizer,
//! LZSA, Shrinkler, apultra, UPKR, pucrunch, LZ4/48/49).
//!
//! ```python
//! from cpclib_python import crunchers
//!
//! crunchers.formats()                       # ['apultra', 'exomizer', ...]
//! packed = crunchers.compress(data, "zx0")  # CompressionResult
//! len(packed.data), packed.delta
//! crunchers.compare(data)                   # [('zx0', 1234), ('upkr', 1250), ...]
//! ```

#![allow(unsafe_op_in_unsafe_fn)]

use std::time::Duration;

use cpclib_crunch::resolve::{ALL_FORMATS, CRUNCHER_TIMEOUT, compress_with_timeout};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// What a cruncher made of some data.
#[pyclass(name = "CompressionResult", frozen)]
pub struct PyCompressionResult {
    /// The format that crunched it.
    #[pyo3(get)]
    format: String,
    data: Vec<u8>,
    /// For in-place decrunching: the gap between the end of the compressed
    /// stream and the end of the decompressed data, when the format says.
    #[pyo3(get)]
    delta: Option<usize>
}

#[pymethods]
impl PyCompressionResult {
    /// The compressed stream.
    #[getter]
    fn data<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.data)
    }

    fn __len__(&self) -> usize {
        self.data.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "CompressionResult(format={:?}, size={}, delta={:?})",
            self.format,
            self.data.len(),
            self.delta
        )
    }
}

/// Every format name `compress` accepts.
#[pyfunction]
fn formats() -> Vec<&'static str> {
    ALL_FORMATS.to_vec()
}

fn timeout_of(timeout: Option<f64>) -> PyResult<Duration> {
    match timeout {
        None => Ok(CRUNCHER_TIMEOUT),
        Some(t) if t.is_finite() && t > 0.0 => Ok(Duration::from_secs_f64(t)),
        Some(t) => Err(PyValueError::new_err(format!("invalid timeout: {t}")))
    }
}

/// Compresses `data` with `format` (one of `formats()`, or `none`). A cruncher
/// that does not finish within `timeout` seconds (default 90) is given up on -
/// some of the embedded ones hang on degenerate input.
///
/// Raises `ValueError` for an unknown format, `RuntimeError` when the cruncher
/// fails or times out.
#[pyfunction]
#[pyo3(signature = (data, format, timeout=None))]
fn compress(
    py: Python,
    data: &[u8],
    format: &str,
    timeout: Option<f64>
) -> PyResult<PyCompressionResult> {
    if format != "none" && !ALL_FORMATS.contains(&format) {
        return Err(PyValueError::new_err(format!(
            "unknown format `{format}` (expected one of: none, {})",
            ALL_FORMATS.join(", ")
        )));
    }
    let timeout = timeout_of(timeout)?;
    let (name, data) = (format.to_string(), data.to_vec());

    let result = py
        .detach(|| compress_with_timeout(name.clone(), data, timeout))
        .map_err(PyRuntimeError::new_err)?;
    Ok(PyCompressionResult {
        format: name,
        data: result.stream,
        delta: result.delta
    })
}

/// Compresses `data` with each of `formats` (default: all of them) and returns
/// `[(format, size_or_None, error_or_None)]`, smallest first, failures last.
#[pyfunction]
#[pyo3(signature = (data, formats=None, timeout=None))]
fn compare(
    py: Python,
    data: &[u8],
    formats: Option<Vec<String>>,
    timeout: Option<f64>
) -> PyResult<Vec<(String, Option<usize>, Option<String>)>> {
    let timeout = timeout_of(timeout)?;
    let formats = formats.unwrap_or_else(|| ALL_FORMATS.iter().map(|f| f.to_string()).collect());
    let data = data.to_vec();

    let mut rows: Vec<(String, Option<usize>, Option<String>)> = py.detach(|| {
        formats
            .into_iter()
            .map(|format| {
                match compress_with_timeout(format.clone(), data.clone(), timeout) {
                    Ok(r) => (format, Some(r.stream.len()), None),
                    Err(e) => (format, None, Some(e))
                }
            })
            .collect()
    });
    rows.sort_by_key(|(_, size, _)| size.unwrap_or(usize::MAX));
    Ok(rows)
}

pub fn crunchers(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCompressionResult>()?;
    m.add_function(wrap_pyfunction!(formats, m)?)?;
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(compare, m)?)?;
    Ok(())
}
