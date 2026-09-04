//! Diagnostics for bndbuild files: YAML validity, build-file structure,
//! dependency existence checks.

use camino::{Utf8Path, Utf8PathBuf};
use tower_lsp::lsp_types::*;

use super::BuildFileAnalyzer;
use crate::common::document::Document;

/// Expand a raw `targets:`/`dependencies:` token the same way the real
/// executor does (brace `{a,b}` + glob `*`/`?`/`[` expansion, reusing
/// `cpclib_bndbuild::expand_glob_in`), so a dependency like
/// `src/{common.asm,public.asm}` is checked as the two real files it
/// actually refers to, not as one literal, never-existing path. Falls back
/// to the token unexpanded when there's no real directory to glob against
/// (e.g. an unsaved buffer with no `file:` URI).
fn expand_dep_token(tok: &str, base_dir: Option<&Utf8Path>) -> Vec<String> {
    match base_dir {
        Some(dir) => cpclib_bndbuild::expand_glob_in(tok, dir),
        None => vec![tok.to_string()]
    }
}

impl BuildFileAnalyzer {
    /// Analyze the build file and return diagnostics
    pub fn analyze(&self, document: &Document) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let text = document.text();

        if text.contains("{{") || text.contains("{%") {
            // Jinja templates routinely make the *raw* text invalid YAML
            // (e.g. a `{% for %}` line at column 0 looks like a flow mapping
            // to a YAML parser), even though the file is perfectly valid
            // once rendered. Validate the rendered form instead, so real
            // syntax errors are still caught; structural/dependency checks
            // run either way — they scan line-by-line and tolerate leftover
            // template syntax on lines expansion couldn't resolve.
            match self.expand_cached(document) {
                Ok(result) => {
                    let expanded = &result.0;
                    if let Err(e) = serde_yaml::from_str::<serde_yaml::Value>(expanded) {
                        diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position {
                                    line: 0,
                                    character: 0
                                },
                                end: Position {
                                    line: 0,
                                    character: 0
                                }
                            },
                            severity: Some(DiagnosticSeverity::ERROR),
                            source: Some("yaml".to_string()),
                            message: format!("YAML parse error in the rendered template: {}", e),
                            ..Default::default()
                        });
                    }
                },
                Err(e) => {
                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: 0,
                                character: 0
                            },
                            end: Position {
                                line: 0,
                                character: 0
                            }
                        },
                        severity: Some(DiagnosticSeverity::INFORMATION),
                        source: Some("bndbuild".to_string()),
                        message: format!(
                            "Jinja template expansion failed ({e}); YAML and dependency validation is limited to what could be resolved."
                        ),
                        ..Default::default()
                    });
                }
            }
            diagnostics.extend(self.validate_build_structure(document));
        }
        else {
            match serde_yaml::from_str::<serde_yaml::Value>(&text) {
                Ok(_) => {
                    diagnostics.extend(self.validate_build_structure(document));
                },
                Err(e) => {
                    let line = e.location().map(|l| l.line()).unwrap_or(0);
                    let column = e.location().map(|l| l.column()).unwrap_or(0);
                    let line_idx = line.saturating_sub(1);
                    // `serde_yaml`'s `column` is a Unicode *character*
                    // count (from libyaml's `mark.column`), not UTF-16
                    // code units - convert via the error line's own text
                    // so wide characters earlier on the line don't desync
                    // the reported squiggle position. (Numeric convention
                    // otherwise unchanged from before - still 1-based-ish,
                    // still a fixed 10-char-wide guess for the end, since
                    // `serde_yaml` only reports a point, not a range.)
                    let error_line = text.lines().nth(line_idx).unwrap_or("");
                    let start_utf16 =
                        crate::common::document::char_count_to_utf16_col(error_line, column);
                    let end_utf16 =
                        crate::common::document::char_count_to_utf16_col(error_line, column + 10);

                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: line_idx as u32,
                                character: start_utf16 as u32
                            },
                            end: Position {
                                line: line_idx as u32,
                                character: end_utf16 as u32
                            }
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        source: Some("yaml".to_string()),
                        message: format!("YAML parse error: {}", e),
                        ..Default::default()
                    });
                }
            }
        }

        diagnostics
    }

    fn validate_build_structure(&self, document: &Document) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let raw_text = document.text();

        // Check for any valid target or task key (using RULE_KEYS from cpclib_bndbuild)
        let has_targets = super::token::TGT_KEY_NAMES
            .iter()
            .any(|key| raw_text.contains(&format!("{}:", key)));
        let has_tasks = super::token::TASK_KEY_NAMES
            .iter()
            .any(|key| raw_text.contains(&format!("{}:", key)));

        if !has_targets && !has_tasks && self.config().warnings.missing_build_structure {
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0
                    },
                    end: Position {
                        line: 0,
                        character: 0
                    }
                },
                severity: Some(DiagnosticSeverity::WARNING),
                source: Some("bndbuild".to_string()),
                message: "Build file should contain 'targets:' or 'tasks:' section".to_string(),
                ..Default::default()
            });
        }

        // Jinja-expand so rules/targets generated by `{% for %}` loops are
        // visible as literal text — otherwise a dependency on a loop-generated
        // target would be flagged as undefined. Fall back to raw text (an
        // identity source map) when expansion fails, preserving the old
        // best-effort behavior on templates that don't fully render.
        let expand_result = self.expand_or_identity(document);
        let (expanded_text, source_map) = (&expand_result.0, &expand_result.1);
        let raw_lines: Vec<&str> = raw_text.lines().collect();

        // ── Resolve base directory for file-existence checks and glob expansion ──
        let base_dir: Option<Utf8PathBuf> = document
            .uri
            .to_file_path()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .and_then(|d| Utf8PathBuf::from_path_buf(d).ok());

        // ── Collect declared target names (from the expanded text) ─────────
        // Glob-expanded: `targets: {a,b}.o` declares both `a.o` and `b.o`.
        let mut declared: std::collections::HashSet<String> = std::collections::HashSet::new();

        for line in expanded_text.lines() {
            let trimmed = line.trim_start();
            let content = if trimmed.starts_with("- ") {
                trimmed[2..].trim_start()
            }
            else {
                trimmed
            };
            for &key in super::token::TGT_KEY_NAMES.iter() {
                if let Some(rest) = content.strip_prefix(key).and_then(|r| r.strip_prefix(':')) {
                    let value = rest.split('#').next().unwrap_or("").trim();
                    // Skip block-scalar indicators; any remaining `{{`/`{%`
                    // means expansion fell back to raw text for this line.
                    if !value.starts_with('>') && !value.starts_with('|') {
                        for tok in value.split_whitespace() {
                            if !tok.contains("{{") && !tok.contains("{%") {
                                for expanded in expand_dep_token(tok, base_dir.as_deref()) {
                                    declared.insert(expanded);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Avoid emitting the same diagnostic once per loop iteration when a
        // `{% for %}`-generated dependency line repeats an undefined token.
        let mut seen: std::collections::HashSet<(u32, String)> = std::collections::HashSet::new();
        let warn_missing_dependency = self.config().warnings.missing_dependency;

        // ── Check each dependency value (on the expanded text) ──────────────
        for (exp_line_idx, line) in expanded_text.lines().enumerate() {
            let trimmed = line.trim_start();
            let content = if trimmed.starts_with("- ") {
                trimmed[2..].trim_start()
            }
            else {
                trimmed
            };

            for &key in super::token::DEP_KEY_NAMES.iter() {
                if let Some(rest) = content.strip_prefix(key).and_then(|r| r.strip_prefix(':')) {
                    let value = rest.split('#').next().unwrap_or("").trim();
                    if value.starts_with('>') || value.starts_with('|') {
                        break;
                    }

                    // Lines synthesised purely by Jinja control structures
                    // (no direct source counterpart) can't be reported.
                    let Some(orig_line_num) = source_map.to_original(exp_line_idx as u32)
                    else {
                        break;
                    };
                    let orig_line_text =
                        raw_lines.get(orig_line_num as usize).copied().unwrap_or("");
                    // A templated original line means the expanded column
                    // offsets computed below don't correspond to any real
                    // position in the source — highlight the whole line instead.
                    let was_templated =
                        orig_line_text.contains("{{") || orig_line_text.contains("{%");

                    // `key.len() + 1` accounts for the stripped `key:` prefix.
                    let mut col_offset = line.len() - content.len() + key.len() + 1;
                    // skip spaces after colon
                    let value_bytes = value.as_bytes();
                    let mut vi = 0;
                    while vi < value_bytes.len() && value_bytes[vi] == b' ' {
                        vi += 1;
                        col_offset += 1;
                    }

                    for tok in value.split_whitespace() {
                        // Still-unresolved Jinja expression (expansion fell
                        // back to raw text on this line) — can't validate.
                        if tok.contains("{{") || tok.contains("{%") {
                            col_offset += tok.len() + 1;
                            continue;
                        }

                        let tok_start = col_offset;
                        let tok_end = tok_start + tok.len();

                        if !declared.contains(tok) {
                            // `tok` may itself be a glob/brace pattern (e.g.
                            // `{common.asm,public.asm}` or `*.o`) — expand it
                            // the same way the real executor does before
                            // checking existence, so each real file it
                            // refers to is checked individually rather than
                            // treating the pattern text itself as a literal
                            // (always-missing) filename.
                            let expanded = expand_dep_token(tok, base_dir.as_deref());
                            let missing: Vec<&String> = expanded
                                .iter()
                                .filter(|e| {
                                    !declared.contains(*e)
                                        && !base_dir
                                            .as_ref()
                                            .map(|d| d.join(e).exists())
                                            .unwrap_or(false)
                                })
                                .collect();

                            if !missing.is_empty() && warn_missing_dependency {
                                let message = if missing.len() == 1 && missing[0] == tok {
                                    format!(
                                        "Dependency '{tok}' does not exist on disk, and no rule in this build file produces it as a target — it cannot be built."
                                    )
                                }
                                else {
                                    let missing_list = missing
                                        .iter()
                                        .map(|s| s.as_str())
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    format!(
                                        "Dependency '{tok}' expands to one or more files that don't exist on disk and aren't produced as a target: {missing_list}"
                                    )
                                };

                                if seen.insert((orig_line_num, message.clone())) {
                                    let range = if was_templated {
                                        Range {
                                            start: Position {
                                                line: orig_line_num,
                                                character: 0
                                            },
                                            end: Position {
                                                line: orig_line_num,
                                                character: orig_line_text.chars().count() as u32
                                            }
                                        }
                                    }
                                    else {
                                        Range {
                                            start: Position {
                                                line: orig_line_num,
                                                character: tok_start as u32
                                            },
                                            end: Position {
                                                line: orig_line_num,
                                                character: tok_end as u32
                                            }
                                        }
                                    };

                                    diagnostics.push(Diagnostic {
                                        range,
                                        severity: Some(DiagnosticSeverity::WARNING),
                                        source: Some("bndbuild".to_string()),
                                        message,
                                        ..Default::default()
                                    });
                                }
                            }
                        }

                        col_offset += tok.len() + 1;
                    }
                    break; // matched this dep_key, no need to check others
                }
            }
        }

        diagnostics
    }

    /// Target/rule names in `document` referencing `source_path` (an
    /// absolute, real filesystem path) - either structurally, via a
    /// glob-expanded `dep:`/`targets:` value, or textually, via a whole-word
    /// mention of `source_path`'s own basename inside a `cmd:`/task value
    /// (e.g. `cmd: basm --snapshot sna.asm -o out.sna` references
    /// `sna.asm` even though it's never declared as a formal `dep:` -  a
    /// real, common authoring style, confirmed against a real build file).
    /// Used to offer build tasks from a `.asm` file's own corresponding
    /// bndbuild file.
    ///
    /// Works on the Jinja-*expanded* text (reusing `expand_or_identity`,
    /// the same machinery `validate_build_structure` already uses) so a
    /// target/dependency/command value written as `{{ SOME_VAR }}` is
    /// resolved to its real value first - a naive raw-text scan would
    /// otherwise never recognize a rule whose only target name is a Jinja
    /// variable (a real, common shape) at all.
    ///
    /// A pragmatic text scan (rule boundary = a `- ` list item at indent 0,
    /// its target name = the first `tgt:`/`targets:`/... value seen after
    /// that) rather than a full YAML-structural walk - covers the common
    /// single-target-per-rule authoring style used throughout this
    /// codebase's own examples, not every possible YAML shape.
    pub fn targets_referencing(&self, document: &Document, source_path: &Utf8Path) -> Vec<String> {
        let Some(base_dir) = document
            .uri
            .to_file_path()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .and_then(|d| Utf8PathBuf::from_path_buf(d).ok())
        else {
            return Vec::new();
        };
        let canonical_source = fs_err::canonicalize(source_path)
            .unwrap_or_else(|_| std::path::PathBuf::from(source_path.as_std_path()));
        let source_basename = source_path.file_name().unwrap_or("");

        let expand_result = self.expand_or_identity(document);
        let text = &expand_result.0;
        let mut out = Vec::new();
        let mut current_rule: Option<String> = None;
        let mut rule_indent = 0usize;

        for raw_line in text.lines() {
            let trimmed = raw_line.trim_start();
            if trimmed.is_empty() {
                continue;
            }
            let indent = raw_line.len() - trimmed.len();
            if indent == 0 && trimmed.starts_with("- ") {
                current_rule = None;
                rule_indent = 0;
            }

            // A whole-word mention of the source file's own basename
            // anywhere inside the current rule's block (a `cmd:` value, or
            // a block-scalar command's own continuation line, which has no
            // `key:` of its own at all) counts as a reference - not every
            // real build file declares its basm input as a formal `dep:`.
            if let Some(rule_name) = &current_rule
                && indent > rule_indent
                && !source_basename.is_empty()
                && contains_whole_word(raw_line, source_basename)
            {
                out.push(rule_name.clone());
            }

            let content = trimmed.strip_prefix("- ").unwrap_or(trimmed);
            let Some((key, value)) = content.split_once(':')
            else {
                continue;
            };
            let key = key.trim();
            let value = value.split('#').next().unwrap_or("").trim();
            let is_tgt_key = super::token::TGT_KEY_NAMES.contains(&key);
            let is_dep_key = super::token::DEP_KEY_NAMES.contains(&key);
            if is_tgt_key
                && current_rule.is_none()
                && let Some(name) = value.split_whitespace().next()
            {
                current_rule = Some(name.to_string());
                rule_indent = indent;
            }
            let Some(rule_name) = current_rule.clone()
            else {
                continue;
            };
            if is_tgt_key || is_dep_key {
                for tok in value.split_whitespace() {
                    for expanded in expand_dep_token(tok, Some(&base_dir)) {
                        let resolved = base_dir.join(&expanded);
                        let canonical_resolved = fs_err::canonicalize(&resolved)
                            .unwrap_or_else(|_| std::path::PathBuf::from(resolved.as_std_path()));
                        if canonical_resolved == canonical_source {
                            out.push(rule_name.clone());
                        }
                    }
                }
            }
        }
        out.sort();
        out.dedup();
        out
    }
}

/// Whether `word` occurs in `haystack` as a whole word - not merely as a
/// substring of a longer identifier/filename (e.g. `sna.asm` inside
/// `mysna.asmfile` must not match). Bounding characters are simple
/// identifier bytes only (letters/digits/`_`); a path separator, quote, or
/// whitespace on either side counts as a valid boundary, so `src/sna.asm`
/// correctly matches `sna.asm`.
fn contains_whole_word(haystack: &str, word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let is_word_byte = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let bytes = haystack.as_bytes();
    let mut start = 0;
    while let Some(rel) = haystack[start..].find(word) {
        let pos = start + rel;
        let before_ok = pos == 0 || !is_word_byte(bytes[pos - 1]);
        let after = pos + word.len();
        let after_ok = after >= bytes.len() || !is_word_byte(bytes[after]);
        if before_ok && after_ok {
            return true;
        }
        start = pos + 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bndbuild::BuildFileAnalyzer;

    fn diagnostics_for(dir: &camino_tempfile::Utf8TempDir, text: &str) -> Vec<Diagnostic> {
        let uri = Url::from_file_path(dir.path().join("build.bnd")).unwrap();
        let document = Document::new(uri, text.to_string(), 1);
        BuildFileAnalyzer::new().analyze(&document)
    }

    /// Regression test for treating `serde_yaml`'s `column` (a Unicode
    /// *character* count) directly as an LSP UTF-16 `character`: a
    /// supplementary-plane character (😀, 1 Rust `char` but 2 UTF-16
    /// units) earlier on the error's own line must not desync the two.
    /// Confirmed empirically that this exact snippet reports its error at
    /// line 2 column 7 (`serde_yaml`'s own 1-based char count landing on
    /// the unexpected `:`); the correctly-converted UTF-16 column is 8
    /// (the emoji contributes 2 units, not 1).
    #[test]
    fn yaml_syntax_error_column_is_utf16_aware_with_a_supplementary_plane_char_before_it() {
        let uri = Url::parse("file:///build.bnd").unwrap();
        let text = "\u{1F600}: [1, 2\n\u{1F600} next: value\n";
        let doc = Document::new(uri, text.to_string(), 1);
        let diags = BuildFileAnalyzer::new().analyze(&doc);
        let yaml_diag = diags
            .iter()
            .find(|d| d.source.as_deref() == Some("yaml"))
            .expect("expected a YAML diagnostic");
        assert_eq!(yaml_diag.range.start.line, 1);
        assert_eq!(yaml_diag.range.start.character, 8);
    }

    #[test]
    fn dependency_on_an_existing_file_is_not_flagged() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("main.asm"), "").unwrap();
        let text = "- targets: out.bin\n  dep: main.asm\n  cmd: basm main.asm\n";
        let diags = diagnostics_for(&tmp, text);
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn brace_expanded_dependency_with_all_files_present_is_not_flagged() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src").join("common.asm"), "").unwrap();
        std::fs::write(tmp.path().join("src").join("public.asm"), "").unwrap();
        // Note: the brace can't be the *first* character of an unquoted YAML
        // scalar (that would parse as a flow mapping) - mirrors the user's
        // own real example, `../linking/src/demosystem/{common.asm,public.asm}`.
        let text = "- targets: out.bin\n  dep: src/{common.asm,public.asm}\n  cmd: basm out.bin\n";
        let diags = diagnostics_for(&tmp, text);
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn brace_expanded_dependency_missing_one_file_is_flagged_with_the_missing_name() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src").join("common.asm"), "").unwrap();
        // public.asm intentionally not created
        let text = "- targets: out.bin\n  dep: src/{common.asm,public.asm}\n  cmd: basm out.bin\n";
        let diags = diagnostics_for(&tmp, text);
        assert_eq!(diags.len(), 1, "{diags:?}");
        // The original brace pattern is echoed in the message for context
        // (so it necessarily mentions both names), but only the actually
        // missing expansion, `src/public.asm`, should appear as a standalone
        // missing entry - `src/common.asm` (which exists) must not.
        assert!(
            diags[0].message.contains("src/public.asm"),
            "{}",
            diags[0].message
        );
        assert!(
            !diags[0].message.contains("src/common.asm"),
            "{}",
            diags[0].message
        );
    }

    #[test]
    fn glob_dependency_matching_an_existing_file_is_not_flagged() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("data.bin"), "").unwrap();
        // A leading `*` would be a YAML alias reference in an unquoted plain
        // scalar - keep the glob character mid-token, same constraint as the
        // brace case above.
        let text = "- targets: out.bin\n  dep: d*.bin\n  cmd: basm out.bin\n";
        let diags = diagnostics_for(&tmp, text);
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn dependency_on_a_declared_target_is_not_flagged() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let text =
            "- targets: a.o\n  cmd: basm a.asm\n- targets: out.bin\n  dep: a.o\n  cmd: link a.o\n";
        let diags = diagnostics_for(&tmp, text);
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn dependency_on_neither_a_target_nor_a_file_is_flagged_clearly() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let text = "- targets: out.bin\n  dep: missing.asm\n  cmd: basm missing.asm\n";
        let diags = diagnostics_for(&tmp, text);
        assert_eq!(diags.len(), 1, "{diags:?}");
        let d = &diags[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::WARNING));
        assert!(
            d.message.contains("does not exist on disk"),
            "{}",
            d.message
        );
        assert!(
            d.message.contains("no rule in this build file produces it"),
            "{}",
            d.message
        );
        assert_eq!(d.range.start.line, 1);
    }

    #[test]
    fn disabling_missing_dependency_warnings_suppresses_them() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let text = "- targets: out.bin\n  dep: missing.asm\n  cmd: basm missing.asm\n";
        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        let document = Document::new(uri, text.to_string(), 1);
        let analyzer = BuildFileAnalyzer::new();
        assert_eq!(analyzer.analyze(&document).len(), 1);

        let mut config = crate::common::config::BndbuildConfig::default();
        config.warnings.missing_dependency = false;
        analyzer.set_config(config);
        assert!(analyzer.analyze(&document).is_empty());
    }

    #[test]
    fn disabling_missing_build_structure_warnings_suppresses_them() {
        let uri = Url::parse("file:///build.bnd").unwrap();
        let text = "foo: bar\n";
        let document = Document::new(uri, text.to_string(), 1);
        let analyzer = BuildFileAnalyzer::new();
        assert_eq!(analyzer.analyze(&document).len(), 1);

        let mut config = crate::common::config::BndbuildConfig::default();
        config.warnings.missing_build_structure = false;
        analyzer.set_config(config);
        assert!(analyzer.analyze(&document).is_empty());
    }

    #[test]
    fn dependency_on_a_loop_generated_target_is_not_flagged() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let text = "{% set files = [\"a\", \"b\"] %}\n\
                     {% for f in files %}\n\
                     - targets: {{f}}.o\n  cmd: basm {{f}}.asm\n\
                     {% endfor %}\n\
                     - targets: out.bin\n  dep: a.o b.o\n  cmd: link a.o b.o\n";
        let diags = diagnostics_for(&tmp, text);
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn repeated_undefined_dependency_across_loop_iterations_is_reported_once() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let text = "{% set files = [\"a\", \"b\"] %}\n\
                     {% for f in files %}\n\
                     - targets: {{f}}.o\n  dep: {{f}}.asm missing_shared.h\n  cmd: basm {{f}}.asm\n\
                     {% endfor %}\n";
        let diags = diagnostics_for(&tmp, text);
        let on_missing_header: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("missing_shared.h"))
            .collect();
        assert_eq!(on_missing_header.len(), 1, "{diags:?}");
    }
}

#[cfg(test)]
mod targets_referencing_tests {
    use super::*;
    use crate::bndbuild::BuildFileAnalyzer;

    #[test]
    fn finds_the_rule_declaring_the_file_as_a_dependency() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("main.asm"), "").unwrap();
        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        let text = "- tgt: out.bin\n  dep: main.asm\n  cmd: basm main.asm\n";
        let document = Document::new(uri, text.to_string(), 1);

        let source_path =
            camino::Utf8PathBuf::from_path_buf(tmp.path().join("main.asm").into_std_path_buf())
                .unwrap();
        let targets = BuildFileAnalyzer::new().targets_referencing(&document, &source_path);
        assert_eq!(targets, vec!["out.bin".to_string()]);
    }

    #[test]
    fn finds_the_rule_declaring_the_file_as_its_own_target() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("out.bin"), "").unwrap();
        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        let text = "- tgt: out.bin\n  cmd: basm main.asm\n";
        let document = Document::new(uri, text.to_string(), 1);

        let source_path =
            camino::Utf8PathBuf::from_path_buf(tmp.path().join("out.bin").into_std_path_buf())
                .unwrap();
        let targets = BuildFileAnalyzer::new().targets_referencing(&document, &source_path);
        assert_eq!(targets, vec!["out.bin".to_string()]);
    }

    #[test]
    fn unrelated_file_matches_no_rule() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("other.asm"), "").unwrap();
        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        let text = "- tgt: out.bin\n  dep: main.asm\n  cmd: basm main.asm\n";
        let document = Document::new(uri, text.to_string(), 1);

        let source_path =
            camino::Utf8PathBuf::from_path_buf(tmp.path().join("other.asm").into_std_path_buf())
                .unwrap();
        let targets = BuildFileAnalyzer::new().targets_referencing(&document, &source_path);
        assert!(targets.is_empty(), "{targets:?}");
    }

    /// Regression test for a real repro (`birthtro/src/build.bnd`): the
    /// source file is used as a `basm` command-line argument inside a
    /// multi-line `cmd: |` block scalar, never declared as a formal `dep:`,
    /// and the rule's own target name is itself a Jinja variable
    /// (`{{ SNA }}`) - both must resolve for the rule to be found at all.
    #[test]
    fn finds_a_rule_using_the_file_as_a_cmd_argument_with_a_jinja_target_name() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("sna.asm"), "").unwrap();
        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        let text = "{% set SNA=\"out.sna\" %}\n\n\
                     - tgt: test\n  dep: {{ SNA }}\n  cmd: -emu --snapshot {{SNA}} run\n\n\
                     - tgt: {{SNA}}\n  cmd: |\n    basm --snapshot sna.asm -o {{SNA}}\n        -DFOO=1\n";
        let document = Document::new(uri, text.to_string(), 1);

        let source_path =
            camino::Utf8PathBuf::from_path_buf(tmp.path().join("sna.asm").into_std_path_buf())
                .unwrap();
        let targets = BuildFileAnalyzer::new().targets_referencing(&document, &source_path);
        assert_eq!(targets, vec!["out.sna".to_string()], "{targets:?}");
    }

    #[test]
    fn does_not_match_a_file_whose_name_is_only_a_substring() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("sna.asm"), "").unwrap();
        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        let text = "- tgt: out.bin\n  cmd: basm mysna.asmfile -o out.bin\n";
        let document = Document::new(uri, text.to_string(), 1);

        let source_path =
            camino::Utf8PathBuf::from_path_buf(tmp.path().join("sna.asm").into_std_path_buf())
                .unwrap();
        let targets = BuildFileAnalyzer::new().targets_referencing(&document, &source_path);
        assert!(targets.is_empty(), "{targets:?}");
    }
}
