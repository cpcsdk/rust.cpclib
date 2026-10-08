//! Every tool bndbuild knows, by name (native half: the `tools` module of the
//! package, in `python/cpclib_python/tools.py`, builds the functions on it).

#![allow(unsafe_op_in_unsafe_fn)]

use std::sync::Arc;

use cpclib_bndbuild::lsp::TASK_TYPES;
use cpclib_bndbuild::task;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::bndbuild::task_from_command;
use crate::observer::PyObserver;

/// Every name list bndbuild recognises, documented or not (the documented ones
/// are in `TASK_TYPES`, which a few tools are missing from).
const ALL_COMMANDS: &[&[&str]] = &[
    task::EMUCTRL_CMDS,
    task::ACE_CMDS,
    task::WINAPE_CMDS,
    task::CPCEC_CMDS,
    task::AMSPIRIT_CMDS,
    task::AMSPIRITLITE_CMDS,
    task::SUGARBOX_CMDS,
    task::CPCEMU_CMDS,
    task::CPCEMUPOWER_CMDS,
    task::CAPRICEFOREVER_CMDS,
    task::CADENCE_CMDS,
    task::EMULATOR_1984_CMDS,
    task::RETROVM_CMDS,
    task::ASMFMT_CMDS,
    task::BASMOPT_CMDS,
    task::BASM_CMDS,
    task::ORGAMS_CMDS,
    task::RASM_CMDS,
    task::SJASMPLUS_CMDS,
    task::UZ80_CMDS,
    task::VASM_CMDS,
    task::BASMDOC_CMDS,
    task::BDASM_CMDS,
    task::DISARK_CMDS,
    task::AT_CMDS,
    task::CHIPNSFX_CMDS,
    task::HSPC_CMDS,
    task::CP_CMDS,
    task::MV_CMDS,
    task::MKDIR_CMDS,
    task::RM_CMDS,
    task::ARCHIVE_CMDS,
    task::BNDBUILD_CMDS,
    task::CONVGENERIC_CMDS,
    task::DISC_CMDS,
    task::CATALOG_CMDS,
    task::LOCOMOTIVE_CMDS,
    task::ECHO_CMDS,
    task::EXTERN_CMDS,
    task::FAP_CMDS,
    task::AYT_CMDS,
    task::MINY_CMDS,
    task::FADE_CMDS,
    task::GRAFX2_CMDS,
    task::IMG2CPC_CMDS,
    task::CPC2IMG_CMDS,
    task::HIDEUR_CMDS,
    task::HXCFE_CMDS,
    task::IMPDISC_CMDS,
    task::MARTINE_CMDS,
    task::SNA_CMDS,
    task::XFER_CMDS,
    task::CPR_CMDS,
    task::CSL_CMDS,
    task::CRUNCH_CMDS,
    task::RTZX_CMDS,
    task::TWO_CDT_CMDS,
    task::SONG2AKM_CMDS,
    task::SONG2AKG_CMDS,
    task::SONG2AKY_CMDS,
    task::SONG2EVENTS_CMDS,
    task::SONG2RAW_CMDS,
    task::SONG2SOUNDEFFECTS_CMDS,
    task::SONG2VGM_CMDS,
    task::SONG2WAV_CMDS,
    task::SONG2YM_CMDS,
    task::Z80PROFILER_CMDS,
    task::VLINK_CMDS
];

/// Every command: `{"name", "aliases", "description", "synopsis", "example"}`
/// (`name` is the canonical one; the texts are empty for the few tools bndbuild
/// does not document).
#[pyfunction]
fn commands<'py>(py: Python<'py>) -> PyResult<Vec<Bound<'py, PyDict>>> {
    let documented = |names: &[&str]| TASK_TYPES.iter().any(|t| t.names.first() == names.first());
    let entry = |names: &[&str], description: &str, synopsis: &str, example: &str| {
        let entry = PyDict::new(py);
        entry.set_item("name", names[0])?;
        entry.set_item("aliases", &names[1..])?;
        entry.set_item("description", description)?;
        entry.set_item("synopsis", synopsis)?;
        entry.set_item("example", example)?;
        PyResult::Ok(entry)
    };

    let mut commands = TASK_TYPES
        .iter()
        .map(|t| entry(t.names, t.description, t.synopsis, t.example))
        .collect::<PyResult<Vec<_>>>()?;
    for names in ALL_COMMANDS {
        if !names.is_empty() && !documented(names) {
            commands.push(entry(names, "", "", "")?);
        }
    }
    Ok(commands)
}

/// Runs the tool `command` with `args` (as on its command line).
///
/// Its output goes to the console, or to `on_output(kind, text)`; with
/// `capture`, returns `{"stdout": ..., "stderr": ...}`. Raises `ValueError`
/// for a command bndbuild does not know and `RuntimeError` when the tool fails.
#[pyfunction]
#[pyo3(signature = (command, args=None, on_output=None, capture=false))]
fn run<'py>(
    py: Python<'py>,
    command: &str,
    args: Option<Vec<String>>,
    on_output: Option<Py<PyAny>>,
    capture: bool
) -> PyResult<Option<Bound<'py, PyDict>>> {
    let task =
        task_from_command(command, &args.unwrap_or_default()).map_err(PyValueError::new_err)?;
    let observer = Arc::new(PyObserver::new(on_output));
    let result = py.detach(|| task.execute(&observer));
    let (stdout, stderr) = observer.captured();
    result.map_err(|e| PyRuntimeError::new_err(format!("{e}\n{stderr}").trim().to_string()))?;
    if capture {
        let captured = PyDict::new(py);
        captured.set_item("stdout", stdout)?;
        captured.set_item("stderr", stderr)?;
        Ok(Some(captured))
    }
    else {
        Ok(None)
    }
}

pub fn tools(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(commands, m)?)?;
    m.add_function(wrap_pyfunction!(run, m)?)?;
    Ok(())
}
