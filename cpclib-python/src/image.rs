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

/// Converts the picture `source` (PNG, GIF, ...) to the CPC's formats - the
/// `img2cpc` tool - and writes it as `to` at `output`:
///
/// - `scr`: an OCP screen file (`-p`: pass `target_args=["-p", "pic.pal"]`);
/// - `sna`: a snapshot showing it; `dsk`: a disc that shows it;
/// - `exec`: a binary to copy to a disc or an M4;
/// - `sprite`, `tile`: sprite or tile data - see `img2cpc sprite --help`.
///
/// Options: `mode` (0, 1 or 2), `pens` (`{pen: firmware ink number}`, the inks fixed by hand),
/// `crop` (cut what does not fit), `fullscreen` / `overscan` / `standard` (the
/// screen's size), `dither` (an algorithm: switches to true-colour
/// conversion), `colors` (at most this many), `resize_filter`. Anything else
/// the tool accepts goes in `extra_args` (before the target) and `target_args`
/// (after it).
///
/// Raises `ValueError` for a bad option and `RuntimeError` with the tool's
/// message when the conversion fails.
#[pyfunction]
#[pyo3(signature = (source, to, output, *, mode=None, pens=None, crop=false, fullscreen=false, overscan=false, standard=false, dither=None, colors=None, resize_filter=None, extra_args=None, target_args=None, on_output=None))]
#[allow(clippy::too_many_arguments)]
fn convert(
    py: Python,
    source: &str,
    to: &str,
    output: &str,
    mode: Option<u8>,
    pens: Option<std::collections::BTreeMap<u8, u8>>,
    crop: bool,
    fullscreen: bool,
    overscan: bool,
    standard: bool,
    dither: Option<String>,
    colors: Option<u8>,
    resize_filter: Option<String>,
    extra_args: Option<Vec<String>>,
    target_args: Option<Vec<String>>,
    on_output: Option<Py<PyAny>>
) -> PyResult<()> {
    let mut args = vec![source.to_string()];
    if let Some(mode) = mode {
        if mode > 2 {
            return Err(PyValueError::new_err("mode must be 0, 1 or 2"));
        }
        args.extend(["--mode".to_string(), mode.to_string()]);
    }
    for (pen, ink) in pens.unwrap_or_default() {
        if pen > 15 {
            return Err(PyValueError::new_err(format!(
                "pen {pen}: pens go from 0 to 15"
            )));
        }
        args.extend([format!("--pen{pen}"), ink.to_string()]);
    }
    for (flag, on) in [
        ("--crop", crop),
        ("--fullscreen", fullscreen),
        ("--overscan", overscan),
        ("--standard", standard)
    ] {
        if on {
            args.push(flag.to_string());
        }
    }
    for (flag, value) in [
        ("--dither", dither),
        ("--colors", colors.map(|c| c.to_string())),
        ("--resize-filter", resize_filter)
    ] {
        if let Some(value) = value {
            args.extend([flag.to_string(), value]);
        }
    }
    args.extend(extra_args.unwrap_or_default());

    // the target: what it writes, and where
    args.push(to.to_string());
    match to {
        "sna" | "dsk" | "exec" => args.push(output.to_string()),
        "scr" | "sprite" | "tile" => args.extend(["-o".to_string(), output.to_string()]),
        other => {
            return Err(PyValueError::new_err(format!(
                "unknown target '{other}' - expected one of: scr, sna, dsk, exec, sprite, tile"
            )));
        }
    }
    args.extend(target_args.unwrap_or_default());

    let task =
        crate::bndbuild::task_from_command("img2cpc", &args).map_err(PyValueError::new_err)?;
    let observer = std::sync::Arc::new(crate::observer::PyObserver::new(on_output));
    let result = py.detach(|| task.execute(&observer));
    let (stdout, stderr) = observer.captured();
    result.map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(
            format!("{e}\n{stderr}{stdout}").trim().to_string()
        )
    })
}

pub fn image(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(convert, m)?)?;
    m.add_function(wrap_pyfunction!(screen_address, m)?)?;
    m.add_function(wrap_pyfunction!(render_screen, m)?)?;
    Ok(())
}
