//! Resolution and content access for files referenced by `INCLUDE`/`INCBIN`/
//! `BINCLUDE` directives — both real files on disk and basm's embedded
//! `inner://...` resources (crunchers, firmware routines, ...), the same set
//! `basm --list-embedded`/`--view-embedded` expose from the CLI.

use cpclib_asm::assembler::embedded::EmbeddedFiles;
use tower_lsp::lsp_types::Url;

/// `true` when `filename` refers to one of basm's embedded resources rather
/// than a real file on disk.
pub(super) fn is_inner_uri(filename: &str) -> bool {
    filename.starts_with("inner://")
}

/// Every embedded `inner://...` key basm knows about (crunchers, firmware
/// routines, ...) — the same list `basm --list-embedded` prints. `pub(crate)`
/// so `common::firmware_docs` (shared by both asm and BASIC hover) can reuse
/// it too, not just this module's own `basm/` siblings.
pub(crate) fn inner_file_names() -> impl Iterator<Item = String> {
    EmbeddedFiles::iter().map(|s| s.into_owned())
}

/// The UTF-8 text content of an embedded `inner://...` resource, or `None`
/// if `filename` isn't a known embedded key or isn't valid UTF-8. `pub(crate)`
/// - see [`inner_file_names`].
pub(crate) fn read_inner_file(filename: &str) -> Option<String> {
    let file = EmbeddedFiles::get(filename)?;
    std::str::from_utf8(file.data.as_ref())
        .ok()
        .map(str::to_string)
}

/// As [`read_inner_file`], but the raw bytes — for `INCBIN` targets, which
/// are binary data and not necessarily valid UTF-8.
pub(super) fn read_inner_file_bytes(filename: &str) -> Option<Vec<u8>> {
    let file = EmbeddedFiles::get(filename)?;
    Some(file.data.as_ref().to_vec())
}

/// Read the content of a file referenced by an `INCLUDE`/`INCBIN`/`BINCLUDE`
/// directive in the document at `doc_uri`, whether it's an embedded
/// `inner://...` resource or a real file on disk — resolved the same way
/// `super::definition::resolve_include_path` resolves it for goto-definition.
pub(super) fn read_included_file(filename: &str, doc_uri: &Url) -> Option<String> {
    if is_inner_uri(filename) {
        return read_inner_file(filename);
    }
    let path = super::definition::resolve_include_path(filename, doc_uri)?;
    std::fs::read_to_string(path).ok()
}

/// As [`read_included_file`], but the raw bytes — for `INCBIN` targets.
pub(super) fn read_included_file_bytes(filename: &str, doc_uri: &Url) -> Option<Vec<u8>> {
    if is_inner_uri(filename) {
        return read_inner_file_bytes(filename);
    }
    let path = super::definition::resolve_include_path(filename, doc_uri)?;
    std::fs::read(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inner_file_names_is_non_empty_and_prefixed() {
        let names: Vec<String> = inner_file_names().collect();
        assert!(!names.is_empty(), "expected embedded assets to be listed");
        assert!(names.iter().all(|n| n.starts_with("inner://")), "{names:?}");
    }

    #[test]
    fn reads_a_known_embedded_file() {
        let names: Vec<String> = inner_file_names().collect();
        let some_name = names.first().expect("at least one embedded file");
        let content = read_inner_file(some_name);
        assert!(content.is_some(), "should read {some_name}");
        assert!(!content.unwrap().is_empty());
    }

    #[test]
    fn unknown_inner_file_yields_none() {
        assert!(read_inner_file("inner://this/does/not/exist.asm").is_none());
    }
}
