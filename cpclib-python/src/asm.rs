//! The basm assembler, with all its options.
//!
//! ```python
//! from cpclib_python import asm
//!
//! built = asm.assemble("org #4000 : start: db 1,2,3 : dw start", listing=True)
//! built.data           # b'\x01\x02\x03\x00@'
//! built.symbols        # {'start': 16384}
//! asm.assemble_file("main.asm", include_dirs=["lib"], defines={"DEBUG": 1},
//!                   snapshot="main.sna")
//! ```

#![allow(unsafe_op_in_unsafe_fn)]

use std::collections::HashMap;
use std::sync::Arc;

use cpclib_common::camino::Utf8PathBuf;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

use crate::bndbuild::task_from_command;
use crate::observer::PyObserver;

/// What an assembly produced.
#[pyclass(name = "Assembled", frozen)]
pub struct PyAssembled {
    data: Option<Vec<u8>>,
    /// The integer symbols (labels, equates) as `{name: value}`.
    #[pyo3(get)]
    symbols: HashMap<String, i64>,
    /// The listing, when asked for.
    #[pyo3(get)]
    listing: Option<String>,
    /// What the assembler said: warnings, `Assembled in 1 pass...`.
    #[pyo3(get)]
    messages: String
}

#[pymethods]
impl PyAssembled {
    /// The bytes produced, as one block (None when a snapshot or a cartridge
    /// was asked for instead).
    #[getter]
    fn data<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.data.as_ref().map(|d| PyBytes::new(py, d))
    }

    fn __repr__(&self) -> String {
        format!(
            "Assembled(bytes={:?}, symbols={})",
            self.data.as_ref().map(Vec::len),
            self.symbols.len()
        )
    }
}

/// `{name: value}` from basm's symbol file (`name equ #4000` lines); the
/// symbols whose value is not an integer are left out.
fn parse_symbols(text: &str) -> HashMap<String, i64> {
    text.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let name = words.next()?;
            if !words.next()?.eq_ignore_ascii_case("equ") {
                return None;
            }
            let value = words.next()?;
            let value = match value
                .strip_prefix('#')
                .or_else(|| value.strip_prefix('$'))
                .or_else(|| value.strip_prefix("0x"))
            {
                Some(hex) => i64::from_str_radix(hex, 16).ok()?,
                None => value.parse().ok()?
            };
            Some((name.to_string(), value))
        })
        .collect()
}

/// What the user can ask basm for.
struct Request {
    defines: Vec<(String, String)>,
    include_dirs: Vec<String>,
    case_insensitive: bool,
    listing: bool,
    symbols: bool,
    snapshot: Option<String>,
    cartridge: Option<String>,
    extra_args: Vec<String>
}

fn run(
    py: Python,
    input: Vec<String>,
    request: Request,
    on_output: Option<Py<PyAny>>
) -> PyResult<PyAssembled> {
    let dir = camino_tempfile::tempdir()
        .map_err(|e| PyRuntimeError::new_err(format!("no temp directory: {e}")))?;
    let (bin, sym, lst) = (
        dir.path().join("out.bin"),
        dir.path().join("out.sym"),
        dir.path().join("out.lst")
    );

    let mut args: Vec<String> = Vec::new();
    for dir in &request.include_dirs {
        args.extend(["-I".to_string(), dir.clone()]);
    }
    for (name, value) in &request.defines {
        args.push(format!("-D{name}={value}"));
    }
    if request.case_insensitive {
        args.push("--case-insensitive".to_string());
    }
    // `-o` is the snapshot or the cartridge, when one is asked for
    let raw_output = request.snapshot.is_none() && request.cartridge.is_none();
    if let Some(snapshot) = &request.snapshot {
        args.extend(["--snapshot".to_string(), "-o".to_string(), snapshot.clone()]);
    }
    else if let Some(cartridge) = &request.cartridge {
        args.extend([
            "--cartridge".to_string(),
            "-o".to_string(),
            cartridge.clone()
        ]);
    }
    else {
        args.extend(["-o".to_string(), bin.to_string()]);
    }
    if request.symbols {
        args.extend(["--sym".to_string(), sym.to_string()]);
    }
    if request.listing {
        args.extend(["--lst".to_string(), lst.to_string()]);
    }
    args.extend(request.extra_args);
    args.extend(input);

    let task = task_from_command("basm", &args).map_err(PyRuntimeError::new_err)?;
    let observer = Arc::new(PyObserver::new(on_output));
    let result = py.detach(|| task.execute(&observer));
    let (stdout, stderr) = observer.captured();
    result.map_err(|e| {
        PyRuntimeError::new_err(format!("{e}\n{stderr}{stdout}").trim().to_string())
    })?;

    let read = |path: &Utf8PathBuf| std::fs::read(path).ok();
    Ok(PyAssembled {
        data: raw_output.then(|| read(&bin)).flatten(),
        symbols: request
            .symbols
            .then(|| read(&sym))
            .flatten()
            .map(|b| parse_symbols(&String::from_utf8_lossy(&b)))
            .unwrap_or_default(),
        listing: request
            .listing
            .then(|| read(&lst))
            .flatten()
            .map(|b| String::from_utf8_lossy(&b).to_string()),
        messages: format!("{stdout}{stderr}")
    })
}

fn defines_of(defines: Option<&Bound<'_, PyDict>>) -> PyResult<Vec<(String, String)>> {
    defines
        .map(|d| {
            d.iter()
                .map(|(k, v)| Ok((k.str()?.to_string(), v.str()?.to_string())))
                .collect()
        })
        .unwrap_or_else(|| Ok(Vec::new()))
}

/// Assembles `source` (Z80 code, in basm's syntax).
///
/// - `defines`: `{"NAME": value}` symbols, as `-DNAME=value`;
/// - `include_dirs`: where `include`/`incbin` look for files;
/// - `case_insensitive`: labels and macros ignore their case;
/// - `symbols` / `listing`: also return the symbol table / the listing;
/// - `extra_args`: any other basm option, as on its command line.
///
/// Raises `RuntimeError`, with the assembler's message, if the assembly fails.
#[pyfunction]
#[pyo3(signature = (source, *, defines=None, include_dirs=None, case_insensitive=false, symbols=true, listing=false, extra_args=None, on_output=None))]
fn assemble(
    py: Python,
    source: &str,
    defines: Option<&Bound<'_, PyDict>>,
    include_dirs: Option<Vec<String>>,
    case_insensitive: bool,
    symbols: bool,
    listing: bool,
    extra_args: Option<Vec<String>>,
    on_output: Option<Py<PyAny>>
) -> PyResult<PyAssembled> {
    let request = Request {
        defines: defines_of(defines)?,
        include_dirs: include_dirs.unwrap_or_default(),
        case_insensitive,
        listing,
        symbols,
        snapshot: None,
        cartridge: None,
        extra_args: extra_args.unwrap_or_default()
    };
    run(
        py,
        vec!["--inline".to_string(), source.to_string()],
        request,
        on_output
    )
}

/// Assembles the file `path`, as `assemble` does a string - and optionally
/// writes a `snapshot` (`.sna`) or a `cartridge` (`.cpr`) instead of returning
/// the bytes.
#[pyfunction]
#[pyo3(signature = (path, *, defines=None, include_dirs=None, case_insensitive=false, symbols=true, listing=false, snapshot=None, cartridge=None, extra_args=None, on_output=None))]
fn assemble_file(
    py: Python,
    path: &str,
    defines: Option<&Bound<'_, PyDict>>,
    include_dirs: Option<Vec<String>>,
    case_insensitive: bool,
    symbols: bool,
    listing: bool,
    snapshot: Option<String>,
    cartridge: Option<String>,
    extra_args: Option<Vec<String>>,
    on_output: Option<Py<PyAny>>
) -> PyResult<PyAssembled> {
    let request = Request {
        defines: defines_of(defines)?,
        include_dirs: include_dirs.unwrap_or_default(),
        case_insensitive,
        listing,
        symbols,
        snapshot,
        cartridge,
        extra_args: extra_args.unwrap_or_default()
    };
    run(py, vec![path.to_string()], request, on_output)
}

pub fn asm(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyAssembled>()?;
    m.add_function(wrap_pyfunction!(assemble, m)?)?;
    m.add_function(wrap_pyfunction!(assemble_file, m)?)?;
    Ok(())
}
