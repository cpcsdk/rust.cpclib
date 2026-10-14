//! CatArt: a disc's catalog shown as art, and the BASIC program that draws it.
//!
//! ```python
//! from cpclib_python.disc import Disc
//! from cpclib_python import catalog
//!
//! raw = Disc.open("art.dsk").catalog_bytes()
//! print(catalog.basic_listing(raw))        # the BASIC that prints it
//! ```

#![allow(unsafe_op_in_unsafe_fn)]

use cpclib_catalog::{catalog_screen_output, catalog_to_basic_listing_with_headers};
use cpclib_catart::entry::CatalogType;
use cpclib_common::event::DiscardObserver;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

fn kind_of(kind: &str) -> PyResult<CatalogType> {
    match kind.to_ascii_lowercase().as_str() {
        "cat" => Ok(CatalogType::Cat),
        "dir" => Ok(CatalogType::Dir),
        other => {
            Err(PyValueError::new_err(format!(
                "unknown catalog kind '{other}' - expected 'cat' (sorted, as `CAT` lists it) or \
                 'dir' (disc order, as `|DIR` does)"
            )))
        },
    }
}

fn checked(catalog_bytes: &[u8]) -> PyResult<()> {
    if catalog_bytes.len() == 64 * 32 {
        Ok(())
    }
    else {
        Err(PyValueError::new_err(format!(
            "a catalog is 2048 bytes (64 entries of 32), not {} - see Disc.catalog_bytes()",
            catalog_bytes.len()
        )))
    }
}

/// What printing the catalog `catalog_bytes` (the 2048 bytes of `Disc.catalog_bytes()`)
/// puts on the screen: the control codes and text, as a string.
#[pyfunction]
#[pyo3(signature = (catalog_bytes, kind="cat"))]
fn screen_output(catalog_bytes: &[u8], kind: &str) -> PyResult<String> {
    checked(catalog_bytes)?;
    catalog_screen_output(catalog_bytes, kind_of(kind)?).map_err(PyValueError::new_err)
}

/// The BASIC listing that prints the catalog art, as source text; `headers`
/// keeps the heading lines.
#[pyfunction]
#[pyo3(signature = (catalog_bytes, kind="cat", headers=true))]
fn basic_listing(catalog_bytes: &[u8], kind: &str, headers: bool) -> PyResult<String> {
    checked(catalog_bytes)?;
    catalog_to_basic_listing_with_headers(catalog_bytes, kind_of(kind)?, headers, &DiscardObserver)
        .map(|program| program.to_string())
        .map_err(PyValueError::new_err)
}

/// The same program, tokenized: what a `.BAS` file holds (see `Disc.add_basic`).
#[pyfunction]
#[pyo3(signature = (catalog_bytes, kind="cat", headers=true))]
fn basic_program<'py>(
    py: Python<'py>,
    catalog_bytes: &[u8],
    kind: &str,
    headers: bool
) -> PyResult<Bound<'py, PyBytes>> {
    checked(catalog_bytes)?;
    let program = catalog_to_basic_listing_with_headers(
        catalog_bytes,
        kind_of(kind)?,
        headers,
        &DiscardObserver
    )
    .map_err(PyValueError::new_err)?;
    Ok(PyBytes::new(py, &program.as_bytes()))
}

pub fn catalog(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(screen_output, m)?)?;
    m.add_function(wrap_pyfunction!(basic_listing, m)?)?;
    m.add_function(wrap_pyfunction!(basic_program, m)?)?;
    Ok(())
}
