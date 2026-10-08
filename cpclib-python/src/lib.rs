use pyo3::prelude::*;
use pyo3::types::PyDict;

mod basm;
mod bndbuild;
mod builders;
mod crunchers;
mod disc;
mod music;
mod observer;
mod sna;

// Lightweight placeholders for crate-specific wrappers.
// These functions are intentionally minimal so the crate builds
// and can be extended to call into the real cpclib-* APIs.

#[pyfunction]
fn hello() -> PyResult<&'static str> {
    Ok("cpclib-python ready")
}

#[pyfunction]
fn crate_info(py: Python) -> PyResult<Py<PyAny>> {
    let d = PyDict::new(py);
    d.set_item("name", "cpclib-python")?;
    d.set_item("version", env!("CARGO_PKG_VERSION"))?;
    Ok(d.into())
}

#[pyfunction]
fn asm_info() -> PyResult<&'static str> {
    Ok("cpclib-asm (placeholder)")
}
#[pyfunction]
fn basic_info() -> PyResult<&'static str> {
    Ok("cpclib-basic (placeholder)")
}
#[pyfunction]
fn basm_info() -> PyResult<&'static str> {
    Ok("cpclib-basm (placeholder)")
}
#[pyfunction]
fn bdasm_info() -> PyResult<&'static str> {
    Ok("cpclib-bdasm (placeholder)")
}
#[pyfunction]
fn bndbuild_info() -> PyResult<&'static str> {
    Ok("cpclib-bndbuild (placeholder)")
}
#[pyfunction]
fn cpr_info() -> PyResult<&'static str> {
    Ok("cpclib-cpr (placeholder)")
}
#[pyfunction]
fn crunchers_info() -> PyResult<&'static str> {
    Ok("cpclib-crunchers (placeholder)")
}

/// Adds `sub` to `parent`, and to `sys.modules`, so that both
/// `cpclib_python.disc.Disc` and `from cpclib_python.disc import Disc` work
/// (pyo3 submodules are only attributes otherwise).
fn add_submodule(parent: &Bound<'_, PyModule>, sub: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_submodule(sub)?;
    // maturin wraps the extension in a package of the same name
    // (`cpclib_python.cpclib_python`): it is the package's name users import
    let parent_name = parent.name()?.to_string();
    let package = parent_name.split('.').next().unwrap_or(&parent_name);
    let qualified = format!("{package}.{}", sub.name()?);
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item(qualified, sub)
}

#[pymodule]
fn cpclib_python(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(hello, m)?)?;
    m.add_function(wrap_pyfunction!(crate_info, m)?)?;

    // create submodules exposing minimal info functions for each component
    let asm_mod = PyModule::new(py, "asm")?;
    asm_mod.add_function(wrap_pyfunction!(asm_info, &asm_mod)?)?;
    add_submodule(m, &asm_mod)?;

    let basic_mod = PyModule::new(py, "basic")?;
    basic_mod.add_function(wrap_pyfunction!(basic_info, &basic_mod)?)?;
    add_submodule(m, &basic_mod)?;

    // basm submodule: assemble helper
    let basm_mod = PyModule::new(py, "basm")?;
    basm_mod.add_function(wrap_pyfunction!(basm_info, &basm_mod)?)?;
    // register the real basm functions
    basm::basm(py, &basm_mod)?;
    add_submodule(m, &basm_mod)?;

    let bdasm_mod = PyModule::new(py, "bdasm")?;
    bdasm_mod.add_function(wrap_pyfunction!(bdasm_info, &bdasm_mod)?)?;
    add_submodule(m, &bdasm_mod)?;

    let bndbuild_mod = PyModule::new(py, "bndbuild")?;
    bndbuild_mod.add_function(wrap_pyfunction!(bndbuild_info, &bndbuild_mod)?)?;
    // expose the `PyBndTask` class (use the constructor from Python)
    bndbuild_mod.add_class::<bndbuild::PyBndTask>()?;
    // expose the builder classes
    bndbuild_mod.add_class::<builders::PySnapshotBuilder>()?;
    bndbuild_mod.add_class::<builders::PyArkosTracker3Builder>()?;
    bndbuild_mod.add_class::<builders::PyChipnsfxBuilder>()?;
    bndbuild_mod.add_class::<builders::PyMinyBuilder>()?;
    bndbuild_mod.add_class::<builders::PyAytBuilder>()?;
    bndbuild_mod.add_class::<builders::PySongConverterBuilder>()?;
    add_submodule(m, &bndbuild_mod)?;

    // music: songs to standalone CPC players, with any of bndbuild's players
    let music_mod = PyModule::new(py, "music")?;
    music::music(py, &music_mod)?;
    add_submodule(m, &music_mod)?;

    let sna_mod = PyModule::new(py, "sna")?;
    sna::sna(py, &sna_mod)?;
    add_submodule(m, &sna_mod)?;

    let disc_mod = PyModule::new(py, "disc")?;
    disc::disc(py, &disc_mod)?;
    add_submodule(m, &disc_mod)?;

    let cpr_mod = PyModule::new(py, "cpr")?;
    cpr_mod.add_function(wrap_pyfunction!(cpr_info, &cpr_mod)?)?;
    add_submodule(m, &cpr_mod)?;

    let crunchers_mod = PyModule::new(py, "crunchers")?;
    crunchers_mod.add_function(wrap_pyfunction!(crunchers_info, &crunchers_mod)?)?;
    crunchers::crunchers(py, &crunchers_mod)?;
    add_submodule(m, &crunchers_mod)?;

    Ok(())
}

// `execute_bndbuild_task` moved to `src/bndbuild.rs`.
