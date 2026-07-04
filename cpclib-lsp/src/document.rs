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
    /// Unknown file type
    Unknown
}

impl DocumentType {
    pub fn from_uri(uri: &Url) -> Self {
        let path = uri.path();
        if path.ends_with(".asm") || path.ends_with(".s") || path.ends_with(".z80") {
            Self::Assembly
        }
        else if path.ends_with(".build") || path.ends_with(".bnd") {
            Self::BuildFile
        }
        else if path.ends_with(".bas") || path.ends_with(".BAS") {
            Self::Basic
        }
        else {
            Self::Unknown
        }
    }

    pub fn from_language_id(language_id: &str) -> Self {
        match language_id {
            "basm" | "asm" | "z80" => Self::Assembly,
            "bndbuild" => Self::BuildFile,
            "locomotive-basic" => Self::Basic,
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
            // Incremental change
            let start_idx = self.offset_from_position(range.start);
            let end_idx = self.offset_from_position(range.end);

            self.rope.remove(start_idx..end_idx);
            self.rope.insert(start_idx, &change.text);
        }
        else {
            // Full document sync
            self.rope = Rope::from_str(&change.text);
        }
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

    /// Convert LSP Position to rope byte offset
    pub fn offset_from_position(&self, position: Position) -> usize {
        let line_idx = position.line as usize;
        let char_idx = position.character as usize;

        if line_idx >= self.rope.len_lines() {
            return self.rope.len_bytes();
        }

        let line_start = self.rope.line_to_byte(line_idx);
        let line = self.rope.line(line_idx);
        let char_offset = line
            .chars()
            .take(char_idx)
            .map(|c| c.len_utf8())
            .sum::<usize>();

        line_start + char_offset
    }

    /// Convert rope byte offset to LSP Position
    pub fn position_from_offset(&self, offset: usize) -> Position {
        let line = self.rope.byte_to_line(offset);
        let line_start = self.rope.line_to_byte(line);
        let char = self.rope.slice(line_start..offset).chars().count();

        Position {
            line: line as u32,
            character: char as u32
        }
    }
}
