use ropey::Rope;
use tower_lsp::lsp_types::*;

/// Represents the type of document
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentType {
    /// Z80 Assembly file (.asm)
    Assembly,
    /// Build file (.build, .bnd) with Jinja templates
    BuildFile,
    /// Locomotive BASIC source file (.bas)
    Basic,
    /// CatArt source (`.CAT`/`.ASC`): the same Locomotive BASIC syntax as
    /// `Basic`, restricted by convention to a whitelist of drawing commands
    /// (INK/PAPER/PEN/PRINT/CURSOR/MODE/LOCATE/WINDOW/BORDER/CLS/SYMBOL).
    /// Reuses `BasicAnalyzer` for everything except diagnostics (see
    /// `locomotive::catart`).
    CatartBasic,
    /// Unknown file type
    Unknown
}

impl DocumentType {
    pub fn from_uri(uri: &Url) -> Self {
        let path = uri.path();
        if path.ends_with(".asm") || path.ends_with(".s") || path.ends_with(".z80") {
            Self::Assembly
        }
        else if path.ends_with(".build")
            || path.ends_with(".bnd")
            || path.ends_with("/bndbuild.yml")
            || path.ends_with("bndbuild.yml") && !path.contains('/')
        {
            // `bndbuild.yml` is one of bndbuild's standard build-file names.
            Self::BuildFile
        }
        else if path.ends_with(".bas") || path.ends_with(".BAS") {
            Self::Basic
        }
        else if path.ends_with(".CAT")
            || path.ends_with(".cat")
            || path.ends_with(".ASC")
            || path.ends_with(".asc")
        {
            Self::CatartBasic
        }
        else {
            Self::Unknown
        }
    }

    pub fn from_language_id(language_id: &str) -> Self {
        // Includes the ids sent by the Zed extension ("Assembly"/"Buildfile",
        // per cpclib-lsp-zed/extension.toml `language_ids`).
        match language_id {
            "basm" | "asm" | "z80" | "Assembly" => Self::Assembly,
            "bndbuild" | "Buildfile" => Self::BuildFile,
            "locomotive-basic" => Self::Basic,
            "catart-basic" => Self::CatartBasic,
            _ => Self::Unknown
        }
    }

    /// Prefer the language_id classification; fall back to URI extension.
    pub fn detect(uri: &Url, language_id: Option<&str>) -> Self {
        if let Some(lid) = language_id {
            let from_lid = Self::from_language_id(lid);
            if from_lid != Self::Unknown {
                return from_lid;
            }
        }
        Self::from_uri(uri)
    }
}

/// Shared core of `Document::byte_column`/`utf16_col_to_byte_offset`: walk
/// `chars` accumulating UTF-16 code units until `utf16_col` is reached,
/// returning the byte offset at that point. Generic over the char source so
/// it works against both a rope line slice (no allocation) and a plain
/// `&str` line.
fn utf16_units_to_byte_offset(chars: impl Iterator<Item = char>, utf16_col: usize) -> usize {
    let mut utf16_units = 0usize;
    let mut byte_offset = 0usize;
    for c in chars {
        if utf16_units >= utf16_col {
            break;
        }
        utf16_units += c.len_utf16();
        byte_offset += c.len_utf8();
    }
    byte_offset
}

/// Convert an LSP `Position`'s `character` (UTF-16 code units) to a byte
/// offset within `line` — for callers that only have a raw `&str` line (no
/// `Document`/rope at hand), e.g. a line already extracted from plain text.
/// See `Document::byte_column` for the rope-backed equivalent.
pub fn utf16_col_to_byte_offset(line: &str, utf16_col: usize) -> usize {
    utf16_units_to_byte_offset(line.chars(), utf16_col)
}

/// The inverse of `utf16_col_to_byte_offset`: convert a byte offset within
/// `line` to a UTF-16 code-unit column — for callers that computed a byte
/// offset via manual `&str`/`&[u8]` scanning and need to hand an LSP
/// `Position` back to the client.
pub fn byte_offset_to_utf16_col(line: &str, byte_offset: usize) -> usize {
    line[..byte_offset.min(line.len())]
        .chars()
        .map(|c| c.len_utf16())
        .sum()
}

/// Shared core of `Document::char_column`: walk `chars` accumulating UTF-16
/// code units until `utf16_col` is reached, returning the `char` count at
/// that point. Generic over the char source so it works against both a rope
/// line slice (no allocation) and a plain `&str` line.
fn utf16_units_to_char_count(chars: impl Iterator<Item = char>, utf16_col: usize) -> usize {
    let mut utf16_units = 0usize;
    let mut char_count = 0usize;
    for c in chars {
        if utf16_units >= utf16_col {
            break;
        }
        utf16_units += c.len_utf16();
        char_count += 1;
    }
    char_count
}

/// The inverse of `Document::char_column`: convert a `char` count within
/// `line` to a UTF-16 code-unit column — for callers that computed a
/// `char`-indexed position (e.g. `token::word_range_at_position`'s span)
/// and need to hand an LSP `Position`/`Range` back to the client.
pub fn char_count_to_utf16_col(line: &str, char_count: usize) -> usize {
    line.chars().take(char_count).map(|c| c.len_utf16()).sum()
}

/// Represents a document managed by the LSP
#[derive(Debug, Clone)]
pub struct Document {
    pub uri: Url,
    pub doc_type: DocumentType,
    pub rope: Rope,
    pub version: i32
}

impl Document {
    pub fn new(uri: Url, text: String, version: i32) -> Self {
        Self::new_with_language(uri, text, version, None)
    }

    pub fn new_with_language(
        uri: Url,
        text: String,
        version: i32,
        language_id: Option<&str>
    ) -> Self {
        let doc_type = DocumentType::detect(&uri, language_id);
        let rope = Rope::from_str(&text);
        Self {
            uri,
            doc_type,
            rope,
            version
        }
    }

    pub fn apply_change(&mut self, change: &TextDocumentContentChangeEvent, version: i32) {
        self.version = version;

        if let Some(range) = change.range {
            // Incremental change. `offset_from_position` returns a byte
            // offset (see its own doc comment), but `Rope::insert`/`remove`
            // are char-indexed - passing bytes straight through is silently
            // correct for ASCII (byte index == char index) and splices at
            // the wrong position anywhere a multi-byte UTF-8 character
            // (accented letters, smart quotes, ...) appears earlier in the
            // document, which is what made edits land at the wrong offset.
            let start_idx = self
                .rope
                .byte_to_char(self.offset_from_position(range.start));
            let end_idx = self.rope.byte_to_char(self.offset_from_position(range.end));

            self.rope.remove(start_idx..end_idx);
            self.rope.insert(start_idx, &change.text);
        }
        else {
            // Full document sync
            self.rope = Rope::from_str(&change.text);
        }

        // Kept for future reports of the same symptom (edits landing at the
        // wrong offset): the root cause here was `offset_from_position`
        // returning bytes into a char-indexed `Rope::insert`/`remove`, now
        // fixed above, but a transcript of every change beats re-deriving
        // one from a bug report. Enable with the top-level `log` setting in
        // `cpclib-lsp.toml` (see `LspConfig::log`) or `RUST_LOG=debug`.
        tracing::debug!(
            uri = %self.uri,
            version,
            range = ?change.range,
            text = %change.text,
            rope_after = %self.rope,
            "apply_change"
        );
    }

    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn line(&self, line_idx: usize) -> Option<String> {
        if line_idx >= self.rope.len_lines() {
            return None;
        }

        let line = self.rope.line(line_idx);
        Some(line.to_string())
    }

    /// Convert an LSP `Position` to a rope byte offset. Per the LSP spec,
    /// `position.character` counts **UTF-16 code units**, not Rust `char`s
    /// or bytes — a character outside the Basic Multilingual Plane (e.g. an
    /// emoji) is 1 `char` but 2 UTF-16 units, so counting `char`s directly
    /// would desync every column past it on the line.
    pub fn offset_from_position(&self, position: Position) -> usize {
        let line_idx = position.line as usize;

        if line_idx >= self.rope.len_lines() {
            return self.rope.len_bytes();
        }

        let line_start = self.rope.line_to_byte(line_idx);
        line_start + self.byte_column(position)
    }

    /// Convert an LSP `Position`'s `character` (UTF-16 code units) to a byte
    /// offset within its own line, i.e. the byte-offset analogue of
    /// `position.character` — for hand-rolled per-line byte scanners
    /// elsewhere in this crate that index into a `&str`/`&[u8]` line rather
    /// than going through the whole rope. Returns 0 for an out-of-range
    /// line.
    pub fn byte_column(&self, position: Position) -> usize {
        let line_idx = position.line as usize;

        if line_idx >= self.rope.len_lines() {
            return 0;
        }

        utf16_units_to_byte_offset(
            self.rope.line(line_idx).chars(),
            position.character as usize
        )
    }

    /// Convert an LSP `Position`'s `character` (UTF-16 code units) to a
    /// Rust `char` count within its own line — for basm's word-boundary
    /// scanners, which index by `char` rather than byte. Returns 0 for an
    /// out-of-range line.
    pub fn char_column(&self, position: Position) -> usize {
        let line_idx = position.line as usize;

        if line_idx >= self.rope.len_lines() {
            return 0;
        }

        utf16_units_to_char_count(
            self.rope.line(line_idx).chars(),
            position.character as usize
        )
    }

    /// Convert a rope byte offset to an LSP `Position` (a UTF-16 code-unit
    /// column - see `offset_from_position`). Only exercised by this module's
    /// own round-trip test today (nothing yet needs to go from a byte
    /// offset back to a `Position`), but it's `offset_from_position`'s exact
    /// inverse and worth keeping test-verified for whichever caller reaches
    /// for it next.
    #[allow(dead_code)]
    pub fn position_from_offset(&self, offset: usize) -> Position {
        let line = self.rope.byte_to_line(offset);
        let line_start = self.rope.line_to_byte(line);
        // `Rope::slice` takes a *char* range, not bytes - `byte_slice` is
        // the byte-indexed equivalent; passing byte offsets to `slice`
        // panics (or silently misbehaves) as soon as the line contains any
        // multi-byte character before `offset`.
        let utf16_col: usize = self
            .rope
            .byte_slice(line_start..offset)
            .chars()
            .map(|c| c.len_utf16())
            .sum();

        Position {
            line: line as u32,
            character: utf16_col as u32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///test.asm").unwrap(), text.to_string(), 1)
    }

    #[test]
    fn offset_from_position_counts_utf16_units_not_chars() {
        // "😀" is one `char` but two UTF-16 code units, so the LSP position
        // just after it must be character=2, not character=1.
        let d = doc("😀X");
        let offset = d.offset_from_position(Position {
            line: 0,
            character: 2
        });
        assert_eq!(&d.text()[offset..], "X");
    }

    #[test]
    fn position_from_offset_is_the_inverse_for_supplementary_plane_chars() {
        let d = doc("😀X");
        let byte_offset = "😀".len();
        let pos = d.position_from_offset(byte_offset);
        assert_eq!(
            pos,
            Position {
                line: 0,
                character: 2
            }
        );
    }

    #[test]
    fn offset_from_position_handles_ascii_as_before() {
        let d = doc("hello");
        let offset = d.offset_from_position(Position {
            line: 0,
            character: 3
        });
        assert_eq!(offset, 3);
    }

    #[test]
    fn byte_column_converts_utf16_units_to_a_line_relative_byte_offset() {
        let d = doc("café ABC");
        // 'c','a','f','é' = 4 UTF-16 units, landing right before the space.
        assert_eq!(
            d.byte_column(Position {
                line: 0,
                character: 4
            }),
            "café".len()
        );
    }

    #[test]
    fn offset_from_position_two_byte_utf8_char_counts_as_one_utf16_unit() {
        // 'é' is 2 bytes in UTF-8 but a single UTF-16 code unit (BMP).
        let d = doc("café");
        let offset = d.offset_from_position(Position {
            line: 0,
            character: 4
        });
        assert_eq!(offset, d.text().len());
    }

    #[test]
    fn char_column_counts_chars_not_utf16_units_for_a_supplementary_plane_char() {
        // "😀" is one `char` but two UTF-16 code units - the char count
        // right after it must be 1, not 2.
        let d = doc("😀X");
        assert_eq!(
            d.char_column(Position {
                line: 0,
                character: 2
            }),
            1
        );
    }

    #[test]
    fn apply_change_inserts_correctly_past_a_multi_byte_char_on_an_earlier_line() {
        // Regression test: `offset_from_position` returns a byte offset, but
        // `Rope::insert`/`remove` are char-indexed. A multi-byte UTF-8 char
        // (e.g. this French comment's accented "é") on an earlier line used
        // to push every later byte offset ahead of its true char index,
        // splicing edits into the wrong place.
        let mut d = doc("; commentaire en français\nline two\n");
        d.apply_change(
            &TextDocumentContentChangeEvent {
                range: Some(Range {
                    start: Position {
                        line: 1,
                        character: 0
                    },
                    end: Position {
                        line: 1,
                        character: 0
                    }
                }),
                range_length: None,
                text: "ld sp, 0\n".to_string()
            },
            2
        );
        assert_eq!(
            d.text(),
            "; commentaire en français\nld sp, 0\nline two\n"
        );
    }
}

#[cfg(test)]
mod document_type_tests {
    use super::*;

    #[test]
    fn cat_and_asc_extensions_detect_as_catart_basic() {
        for ext in [".CAT", ".cat", ".ASC", ".asc"] {
            let uri = Url::parse(&format!("file:///t{ext}")).unwrap();
            assert_eq!(
                DocumentType::from_uri(&uri),
                DocumentType::CatartBasic,
                "extension {ext}"
            );
        }
    }

    #[test]
    fn catart_basic_language_id_detects_as_catart_basic() {
        assert_eq!(
            DocumentType::from_language_id("catart-basic"),
            DocumentType::CatartBasic
        );
    }

    #[test]
    fn detect_prefers_language_id_over_extension_for_catart() {
        let uri = Url::parse("file:///t.CAT").unwrap();
        assert_eq!(
            DocumentType::detect(&uri, Some("catart-basic")),
            DocumentType::CatartBasic
        );
        assert_eq!(DocumentType::detect(&uri, None), DocumentType::CatartBasic);
    }
}
