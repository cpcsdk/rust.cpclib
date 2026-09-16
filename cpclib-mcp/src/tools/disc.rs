//! `disc_list`/`disc_add_file`/`disc_extract_file` - thin wrappers over
//! `cpclib_disc`'s `Disc` trait (`ExtendedDsk`, a `.dsk`/extended-DSK image)
//! and its Amsdos catalog manager.

use base64::Engine;
use cpclib_disc::amsdos::{AmsdosAddBehavior, AmsdosFile, AmsdosFileName, AmsdosManagerMut, AmsdosManagerNonMut};
use cpclib_disc::disc::Disc;
use cpclib_disc::edsk::{ExtendedDsk, Head};
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::{Value, json};

use crate::McpServer;
use crate::error::{ToolError, ToolErrorKind, ToolResult};

fn open_disc(path: &str) -> Result<ExtendedDsk, ToolError> {
    ExtendedDsk::open(path).map_err(|e| ToolError::new(ToolErrorKind::Disc, e))
}

fn parse_filename(name: &str) -> Result<AmsdosFileName, ToolError> {
    AmsdosFileName::try_from(name)
        .map_err(|e| ToolError::new(ToolErrorKind::Amsdos, format!("invalid AMSDOS filename '{name}': {e}")))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DiscPathInput {
    pub disc_path: String
}

/// The disc's Amsdos catalog: every non-erased entry's filename, user,
/// size, and read-only/system flags. Read-only.
pub(crate) fn disc_list(input: DiscPathInput) -> ToolResult {
    let disc = open_disc(&input.disc_path)?;
    let manager = AmsdosManagerNonMut::new_from_disc(&disc, Head::A);
    let catalog = manager
        .catalog()
        .map_err(|e| ToolError::new(ToolErrorKind::Amsdos, e.to_string()))?;
    let entries: Vec<Value> = catalog
        .visible_entries()
        .filter(|e| !e.is_erased())
        .map(|e| {
            json!({
                "filename": e.filename_with_user(),
                "user": e.user(),
                "size": e.used_space(),
                "read_only": e.is_read_only(),
                "system": e.is_system()
            })
        })
        .collect();
    Ok(json!({ "disc_path": input.disc_path, "entries": entries }))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DiscAddFileInput {
    pub disc_path: String,
    /// Local file whose raw bytes are added to the disc as-is (no AMSDOS
    /// header is synthesized).
    pub file_path: String,
    /// AMSDOS filename to store it under (8.3, uppercased). Defaults to
    /// `file_path`'s own basename.
    pub amsdos_filename: Option<String>,
    /// AMSDOS user number (0-15). Default 0.
    pub user: Option<u8>
}

/// **MUTATING**: adds a local file to the disc's Amsdos catalog, replacing
/// any existing entry of the same name, then saves the disc image back to
/// `disc_path`.
pub(crate) fn disc_add_file(input: DiscAddFileInput) -> ToolResult {
    let mut disc = open_disc(&input.disc_path)?;
    let data = fs_err::read(&input.file_path)
        .map_err(|e| ToolError::io(format!("cannot read {}: {e}", input.file_path)))?;

    let default_name = camino::Utf8Path::new(&input.file_path)
        .file_name()
        .map(str::to_owned)
        .unwrap_or_else(|| input.file_path.clone());
    let name = input.amsdos_filename.as_deref().unwrap_or(&default_name);
    let mut filename = parse_filename(name)?;
    filename.set_user(input.user.unwrap_or(0));

    let amsdos_file = AmsdosFile::from_buffer(&data);
    {
        let mut manager = AmsdosManagerMut::new_from_disc(&mut disc, Head::A);
        manager
            .add_file(
                &amsdos_file,
                Some(&filename),
                false,
                false,
                AmsdosAddBehavior::ReplaceIfPresent
            )
            .map_err(|e| ToolError::new(ToolErrorKind::Amsdos, e.to_string()))?;
    }
    disc.save(&input.disc_path)
        .map_err(|e| ToolError::new(ToolErrorKind::Disc, e))?;

    Ok(json!({
        "added": true,
        "disc_path": input.disc_path,
        "filename": filename.filename_with_user(),
        "size": data.len()
    }))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DiscExtractFileInput {
    pub disc_path: String,
    /// AMSDOS filename of the catalog entry to extract (8.3).
    pub entry_name: String
}

/// Extract a file's raw content (AMSDOS header stripped when present) from
/// the disc, base64-encoded. Read-only.
pub(crate) fn disc_extract_file(input: DiscExtractFileInput) -> ToolResult {
    let disc = open_disc(&input.disc_path)?;
    let manager = AmsdosManagerNonMut::new_from_disc(&disc, Head::A);
    let filename = parse_filename(&input.entry_name)?;
    let file = manager
        .get_file(filename)
        .map_err(|e| ToolError::new(ToolErrorKind::Amsdos, e.to_string()))?
        .ok_or_else(|| {
            ToolError::new(
                ToolErrorKind::Amsdos,
                format!("no such catalog entry: {}", input.entry_name)
            )
        })?;
    let content = file.content();
    Ok(json!({
        "entry_name": input.entry_name,
        "size": content.len(),
        "data_base64": base64::engine::general_purpose::STANDARD.encode(content)
    }))
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Value>, Json<Value>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = disc_router, vis = "pub(crate)")]
impl McpServer {
    #[tool(description = "List a disc image's Amsdos catalog. Read-only.")]
    async fn disc_list(
        &self,
        Parameters(input): Parameters<DiscPathInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(disc_list(input))
    }

    #[tool(description = "MUTATING: adds a local file to a disc image's Amsdos catalog, \
                           replacing any existing entry of the same name.")]
    async fn disc_add_file(
        &self,
        Parameters(input): Parameters<DiscAddFileInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(disc_add_file(input))
    }

    #[tool(description = "Extract a file's raw content from a disc image's Amsdos catalog, \
                           base64-encoded. Read-only.")]
    async fn disc_extract_file(
        &self,
        Parameters(input): Parameters<DiscExtractFileInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(disc_extract_file(input))
    }
}
