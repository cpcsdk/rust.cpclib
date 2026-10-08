//! CSL scripts (CPC Script Language): the scenario files emulators replay.

#![allow(unsafe_op_in_unsafe_fn)]

use cpclib_csl::parse_csl_with_rich_errors;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Parses the CSL script `script` and returns its instructions, one string each.
///
/// Raises `ValueError` with the parser's own message (the line, what was
/// expected) if the script is not valid.
#[pyfunction]
#[pyo3(signature = (script, filename=None))]
fn parse(script: &str, filename: Option<String>) -> PyResult<Vec<String>> {
    parse_csl_with_rich_errors(script, filename)
        .map(|s| s.instructions().iter().map(|i| i.to_string()).collect())
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Parses, and writes the script back, normalised.
#[pyfunction]
#[pyo3(signature = (script, filename=None))]
fn normalize(script: &str, filename: Option<String>) -> PyResult<String> {
    parse_csl_with_rich_errors(script, filename)
        .map(|s| s.to_string())
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

pub fn csl(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(normalize, m)?)?;
    Ok(())
}
