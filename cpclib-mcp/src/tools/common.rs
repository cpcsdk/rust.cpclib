//! Shared helpers used by more than one tool module.

use std::hash::{Hash, Hasher};

use cpclib_lsp::common::document::Document;
use tower_lsp::lsp_types::Url;

use crate::error::ToolError;

/// A real `file://` URI for `path`, canonicalized when possible so it
/// matches whatever `AssemblyAnalyzer`'s project-graph cache and `INCLUDE`
/// resolution compare against on disk.
pub(crate) fn uri_for_path(path: &str) -> Result<Url, ToolError> {
    let path_buf = std::path::Path::new(path);
    let absolute = std::fs::canonicalize(path_buf).unwrap_or_else(|_| {
        // Not on disk (yet), or a relative path that doesn't resolve from
        // the current directory - fall back to joining with cwd so we
        // still produce *some* absolute `file://` URI rather than failing
        // outright; a document that never really existed on disk simply
        // won't match any real project graph, matching this analyzer's own
        // "falls back toward Unknown" philosophy.
        std::env::current_dir()
            .map(|cwd| cwd.join(path_buf))
            .unwrap_or_else(|_| path_buf.to_path_buf())
    });
    Url::from_file_path(&absolute)
        .map_err(|_| ToolError::invalid_input(format!("not a usable file path: {path}")))
}

/// Deterministic-enough `i32` "version" derived from `text`'s content, used
/// as `Document::new`'s version field. `AssemblyAnalyzer`'s internal caches
/// are keyed by `(Url, version)`: reusing a real edit-counter isn't
/// possible from a one-shot MCP tool call (there is no live editing
/// session), but a content hash gives the same property that matters here -
/// identical content across two calls hits the cache, different content
/// misses it. Collisions are not a correctness concern for this use case
/// (see the crate-level task notes this was specified against).
fn version_from_content(text: &str) -> i32 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    (hasher.finish() & 0x7fff_ffff) as i32
}

/// Build a [`Document`] from an MCP tool's `path`/`code` input pair:
/// - `path` only: read the file, identity = the file's own `file://` URI.
/// - `code` only: identity = a synthetic `file:///untitled.asm` URI (no
///   `INCLUDE`/project-graph resolution is possible without a real path).
/// - both: check `code`'s text, but under `path`'s real identity - lets a
///   caller check unsaved edits while still resolving `INCLUDE`s/the
///   project graph relative to where the file really lives.
/// - neither: an error - callers must provide at least one.
pub(crate) fn document_from_input(
    path: Option<&str>,
    code: Option<&str>
) -> Result<Document, ToolError> {
    let (text, uri) = match (path, code) {
        (Some(p), Some(c)) => (c.to_string(), uri_for_path(p)?),
        (Some(p), None) => {
            let text = fs_err::read_to_string(p)
                .map_err(|e| ToolError::io(format!("cannot read {p}: {e}")))?;
            (text, uri_for_path(p)?)
        },
        (None, Some(c)) => {
            let uri = Url::parse("file:///untitled.asm").expect("static URI is valid");
            (c.to_string(), uri)
        },
        (None, None) => {
            return Err(ToolError::invalid_input(
                "either `path` or `code` must be provided"
            ));
        }
    };
    let version = version_from_content(&text);
    Ok(Document::new(uri, text, version))
}
