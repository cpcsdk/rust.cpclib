//! The M4 board / CPC Wifi: send programs to a real CPC over the network.
//!
//! ```python
//! from cpclib_python.xfer import M4
//!
//! m4 = M4("192.168.1.50")
//! m4.upload_and_run("demo.sna")
//! m4.ls()
//! ```

#![allow(unsafe_op_in_unsafe_fn)]

use cpclib_common::camino::Utf8Path;
use cpclib_disc::amsdos::AmsdosFileType;
use cpclib_sna::Snapshot;
use cpclib_xfer::{CpcXfer, XferError};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

/// Runs a network call with the GIL released (its error is not `Send`: it is
/// turned into text before it crosses threads).
fn net<R: Send>(
    py: Python,
    call: impl FnOnce() -> Result<R, Box<XferError>> + Send
) -> PyResult<R> {
    py.detach(|| call().map_err(|e| e.to_string()))
        .map_err(PyRuntimeError::new_err)
}

/// The AMSDOS header to give a file that has none: `(type, load, exec)`.
fn header_of(header: Option<(&str, u16, u16)>) -> PyResult<Option<(AmsdosFileType, u16, u16)>> {
    header
        .map(|(kind, load, exec)| {
            let kind = match kind.to_ascii_lowercase().as_str() {
                "binary" => AmsdosFileType::Binary,
                "basic" => AmsdosFileType::Basic,
                "protected" => AmsdosFileType::Protected,
                other => {
                    return Err(PyValueError::new_err(format!(
                        "unknown file type '{other}' - expected binary, basic or protected"
                    )));
                }
            };
            Ok((kind, load, exec))
        })
        .transpose()
}

/// A M4 board (or CPC Wifi) on the network.
#[pyclass(name = "M4", frozen)]
pub struct PyM4 {
    xfer: CpcXfer
}

#[pymethods]
impl PyM4 {
    /// The board at `hostname` (an address or a name).
    #[new]
    fn new(hostname: &str) -> Self {
        Self {
            xfer: CpcXfer::new(hostname)
        }
    }

    #[getter]
    fn hostname(&self) -> String {
        self.xfer.hostname().to_string()
    }

    /// Resets the M4 board.
    fn reset_m4(&self, py: Python) -> PyResult<()> {
        net(py, || self.xfer.reset_m4())
    }

    /// Resets the CPC.
    fn reset_cpc(&self, py: Python) -> PyResult<()> {
        net(py, || self.xfer.reset_cpc())
    }

    /// Runs the file at `path` on the board's card.
    fn run(&self, py: Python, path: &str) -> PyResult<()> {
        net(py, || self.xfer.run(path))
    }

    /// Deletes `path` from the board's card.
    fn rm(&self, py: Python, path: &str) -> PyResult<()> {
        net(py, || self.xfer.rm(path))
    }

    /// Sends the local file `path` to `remote_dir` on the card. `header`, a
    /// `(type, load_address, execution_address)` tuple (type: `binary`, `basic`
    /// or `protected`), gives it an AMSDOS header if it has none.
    #[pyo3(signature = (path, remote_dir="/tmp", header=None))]
    fn upload(
        &self,
        py: Python,
        path: &str,
        remote_dir: &str,
        header: Option<(String, u16, u16)>
    ) -> PyResult<()> {
        let header = header_of(header.as_ref().map(|(k, l, e)| (k.as_str(), *l, *e)))?;
        net(py, || {
            self.xfer.upload(Utf8Path::new(path), remote_dir, header)
        })
    }

    /// Sends the local file `path` and runs it (a `.sna` is sent as a snapshot).
    #[pyo3(signature = (path, header=None))]
    fn upload_and_run(
        &self,
        py: Python,
        path: &str,
        header: Option<(String, u16, u16)>
    ) -> PyResult<()> {
        let header = header_of(header.as_ref().map(|(k, l, e)| (k.as_str(), *l, *e)))?;
        py.detach(|| {
            if path.to_ascii_lowercase().ends_with(".sna") {
                let sna = Snapshot::load(Utf8Path::new(path)).map_err(|e| e.to_string())?;
                self.xfer
                    .upload_and_run_sna(&sna)
                    .map_err(|e| e.to_string())
            }
            else {
                self.xfer
                    .upload_and_run(Utf8Path::new(path), header)
                    .map_err(|e| e.to_string())
            }
        })
        .map_err(PyRuntimeError::new_err)
    }

    /// The board's current directory.
    fn pwd(&self, py: Python) -> PyResult<String> {
        net(py, || self.xfer.current_working_directory())
    }

    /// Goes to `directory` on the card.
    fn cd(&self, py: Python, directory: &str) -> PyResult<()> {
        net(py, || self.xfer.cd(directory))
    }

    /// The names of the files in the current directory.
    fn ls(&self, py: Python) -> PyResult<Vec<String>> {
        net(py, || {
            self.xfer.current_folder_content().map(|listing| {
                listing
                    .files()
                    .iter()
                    .map(|f| f.fname().to_string())
                    .collect()
            })
        })
    }

    fn __repr__(&self) -> String {
        format!("M4({:?})", self.xfer.hostname())
    }
}

pub fn xfer(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyM4>()?;
    Ok(())
}
