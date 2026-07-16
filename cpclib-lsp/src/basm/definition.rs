//! Goto-definition and references for assembly files: labels/symbols,
//! include-file navigation, embedded-BASIC line targets.

use cpclib_asm::parser::obtained::MayHaveSpan;
use cpclib_tokens::ListingElement;
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use super::embedded_basic::extract_locomotive_blocks;
use super::token::is_ident_byte;
use crate::common::document::Document;

impl AssemblyAnalyzer {
    /// Find the definition of a symbol — looks up the word under the cursor in the parsed listing.
    pub fn goto_definition(&self, document: &Document, position: Position) -> Option<Location> {
        let line = document.line(position.line as usize)?;
        let col = position.character as usize;

        // CTRL+CLICK on a filename string inside INCLUDE / INCBIN / BINCLUDE.
        if let Some(target_uri) = resolve_include_at(&line, col, &document.uri) {
            return Some(Location {
                uri: target_uri,
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0
                    },
                    end: Position {
                        line: 0,
                        character: 0
                    }
                }
            });
        }

        // Delegate to BASIC goto-definition for LOCOMOTIVE block content.
        {
            let text = document.text();
            let loco_blocks = extract_locomotive_blocks(&text);
            let line_idx = position.line as usize;
            if let Some(block) = loco_blocks
                .iter()
                .find(|b| b.basic_range.contains(&line_idx))
            {
                let all_lines: Vec<&str> = text.lines().collect();
                let basic_text: String = block
                    .basic_range
                    .clone()
                    .map(|i| all_lines[i])
                    .collect::<Vec<_>>()
                    .join("\n");
                return crate::locomotive::definition::locomotive_basic_goto_definition(
                    &basic_text,
                    position,
                    block.basic_range.start as u32,
                    &document.uri
                );
            }
        }

        let word = self.extract_word_at_position(&line, col)?;
        let word_upper = word.to_uppercase();

        // The backend will try other open documents if this returns None.
        self.find_definition_in(document, &word_upper)
    }

    /// Extract the word (ASM identifier) under the cursor, or `None`.
    pub fn word_at_position(&self, document: &Document, position: Position) -> Option<String> {
        let line = document.line(position.line as usize)?;
        self.extract_word_at_position(&line, position.character as usize)
    }

    /// Search `document` for a definition of `word_upper` (already uppercased).
    /// Returns the first matching `Location`, or `None`.
    pub fn find_definition_in(&self, document: &Document, word_upper: &str) -> Option<Location> {
        let listing = self.parse_document(document).ok()?;
        for token in listing.iter() {
            let source_name: &str = if token.is_label() {
                token.label_symbol()
            }
            else if token.is_equ() {
                token.equ_symbol()
            }
            else if token.is_assign() {
                token.assign_symbol()
            }
            else if token.is_macro_definition() {
                token.macro_definition_name()
            }
            else if token.is_module() {
                token.module_name()
            }
            else {
                continue;
            };
            if source_name.to_uppercase() == word_upper {
                let span = token.span();
                let (line_1based, col_1based) = span.relative_line_and_column();
                let lsp_line = line_1based.saturating_sub(1) as u32;
                let lsp_char = col_1based.saturating_sub(1) as u32;
                return Some(Location {
                    uri: document.uri.clone(),
                    range: Range {
                        start: Position {
                            line: lsp_line,
                            character: lsp_char
                        },
                        end: Position {
                            line: lsp_line,
                            character: lsp_char + source_name.len() as u32
                        }
                    }
                });
            }
        }
        None
    }

    /// Find all occurrences of `word_upper` (already uppercased) as whole words in `document`.
    pub fn find_references_in(&self, document: &Document, word_upper: &str) -> Vec<Location> {
        let text = document.text();
        let mut refs = Vec::new();
        for (line_idx, line) in text.lines().enumerate() {
            let line_up = line.to_uppercase();
            let wlen = word_upper.len();
            let mut start = 0;
            while start + wlen <= line_up.len() {
                if let Some(pos) = line_up[start..].find(word_upper) {
                    let abs = start + pos;
                    let before_ok = abs == 0 || !is_ident_byte(line.as_bytes()[abs - 1]);
                    let after_ok =
                        abs + wlen >= line.len() || !is_ident_byte(line.as_bytes()[abs + wlen]);
                    if before_ok && after_ok {
                        refs.push(Location {
                            uri: document.uri.clone(),
                            range: Range {
                                start: Position {
                                    line: line_idx as u32,
                                    character: abs as u32
                                },
                                end: Position {
                                    line: line_idx as u32,
                                    character: (abs + wlen) as u32
                                }
                            }
                        });
                    }
                    start = abs + 1;
                }
                else {
                    break;
                }
            }
        }
        refs
    }

    /// Find all references to a symbol
    pub fn find_references(&self, document: &Document, position: Position) -> Vec<Location> {
        let word = match self.word_at_position(document, position) {
            Some(w) => w.to_uppercase(),
            None => return Vec::new()
        };
        self.find_references_in(document, &word)
    }
}

// ─── Include file navigation ──────────────────────────────────────────────────

const INCLUDE_DIRECTIVES: &[&str] = &["INCLUDE", "INCBIN", "BINCLUDE"];

/// Directory-level markers that indicate the project root.  We stop walking
/// up the ancestor tree when we find one of these in the current directory.
const PROJECT_ROOT_MARKERS: &[&str] = &[
    ".git",
    ".hg",
    "Cargo.toml",
    "Cargo.lock",
    "Makefile",
    "makefile"
];

/// If `col` is inside a double-quoted string on a line that starts with an
/// include-like directive, return the resolved file URI.
fn resolve_include_at(line: &str, col: usize, doc_uri: &Url) -> Option<Url> {
    let bytes = line.as_bytes();
    if col >= bytes.len() {
        return None;
    }

    // Find the `"..."` string that contains (or starts at) `col`.
    let (str_start, str_end) = find_quoted_string(bytes, col)?;
    let filename = &line[str_start + 1..str_end]; // strip surrounding quotes

    // The part before the string must end with a recognised include keyword.
    let before = line[..str_start].trim().to_uppercase();
    let is_include = INCLUDE_DIRECTIVES.iter().any(|d| {
        before == *d || before.ends_with(&format!(" {d}")) || before.ends_with(&format!("\t{d}"))
    });
    if !is_include {
        return None;
    }

    let doc_path = doc_uri.to_file_path().ok()?;
    let mut dir = doc_path.parent()?;

    // Walk up the ancestor tree: try each directory as a base for `filename`.
    // Stop once we hit a project-root marker or the filesystem root.
    loop {
        let candidate = dir.join(filename);
        if candidate.exists() {
            return Url::from_file_path(candidate).ok();
        }
        // If this directory contains a project-root marker, don't go further up.
        let at_root = PROJECT_ROOT_MARKERS.iter().any(|m| dir.join(m).exists());
        match dir.parent() {
            Some(parent) if !at_root => dir = parent,
            _ => break
        }
    }
    None
}

/// Find the byte range of the quoted string `"..."` that covers position `col`.
/// Returns `(open_quote_pos, close_quote_pos)` where both positions are byte indices.
fn find_quoted_string(bytes: &[u8], col: usize) -> Option<(usize, usize)> {
    // Scan leftward to find the opening quote.
    let open = (0..=col).rev().find(|&i| bytes[i] == b'"')?;
    // Scan rightward to find the closing quote.
    let close = (col + 1..bytes.len()).find(|&i| bytes[i] == b'"')?;
    // `col` must be inside or on the opening/closing quote.
    if col >= open && col <= close {
        Some((open, close))
    }
    else {
        None
    }
}
