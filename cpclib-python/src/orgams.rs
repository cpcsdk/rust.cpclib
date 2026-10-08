//! Orgams sources.

#![allow(unsafe_op_in_unsafe_fn)]

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// The text of a source saved by the Orgams assembler in its binary format
/// (a `.O` file, AMSDOS header stripped).
///
/// Raises `ValueError` if `data` is not an Orgams source.
#[pyfunction]
fn to_utf8(data: &[u8]) -> PyResult<String> {
    cpclib_orgams_ascii::convert::binary_orgams_to_utf8(data).map_err(PyValueError::new_err)
}

pub fn orgams(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(to_utf8, m)?)?;
    Ok(())
}
