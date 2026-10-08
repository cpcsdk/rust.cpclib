//! Plus-range cartridges (`.cpr`): their 16 KB banks.
//!
//! ```python
//! from cpclib_python.cpr import Cartridge
//!
//! cpr = Cartridge.load("game.cpr")
//! cpr.banks()                       # [0, 1, 2]
//! cpr.bank(0)[:4]
//! cpr.set_bank(3, bytes(0x4000))
//! cpr.save("patched.cpr")
//! ```

#![allow(unsafe_op_in_unsafe_fn)]

use std::sync::Mutex;

use cpclib_common::camino::Utf8Path;
use cpclib_common::riff::RiffChunk;
use cpclib_cpr::{CartridgeBank, Cpr as Cartridge};
use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

const BANK_SIZE: usize = 0x4000;

/// A cartridge image.
#[pyclass(name = "Cartridge")]
pub struct PyCartridge {
    inner: Mutex<Cartridge>
}

impl PyCartridge {
    fn with<R>(&self, f: impl FnOnce(&mut Cartridge) -> R) -> R {
        f(&mut self.inner.lock().expect("cartridge lock poisoned"))
    }
}

#[pymethods]
impl PyCartridge {
    /// An empty cartridge.
    #[new]
    fn new() -> Self {
        Self {
            inner: Mutex::new(Cartridge::empty())
        }
    }

    /// Reads the cartridge at `path`.
    #[staticmethod]
    fn load(path: &str) -> PyResult<Self> {
        Cartridge::load(Utf8Path::new(path))
            .map(|c| {
                Self {
                    inner: Mutex::new(c)
                }
            })
            .map_err(PyRuntimeError::new_err)
    }

    /// Reads a cartridge from its raw bytes.
    #[staticmethod]
    fn from_bytes(data: &[u8]) -> PyResult<Self> {
        Cartridge::from_buffer(data.to_vec())
            .map(|c| {
                Self {
                    inner: Mutex::new(c)
                }
            })
            .map_err(PyRuntimeError::new_err)
    }

    /// The numbers of the banks it holds, in order.
    fn banks(&self) -> Vec<u16> {
        self.with(|c| c.banks().iter().map(|b| u16::from(b.number())).collect())
    }

    /// The content of bank `number`. Raises `KeyError` if there is none.
    fn bank<'py>(&self, py: Python<'py>, number: u8) -> PyResult<Bound<'py, PyBytes>> {
        self.with(|c| c.bank_by_num(number).map(|b| PyBytes::new(py, b.data())))
            .ok_or_else(|| PyKeyError::new_err(format!("no bank {number}")))
    }

    /// Sets bank `number` to `data` (at most 16 KB, padded with zeros), adding
    /// the bank if the cartridge has none of that number.
    fn set_bank(&self, number: u8, data: &[u8]) -> PyResult<()> {
        if data.len() > BANK_SIZE {
            return Err(PyValueError::new_err(format!(
                "a bank holds {BANK_SIZE} bytes, not {}",
                data.len()
            )));
        }
        if number > 31 {
            return Err(PyValueError::new_err("bank numbers go from 0 to 31"));
        }
        let mut content = data.to_vec();
        content.resize(BANK_SIZE, 0);
        let chunk = RiffChunk::new(CartridgeBank::code_for(number), content);
        let bank = CartridgeBank::try_from(chunk).map_err(PyValueError::new_err)?;
        self.with(|c| {
            c.remove_bank(number);
            c.add_bank(bank);
        });
        Ok(())
    }

    /// Removes bank `number`; returns whether there was one.
    fn remove_bank(&self, number: u8) -> bool {
        self.with(|c| c.remove_bank(number)).is_some()
    }

    /// Writes the cartridge at `path`.
    fn save(&self, path: &str) -> PyResult<()> {
        self.with(|c| c.save(Utf8Path::new(path)))
            .map_err(|e| PyRuntimeError::new_err(format!("cannot write {path}: {e}")))
    }

    /// The cartridge as raw bytes.
    fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let mut buffer = Vec::new();
        self.with(|c| c.write_all(&mut buffer))
            .map_err(|e| PyRuntimeError::new_err(format!("cannot serialise the cartridge: {e}")))?;
        Ok(PyBytes::new(py, &buffer))
    }

    fn __repr__(&self) -> String {
        format!("Cartridge(banks={:?})", self.banks())
    }
}

pub fn cpr(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCartridge>()?;
    Ok(())
}
