//! The Z80 disassembler.
//!
//! ```python
//! from cpclib_python import bdasm
//!
//! bdasm.disassemble(b"\x3e\x01\xc9", origin=0x4000)
//! # [{'address': 16384, 'bytes': b'>\x01', 'text': 'LD A, 1', 'nops': 2}, ...]
//! ```

#![allow(unsafe_op_in_unsafe_fn)]

use cpclib_asm::preamble::*;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

/// Disassembles `data`, as if loaded at `origin`: one dict per instruction with
/// its `address`, its `bytes`, its `text` and its `nops` (the cost in NOPs, None
/// when it depends on a condition).
#[pyfunction]
#[pyo3(signature = (data, origin=0))]
fn disassemble<'py>(
    py: Python<'py>,
    data: &[u8],
    origin: u16
) -> PyResult<Vec<Bound<'py, PyDict>>> {
    let listing = cpclib_asm::disass::disassemble(data);
    let mut address = usize::from(origin);
    let mut instructions = Vec::new();
    for token in listing.listing() {
        let bytes = token.to_bytes().unwrap_or_default();
        let instruction = PyDict::new(py);
        instruction.set_item("address", address & 0xFFFF)?;
        instruction.set_item("bytes", PyBytes::new(py, &bytes))?;
        instruction.set_item("text", token.to_string().trim().to_string())?;
        instruction.set_item("nops", token.estimated_duration().ok())?;
        instructions.push(instruction);
        address += bytes.len();
    }
    Ok(instructions)
}

/// `disassemble`, as source text: an `org` then one instruction per line.
#[pyfunction]
#[pyo3(signature = (data, origin=0))]
fn disassemble_to_source(data: &[u8], origin: u16) -> String {
    let listing = cpclib_asm::disass::disassemble(data);
    let mut source = format!("    org {origin:#06x}\n");
    for token in listing.listing() {
        source.push_str("    ");
        source.push_str(token.to_string().trim());
        source.push('\n');
    }
    source
}

pub fn bdasm(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(disassemble, m)?)?;
    m.add_function(wrap_pyfunction!(disassemble_to_source, m)?)?;
    Ok(())
}
