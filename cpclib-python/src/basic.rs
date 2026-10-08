//! Locomotive BASIC: source text to the tokenized form stored on disc, and back.
//!
//! ```python
//! from cpclib_python import basic
//!
//! data = basic.tokenize('10 PRINT "HELLO"\n20 GOTO 10\n')
//! basic.detokenize(data)      # '10 PRINT "HELLO"\n20 GOTO 10\n'
//! ```

#![allow(unsafe_op_in_unsafe_fn)]

use cpclib_basic::BasicProgram;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// The tokenized bytes of a BASIC program (the content of a `.BAS` file, without
/// its AMSDOS header - see `Disc.add_basic`).
///
/// Raises `ValueError` if `code` is not valid Locomotive BASIC.
#[pyfunction]
fn tokenize<'py>(py: Python<'py>, code: &str) -> PyResult<Bound<'py, PyBytes>> {
    let program = BasicProgram::parse(code).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(PyBytes::new(py, &program.as_bytes()))
}

/// The source text of a tokenized BASIC program.
///
/// Raises `ValueError` if `data` is not a tokenized program.
#[pyfunction]
fn detokenize(data: &[u8]) -> PyResult<String> {
    BasicProgram::decode(data)
        .map(|p| p.to_string())
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// The line numbers of `code`, in order - checks that it parses.
#[pyfunction]
fn line_numbers(code: &str) -> PyResult<Vec<u16>> {
    let program = BasicProgram::parse(code).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(program.lines().iter().map(|l| l.line_number()).collect())
}

/// A snapshot of a CPC that has the program loaded in BASIC memory, ready to
/// `RUN` (see `sna.Snapshot`).
///
/// Raises `ValueError` if `code` is not valid Locomotive BASIC.
#[pyfunction]
fn to_snapshot(code: &str) -> PyResult<crate::sna::PySnapshot> {
    let program = BasicProgram::parse(code).map_err(|e| PyValueError::new_err(e.to_string()))?;
    program
        .as_sna()
        .map(crate::sna::PySnapshot::from_inner)
        .map_err(PyValueError::new_err)
}

pub fn basic(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(tokenize, m)?)?;
    m.add_function(wrap_pyfunction!(detokenize, m)?)?;
    m.add_function(wrap_pyfunction!(line_numbers, m)?)?;
    m.add_function(wrap_pyfunction!(to_snapshot, m)?)?;
    Ok(())
}
