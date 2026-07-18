//! Goto-definition and references for assembly files: labels/symbols,
//! include-file navigation, embedded-BASIC line targets.

use cpclib_asm::parser::obtained::MayHaveSpan;
use cpclib_tokens::{ListingElement, Token};
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
    ///
    /// A *definition* is a label token, or a directive that assigns the symbol
    /// (`EQU` / `=`), or a macro/module declaration — never a mere reference
    /// (e.g. the operand of a `CALL`/`JR`).
    ///
    /// Returns the first matching `Location`, or `None`.
    pub fn find_definition_in(&self, document: &Document, word_upper: &str) -> Option<Location> {
        if let Ok(listing) = self.parse_document(document) {
            for token in super::token::flatten_listing(listing.iter()) {
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
                    // TODO add the same for struct
                    token.macro_definition_name()
                }
                else if token.is_module() {
                    token.module_name()
                }
                else {
                    // A section's *definition*: `RANGE`/`DEFSECTION` start,
                    // stop, name — the name is the last argument, so `word_upper`
                    // here is what a `SECTION name` usage (or the definition
                    // itself) resolves to. `to_token()` is needed to extract it
                    // (no `is_range`/`range_*` accessor exists) and is only
                    // ever called for tokens that already look like a `RANGE`/
                    // `DEFSECTION` statement — see `starts_with_range_keyword`.
                    if token.is_directive()
                        && super::token::starts_with_range_keyword(token)
                        && let Token::Range(name, ..) = token.to_token().into_owned()
                        && name.to_uppercase() == word_upper
                    {
                        let (lsp_line, lsp_char) =
                            super::token::locate_name_in_statement(token, &name);
                        return Some(Location {
                            uri: document.uri.clone(),
                            range: Range {
                                start: Position {
                                    line: lsp_line,
                                    character: lsp_char
                                },
                                end: Position {
                                    line: lsp_line,
                                    character: lsp_char + name.len() as u32
                                }
                            }
                        });
                    }
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
        }

        // Text-based fallback, used both when the document does not fully
        // parse (goto-definition must keep working in files that don't
        // assemble yet, e.g. work-in-progress or disassembler output) and
        // when the parsed listing did not yield the symbol.
        self.find_definition_by_text(document, word_upper)
    }

    /// Line-oriented definition scan, used when the parsed listing yields
    /// nothing: matches `word:` / `word` at line start, and `word EQU ...` /
    /// `word = ...` anywhere the symbol starts the statement.
    fn find_definition_by_text(&self, document: &Document, word_upper: &str) -> Option<Location> {
        let text = document.text();
        for (line_idx, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            let indent = line.len() - trimmed.len();
            let upper = trimmed.to_uppercase();

            let Some(rest) = upper.strip_prefix(word_upper)
            else {
                continue;
            };
            // Must be a whole word.
            if rest.as_bytes().first().is_some_and(|&b| is_ident_byte(b)) {
                continue;
            }

            let rest_trimmed = rest.trim_start();
            let is_label_def = (rest.starts_with(':') && !rest.starts_with("::"))
                || (indent == 0 && (rest_trimmed.is_empty() || rest_trimmed.starts_with(';')));
            let is_symbol_def = rest_trimmed.starts_with("EQU")
                && !rest_trimmed
                    .as_bytes()
                    .get(3)
                    .is_some_and(|&b| is_ident_byte(b))
                || (rest_trimmed.starts_with('=') && !rest_trimmed.starts_with("=="));

            if is_label_def || is_symbol_def {
                return Some(Location {
                    uri: document.uri.clone(),
                    range: Range {
                        start: Position {
                            line: line_idx as u32,
                            character: indent as u32
                        },
                        end: Position {
                            line: line_idx as u32,
                            character: (indent + word_upper.len()) as u32
                        }
                    }
                });
            }
        }
        None
    }

    /// TODO rename it find_reference_in_by_text and rewrite find_reference_in using the listing. The references will be stored in expressions
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
    let filename = include_filename_at(line, col)?;
    let path = resolve_include_path(&filename, doc_uri)?;
    Url::from_file_path(path).ok()
}

/// If `col` is inside a double-quoted string on a line that starts with an
/// include-like directive (`INCLUDE`/`INCBIN`/`BINCLUDE`), return the raw
/// filename text (unresolved — may be a relative on-disk path or an
/// `inner://...` embedded-resource reference). Shared by ctrl+click
/// navigation (`resolve_include_at`) and hover content preview.
pub(super) fn include_filename_at(line: &str, col: usize) -> Option<String> {
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

    Some(filename.to_string())
}

/// Walk up from `doc_uri`'s directory, trying each ancestor as a base for
/// `filename`, stopping once a project-root marker or the filesystem root is
/// reached. Shared by ctrl+click include navigation and the eager
/// cross-file goto-definition fallback.
pub fn resolve_include_path(filename: &str, doc_uri: &Url) -> Option<std::path::PathBuf> {
    let doc_path = doc_uri.to_file_path().ok()?;
    let mut dir = doc_path.parent()?;
    loop {
        let candidate = dir.join(filename);
        if candidate.exists() {
            return Some(candidate);
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

/// Every filename referenced by an `INCLUDE`/`INCBIN`/`BINCLUDE` directive in
/// `text`, in document order. Best-effort text scan (recognizes the same
/// directives as `resolve_include_at`, but line-anchored rather than
/// cursor-anchored so the whole file can be scanned in one pass).
pub fn extract_include_filenames(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let upper = trimmed.to_uppercase();

        let Some(directive) = INCLUDE_DIRECTIVES.iter().find(|d| {
            upper == **d
                || upper.starts_with(&format!("{d} "))
                || upper.starts_with(&format!("{d}\t"))
        })
        else {
            continue;
        };

        let after = &trimmed[directive.len()..];
        let Some(q1) = after.find('"')
        else {
            continue;
        };
        let Some(q2_rel) = after[q1 + 1..].find('"')
        else {
            continue;
        };
        out.push(after[q1 + 1..q1 + 1 + q2_rel].to_string());
    }
    out
}

/// The nearest ancestor directory of `doc_uri` containing a project-root
/// marker (`.git`, `Cargo.toml`, ...), or the document's own directory if no
/// marker is found anywhere up to the filesystem root. Used as the search
/// base for the eager cross-file goto-definition fallback when the LSP
/// client didn't report any workspace folder.
pub fn project_root_for(doc_uri: &Url) -> Option<std::path::PathBuf> {
    let doc_path = doc_uri.to_file_path().ok()?;
    let own_dir = doc_path.parent()?.to_path_buf();
    let mut dir = own_dir.clone();
    loop {
        if PROJECT_ROOT_MARKERS.iter().any(|m| dir.join(m).exists()) {
            return Some(dir);
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => return Some(own_dir)
        }
    }
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

#[cfg(test)]
mod definition_tests {
    use super::*;

    #[test]
    fn label_definition_found_not_reference() {
        let text = r#"
        call    nz,output_char    ;{{c390:c4a0c3}} ; display text char
        jr      nz,_output_asciiz_string_2;{{c393:20f8}}  (-$08)

        pop     hl                ;{{c395:e1}}
        pop     af                ;{{c396:f1}}
        ret                       ;{{c397:c9}}
output_char:                      ;{{Addr=$c3a0 Code Calls/jump count: 12 Data
        ret
"#;
        let uri = tower_lsp::lsp_types::Url::parse("file:///test.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        let loc = analyzer.find_definition_in(&doc, "OUTPUT_CHAR");
        assert!(loc.is_some(), "definition of output_char should be found");
        let loc = loc.unwrap();
        assert_eq!(
            loc.range.start.line, 7,
            "definition is the label line, not the call reference"
        );
    }

    #[test]
    fn label_definition_found_despite_parse_error_elsewhere() {
        // The `!!!` line does not assemble; goto-definition must still work.
        let text = "        call nz,output_char\n        !!! invalid line !!!\noutput_char:\n        ret\n";
        let uri = tower_lsp::lsp_types::Url::parse("file:///test2.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        let loc = analyzer.find_definition_in(&doc, "OUTPUT_CHAR");
        assert!(
            loc.is_some(),
            "definition should be found even with parse errors"
        );
        assert_eq!(loc.unwrap().range.start.line, 2);
    }

    #[test]
    fn equ_and_assign_definitions_found() {
        let text = "        ld a,(screen_base)\nscreen_base equ 0xC000\nother_sym = 12\n        ld hl,other_sym\n";
        let uri = tower_lsp::lsp_types::Url::parse("file:///test3.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        let loc = analyzer.find_definition_in(&doc, "SCREEN_BASE");
        assert_eq!(loc.expect("equ definition").range.start.line, 1);
        let loc = analyzer.find_definition_in(&doc, "OTHER_SYM");
        assert_eq!(loc.expect("= definition").range.start.line, 2);
    }

    /// Regression test: a label wrapped in an `ifndef ... endif` header
    /// guard must still be reachable via goto-definition — `listing.iter()`
    /// alone only sees the top-level `IF` token, not what's inside it.
    #[test]
    fn definition_wrapped_in_an_ifndef_guard_is_found() {
        let text = "    ifndef GUARD\nGUARDED_LABEL:\n    ret\n    endif\n";
        let uri = tower_lsp::lsp_types::Url::parse("file:///guarded.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        let loc = analyzer.find_definition_in(&doc, "GUARDED_LABEL");
        assert_eq!(loc.expect("label inside ifndef").range.start.line, 1);
    }

    /// A `SECTION name` usage must jump to the `RANGE`/`DEFSECTION` line
    /// that defines that section name (the last argument), landing on the
    /// name itself rather than on the `RANGE`/`DEFSECTION` keyword.
    #[test]
    fn section_usage_jumps_to_its_range_definition() {
        let text = "RANGE 0x4000, 0x8000, MY_SECTION\nSECTION MY_SECTION\n    ret\n";
        let uri = tower_lsp::lsp_types::Url::parse("file:///sect.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        let loc = analyzer
            .find_definition_in(&doc, "MY_SECTION")
            .expect("section definition");
        assert_eq!(loc.range.start.line, 0, "{loc:?}");
        assert_eq!(loc.range.start.character, 22, "{loc:?}");
        assert_eq!(loc.range.end.character, 32, "{loc:?}");
    }

    #[test]
    fn goto_definition_from_a_section_usage_position_finds_the_range_line() {
        let text = "RANGE 0x4000, 0x8000, MY_SECTION\nSECTION MY_SECTION\n    ret\n";
        let uri = tower_lsp::lsp_types::Url::parse("file:///sect2.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        // Cursor on "MY_SECTION" within the `SECTION MY_SECTION` usage line.
        let loc = analyzer
            .goto_definition(
                &doc,
                Position {
                    line: 1,
                    character: 10
                }
            )
            .expect("goto-definition from a SECTION usage");
        assert_eq!(loc.range.start.line, 0, "{loc:?}");
    }

    #[test]
    fn goto_definition_on_the_range_name_itself_returns_its_own_position() {
        let text = "RANGE 0x4000, 0x8000, MY_SECTION\n    ret\n";
        let uri = tower_lsp::lsp_types::Url::parse("file:///sect3.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        let loc = analyzer
            .goto_definition(
                &doc,
                Position {
                    line: 0,
                    character: 25
                }
            )
            .expect("goto-definition on the section name inside its own definition");
        assert_eq!(loc.range.start.line, 0, "{loc:?}");
        assert_eq!(loc.range.start.character, 22, "{loc:?}");
    }
}
