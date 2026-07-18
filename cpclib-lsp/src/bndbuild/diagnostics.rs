//! Diagnostics for bndbuild files: YAML validity, build-file structure,
//! dependency existence checks.

use tower_lsp::lsp_types::*;

use super::BuildFileAnalyzer;
use crate::common::document::Document;

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
            let file_dir = document
                .uri
                .to_file_path()
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()));
            match super::sourcemap::expand_with_source_map(&text, file_dir.as_deref()) {
                Ok((expanded, _map)) => {
                    if let Err(e) = serde_yaml::from_str::<serde_yaml::Value>(&expanded) {
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

                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: line.saturating_sub(1) as u32,
                                character: column as u32
                            },
                            end: Position {
                                line: line.saturating_sub(1) as u32,
                                character: (column + 10) as u32
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

        // Check for common build file keys
        let has_targets = raw_text.contains("targets:");
        let has_tasks = raw_text.contains("tasks:");

        if !has_targets && !has_tasks {
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
        let file_dir = document
            .uri
            .to_file_path()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));
        let (expanded_text, source_map) =
            match super::sourcemap::expand_with_source_map(&raw_text, file_dir.as_deref()) {
                Ok((text, map)) => (text, map),
                Err(_) => {
                    let lines = raw_text.lines().count();
                    (
                        raw_text.clone(),
                        super::sourcemap::SourceMap::identity(lines)
                    )
                }
            };
        let raw_lines: Vec<&str> = raw_text.lines().collect();

        // ── Collect declared target names (from the expanded text) ─────────
        let tgt_keys: &[&str] = &["targets", "tgt", "target", "build"];
        let dep_keys: &[&str] = &["dependencies", "dep", "dependency", "requires"];

        let mut declared: std::collections::HashSet<String> = std::collections::HashSet::new();

        for line in expanded_text.lines() {
            let trimmed = line.trim_start();
            let content = if trimmed.starts_with("- ") {
                trimmed[2..].trim_start()
            }
            else {
                trimmed
            };
            for &key in tgt_keys {
                let prefix = format!("{}:", key);
                if let Some(rest) = content.strip_prefix(prefix.as_str()) {
                    let value = rest.split('#').next().unwrap_or("").trim();
                    // Skip block-scalar indicators; any remaining `{{`/`{%`
                    // means expansion fell back to raw text for this line.
                    if !value.starts_with('>') && !value.starts_with('|') {
                        for tok in value.split_whitespace() {
                            if !tok.contains("{{") && !tok.contains("{%") {
                                declared.insert(tok.to_string());
                            }
                        }
                    }
                }
            }
        }

        // ── Resolve base directory for file-existence checks ───────────────
        let base_dir = file_dir;

        // Avoid emitting the same diagnostic once per loop iteration when a
        // `{% for %}`-generated dependency line repeats an undefined token.
        let mut seen: std::collections::HashSet<(u32, String)> = std::collections::HashSet::new();

        // ── Check each dependency value (on the expanded text) ──────────────
        for (exp_line_idx, line) in expanded_text.lines().enumerate() {
            let trimmed = line.trim_start();
            let content = if trimmed.starts_with("- ") {
                trimmed[2..].trim_start()
            }
            else {
                trimmed
            };

            for &key in dep_keys {
                let prefix = format!("{}:", key);
                if let Some(rest) = content.strip_prefix(prefix.as_str()) {
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

                    let mut col_offset = line.len() - content.len() + prefix.len();
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
                            let file_exists = base_dir
                                .as_ref()
                                .map(|d| d.join(tok).exists())
                                .unwrap_or(false);

                            if !file_exists {
                                let message = format!(
                                    "Dependency '{tok}' does not exist on disk, and no rule in this build file produces it as a target — it cannot be built."
                                );

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

    #[test]
    fn dependency_on_an_existing_file_is_not_flagged() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("main.asm"), "").unwrap();
        let text = "- targets: out.bin\n  dep: main.asm\n  cmd: basm main.asm\n";
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
