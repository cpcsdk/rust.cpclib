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
                    for slot in &mut byte_to_utf16[byte_pos..byte_pos + clen] {
                        *slot = utf16;
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

    /// Emit a CodeLens "▶ Run" button on each rule declared in a bndbuild
    /// file, plus one "▶ Run this command" button on each individual task
    /// line within that rule (bypassing the normal target-run path -
    /// dependency resolution, up-to-date checks, and every *other* task in
    /// the rule are skipped, only that one command executes). Delegates
    /// target detection to `target_symbols` so that Jinja expansion, block
    /// scalars, and all key aliases are handled consistently.
    ///
    /// Both the rule-level lens (`cpclib.runRuleInTerminal`) and the
    /// per-task lens (`cpclib.runTaskInTerminal`) run via a real VS Code
    /// Task/terminal, client-side only - `bndbuild --only-task RULE:INDEX`
    /// (`cpclib_bndbuild::BndBuilder::execute_task`) gives "run just task N
    /// of rule R" a real CLI equivalent now, with the same Jinja/automatic-
    /// variable context a normal build gets.
    ///
    /// Per-task lenses assume the common authoring style where a rule's
    /// declaring key (`tgt:`/`targets:`/...) sits on the rule's own `- `
    /// list-item line (true for every real-world example seen in this
    /// codebase's own tests/docs) - a rule using a nested `targets:` list
    /// form instead simply gets no per-task lenses, rather than a wrong one.
    ///
    /// A rule can declare *several* target names on one `tgt:`/`targets:`
    /// line (e.g. `- tgt: a.asm b.asm`) - `target_symbols` returns one
    /// `DocumentSymbol` per name, all sharing the same `rule_line`; those are
    /// grouped into a single lens whose title joins every name, rather than
    /// one indistinguishable, overlapping lens per name. Per-task lenses are
    /// still only generated once per `rule_line`, not once per target name.
    pub fn code_lens(&self, document: &Document) -> Vec<CodeLens> {
        let file_path = document
            .uri
            .to_file_path()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let text = document.text();

        // Group target symbols sharing the same rule line: a rule can
        // declare several names on one `tgt:`/`targets:` line (e.g.
        // `- tgt: a.asm b.asm`) and `target_symbols` returns one
        // `DocumentSymbol` per name - grouped here so the rule gets exactly
        // one "▶ Run" button whose title lists every target name, rather
        // than one indistinguishable, overlapping lens per name.
        let mut groups: Vec<(usize, Vec<DocumentSymbol>)> = Vec::new();
        for sym in self.target_symbols(document) {
            let rule_line = sym.selection_range.start.line as usize;
            match groups.iter_mut().find(|(line, _)| *line == rule_line) {
                Some((_, syms)) => syms.push(sym),
                None => groups.push((rule_line, vec![sym]))
            }
        }

        let mut lenses = Vec::new();

        // A second, summary lens per rule, all pinned to line 0 so VS Code
        // stacks them together in one lens bar above the file - lets the
        // user run/debug any rule without scrolling to find it. Reuses the
        // exact same command/arguments as the per-rule lens below; only the
        // range and title differ.
        let top_of_file = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 0 }
        };
        for (rule_line, syms) in &groups {
            let names = syms
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let first_name = syms[0].name.clone();
            lenses.push(CodeLens {
                range: top_of_file,
                command: Some(Command {
                    title: format!("Build: {names}"),
                    command: "cpclib.runRuleInTerminal".to_string(),
                    arguments: Some(vec![
                        serde_json::json!(first_name),
                        serde_json::json!(file_path),
                    ])
                }),
                data: None
            });
            if super::command::task_lines_in_rule(&text, *rule_line)
                .iter()
                .any(|(_, content)| rule_launches_an_emulator(content))
            {
                lenses.push(CodeLens {
                    range: top_of_file,
                    command: Some(Command {
                        title: format!("Debug: {names}"),
                        command: "cpclib.debugRule".to_string(),
                        arguments: Some(vec![
                            serde_json::json!(names),
                            serde_json::json!(file_path),
                        ])
                    }),
                    data: None
                });
            }
        }

        for (rule_line, syms) in groups {
            let names = syms
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            // Any one of the group's target names runs the whole rule (and
            // therefore builds every target it declares) - execution keeps
            // using the first name, only the button's title changes.
            let first_name = syms[0].name.clone();
            let range = Range {
                start: syms[0].selection_range.start,
                end: syms.last().unwrap().selection_range.end
            };
            lenses.push(CodeLens {
                range,
                command: Some(Command {
                    title: format!("▶ Run: {names}"),
                    // A real, on-disk `.bnd` file has a working CLI
                    // invocation (`bndbuild -f file target`), so this lens
                    // runs it as a real VS Code Task/terminal - reusing
                    // `BndbuildTaskProvider`'s task construction client-side
                    // - rather than the LSP's own `cpclib.runRule` streaming
                    // path, which VS Code's terminal+problemMatcher handles
                    // more reliably (clickable errors "for free"). This is a
                    // client-only command, never sent to the server - unlike
                    // `cpclib.runRule`/`cpclib.runTask` (still used by the
                    // embedded-bndbuild-in-.asm-block lenses below, which
                    // have no on-disk file for a CLI to target), it is
                    // deliberately *not* in `executeCommandProvider.commands`.
                    command: "cpclib.runRuleInTerminal".to_string(),
                    arguments: Some(vec![
                        serde_json::json!(first_name),
                        serde_json::json!(file_path),
                    ])
                }),
                data: None
            });

            // A rule that launches an emulator can also be debugged: the same
            // command is re-issued with `debug` instead of `run`, so the user
            // does not have to keep a second, drifting rule for it.
            if super::command::task_lines_in_rule(&text, rule_line)
                .iter()
                .any(|(_, content)| rule_launches_an_emulator(content))
            {
                lenses.push(CodeLens {
                    range,
                    command: Some(Command {
                        title: format!("🐞 Debug: {names}"),
                        command: "cpclib.debugRule".to_string(),
                        arguments: Some(vec![
                            serde_json::json!(names),
                            serde_json::json!(file_path),
                        ])
                    }),
                    data: None
                });
            }

            for (task_idx, (line_idx, _content)) in
                super::command::task_lines_in_rule(&text, rule_line)
                    .into_iter()
                    .enumerate()
            {
                let range = Range {
                    start: Position {
                        line: line_idx as u32,
                        character: 0
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: 0
                    }
                };
                lenses.push(CodeLens {
                    range,
                    command: Some(Command {
                        title: "▶ Run this command".to_string(),
                        // As with the rule-level lens above: a real on-disk
                        // `.bnd` file has a working CLI equivalent for "run
                        // just task N of rule R" too now
                        // (`bndbuild --only-task rule:N`, backed by
                        // `cpclib_bndbuild::BndBuilder::execute_task`), so
                        // this also runs via a real terminal instead of the
                        // LSP's own `cpclib.runTask` streaming path. Client-
                        // only command, deliberately not in
                        // `executeCommandProvider.commands`.
                        command: "cpclib.runTaskInTerminal".to_string(),
                        arguments: Some(vec![
                            serde_json::json!(first_name),
                            serde_json::json!(file_path),
                            serde_json::json!(task_idx),
                        ])
                    }),
                    data: None
                });
            }
        }
        lenses
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
        let mut lenses = Vec::new();
        for sym in self.scan_symbols_from_text(&expanded, &source_map, &super::token::TGT_KEY_NAMES)
        {
            let mut sel = sym.selection_range;
            sel.start.line += line_offset;
            sel.end.line += line_offset;
            lenses.push(CodeLens {
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
            });

            // A rule that launches an emulator can be debugged, wherever it is
            // written. Rules kept in a `.asm` file's own comments are still
            // rules - offering the button only in a standalone build file made
            // the feature depend on where you put them.
            //
            // The rule's own line inside the block is where its tasks are
            // looked up, so the same scan the standalone path uses works here
            // against the block's text.
            let rule_line = sym.selection_range.start.line as usize;
            if super::command::task_lines_in_rule(&expanded, rule_line)
                .iter()
                .any(|(_, content)| rule_launches_an_emulator(content))
            {
                lenses.push(CodeLens {
                    range: sel,
                    command: Some(Command {
                        title: format!("🐞 Debug: {}", sym.name),
                        command: "cpclib.debugRule".to_string(),
                        arguments: Some(vec![
                            serde_json::json!(sym.name),
                            serde_json::json!(host_file_path),
                        ])
                    }),
                    data: None
                });
            }
        }
        lenses
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
    fn code_lens_emits_a_per_task_lens_alongside_the_rule_lens() {
        let uri = Url::parse("file:///build.bnd").unwrap();
        let text = "- tgt: multi\n  phony: true\n  cmd:\n   - echo one\n   - echo two\n";
        let doc = Document::new(uri, text.to_string(), 1);
        let lenses = BuildFileAnalyzer::new().code_lens(&doc);

        assert!(
            lenses
                .iter()
                .any(|l| l.command.as_ref().unwrap().command == "cpclib.runRuleInTerminal"),
            "{lenses:?}"
        );
        let task_lenses: Vec<_> = lenses
            .iter()
            .filter(|l| l.command.as_ref().unwrap().command == "cpclib.runTaskInTerminal")
            .collect();
        assert_eq!(task_lenses.len(), 2, "{lenses:?}");
        for (i, lens) in task_lenses.iter().enumerate() {
            let args = lens.command.as_ref().unwrap().arguments.as_ref().unwrap();
            assert_eq!(args[0], serde_json::json!("multi"));
            assert_eq!(args[2], serde_json::json!(i));
        }
        // Task 0 ("echo one") is on line 3, task 1 ("echo two") is on line 4.
        assert_eq!(task_lenses[0].range.start.line, 3);
        assert_eq!(task_lenses[1].range.start.line, 4);
    }

    /// Regression test for a real-world rule shape (`birthtro/src/build.bnd`):
    /// a single scalar `cmd: |` block-scalar command spanning several
    /// indented continuation lines must still get exactly one CodeLens,
    /// anchored on the `cmd: |` line itself - not zero (the block-scalar
    /// indicator used to be explicitly skipped), and not one lens per
    /// continuation line (they aren't separate tasks).
    #[test]
    fn code_lens_covers_a_multiline_block_scalar_command_as_one_task() {
        let uri = Url::parse("file:///build.bnd").unwrap();
        let text =
            "- tgt: out.sna\n  cmd: |\n    basm --snapshot sna.asm -o out.sna\n        -DFOO=1\n";
        let doc = Document::new(uri, text.to_string(), 1);
        let lenses = BuildFileAnalyzer::new().code_lens(&doc);

        let task_lenses: Vec<_> = lenses
            .iter()
            .filter(|l| l.command.as_ref().unwrap().command == "cpclib.runTaskInTerminal")
            .collect();
        assert_eq!(task_lenses.len(), 1, "{lenses:?}");
        assert_eq!(task_lenses[0].range.start.line, 1);
    }

    /// Regression test for a real-world rule shape (`demo.bnd5/linking/build.bnd`):
    /// `- tgt: a.asm b.asm` declares two target names on one rule - they
    /// must collapse into a *single* "▶ Run" lens whose title lists both
    /// names (not one lens per name, which rendered as an indistinguishable
    /// duplicate button), and the rule's own task(s) must only get per-task
    /// lenses generated *once*, not once per target name sharing the rule.
    #[test]
    fn code_lens_does_not_duplicate_task_lenses_for_a_multi_target_rule() {
        let uri = Url::parse("file:///build.bnd").unwrap();
        let text = "- tgt: a.asm b.asm\n  cmd: echo one\n";
        let doc = Document::new(uri, text.to_string(), 1);
        let lenses = BuildFileAnalyzer::new().code_lens(&doc);

        let run_rule_lenses: Vec<_> = lenses
            .iter()
            .filter(|l| l.command.as_ref().unwrap().command == "cpclib.runRuleInTerminal")
            .collect();
        // One at the rule's own line, one more in the top-of-file summary -
        // still exactly one per rule at each of those two places, not one
        // per target name either place.
        assert_eq!(run_rule_lenses.len(), 2, "{lenses:?}");
        // The rule is declared on the file's very first line, so both lenses
        // land on line 0 - the summary one is the exact-zero range, the
        // per-rule one still spans the symbol's own selection range.
        let zero = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 0 }
        };
        let at_rule = run_rule_lenses
            .iter()
            .find(|l| l.range != zero)
            .unwrap_or_else(|| panic!("no per-rule lens: {lenses:?}"));
        assert_eq!(
            at_rule.command.as_ref().unwrap().title,
            "▶ Run: a.asm, b.asm"
        );
        let summary = run_rule_lenses
            .iter()
            .find(|l| l.range == zero)
            .unwrap_or_else(|| panic!("no top-of-file summary lens: {lenses:?}"));
        assert_eq!(summary.command.as_ref().unwrap().title, "Build: a.asm, b.asm");

        let task_lenses: Vec<_> = lenses
            .iter()
            .filter(|l| l.command.as_ref().unwrap().command == "cpclib.runTaskInTerminal")
            .collect();
        assert_eq!(
            task_lenses.len(),
            1,
            "the single task must only get one lens, not one per target name: {lenses:?}"
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

/// Whether a rule's command line launches an emulator with `run`.
///
/// Only such a rule can be debugged: the rewrite turns its `run` into `debug`,
/// and there is nothing to turn if the rule builds a disc or assembles a file.
///
/// Two details of bndbuild's own syntax matter here, and getting either wrong
/// makes the button silently not appear:
///
/// * a tool may be prefixed with `-` to ignore its errors (`-emu ... run`);
/// * a tool has several aliases, and the list belongs to bndbuild rather than
///   to this file - hence `EMUCTRL_CMDS` rather than a copy that would drift.
fn rule_launches_an_emulator(command: &str) -> bool {
    let Some((program, arguments)) = command.split_once(' ')
    else {
        return false;
    };
    let program = program.strip_prefix('-').unwrap_or(program);
    cpclib_bndbuild::task::EMUCTRL_CMDS.contains(&program)
        && cpclib_bndbuild::pipeline::debug::debug_arguments(arguments).is_some()
}

#[cfg(test)]
mod debug_lens_tests {
    use super::rule_launches_an_emulator;

    #[test]
    fn a_rule_that_runs_an_emulator_can_be_debugged() {
        assert!(rule_launches_an_emulator("emu --snapshot demo.sna run"));
        assert!(rule_launches_an_emulator("cpc --drivea test.dsk run"));
        assert!(rule_launches_an_emulator("emucontrol --emulator ace run"));
        assert!(rule_launches_an_emulator("emuctrl --snapshot d.sna run"));
    }

    /// A tool prefixed with `-` ignores its errors - and is still that tool.
    /// This is the form a real build file uses, and missing it is why the
    /// button did not appear.
    #[test]
    fn an_error_ignoring_prefix_still_counts() {
        assert!(rule_launches_an_emulator(
            "-emu --emulator ace   --snapshot demo.sna run"
        ));
        assert!(rule_launches_an_emulator("-cpc --drivea test.dsk run"));
    }

    /// Everything else offers no debug button, because there is nothing to
    /// rewrite.
    #[test]
    fn other_rules_are_left_alone() {
        assert!(!rule_launches_an_emulator("basm src/main.asm -o demo.sna"));
        assert!(!rule_launches_an_emulator("dsk demo.dsk format"));
        assert!(!rule_launches_an_emulator("emu --snapshot demo.sna orgams"));
        assert!(!rule_launches_an_emulator(""));
    }

    /// A path that merely contains the word is not the subcommand - the same
    /// trap the rewrite itself avoids.
    #[test]
    fn a_path_containing_run_is_not_enough() {
        assert!(!rule_launches_an_emulator("basm build/run/main.asm"));
    }
}

#[cfg(test)]
mod embedded_debug_lens_tests {
    use tower_lsp::lsp_types::Url;

    use super::*;
    use crate::common::document::Document;

    /// A rule embedded in a `.asm` file's comments gets the Debug button too.
    ///
    /// It only appeared for standalone build files, which made the feature
    /// depend on where the rules were written rather than on what they do.
    #[test]
    fn an_embedded_rule_that_launches_an_emulator_offers_the_debug_lens() {
        let analyzer = BuildFileAnalyzer::new();
        let yaml = "- tgt: run\n  dep: demo.sna\n  cmd: -emu --snapshot demo.sna run\n";
        let lenses = analyzer.code_lens_for_embedded_block(yaml, 3, "/p/demo.asm", None);

        let debug: Vec<_> = lenses
            .iter()
            .filter(|l| l.command.as_ref().unwrap().command == "cpclib.debugRule")
            .collect();
        assert_eq!(debug.len(), 1, "{lenses:?}");
        assert_eq!(
            debug[0].command.as_ref().unwrap().title,
            "🐞 Debug: run",
            "{lenses:?}"
        );
        // It carries the host `.asm`, which is what the adapter opens to find
        // the block again.
        assert_eq!(
            debug[0]
                .command
                .as_ref()
                .unwrap()
                .arguments
                .as_ref()
                .unwrap()[1],
            serde_json::json!("/p/demo.asm")
        );
        // ...and lands on the rule's own line inside the host file.
        assert_eq!(debug[0].range.start.line, 3, "{lenses:?}");
    }

    /// A rule that builds something but launches nothing gets no Debug button -
    /// the same rule as a standalone build file.
    #[test]
    fn an_embedded_rule_that_launches_nothing_offers_no_debug_lens() {
        let analyzer = BuildFileAnalyzer::new();
        let yaml = "- tgt: demo.sna\n  cmd: basm demo.asm -o demo.sna\n";
        let lenses = analyzer.code_lens_for_embedded_block(yaml, 0, "/p/demo.asm", None);

        assert!(
            lenses
                .iter()
                .all(|l| l.command.as_ref().unwrap().command != "cpclib.debugRule"),
            "{lenses:?}"
        );
        assert!(
            lenses
                .iter()
                .any(|l| l.command.as_ref().unwrap().command == "cpclib.runRule"),
            "the Run button is still there: {lenses:?}"
        );
    }

    /// The whole pipeline: a real `.asm` document with a block in its comments.
    #[test]
    fn a_real_asm_file_with_an_embedded_rule_shows_both_buttons() {
        let uri = Url::parse("file:///p/demo.asm").unwrap();
        let text = "  org 0x4000\n\
                    ; #!bndbuild\n\
                    ; - tgt: run\n\
                    ;   cmd: -emu --snapshot demo.sna run\n\
                      nop\n";
        let document = Document::new(uri, text.to_string(), 1);
        let lenses = crate::basm::AssemblyAnalyzer::new().code_lens(&document);

        let titles: Vec<&str> = lenses
            .iter()
            .filter_map(|l| l.command.as_ref().map(|c| c.title.as_str()))
            .collect();
        assert!(
            titles.iter().any(|t| t.starts_with("🐞 Debug")),
            "{titles:?}"
        );
        assert!(titles.iter().any(|t| t.starts_with("▶ Run")), "{titles:?}");
    }
}
