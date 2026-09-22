//! `size_map` - the MCP-facing wrapper around `cpclib_crunch::sizemap`
//! (where do the bytes go?), which holds the real logic - assemble,
//! attribute the source map, measure crunched costs. This file only parses
//! MCP input and serializes the resulting report to JSON.

use camino::Utf8Path;
use cpclib_crunch::sizemap::SizeMapOptions;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::Value;

use crate::McpServer;
use crate::error::{ToolError, ToolErrorKind, ToolResult};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SizeMapInput {
    /// The `.asm` file to assemble (as a build would, so give the build's
    /// `include_dirs` / `defines` - see `project_context`).
    pub path: String,
    /// Extra `INCLUDE` search directories.
    pub include_dirs: Option<Vec<String>>,
    /// Symbols to define first, `NAME` or `NAME=VALUE`, like `basm -D`.
    pub defines: Option<Vec<String>>,
    /// Only regions at or above this address (default: where the produced
    /// bytes start).
    pub min_address: Option<u32>,
    /// Only regions below this address (default: end of the produced
    /// bytes).
    pub max_address: Option<u32>,
    /// Also measure each region's crunched cost with this cruncher (any
    /// `compare_crunchers` format name, e.g. `shrinkler`).
    pub cruncher: Option<String>,
    /// How many of the biggest regions to list (default 25).
    pub top: Option<usize>
}

pub(crate) fn size_map(input: SizeMapInput) -> ToolResult {
    let options = SizeMapOptions {
        include_dirs: input.include_dirs.unwrap_or_default(),
        defines: input.defines.unwrap_or_default(),
        min_address: input.min_address,
        max_address: input.max_address,
        cruncher: input.cruncher,
        top: input.top
    };
    let report = cpclib_crunch::sizemap::size_map(Utf8Path::new(&input.path), &options).map_err(|e| {
        let kind = if e.starts_with("unknown cruncher") || e.contains("address range is empty") {
            ToolErrorKind::InvalidInput
        }
        else {
            ToolErrorKind::Assembler
        };
        ToolError::new(kind, e)
    })?;
    Ok(serde_json::to_value(report).expect("SizeMapReport always serializes"))
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Value>, Json<Value>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = sizemap_router, vis = "pub(crate)")]
impl McpServer {
    #[tool(description = "Read-only: where do the bytes go? Assembles a source (like a build \
                           would - pass include_dirs/defines from project_context) and returns \
                           a markdown `table` of the biggest global labels: address, bytes to \
                           the next global label, share of the image, number of local labels. \
                           With `cruncher` (e.g. shrinkler) it also measures each region's \
                           *crunched* cost - the whole image crunched, minus the image without \
                           that region - which can rank routines very differently from raw size \
                           and is what a size-limited intro needs. Labels come from the \
                           assembler's own table (real addresses only, never `equ` constants), \
                           not a .sym file. From the assembler's source map (the structured \
                           listing) it adds a code/data split per region and three more tables: \
                           bytes by source file, by macro (what a macro's call sites cost \
                           together) and by individual source line, each with the number of \
                           places it was instantiated. Every crunched section in the source \
                           (LZ48/LZSHRINKLER/...) also gets its own breakdown of what went into \
                           it, region by region, since the image only holds its compressed \
                           output. Spans include any data placed between two labels; `SAVE` \
                           directives are not executed.")]
    async fn size_map(&self, Parameters(input): Parameters<SizeMapInput>) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(size_map(input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(path: &str) -> SizeMapInput {
        SizeMapInput {
            path: path.to_string(),
            include_dirs: None,
            defines: None,
            min_address: None,
            max_address: None,
            cruncher: None,
            top: None
        }
    }

    /// Just enough coverage to pin the MCP-facing shape (error kinds, JSON
    /// serialization) - `cpclib-crunch`'s own tests cover the actual logic.
    #[test]
    fn size_map_reports_an_invalid_cruncher_as_invalid_input() {
        let dir = camino_tempfile::tempdir().unwrap();
        let path = dir.path().join("m.asm");
        fs_err::write(&path, "org 0x4000\nret\n").unwrap();
        let err = size_map(SizeMapInput { cruncher: Some("not_a_real_cruncher".to_string()), ..base(path.as_str()) }).unwrap_err();
        assert_eq!(err.kind, "invalid_input");
    }

    #[test]
    fn size_map_reports_a_missing_file_as_an_assembler_error() {
        let err = size_map(base("/no/such/file.asm")).unwrap_err();
        assert_eq!(err.kind, "assembler");
    }

    #[test]
    fn a_real_source_serializes_to_the_expected_json_shape() {
        let dir = camino_tempfile::tempdir().unwrap();
        let path = dir.path().join("m.asm");
        fs_err::write(&path, "org 0x4000\nstart:\n ld hl,0x1234\n ret\ntable:\n db 1,2,3,4\n").unwrap();
        let out = size_map(base(path.as_str())).expect("size_map should succeed");
        let regions = out["regions"].as_array().unwrap();
        assert!(regions.iter().any(|r| r["label"] == "table"));
        assert!(out["table"].as_str().unwrap().contains("table"));
        assert!(out["listing"]["files_table"].as_str().unwrap().contains("m.asm"));
        assert_eq!(out["crunched_sections"].as_array().unwrap().len(), 0);
    }
}
