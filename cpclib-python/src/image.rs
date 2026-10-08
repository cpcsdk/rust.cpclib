//! CPC pictures: screen memory to PNG.
//!
//! ```python
//! from cpclib_python import image
//! from cpclib_python.sna import Snapshot
//!
//! sna = Snapshot.load("demo.sna")
//! png = image.render_screen(sna.memory(), mode=1)["png"]
//! open("screen.png", "wb").write(png)
//! ```

#![allow(unsafe_op_in_unsafe_fn)]

use base64::Engine;
use cpclib_dap::inspect::{
    DEFAULT_SCREEN_HEIGHT, DEFAULT_SCREEN_WIDTH, ScreenEncoding, crtc_screen_start_address,
    render_screen_view
};
use cpclib_image::ink::Ink;
use cpclib_image::palette::Palette;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

/// The real address space: `render_screen` always reads a full 64 KB image.
const FULL_MEMORY_SIZE: usize = 0x10000;

/// The address the screen starts at, from the CRTC registers 12 and 13.
#[pyfunction]
fn screen_address(r12: u8, r13: u8) -> usize {
    crtc_screen_start_address(r12, r13)
}

/// Renders the screen in `memory` (the 64 KB of the machine, `address` 0 first -
/// shorter dumps are padded) as a PNG.
///
/// - `address`: where the screen starts (default `0xC000`; see `screen_address`);
/// - `mode`: 0 to 3 (default 1); `width` in bytes (80), `height` in lines (200);
/// - `palette`: the firmware ink number (0 to 31) of each of the 16 pens
///   (default: the firmware's palette); `palette_override`: the same, `None`
///   for a pen left alone;
/// - `lines_per_char_row`: the CRTC's `R9 + 1` (8);
/// - `encoding`: `screen` (CRTC-accurate, the default) or `cpc` (the bytes as
///   they come, wrapping at 64 KB).
///
/// Returns `{"png": bytes, "width", "height", "mode", "address", "palette", ...}`.
#[pyfunction]
#[pyo3(signature = (memory, *, address=0xC000, mode=1, width=None, height=None, palette=None, palette_override=None, lines_per_char_row=8, encoding="screen"))]
fn render_screen<'py>(
    py: Python<'py>,
    memory: &[u8],
    address: usize,
    mode: u8,
    width: Option<usize>,
    height: Option<usize>,
    palette: Option<Vec<u8>>,
    palette_override: Option<Vec<Option<u8>>>,
    lines_per_char_row: usize,
    encoding: &str
) -> PyResult<Bound<'py, PyDict>> {
    let encoding = match encoding.to_ascii_lowercase().as_str() {
        "screen" => ScreenEncoding::Screen,
        "cpc" => ScreenEncoding::Cpc,
        other => {
            return Err(PyValueError::new_err(format!(
                "unknown encoding '{other}' - expected 'screen' or 'cpc'"
            )));
        }
    };

    let mut ink_palette = Palette::<Ink>::default();
    for (pen, &ink) in palette.iter().flatten().enumerate() {
        if ink > 31 {
            return Err(PyValueError::new_err(format!(
                "palette[{pen}] = {ink} is not a firmware ink number (0 to 31)"
            )));
        }
        ink_palette.set(pen as u8, Ink::from(ink));
    }
    let overrides: Vec<Option<Ink>> = palette_override
        .unwrap_or_default()
        .into_iter()
        .map(|ink| ink.map(Ink::from))
        .collect();

    let mut memory = memory.to_vec();
    memory.resize(FULL_MEMORY_SIZE, 0);
    memory.truncate(FULL_MEMORY_SIZE);

    let value = py
        .detach(|| {
            render_screen_view(
                address,
                width.unwrap_or(DEFAULT_SCREEN_WIDTH),
                height.unwrap_or(DEFAULT_SCREEN_HEIGHT),
                mode,
                &ink_palette,
                &memory,
                lines_per_char_row,
                &overrides,
                encoding
            )
        })
        .map_err(PyValueError::new_err)?;

    let rendered = PyDict::new(py);
    let png = value["png"]
        .as_str()
        .and_then(|b64| base64::engine::general_purpose::STANDARD.decode(b64).ok())
        .ok_or_else(|| PyValueError::new_err("the renderer returned no PNG"))?;
    rendered.set_item("png", PyBytes::new(py, &png))?;
    for key in ["address", "width", "height", "mode", "encoding"] {
        if let Some(v) = value.get(key) {
            match v {
                serde_json::Value::Number(n) if n.is_i64() => rendered.set_item(key, n.as_i64())?,
                serde_json::Value::String(s) => rendered.set_item(key, s)?,
                _ => {}
            }
        }
    }
    if let Some(palette) = value.get("palette") {
        rendered.set_item("palette", palette.to_string())?;
    }
    Ok(rendered)
}

pub fn image(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(screen_address, m)?)?;
    m.add_function(wrap_pyfunction!(render_screen, m)?)?;
    Ok(())
}
