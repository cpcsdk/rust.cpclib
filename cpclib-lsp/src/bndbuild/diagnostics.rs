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

        // Try to parse as YAML
        match serde_yaml::from_str::<serde_yaml::Value>(&text) {
            Ok(_) => {
                // Valid YAML - now check for build file structure
                diagnostics.extend(self.validate_build_structure(document));
            },
            Err(e) => {
                // Check if it contains Jinja templates (which would cause YAML parse errors)
                if text.contains("{{") || text.contains("{%") {
                    // This is likely a Jinja template - provide a warning
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
                        code: None,
                        code_description: None,
                        source: Some("bndbuild".to_string()),
                        message: "File contains Jinja templates. YAML validation is limited."
                            .to_string(),
                        related_information: None,
                        tags: None,
                        data: None
                    });
                }
                else {
                    // Real YAML error
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
                        code: None,
                        code_description: None,
                        source: Some("yaml".to_string()),
                        message: format!("YAML parse error: {}", e),
                        related_information: None,
                        tags: None,
                        data: None
                    });
                }
            }
        }

        diagnostics
    }

    fn validate_build_structure(&self, document: &Document) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let text = document.text();

        // Check for common build file keys
        let has_targets = text.contains("targets:");
        let has_tasks = text.contains("tasks:");

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

        // ── Collect declared target names ──────────────────────────────────
        let tgt_keys: &[&str] = &["targets", "tgt", "target", "build"];
        let dep_keys: &[&str] = &["dependencies", "dep", "dependency", "requires"];

        let mut declared: std::collections::HashSet<String> = std::collections::HashSet::new();

        for line in text.lines() {
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
                    // Skip Jinja expressions and block-scalar indicators
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
        let base_dir = document
            .uri
            .to_file_path()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));

        // ── Check each dependency value ────────────────────────────────────
        for (line_num, line) in text.lines().enumerate() {
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

                    let mut col_offset = line.len() - content.len() + prefix.len();
                    // skip spaces after colon
                    let value_bytes = value.as_bytes();
                    let mut vi = 0;
                    while vi < value_bytes.len() && value_bytes[vi] == b' ' {
                        vi += 1;
                        col_offset += 1;
                    }

                    for tok in value.split_whitespace() {
                        // Skip Jinja expressions
                        if tok.contains("{{") || tok.contains("{%") {
                            col_offset += tok.len() + 1;
                            continue;
                        }

                        let tok_start = col_offset;
                        let tok_end = tok_start + tok.len();

                        if !declared.contains(tok) {
                            // Also skip if the file exists on disk
                            let file_exists = base_dir
                                .as_ref()
                                .map(|d| d.join(tok).exists())
                                .unwrap_or(false);

                            if !file_exists {
                                diagnostics.push(Diagnostic {
                                    range: Range {
                                        start: Position { line: line_num as u32, character: tok_start as u32 },
                                        end:   Position { line: line_num as u32, character: tok_end   as u32 },
                                    },
                                    severity: Some(DiagnosticSeverity::WARNING),
                                    source:   Some("bndbuild".to_string()),
                                    message:  format!("Undefined dependency '{}': not a declared target or existing file", tok),
                                    ..Default::default()
                                });
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
