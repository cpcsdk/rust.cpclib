//! The basm source formatter.
//!
//! ```python
//! from cpclib_python import fmt
//!
//! fmt.format_source("ld a,1\nloop: djnz loop\n")
//! fmt.format_source(code, {"indent_size": 2})
//! fmt.default_options()           # every option, with its default
//! ```

#![allow(unsafe_op_in_unsafe_fn)]

use cpclib_asmfmt::AsmFormatOptions;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

fn options_of(py: Python, options: Option<&Bound<'_, PyDict>>) -> PyResult<AsmFormatOptions> {
    let Some(options) = options
    else {
        return serde_json::from_value(serde_json::json!({}))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()));
    };
    let json = py.import("json")?.call_method1("dumps", (options,))?;
    serde_json::from_str(&json.extract::<String>()?)
        .map_err(|e| PyValueError::new_err(format!("invalid formatter options: {e}")))
}

/// The options with their default values, as a dict.
#[pyfunction]
fn default_options(py: Python) -> PyResult<Py<PyAny>> {
    let options: AsmFormatOptions = serde_json::from_value(serde_json::json!({}))
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    let json =
        serde_json::to_string(&options).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    Ok(py.import("json")?.call_method1("loads", (json,))?.unbind())
}

/// Formats the basm source `code`. `options` (a dict, see `default_options()`)
/// only needs the options to change. Raises `ValueError` for an unknown
/// option and `RuntimeError` if the source does not parse.
#[pyfunction]
#[pyo3(signature = (code, options=None))]
fn format_source(py: Python, code: &str, options: Option<&Bound<'_, PyDict>>) -> PyResult<String> {
    let options = options_of(py, options)?;
    cpclib_asmfmt::format(code, &options).map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

/// Formats only the lines `start_line` to `end_line` (1-based, inclusive).
#[pyfunction]
#[pyo3(signature = (code, start_line, end_line, options=None))]
fn format_range(
    py: Python,
    code: &str,
    start_line: usize,
    end_line: usize,
    options: Option<&Bound<'_, PyDict>>
) -> PyResult<String> {
    let options = options_of(py, options)?;
    cpclib_asmfmt::format_range(code, &options, start_line, end_line)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

pub fn fmt(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(default_options, m)?)?;
    m.add_function(wrap_pyfunction!(format_source, m)?)?;
    m.add_function(wrap_pyfunction!(format_range, m)?)?;
    Ok(())
}
