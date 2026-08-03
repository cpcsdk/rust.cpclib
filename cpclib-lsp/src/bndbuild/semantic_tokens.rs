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
        assert_eq!(run_rule_lenses.len(), 1, "{lenses:?}");
        assert_eq!(
            run_rule_lenses[0].command.as_ref().unwrap().title,
            "▶ Run: a.asm, b.asm"
        );

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
