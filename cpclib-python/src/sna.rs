//! Amstrad CPC snapshots (`.sna`): read, inspect, patch and write them.
//!
//! ```python
//! from cpclib_python.sna import Snapshot
//!
//! sna = Snapshot.load("demo.sna")
//! sna.get("Z80_PC"), sna.get("CRTC_REG:1")
//! sna.write(0x4000, b"\x01\x02")
//! sna.set("Z80_PC", 0x4000)
//! sna.save("patched.sna")
//! ```

#![allow(unsafe_op_in_unsafe_fn)]

use std::sync::Mutex;

use cpclib_common::camino::Utf8Path;
use cpclib_sna::flags::{FlagValue, SnapshotFlag};
use cpclib_sna::{Snapshot, SnapshotVersion};
use pyo3::exceptions::{PyIndexError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};

fn version_of(version: u8) -> PyResult<SnapshotVersion> {
    match version {
        1 => Ok(SnapshotVersion::V1),
        2 => Ok(SnapshotVersion::V2),
        3 => Ok(SnapshotVersion::V3),
        v => {
            Err(PyValueError::new_err(format!(
                "snapshot version must be 1, 2 or 3, not {v}"
            )))
        },
    }
}

fn flag_of(name: &str) -> PyResult<SnapshotFlag> {
    name.parse().map_err(|e: String| {
        PyValueError::new_err(format!(
            "{name}: {e} (flags look like `Z80_PC`, `GA_ROMCFG`, `CRTC_REG:6`, `GA_PAL:0`)"
        ))
    })
}

fn flag_value<'py>(py: Python<'py>, value: &FlagValue) -> PyResult<Bound<'py, PyAny>> {
    Ok(match value {
        FlagValue::Byte(b) => b.into_pyobject(py)?.into_any(),
        FlagValue::Word(w) => w.into_pyobject(py)?.into_any(),
        FlagValue::Array(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(flag_value(py, item)?)?;
            }
            list.into_any()
        }
    })
}

/// The name of `flag` as `Snapshot.get` takes it (an indexed flag, whole, is
/// the bare name).
fn flag_name(flag: &SnapshotFlag) -> String {
    let debug = format!("{flag:?}");
    debug.split('(').next().unwrap_or(&debug).to_string()
}

/// A CPC snapshot, in memory.
#[pyclass(name = "Snapshot")]
pub struct PySnapshot {
    inner: Mutex<Snapshot>
}

impl PySnapshot {
    /// Wraps a snapshot made elsewhere (`basic.to_snapshot`).
    pub(crate) fn from_inner(snapshot: Snapshot) -> Self {
        Self {
            inner: Mutex::new(snapshot)
        }
    }

    fn with<R>(&self, f: impl FnOnce(&mut Snapshot) -> R) -> R {
        f(&mut self.inner.lock().expect("snapshot lock poisoned"))
    }
}

#[pymethods]
impl PySnapshot {
    /// A fresh 6128 snapshot.
    #[new]
    fn new() -> PyResult<Self> {
        Snapshot::new_6128()
            .map(|s| {
                Self {
                    inner: Mutex::new(s)
                }
            })
            .map_err(PyRuntimeError::new_err)
    }

    /// Reads the snapshot at `path`.
    #[staticmethod]
    fn load(path: &str) -> PyResult<Self> {
        Snapshot::load(Utf8Path::new(path))
            .map(|s| {
                Self {
                    inner: Mutex::new(s)
                }
            })
            .map_err(PyRuntimeError::new_err)
    }

    /// Reads a snapshot from its raw bytes.
    #[staticmethod]
    fn from_bytes(data: &[u8]) -> PyResult<Self> {
        Snapshot::from_buffer(data.to_vec())
            .map(|s| {
                Self {
                    inner: Mutex::new(s)
                }
            })
            .map_err(PyRuntimeError::new_err)
    }

    /// The snapshot's format version: 1, 2 or 3 (3 stores memory and more in chunks).
    #[getter]
    fn version(&self) -> u8 {
        self.with(|s| s.version_header())
    }

    /// The memory the snapshot holds, in KB (a version 3 snapshot may keep
    /// it in chunks, with 0 declared in its header).
    #[getter]
    fn memory_kb(&self) -> PyResult<usize> {
        let dump = self
            .with(|s| s.memory_dump())
            .map_err(PyRuntimeError::new_err)?;
        Ok(dump.len() / 1024)
    }

    /// The value of a header flag: an int, or a list of ints for an array flag
    /// (`GA_PAL`, `CRTC_REG`, `PSG_REG`, ... without an index).
    fn get<'py>(&self, py: Python<'py>, flag: &str) -> PyResult<Bound<'py, PyAny>> {
        let flag = match flag.to_uppercase().as_str() {
            // the arrays, whole
            "GA_PAL" => SnapshotFlag::GA_PAL(None),
            "CRTC_REG" => SnapshotFlag::CRTC_REG(None),
            "PSG_REG" => SnapshotFlag::PSG_REG(None),
            "GA_MULTIMODE" => SnapshotFlag::GA_MULTIMODE(None),
            _ => flag_of(flag)?
        };
        let value = self.with(|s| s.get_value(&flag));
        flag_value(py, &value)
    }

    /// Sets a header flag. Raises `ValueError` for an unknown flag or a value
    /// that does not fit it.
    fn set(&self, flag: &str, value: u16) -> PyResult<()> {
        let flag = flag_of(flag)?;
        self.with(|s| s.set_value(flag, value))
            .map_err(|e| PyValueError::new_err(format!("{e:?}")))
    }

    /// Every header flag, as `{name: value}` (the arrays as lists).
    fn flags<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let flags = PyDict::new(py);
        for flag in SnapshotFlag::enumerate() {
            let value = self.with(|s| s.get_value(flag));
            flags.set_item(flag_name(flag), flag_value(py, &value)?)?;
        }
        Ok(flags)
    }

    /// The whole memory, linear.
    fn memory<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let dump = self
            .with(|s| s.memory_dump())
            .map_err(PyRuntimeError::new_err)?;
        Ok(PyBytes::new(py, &dump))
    }

    /// `count` bytes of memory from `address`.
    fn read<'py>(
        &self,
        py: Python<'py>,
        address: usize,
        count: usize
    ) -> PyResult<Bound<'py, PyBytes>> {
        let dump = self
            .with(|s| s.memory_dump())
            .map_err(PyRuntimeError::new_err)?;
        let end = address
            .checked_add(count)
            .filter(|&e| e <= dump.len())
            .ok_or_else(|| {
                PyIndexError::new_err(format!(
                    "reading {count} bytes at {address:#x}: the snapshot has {} bytes of memory",
                    dump.len()
                ))
            })?;
        Ok(PyBytes::new(py, &dump[address..end]))
    }

    /// Writes `data` to memory from `address` (growing the memory if needed).
    fn write(&self, address: u32, data: &[u8]) {
        self.with(|s| {
            for (offset, byte) in data.iter().enumerate() {
                s.set_byte(address + offset as u32, *byte);
            }
        })
    }

    /// Loads the file at `path` into memory from `address`.
    fn load_file(&self, path: &str, address: u32) -> PyResult<()> {
        let data = std::fs::read(path)
            .map_err(|e| PyRuntimeError::new_err(format!("cannot read {path}: {e}")))?;
        self.write(address, &data);
        Ok(())
    }

    /// The four-letter codes of the chunks of a version 3 snapshot.
    fn chunks(&self) -> Vec<String> {
        self.with(|s| {
            s.chunks()
                .iter()
                .map(|c| c.code().as_str().to_string())
                .collect()
        })
    }

    /// Writes the snapshot at `path`, as `version` 1, 2 (default) or 3.
    #[pyo3(signature = (path, version=2))]
    fn save(&self, path: &str, version: u8) -> PyResult<()> {
        let version = version_of(version)?;
        self.with(|s| s.save(Utf8Path::new(path), version))
            .map_err(|e| PyRuntimeError::new_err(format!("cannot write {path}: {e}")))
    }

    /// The snapshot as raw bytes, as `version` 1, 2 (default) or 3.
    #[pyo3(signature = (version=2))]
    fn to_bytes<'py>(&self, py: Python<'py>, version: u8) -> PyResult<Bound<'py, PyBytes>> {
        let version = version_of(version)?;
        let mut buffer = Vec::new();
        self.with(|s| s.write_all(&mut buffer, version))
            .map_err(|e| PyRuntimeError::new_err(format!("cannot serialise the snapshot: {e}")))?;
        Ok(PyBytes::new(py, &buffer))
    }

    fn __repr__(&self) -> String {
        self.with(|s| {
            format!(
                "Snapshot(version={}, memory_header_kb={}, chunks={})",
                s.version_header(),
                s.memory_size_header(),
                s.nb_chunks()
            )
        })
    }
}

pub fn sna(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySnapshot>()?;
    Ok(())
}
