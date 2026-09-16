//! `render_screen` - turns raw CPC screen memory into a PNG, via
//! `cpclib_dap::inspect::render_screen_view` (already a complete, tested
//! pure function - this module's whole job is assembling a full 64K memory
//! buffer and a `Palette<Ink>` for it to call into, then passing its ready-
//! made JSON straight through).

use cpclib_dap::inspect::{
    DEFAULT_SCREEN_HEIGHT, DEFAULT_SCREEN_WIDTH, ScreenEncoding, crtc_screen_start_address,
    render_screen_view
};
use cpclib_image::ink::Ink;
use cpclib_image::palette::Palette;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::Value;

use crate::McpServer;
use crate::error::{ToolError, ToolErrorKind, ToolResult};

/// Real address space size: `render_screen_view`'s `memory` argument is
/// always `memory[0]` = real address 0x0000, the full 64K space, regardless
/// of encoding.
const FULL_MEMORY_SIZE: usize = 0x10000;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RenderScreenInput {
    /// Path to the memory to render: a raw dump, a `.scr`, or a `.sna`
    /// (detected by `.sna` extension; anything else is treated as a raw
    /// linear memory image, zero-padded/truncated to 64K).
    pub path: String,
    /// Screen start address (0-0xFFFF). Give this or `crtc_r12`+`crtc_r13`,
    /// not both.
    pub address: Option<usize>,
    /// CRTC register 12, to derive the screen start address instead of
    /// giving `address` directly. Requires `crtc_r13` too.
    pub crtc_r12: Option<u8>,
    /// CRTC register 13 - see `crtc_r12`.
    pub crtc_r13: Option<u8>,
    /// CPC screen mode (0-3). Default 1.
    pub mode: Option<u8>,
    /// Width in bytes. Default 80.
    pub width: Option<usize>,
    /// Height in pixel lines. Default 200.
    pub height: Option<usize>,
    /// Lines per character row - the CRTC's `R9 + 1`. Default 8.
    pub lines_per_char_row: Option<usize>,
    /// Firmware ink number (0-31) for each of the 16 pens, base palette.
    /// Defaults to the firmware's own default palette when omitted.
    pub firmware_palette: Option<Vec<u8>>,
    /// Per-pen override on top of `firmware_palette`/the default palette -
    /// `null` entries mean "no override for this pen". Indexed by pen
    /// number starting at 0.
    pub palette_override: Option<Vec<Option<u8>>>,
    /// "screen" (default; CRTC-accurate interleaved addressing confined to
    /// the screen's own 16K bank) or "cpc" (raw sequential bytes, wrapped
    /// at the full 64K space).
    pub encoding: Option<String>
}

fn full_memory_from_path(path: &str) -> Result<Vec<u8>, ToolError> {
    let lower = path.to_ascii_lowercase();
    let mut memory = if lower.ends_with(".sna") {
        let snapshot = cpclib_sna::Snapshot::load(path)
            .map_err(|e| ToolError::new(ToolErrorKind::Snapshot, e))?;
        snapshot
            .memory_dump()
            .map_err(|e| ToolError::new(ToolErrorKind::Snapshot, e))?
    }
    else {
        fs_err::read(path).map_err(|e| ToolError::io(format!("cannot read {path}: {e}")))?
    };
    memory.resize(FULL_MEMORY_SIZE, 0);
    memory.truncate(FULL_MEMORY_SIZE);
    Ok(memory)
}

fn parse_encoding(encoding: Option<&str>) -> Result<ScreenEncoding, ToolError> {
    match encoding.map(str::to_ascii_lowercase).as_deref() {
        None | Some("screen") => Ok(ScreenEncoding::Screen),
        Some("cpc") => Ok(ScreenEncoding::Cpc),
        Some(other) => {
            Err(ToolError::invalid_input(format!(
                "unknown encoding '{other}' - expected 'screen' or 'cpc'"
            )))
        }
    }
}

/// Render raw CPC screen memory (a dump, `.scr`, or `.sna`) to a PNG.
/// Read-only.
pub(crate) fn render_screen(input: RenderScreenInput) -> ToolResult {
    let address = match (input.address, input.crtc_r12, input.crtc_r13) {
        (Some(a), _, _) => a,
        (None, Some(r12), Some(r13)) => crtc_screen_start_address(r12, r13),
        _ => {
            return Err(ToolError::invalid_input(
                "either `address` or both `crtc_r12`/`crtc_r13` must be given"
            ));
        }
    };
    let memory = full_memory_from_path(&input.path)?;
    let encoding = parse_encoding(input.encoding.as_deref())?;

    let mut palette = Palette::<Ink>::default();
    if let Some(firmware) = &input.firmware_palette {
        for (pen, &ink_num) in firmware.iter().enumerate() {
            if ink_num > 31 {
                return Err(ToolError::invalid_input(format!(
                    "firmware_palette[{pen}] = {ink_num} is not a valid firmware ink number \
                     (0-31)"
                )));
            }
            palette.set(pen as u8, Ink::from(ink_num));
        }
    }

    let palette_override: Vec<Option<Ink>> = input
        .palette_override
        .unwrap_or_default()
        .into_iter()
        .map(|maybe_ink| maybe_ink.map(Ink::from))
        .collect();

    let value = render_screen_view(
        address,
        input.width.unwrap_or(DEFAULT_SCREEN_WIDTH),
        input.height.unwrap_or(DEFAULT_SCREEN_HEIGHT),
        input.mode.unwrap_or(1),
        &palette,
        &memory,
        input.lines_per_char_row.unwrap_or(8),
        &palette_override,
        encoding
    )
    .map_err(|e| ToolError::new(ToolErrorKind::Io, e))?;

    Ok(value)
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Value>, Json<Value>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = render_router, vis = "pub(crate)")]
impl McpServer {
    #[tool(description = "Render raw CPC screen memory (a dump, .scr, or .sna) to a PNG. \
                           Read-only.")]
    async fn render_screen(
        &self,
        Parameters(input): Parameters<RenderScreenInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(render_screen(input))
    }
}
