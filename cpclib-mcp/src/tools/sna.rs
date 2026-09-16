//! `sna_inspect`/`sna_patch_memory`/`sna_create` - thin wrappers over
//! `cpclib_sna::Snapshot`.

use base64::Engine;
use cpclib_sna::flags::{FlagValue, SnapshotFlag};
use cpclib_sna::{Snapshot, SnapshotVersion};
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::{Map, Value, json};

use crate::McpServer;
use crate::error::{ToolError, ToolErrorKind, ToolResult};

fn flag_value_to_json(v: &FlagValue) -> Value {
    match v {
        FlagValue::Byte(b) => json!(b),
        FlagValue::Word(w) => json!(w),
        FlagValue::Array(items) => Value::Array(items.iter().map(flag_value_to_json).collect())
    }
}

const KEY_REGISTERS: &[(&str, SnapshotFlag)] = &[
    ("af", SnapshotFlag::Z80_AF),
    ("bc", SnapshotFlag::Z80_BC),
    ("de", SnapshotFlag::Z80_DE),
    ("hl", SnapshotFlag::Z80_HL),
    ("sp", SnapshotFlag::Z80_SP),
    ("pc", SnapshotFlag::Z80_PC),
    ("ix", SnapshotFlag::Z80_IX),
    ("iy", SnapshotFlag::Z80_IY),
    ("i", SnapshotFlag::Z80_I),
    ("r", SnapshotFlag::Z80_R)
];

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SnaPathInput {
    pub path: String
}

/// Version, memory size, and the key Z80 registers of a `.sna` snapshot.
/// Read-only.
pub(crate) fn sna_inspect(input: SnaPathInput) -> ToolResult {
    let snapshot = Snapshot::load(&input.path).map_err(|e| ToolError::new(ToolErrorKind::Snapshot, e))?;
    let mut registers = Map::new();
    for (name, flag) in KEY_REGISTERS {
        registers.insert(
            (*name).to_string(),
            flag_value_to_json(&snapshot.get_value(flag))
        );
    }
    Ok(json!({
        "path": input.path,
        "version": format!("{:?}", snapshot.version()),
        "memory_size_kb": snapshot.memory_size_header(),
        "registers": Value::Object(registers)
    }))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SnaPatchMemoryInput {
    pub path: String,
    /// Real address (0-0xFFFF) of the first patched byte.
    pub address: u32,
    /// Base64-encoded bytes to write starting at `address`.
    pub data_base64: String,
    /// Where to save the patched snapshot - never `path` itself. Required.
    pub out_path: String
}

/// **MUTATING** (of `out_path`, never `path`): loads a snapshot, patches
/// memory starting at `address`, and saves the result to `out_path`.
pub(crate) fn sna_patch_memory(input: SnaPatchMemoryInput) -> ToolResult {
    let mut snapshot = Snapshot::load(&input.path).map_err(|e| ToolError::new(ToolErrorKind::Snapshot, e))?;
    let data = base64::engine::general_purpose::STANDARD
        .decode(&input.data_base64)
        .map_err(|e| ToolError::invalid_input(format!("data_base64 is not valid base64: {e}")))?;
    for (i, byte) in data.iter().enumerate() {
        snapshot.set_byte(input.address + i as u32, *byte);
    }
    snapshot
        .save(&input.out_path, SnapshotVersion::V2)
        .map_err(|e| ToolError::io(format!("cannot write {}: {e}", input.out_path)))?;
    Ok(json!({
        "out_path": input.out_path,
        "address": input.address,
        "count": data.len()
    }))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SnaFileToLoad {
    /// Local file whose bytes are embedded into the snapshot's memory.
    pub path: String,
    /// Address to load it at.
    pub address: usize
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SnaCreateInput {
    /// Where to save the new snapshot.
    pub out_path: String,
    /// Files to load into memory (a fresh 6128 snapshot is the base).
    pub files: Vec<SnaFileToLoad>
}

/// **MUTATING** (of `out_path`): creates a fresh 6128 snapshot with the
/// given files loaded into memory, and saves it to `out_path`.
pub(crate) fn sna_create(input: SnaCreateInput) -> ToolResult {
    let mut snapshot = Snapshot::new_6128().map_err(|e| ToolError::new(ToolErrorKind::Snapshot, e))?;
    for file in &input.files {
        let data = fs_err::read(&file.path)
            .map_err(|e| ToolError::io(format!("cannot read {}: {e}", file.path)))?;
        // Not `Snapshot::add_file`/`add_data`: those index straight into
        // `self.memory` without first unwrapping it out of a memory chunk
        // (their own "TODO: re-implement with set_byte" comment admits as
        // much), which panics on a snapshot loaded from a real embedded
        // `.sna` like `new_6128` - real memory there starts out chunked.
        // `set_byte` does the unwrap-and-resize itself, one byte at a time.
        for (offset, byte) in data.iter().enumerate() {
            snapshot.set_byte((file.address + offset) as u32, *byte);
        }
    }
    snapshot
        .save(&input.out_path, SnapshotVersion::V2)
        .map_err(|e| ToolError::io(format!("cannot write {}: {e}", input.out_path)))?;
    Ok(json!({ "out_path": input.out_path, "files_added": input.files.len() }))
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Value>, Json<Value>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = sna_router, vis = "pub(crate)")]
impl McpServer {
    #[tool(description = "Inspect a .sna snapshot's version, memory size, and key Z80 \
                           registers. Read-only.")]
    async fn sna_inspect(
        &self,
        Parameters(input): Parameters<SnaPathInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(sna_inspect(input))
    }

    #[tool(description = "MUTATING (of out_path, never the input path): patches a snapshot's \
                           memory and saves the result to out_path.")]
    async fn sna_patch_memory(
        &self,
        Parameters(input): Parameters<SnaPatchMemoryInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(sna_patch_memory(input))
    }

    #[tool(description = "MUTATING (of out_path): creates a fresh 6128 snapshot with the given \
                           files loaded into memory.")]
    async fn sna_create(
        &self,
        Parameters(input): Parameters<SnaCreateInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(sna_create(input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sna_create_then_sna_inspect_round_trips() {
        let dir = camino_tempfile::tempdir().expect("tempdir");
        let data_path = dir.path().join("payload.bin");
        fs_err::write(&data_path, [0xAAu8, 0xBB, 0xCC, 0xDD]).expect("write payload");
        let out_path = dir.path().join("out.sna");

        let create_result = sna_create(SnaCreateInput {
            out_path: out_path.to_string(),
            files: vec![SnaFileToLoad {
                path: data_path.to_string(),
                address: 0x4000
            }]
        })
        .expect("sna_create should succeed");
        assert_eq!(create_result["files_added"], 1, "{create_result:#}");

        let inspect_result = sna_inspect(SnaPathInput {
            path: out_path.to_string()
        })
        .expect("sna_inspect should succeed on the file we just created");
        assert!(
            inspect_result["registers"]["pc"].is_number(),
            "{inspect_result:#}"
        );
    }

    #[test]
    fn sna_inspect_reports_a_missing_file_as_a_snapshot_error() {
        let err = sna_inspect(SnaPathInput {
            path: "/nonexistent/path/does-not-exist.sna".to_string()
        })
        .expect_err("a missing file should be an error");
        assert_eq!(err.kind, "snapshot");
    }
}
