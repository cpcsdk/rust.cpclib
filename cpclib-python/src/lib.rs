use pyo3::prelude::*;
use pyo3::types::PyDict;

mod bndbuild;

// Lightweight placeholders for crate-specific wrappers.
// These functions are intentionally minimal so the crate builds
// and can be extended to call into the real cpclib-* APIs.

#[pyfunction]
fn hello() -> PyResult<&'static str> {
    Ok("cpclib-python ready")
}

#[pyfunction]
fn crate_info(py: Python) -> PyResult<PyObject> {
    let d = PyDict::new(py);
    d.set_item("name", "cpclib-python")?;
    d.set_item("version", env!("CARGO_PKG_VERSION"))?;
    Ok(d.into())
}

#[pyfunction]
fn asm_info() -> PyResult<&'static str> { Ok("cpclib-asm (placeholder)") }
#[pyfunction]
fn basic_info() -> PyResult<&'static str> { Ok("cpclib-basic (placeholder)") }
#[pyfunction]
fn basm_info() -> PyResult<&'static str> { Ok("cpclib-basm (placeholder)") }
#[pyfunction]
fn bdasm_info() -> PyResult<&'static str> { Ok("cpclib-bdasm (placeholder)") }
#[pyfunction]
fn bndbuild_info() -> PyResult<&'static str> { Ok("cpclib-bndbuild (placeholder)") }
#[pyfunction]
fn cpr_info() -> PyResult<&'static str> { Ok("cpclib-cpr (placeholder)") }
#[pyfunction]
fn crunchers_info() -> PyResult<&'static str> { Ok("cpclib-crunchers (placeholder)") }

#[pymodule]
fn cpclib_python(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(hello, m)?)?;
    m.add_function(wrap_pyfunction!(crate_info, m)?)?;

    // create submodules exposing minimal info functions for each component
    let asm_mod = PyModule::new(py, "asm")?;
    asm_mod.add_function(wrap_pyfunction!(asm_info, asm_mod)?)?;
    m.add_submodule(asm_mod)?;

    let basic_mod = PyModule::new(py, "basic")?;
    basic_mod.add_function(wrap_pyfunction!(basic_info, basic_mod)?)?;
    m.add_submodule(basic_mod)?;

    let basm_mod = PyModule::new(py, "basm")?;
    basm_mod.add_function(wrap_pyfunction!(basm_info, basm_mod)?)?;
    m.add_submodule(basm_mod)?;

    let bdasm_mod = PyModule::new(py, "bdasm")?;
    bdasm_mod.add_function(wrap_pyfunction!(bdasm_info, bdasm_mod)?)?;
    m.add_submodule(bdasm_mod)?;

    let bndbuild_mod = PyModule::new(py, "bndbuild")?;
    bndbuild_mod.add_function(wrap_pyfunction!(bndbuild_info, bndbuild_mod)?)?;
    // expose the `PyBndTask` class (use the constructor from Python)
    bndbuild_mod.add_class::<bndbuild::PyBndTask>()?;
    m.add_submodule(bndbuild_mod)?;

    let cpr_mod = PyModule::new(py, "cpr")?;
    cpr_mod.add_function(wrap_pyfunction!(cpr_info, cpr_mod)?)?;
    m.add_submodule(cpr_mod)?;

    let crunchers_mod = PyModule::new(py, "crunchers")?;
    crunchers_mod.add_function(wrap_pyfunction!(crunchers_info, crunchers_mod)?)?;
    m.add_submodule(crunchers_mod)?;

    Ok(())
}

// `execute_bndbuild_task` moved to `src/bndbuild.rs`.

