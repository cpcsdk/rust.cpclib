//! Semantic tokens (syntax highlighting) and code lenses for bndbuild files.

use tower_lsp::lsp_types::*;

use super::BuildFileAnalyzer;
use super::token::*;
use crate::common::document::Document;

impl BuildFileAnalyzer {
    /// Semantic tokens for bndbuild (YAML + Jinja2) files.
    pub fn semantic_tokens(&self, document: &Document) -> Vec<SemanticToken> {
        let mut raw: Vec<(u32, u32, u32, u32, u32)> = Vec::new();

        let line_count = document.rope.len_lines();
        for line_num in 0..line_count {
            let line_str = match document.line(line_num) {
                Some(s) => s,
                None => continue
            };
            // Strip trailing newline for length accounting
            let line_str = line_str.trim_end_matches(['\n', '\r']);
            let bytes = line_str.as_bytes();
            let len = bytes.len();
            let line_u = line_num as u32;
            let mut col = 0usize;

            // Skip leading whitespace
            while col < len && matches!(bytes[col], b' ' | b'\t') {
                col += 1;
            }
            if col >= len {
                continue;
            }

            // Full-line comment
            if bytes[col] == b'#' {
                raw.push((line_u, col as u32, (len - col) as u32, TT_COMMENT, 0));
                continue;
            }

            // YAML list item marker `- `
            if bytes[col] == b'-' && (col + 1 >= len || matches!(bytes[col + 1], b' ' | b'\t')) {
                raw.push((line_u, col as u32, 1, TT_OPERATOR, 0));
                col += 1;
                while col < len && matches!(bytes[col], b' ' | b'\t') {
                    col += 1;
                }
            }

            while col < len {
                // Inline YAML comment — but only if not inside a Jinja construct.
                // A bare `#` that follows `{` or `%` is handled by the Jinja branches below.
                if bytes[col] == b'#' && (col == 0 || !matches!(bytes[col - 1], b'{' | b'%')) {
                    raw.push((line_u, col as u32, (len - col) as u32, TT_COMMENT, 0));
                    break;
                }

                // Jinja {# comment #} — skip entirely; TM grammar handles it.
                if col + 1 < len && bytes[col] == b'{' && bytes[col + 1] == b'#' {
                    col += 2;
                    while col + 1 < len && !(bytes[col] == b'#' && bytes[col + 1] == b'}') {
                        col += 1;
                    }
                    col = if col + 1 < len { col + 2 } else { len };
                    continue;
                }

                // Jinja {{ expression }} — skip; TM grammar colors the internals.
                if col + 1 < len && bytes[col] == b'{' && bytes[col + 1] == b'{' {
                    col += 2;
                    while col + 1 < len && !(bytes[col] == b'}' && bytes[col + 1] == b'}') {
                        col += 1;
                    }
                    col = if col + 1 < len { col + 2 } else { len };
                    continue;
                }

                // Jinja {% statement %} — skip; TM grammar colors keywords inside.
                if col + 1 < len && bytes[col] == b'{' && bytes[col + 1] == b'%' {
                    col += 2;
                    while col + 1 < len && !(bytes[col] == b'%' && bytes[col + 1] == b'}') {
                        col += 1;
                    }
                    col = if col + 1 < len { col + 2 } else { len };
                    continue;
                }

                // Double-quoted string
                if bytes[col] == b'"' {
                    let start = col;
                    col += 1;
                    while col < len {
                        if bytes[col] == b'"' && (col == start + 1 || bytes[col - 1] != b'\\') {
                            col += 1;
                            break;
                        }
                        col += 1;
                    }
                    raw.push((line_u, start as u32, (col - start) as u32, TT_STRING, 0));
                    continue;
                }

                // Single-quoted string
                if bytes[col] == b'\'' {
                    let start = col;
                    col += 1;
                    while col < len && bytes[col] != b'\'' {
                        col += 1;
                    }
                    if col < len {
                        col += 1;
                    }
                    raw.push((line_u, start as u32, (col - start) as u32, TT_STRING, 0));
                    continue;
                }

                // Identifier / keyword / YAML key
                if bytes[col].is_ascii_alphabetic() || bytes[col] == b'_' {
                    let start = col;
                    while col < len
                        && (bytes[col].is_ascii_alphanumeric() || matches!(bytes[col], b'_' | b'-'))
                    {
                        col += 1;
                    }
                    let word = &line_str[start..col];

                    // YAML key: word followed by ':'
                    if col < len && bytes[col] == b':' && (col + 1 >= len || bytes[col + 1] != b':')
                    {
                        let mods = if RULE_KEYS.contains(word) {
                            MOD_DECLARATION
                        }
                        else {
                            0
                        };
                        raw.push((
                            line_u,
                            start as u32,
                            (col - start) as u32,
                            TT_ENUM_MEMBER,
                            mods
                        ));
                        raw.push((line_u, col as u32, 1, TT_OPERATOR, 0));
                        col += 1;
                        continue;
                    }

                    // Boolean / null
                    if matches!(
                        word,
                        "true"
                            | "false"
                            | "yes"
                            | "no"
                            | "True"
                            | "False"
                            | "Yes"
                            | "No"
                            | "null"
                            | "Null"
                    ) {
                        raw.push((line_u, start as u32, (col - start) as u32, TT_KEYWORD, 0));
                    }
                    // (other identifiers emitted with no token — they're uncoloured)
                    continue;
                }

                // Numbers in bndbuild are almost always part of filenames, version strings,
                // or build arguments — don't color them to avoid visual noise.
                if bytes[col].is_ascii_digit() {
                    while col < len && (bytes[col].is_ascii_alphanumeric() || bytes[col] == b'.') {
                        col += 1;
                    }
                    continue;
                }

                col += 1;
            }
        }

        // Delta-encode for LSP protocol
        let mut result = Vec::with_capacity(raw.len());
        let mut prev_line = 0u32;
        let mut prev_start = 0u32;
        for (line, start, length, tok_type, modifiers) in raw {
            let delta_line = line - prev_line;
            let delta_start = if delta_line == 0 {
                start - prev_start
            }
            else {
                start
            };
            result.push(SemanticToken {
                delta_line,
                delta_start,
                length,
                token_type: tok_type,
                token_modifiers_bitset: modifiers
            });
            prev_line = line;
            prev_start = start;
        }
        result
    }

    /// Emit a CodeLens "▶ Run" button on each target declared in a bndbuild file.
    /// Delegates target detection to `document_symbols` so that Jinja expansion,
    /// block scalars, and all key aliases are handled consistently.
    pub fn code_lens(&self, document: &Document) -> Vec<CodeLens> {
        let file_path = document
            .uri
            .to_file_path()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        self.document_symbols(document)
            .into_iter()
            .map(|sym| {
                CodeLens {
                    range: sym.selection_range,
                    command: Some(Command {
                        title: format!("▶ Run: {}", sym.name),
                        command: "cpclib.runRule".to_string(),
                        arguments: Some(vec![
                            serde_json::json!(sym.name),
                            serde_json::json!(file_path),
                        ])
                    }),
                    data: None
                }
            })
            .collect()
    }
}
