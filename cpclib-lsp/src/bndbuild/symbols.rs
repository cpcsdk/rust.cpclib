//! Document symbols (outline) for bndbuild files: one symbol per rule/target,
//! Jinja-aware via the source map.

use tower_lsp::lsp_types::*;

use super::BuildFileAnalyzer;
use super::token::Collecting;
use crate::common::document::Document;

impl BuildFileAnalyzer {
    pub fn document_symbols(&self, document: &Document) -> Vec<DocumentSymbol> {
        let target_names: Vec<&'static str> = cpclib_bndbuild::lsp::RULE_KEYS
            .iter()
            .find(|k| k.names.contains(&"targets"))
            .map(|k| k.names.to_vec())
            .unwrap_or_default();

        // Try Jinja expansion so loop-generated rules appear in the outline.
        // Fall back to raw text when expansion fails (missing variables, syntax
        // errors, etc.) so the outline still works on template-only edits.
        let file_dir = document
            .uri
            .to_file_path()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));
        let raw_text = document.text();
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

        self.scan_symbols_from_text(&expanded_text, &source_map, &target_names)
    }

    fn scan_symbols_from_text(
        &self,
        text: &str,
        source_map: &super::sourcemap::SourceMap,
        target_names: &[&'static str]
    ) -> Vec<DocumentSymbol> {
        let mut symbols = Vec::new();
        let mut rule_tgt: Option<(String, u32)> = None;
        let mut rule_help: Option<String> = None;
        let mut rule_start: u32 = 0;
        let mut rule_end: u32 = 0;
        let mut in_rule: bool = false;
        let mut collecting: Collecting = Collecting::Nothing;
        let mut block_base: Option<usize> = None; // indent of first content line
        let mut block_buf: String = String::new();

        // Finalise whatever block scalar was being accumulated and reset state.
        macro_rules! finalize_block {
            () => {
                if collecting != Collecting::Nothing {
                    let val = std::mem::take(&mut block_buf);
                    match collecting {
                        Collecting::Target(tgt_line) if rule_tgt.is_none() && !val.is_empty() => {
                            rule_tgt = Some((val, tgt_line));
                        },
                        Collecting::Help if rule_help.is_none() && !val.is_empty() => {
                            rule_help = Some(val);
                        },
                        _ => {}
                    }
                    collecting = Collecting::Nothing;
                    block_base = None;
                }
            };
        }

        // Emit DocumentSymbol entries for a completed rule.
        macro_rules! flush_rule {
            () => {
                if let Some((tgt_str, tgt_line)) = rule_tgt.take() {
                    for target in tgt_str.split_whitespace() {
                        let sel_end_char = target.len() as u32;

                        // VS Code requires selectionRange ⊆ fullRange.
                        // Source-map translation can produce out-of-order line
                        // numbers (e.g. {% for %} marker maps backwards), so
                        // build range defensively from the actual sel coordinates.
                        let range_start = rule_start.min(tgt_line);
                        let range_end = rule_end.max(tgt_line);
                        let range_end_char = if range_end == tgt_line {
                            sel_end_char
                        }
                        else {
                            0
                        };

                        let range = Range {
                            start: Position {
                                line: range_start,
                                character: 0
                            },
                            end: Position {
                                line: range_end,
                                character: range_end_char
                            }
                        };
                        let sel = Range {
                            start: Position {
                                line: tgt_line,
                                character: 0
                            },
                            end: Position {
                                line: tgt_line,
                                character: sel_end_char
                            }
                        };
                        #[allow(deprecated)]
                        symbols.push(DocumentSymbol {
                            name: target.to_string(),
                            detail: rule_help.clone(),
                            kind: SymbolKind::FILE,
                            tags: None,
                            deprecated: None,
                            range,
                            selection_range: sel,
                            children: None
                        });
                    }
                }
                rule_help = None;
            };
        }

        for (exp_idx, line_str) in text.lines().enumerate() {
            let orig = source_map
                .to_original(exp_idx as u32)
                .unwrap_or(exp_idx as u32);

            let indent = line_str.len() - line_str.trim_start().len();
            let trimmed = line_str.trim_start();

            // Skip blank lines; they don't terminate block scalars in YAML.
            if trimmed.is_empty() {
                continue;
            }

            // ── Block-scalar continuation ────────────────────────────────────
            if collecting != Collecting::Nothing {
                let base = *block_base.get_or_insert(indent);
                if indent >= base {
                    // Still inside the block — accumulate (strip trailing
                    // whitespace but keep words separated by a single space).
                    if !block_buf.is_empty() {
                        block_buf.push(' ');
                    }
                    block_buf.push_str(trimmed.trim_end());
                    if in_rule {
                        rule_end = orig;
                    }
                    continue;
                }
                // Indentation dropped → block is done; fall through to normal
                // processing of the current line.
                finalize_block!();
            }

            // ── Normal line processing ───────────────────────────────────────
            if trimmed.starts_with("- ") {
                if in_rule {
                    finalize_block!();
                    flush_rule!();
                }
                in_rule = true;
                rule_start = orig;
                rule_end = orig;

                let rest = trimmed[2..].trim_start();
                self.process_key_value(
                    rest,
                    orig,
                    target_names,
                    &mut rule_tgt,
                    &mut rule_help,
                    &mut collecting,
                    &mut block_base,
                    &mut block_buf
                );
            }
            else if in_rule {
                rule_end = orig;
                self.process_key_value(
                    trimmed,
                    orig,
                    target_names,
                    &mut rule_tgt,
                    &mut rule_help,
                    &mut collecting,
                    &mut block_base,
                    &mut block_buf
                );
            }
        }

        finalize_block!();
        if in_rule {
            flush_rule!();
        }

        symbols
    }

    /// Parse one line as `key: value` and update rule state.
    /// Recognises `>` and `|` as block-scalar indicators and switches
    /// `collecting` so the caller accumulates subsequent indented lines.
    fn process_key_value(
        &self,
        line: &str,
        orig: u32,
        target_names: &[&'static str],
        rule_tgt: &mut Option<(String, u32)>,
        rule_help: &mut Option<String>,
        collecting: &mut Collecting,
        block_base: &mut Option<usize>,
        block_buf: &mut String
    ) {
        let colon = match line.find(':') {
            Some(i) => i,
            None => return
        };
        let key = line[..colon].trim();
        let value = line[colon + 1..].split('#').next().unwrap_or("").trim();

        if rule_tgt.is_none() && target_names.contains(&key) {
            match value {
                ">" | "|" => {
                    *collecting = Collecting::Target(orig);
                    *block_base = None;
                    block_buf.clear();
                },
                "" => {},
                v => {
                    *rule_tgt = Some((v.to_string(), orig));
                }
            }
        }
        else if rule_help.is_none() && key == "help" {
            match value {
                ">" | "|" => {
                    *collecting = Collecting::Help;
                    *block_base = None;
                    block_buf.clear();
                },
                "" => {},
                v => {
                    *rule_help = Some(v.to_string());
                }
            }
        }
    }

    // Helper methods
}
