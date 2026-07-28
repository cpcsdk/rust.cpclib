//! Semantic tokens (syntax highlighting) and code lenses for bndbuild files.

use tower_lsp::lsp_types::*;

use super::BuildFileAnalyzer;
use super::sourcemap::SourceMap;
use super::token::*;
use crate::common::document::Document;

/// Jinja-expand an embedded block's own text (so a `{% include %}` inside
/// it - e.g. pulling in another bndbuild file's rules - is resolved before
/// scanning for targets), falling back to the raw text + an identity source
/// map on any expansion failure. Mirrors `expand_or_identity`'s own
/// best-effort fallback shape, but for a block's standalone text rather
/// than a whole `Document` (there's no on-disk file for just the block, so
/// this isn't cached the way a real document's expansion is).
fn expand_embedded_block_or_identity(
    yaml_text: &str,
    file_dir: Option<&std::path::Path>
) -> (String, SourceMap) {
    super::sourcemap::expand_with_source_map(yaml_text, file_dir).unwrap_or_else(|_| {
        (
            yaml_text.to_string(),
            SourceMap::identity(yaml_text.lines().count())
        )
    })
}

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

            // Map every byte offset on this line to its UTF-16 code-unit
            // column, so the byte-offset token spans the scanner below finds
            // can be reported to the client in UTF-16 units — what the LSP
            // semantic-tokens protocol requires. Without this, any
            // non-ASCII content (an accented character in a comment or
            // string) would misplace every subsequent token's highlight on
            // that line.
            let mut byte_to_utf16 = vec![0u32; len + 1];
            {
                let mut utf16 = 0u32;
                let mut byte_pos = 0usize;
                for c in line_str.chars() {
                    let clen = c.len_utf8();
                    for b in byte_pos..byte_pos + clen {
                        byte_to_utf16[b] = utf16;
                    }
                    byte_pos += clen;
                    utf16 += c.len_utf16() as u32;
                }
                byte_to_utf16[len] = utf16;
            }

            // Skip leading whitespace
            while col < len && matches!(bytes[col], b' ' | b'\t') {
                col += 1;
            }
            if col >= len {
                continue;
            }

            // Full-line comment
            if bytes[col] == b'#' {
                raw.push((
                    line_u,
                    byte_to_utf16[col],
                    byte_to_utf16[len] - byte_to_utf16[col],
                    TT_COMMENT,
                    0
                ));
                continue;
            }

            // YAML list item marker `- `
            if bytes[col] == b'-' && (col + 1 >= len || matches!(bytes[col + 1], b' ' | b'\t')) {
                raw.push((line_u, byte_to_utf16[col], 1, TT_OPERATOR, 0));
                col += 1;
                while col < len && matches!(bytes[col], b' ' | b'\t') {
                    col += 1;
                }
            }

            while col < len {
                // Inline YAML comment — but only if not inside a Jinja construct.
                // A bare `#` that follows `{` or `%` is handled by the Jinja branches below.
                if bytes[col] == b'#' && (col == 0 || !matches!(bytes[col - 1], b'{' | b'%')) {
                    raw.push((
                        line_u,
                        byte_to_utf16[col],
                        byte_to_utf16[len] - byte_to_utf16[col],
                        TT_COMMENT,
                        0
                    ));
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
                    raw.push((
                        line_u,
                        byte_to_utf16[start],
                        byte_to_utf16[col] - byte_to_utf16[start],
                        TT_STRING,
                        0
                    ));
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
                    raw.push((
                        line_u,
                        byte_to_utf16[start],
                        byte_to_utf16[col] - byte_to_utf16[start],
                        TT_STRING,
                        0
                    ));
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
                            byte_to_utf16[start],
                            byte_to_utf16[col] - byte_to_utf16[start],
                            TT_ENUM_MEMBER,
                            mods
                        ));
                        raw.push((line_u, byte_to_utf16[col], 1, TT_OPERATOR, 0));
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
                        raw.push((
                            line_u,
                            byte_to_utf16[start],
                            byte_to_utf16[col] - byte_to_utf16[start],
                            TT_KEYWORD,
                            0
                        ));
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
    /// Delegates target detection to `target_symbols` so that Jinja expansion,
    /// block scalars, and all key aliases are handled consistently.
    pub fn code_lens(&self, document: &Document) -> Vec<CodeLens> {
        let file_path = document
            .uri
            .to_file_path()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        self.target_symbols(document)
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

    /// Target/rule names declared in an already-extracted embedded block's
    /// own YAML text (`basm::embedded_bndbuild::EmbeddedBndbuildBlock`).
    /// Shared by `code_lens_for_embedded_block` and
    /// `basm::embedded_bndbuild::find_block_for_rule` (execution-time block
    /// disambiguation, when a file has more than one `#!bndbuild` block).
    pub(crate) fn target_names_for_embedded_block(
        &self,
        yaml_text: &str,
        file_dir: Option<&std::path::Path>
    ) -> Vec<String> {
        let (expanded, source_map) = expand_embedded_block_or_identity(yaml_text, file_dir);
        self.scan_symbols_from_text(&expanded, &source_map, &super::token::TGT_KEY_NAMES)
            .into_iter()
            .map(|s| s.name)
            .collect()
    }

    /// `code_lens`'s embedded-block counterpart: `yaml_text` is an
    /// already-extracted, line-preserving block
    /// (`basm::embedded_bndbuild::EmbeddedBndbuildBlock`); `line_offset` is
    /// added to every produced line number to translate block-relative
    /// coordinates back into the hosting `.asm` file's own coordinates.
    /// `host_file_path` becomes arg[1] of `"cpclib.runRule"` — the *.asm*
    /// file's own absolute path, since (unlike a real build file) there is
    /// no separate on-disk YAML file to reference. `file_dir` (the `.asm`
    /// file's own parent directory) is the base path used to resolve a
    /// `{% include %}` inside the block, so rules brought in from another
    /// bndbuild file (e.g. `{% include "build.bnd" %}`) get their own code
    /// lens too — matches how a real `.bnd` file's own `code_lens` already
    /// expands before scanning (`target_symbols`/`expand_or_identity`);
    /// `scan_symbols_from_text` already translates expanded-text positions
    /// back to block-relative original-text ones via the source map, so
    /// `line_offset` composes with that translation unchanged.
    ///
    /// Character-column offsets are deliberately not corrected for the
    /// stripped comment prefix — only line numbers are remapped. A client
    /// renders a CodeLens per-line regardless of `character`, and the
    /// dedented line is always shorter than or equal to its real `.asm`
    /// counterpart, so the (uncorrected) `character` value stays in-bounds.
    pub(crate) fn code_lens_for_embedded_block(
        &self,
        yaml_text: &str,
        line_offset: u32,
        host_file_path: &str,
        file_dir: Option<&std::path::Path>
    ) -> Vec<CodeLens> {
        let (expanded, source_map) = expand_embedded_block_or_identity(yaml_text, file_dir);
        self.scan_symbols_from_text(&expanded, &source_map, &super::token::TGT_KEY_NAMES)
            .into_iter()
            .map(|sym| {
                let mut sel = sym.selection_range;
                sel.start.line += line_offset;
                sel.end.line += line_offset;
                CodeLens {
                    range: sel,
                    command: Some(Command {
                        title: format!("▶ Run: {}", sym.name),
                        command: "cpclib.runRule".to_string(),
                        arguments: Some(vec![
                            serde_json::json!(sym.name),
                            serde_json::json!(host_file_path),
                        ])
                    }),
                    data: None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_positions(tokens: &[SemanticToken]) -> Vec<(u32, u32)> {
        let mut line = 0u32;
        let mut col = 0u32;
        let mut out = Vec::new();
        for t in tokens {
            if t.delta_line == 0 {
                col += t.delta_start;
            }
            else {
                line += t.delta_line;
                col = t.delta_start;
            }
            out.push((line, col));
        }
        out
    }

    #[test]
    fn semantic_tokens_use_utf16_columns_not_byte_offsets() {
        let uri = Url::parse("file:///build.bnd").unwrap();
        // 'é' is 2 bytes in UTF-8 but a single UTF-16 code unit - the
        // "flag" key token must be reported at UTF-16 column 14, not the
        // byte column 15 a naive byte-offset scan would produce.
        let text = "  dep: \"caf\u{e9}\" flag: 1\n";
        let doc = Document::new(uri, text.to_string(), 1);
        let tokens = BuildFileAnalyzer::new().semantic_tokens(&doc);
        let positions = decode_positions(&tokens);
        assert!(
            positions.contains(&(0, 14)),
            "expected a token starting at UTF-16 column 14 (the 'flag' key); got {positions:?}"
        );
        assert!(
            !positions.iter().any(|&(_, c)| c == 15),
            "no token should be reported at byte column 15; got {positions:?}"
        );
    }

    #[test]
    fn code_lens_for_embedded_block_shifts_lines_by_the_given_offset() {
        let yaml_text = "- tgt: test\n  cmd: echo hi\n";
        let lenses = BuildFileAnalyzer::new().code_lens_for_embedded_block(
            yaml_text,
            5,
            "/tmp/foo.asm",
            None
        );
        assert_eq!(lenses.len(), 1);
        let lens = &lenses[0];
        assert_eq!(lens.range.start.line, 5);
        let command = lens.command.as_ref().unwrap();
        assert_eq!(command.title, "▶ Run: test");
        assert_eq!(command.command, "cpclib.runRule");
        assert_eq!(
            command.arguments.as_ref().unwrap(),
            &vec![serde_json::json!("test"), serde_json::json!("/tmp/foo.asm")]
        );
    }

    #[test]
    fn code_lens_for_embedded_block_handles_multiple_targets_in_one_block() {
        let yaml_text = "- tgt: one\n  cmd: echo one\n- tgt: two\n  cmd: echo two\n";
        let lenses = BuildFileAnalyzer::new().code_lens_for_embedded_block(
            yaml_text,
            0,
            "/tmp/foo.asm",
            None
        );
        let titles: Vec<&str> = lenses
            .iter()
            .map(|l| l.command.as_ref().unwrap().title.as_str())
            .collect();
        assert_eq!(titles, vec!["▶ Run: one", "▶ Run: two"]);
    }

    #[test]
    fn code_lens_for_embedded_block_expands_an_included_file_s_rules() {
        // `{% import %}` (standard Jinja: pulls in macro/variable
        // *definitions* only, never a template's rendered output) is not
        // the right directive for pulling in another bndbuild file's rule
        // list - `{% include %}` is, since it splices the target's
        // rendered content in place. This proves the embedded-block path
        // now expands Jinja (with a working `{% include %}` loader) before
        // scanning, the same way a real `.bnd` file's own `code_lens`
        // already does via `target_symbols`/`expand_or_identity`.
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("build.bnd"),
            "- tgt: imported\n  cmd: echo hi\n"
        )
        .unwrap();
        let yaml_text = "{% include \"build.bnd\" %}\n- tgt: local\n  cmd: echo local\n";

        let lenses = BuildFileAnalyzer::new().code_lens_for_embedded_block(
            yaml_text,
            0,
            "/tmp/foo.asm",
            Some(tmp.path().as_std_path())
        );
        let titles: Vec<&str> = lenses
            .iter()
            .map(|l| l.command.as_ref().unwrap().title.as_str())
            .collect();
        assert_eq!(titles, vec!["▶ Run: imported", "▶ Run: local"]);
    }
}
