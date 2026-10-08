//! Amstrad CPC discs (`.dsk`) and their AMSDOS catalog.
//!
//! ```python
//! from cpclib_python.disc import Disc
//!
//! dsk = Disc.create("new.dsk")                  # a formatted data disc
//! dsk.add_binary(code, "GAME.BIN", 0x4000, 0x4000)
//! dsk.catalog()                                 # [{'filename': 'GAME.BIN', ...}]
//! dsk.extract("GAME.BIN")                       # the bytes, header stripped
//! dsk.save()
//! ```

#![allow(unsafe_op_in_unsafe_fn)]

use std::sync::{Arc, Mutex};

use cpclib_common::camino::Utf8Path;
use cpclib_disc::amsdos::{
    AmsdosAddBehavior, AmsdosFile, AmsdosFileName, AmsdosFileType, AmsdosManagerMut,
    AmsdosManagerNonMut
};
use cpclib_disc::disc::Disc;
use cpclib_disc::edsk::{ExtendedDsk, Head};
use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

use crate::observer::PyObserver;

fn filename(name: &str, user: u8) -> PyResult<AmsdosFileName> {
    let mut filename = AmsdosFileName::try_from(name)
        .map_err(|e| PyValueError::new_err(format!("invalid AMSDOS filename '{name}': {e}")))?;
    filename.set_user(user);
    Ok(filename)
}

fn runtime<E: std::fmt::Display>(e: E) -> PyErr {
    PyRuntimeError::new_err(e.to_string())
}

/// A `.dsk` disc image.
#[pyclass(name = "Disc")]
pub struct PyDisc {
    path: Mutex<Option<String>>,
    dsk: Mutex<ExtendedDsk>
}

impl PyDisc {
    fn with<R>(&self, f: impl FnOnce(&mut ExtendedDsk) -> R) -> R {
        f(&mut self.dsk.lock().expect("disc lock poisoned"))
    }

    fn add(&self, file: AmsdosFile, name: &AmsdosFileName, replace: bool) -> PyResult<()> {
        let behavior = if replace {
            AmsdosAddBehavior::ReplaceAndEraseIfPresent
        }
        else {
            AmsdosAddBehavior::FailIfPresent
        };
        self.with(|dsk| {
            AmsdosManagerMut::new_from_disc(dsk, Head::A)
                .add_file(&file, Some(name), false, false, behavior)
                .map_err(runtime)
        })
    }
}

#[pymethods]
impl PyDisc {
    /// Opens the disc image at `path`.
    #[staticmethod]
    fn open(path: &str) -> PyResult<Self> {
        let dsk = ExtendedDsk::open(Utf8Path::new(path)).map_err(PyRuntimeError::new_err)?;
        Ok(Self {
            path: Mutex::new(Some(path.to_string())),
            dsk: Mutex::new(dsk)
        })
    }

    /// Creates a formatted, empty disc, saved at `path`. `format` is `data`
    /// (the default) or `data42` (42 tracks); anything else is read as the
    /// path of a format description file.
    #[staticmethod]
    #[pyo3(signature = (path, format="data"))]
    fn create(py: Python, path: &str, format: &str) -> PyResult<Self> {
        // the same operation as `dsk <path> format -f <format>`
        let task_args = shlex::try_join([path, "format", "-f", format]).map_err(runtime)?;
        let task: cpclib_bndbuild::task::Task = cpclib_bndbuild::task::InnerTask::Disc(
            cpclib_bndbuild::task::StandardTaskArguments::new(task_args)
        )
        .into();
        let observer = Arc::new(PyObserver::new(None));
        py.detach(|| task.execute(&observer))
            .map_err(PyRuntimeError::new_err)?;
        Self::open(path)
    }

    /// Where the disc was read from, or last saved to.
    #[getter]
    fn path(&self) -> Option<String> {
        self.path.lock().expect("disc lock poisoned").clone()
    }

    /// The AMSDOS catalog: one dict per file with `filename` (as `NAME.EXT`),
    /// `user`, `size_kb`, `read_only` and `system`.
    fn catalog<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyDict>>> {
        let entries = self.with(|dsk| {
            AmsdosManagerNonMut::new_from_disc(dsk, Head::A)
                .catalog()
                .map_err(runtime)
        })?;
        entries
            .visible_entries()
            .filter(|e| !e.is_erased())
            .map(|e| {
                let entry = PyDict::new(py);
                entry.set_item("filename", e.amsdos_filename().filename())?;
                entry.set_item("user", e.amsdos_filename().user())?;
                entry.set_item("size_kb", e.used_space())?;
                entry.set_item("read_only", e.is_read_only())?;
                entry.set_item("system", e.is_system())?;
                Ok(entry)
            })
            .collect()
    }

    /// The content of the file `name` (header stripped, when it has one).
    /// Raises `KeyError` if it is not on the disc.
    #[pyo3(signature = (name, user=0))]
    fn extract<'py>(&self, py: Python<'py>, name: &str, user: u8) -> PyResult<Bound<'py, PyBytes>> {
        let file = self.file(name, user)?;
        Ok(PyBytes::new(py, file.content()))
    }

    /// What the AMSDOS header of `name` says: `{"type": "basic" | "binary" |
    /// "protected" | "ascii", "load_address", "execution_address", "length"}`
    /// (the addresses only for a binary).
    #[pyo3(signature = (name, user=0))]
    fn file_info<'py>(
        &self,
        py: Python<'py>,
        name: &str,
        user: u8
    ) -> PyResult<Bound<'py, PyDict>> {
        let file = self.file(name, user)?;
        let info = PyDict::new(py);
        info.set_item("length", file.content().len())?;
        match file.header() {
            Some(header) if header.represent_a_valid_file() => {
                let kind = match header.file_type().map_err(runtime)? {
                    AmsdosFileType::Basic => "basic",
                    AmsdosFileType::Protected => "protected",
                    AmsdosFileType::Binary => "binary"
                };
                info.set_item("type", kind)?;
                if kind != "basic" {
                    info.set_item("load_address", header.loading_address())?;
                    info.set_item("execution_address", header.execution_address())?;
                }
            },
            _ => info.set_item("type", "ascii")?
        }
        Ok(info)
    }

    /// Adds the file at `path` as it is (no AMSDOS header is made), under
    /// `name` (default: its own name).
    #[pyo3(signature = (path, name=None, user=0, replace=true))]
    fn add_file(&self, path: &str, name: Option<&str>, user: u8, replace: bool) -> PyResult<()> {
        let data = std::fs::read(path)
            .map_err(|e| PyRuntimeError::new_err(format!("cannot read {path}: {e}")))?;
        let default_name = Utf8Path::new(path).file_name().unwrap_or(path);
        let name = filename(name.unwrap_or(default_name), user)?;
        self.add(AmsdosFile::from_buffer(&data), &name, replace)
    }

    /// Adds `data` as a binary file with an AMSDOS header (`load_address`,
    /// `execution_address`).
    #[pyo3(signature = (data, name, load_address, execution_address=None, user=0, replace=true))]
    fn add_binary(
        &self,
        data: &[u8],
        name: &str,
        load_address: u16,
        execution_address: Option<u16>,
        user: u8,
        replace: bool
    ) -> PyResult<()> {
        let name = filename(name, user)?;
        let file = AmsdosFile::binary_file_from_buffer(
            &name,
            load_address,
            execution_address.unwrap_or(load_address),
            data
        )
        .map_err(runtime)?;
        self.add(file, &name, replace)
    }

    /// Adds `data` (a tokenized BASIC program) as a BASIC file with its AMSDOS header.
    #[pyo3(signature = (data, name, user=0, replace=true))]
    fn add_basic(&self, data: &[u8], name: &str, user: u8, replace: bool) -> PyResult<()> {
        let name = filename(name, user)?;
        let file = AmsdosFile::basic_file_from_buffer(&name, data).map_err(runtime)?;
        self.add(file, &name, replace)
    }

    /// Adds `data` as an ASCII file (no header).
    #[pyo3(signature = (data, name, user=0, replace=true))]
    fn add_ascii(&self, data: &[u8], name: &str, user: u8, replace: bool) -> PyResult<()> {
        let name = filename(name, user)?;
        let file = AmsdosFile::ascii_file_from_buffer_with_name(&name, data);
        self.add(file, &name, replace)
    }

    /// Erases the file `name` from the catalog (and its sectors with `wipe`).
    #[pyo3(signature = (name, user=0, wipe=false))]
    fn erase(&self, name: &str, user: u8, wipe: bool) -> PyResult<()> {
        let name = filename(name, user)?;
        self.with(|dsk| {
            AmsdosManagerMut::new_from_disc(dsk, Head::A)
                .erase_file(name, wipe)
                .map_err(runtime)
        })
    }

    /// Renames `name` to `new_name`.
    #[pyo3(signature = (name, new_name, user=0))]
    fn rename(&self, name: &str, new_name: &str, user: u8) -> PyResult<()> {
        let (name, new_name) = (filename(name, user)?, filename(new_name, user)?);
        self.with(|dsk| {
            AmsdosManagerMut::new_from_disc(dsk, Head::A)
                .rename(name, new_name)
                .map_err(runtime)
        })
    }

    /// The raw bytes of one sector (`head` 0 or 1).
    fn read_sector<'py>(
        &self,
        py: Python<'py>,
        head: u8,
        track: u8,
        sector_id: u8
    ) -> PyResult<Bound<'py, PyBytes>> {
        self.with(|dsk| dsk.sector_read_bytes(head, track, sector_id))
            .map(|bytes| PyBytes::new(py, &bytes))
            .ok_or_else(|| {
                PyKeyError::new_err(format!(
                    "no sector {sector_id:#04x} on head {head}, track {track}"
                ))
            })
    }

    /// Overwrites one sector.
    fn write_sector(&self, head: u8, track: u8, sector_id: u8, data: &[u8]) -> PyResult<()> {
        self.with(|dsk| dsk.sector_write_bytes(head, track, sector_id, data))
            .map_err(PyRuntimeError::new_err)
    }

    /// Writes the disc to `path`, or back to where it came from.
    #[pyo3(signature = (path=None))]
    fn save(&self, path: Option<&str>) -> PyResult<()> {
        let mut known = self.path.lock().expect("disc lock poisoned");
        let target = path
            .map(str::to_string)
            .or_else(|| known.clone())
            .ok_or_else(|| PyValueError::new_err("this disc has no path: give one to save()"))?;
        self.with(|dsk| dsk.save(Utf8Path::new(&target)))
            .map_err(PyRuntimeError::new_err)?;
        *known = Some(target);
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!("Disc(path={:?})", self.path())
    }
}

impl PyDisc {
    fn file(&self, name: &str, user: u8) -> PyResult<AmsdosFile> {
        let wanted = filename(name, user)?;
        self.with(|dsk| {
            AmsdosManagerNonMut::new_from_disc(dsk, Head::A)
                .get_file(wanted)
                .map_err(runtime)
        })?
        .ok_or_else(|| PyKeyError::new_err(format!("no such file on the disc: {name}")))
    }
}

/// `data` with an AMSDOS header in front (what the `hideur` tool adds): a
/// binary at `load_address`, entered at `execution_address` (default: the load
/// address), named `name`.
#[pyfunction]
#[pyo3(signature = (data, name, load_address, execution_address=None))]
fn add_amsdos_header<'py>(
    py: Python<'py>,
    data: &[u8],
    name: &str,
    load_address: u16,
    execution_address: Option<u16>
) -> PyResult<Bound<'py, PyBytes>> {
    let name = filename(name, 0)?;
    let file = AmsdosFile::binary_file_from_buffer(
        &name,
        load_address,
        execution_address.unwrap_or(load_address),
        data
    )
    .map_err(runtime)?;
    Ok(PyBytes::new(py, file.header_and_content()))
}

/// Reads the AMSDOS header of `data` (a file as it is on a disc):
/// `(info, content)`, where `info` is `None` if there is no valid header and
/// else `{"type", "load_address", "execution_address", "length"}` as
/// `Disc.file_info` gives it.
#[pyfunction]
fn read_amsdos_header<'py>(
    py: Python<'py>,
    data: &[u8]
) -> PyResult<(Option<Bound<'py, PyDict>>, Bound<'py, PyBytes>)> {
    let file = AmsdosFile::from_buffer(data);
    let header = file.header().filter(|h| h.represent_a_valid_file());
    let info = match header {
        Some(header) => {
            let info = PyDict::new(py);
            let kind = match header.file_type().map_err(runtime)? {
                AmsdosFileType::Basic => "basic",
                AmsdosFileType::Protected => "protected",
                AmsdosFileType::Binary => "binary"
            };
            info.set_item("type", kind)?;
            info.set_item("length", file.content().len())?;
            if kind != "basic" {
                info.set_item("load_address", header.loading_address())?;
                info.set_item("execution_address", header.execution_address())?;
            }
            Some(info)
        },
        None => None
    };
    let content = if info.is_some() { file.content() } else { data };
    Ok((info, PyBytes::new(py, content)))
}

pub fn disc(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDisc>()?;
    m.add_function(wrap_pyfunction!(add_amsdos_header, m)?)?;
    m.add_function(wrap_pyfunction!(read_amsdos_header, m)?)?;
    Ok(())
}
