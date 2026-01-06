use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// Assemble a Z80 assembly string and return the produced bytes as Python `bytes`.
#[pyfunction]
fn assemble(py: Python, code: String) -> PyResult<Py<PyAny>> {
    match cpclib_asm::assemble(&code) {
        Ok(bytes) => Ok(PyBytes::new(py, &bytes).into()),
        Err(e) => Err(PyRuntimeError::new_err(format!("Assemble error: {}", e)))
    }
}

#[pymodule]
pub fn basm(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(assemble, m)?)?;
    Ok(())
}
