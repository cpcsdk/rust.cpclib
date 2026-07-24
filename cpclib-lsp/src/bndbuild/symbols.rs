//! Document symbols (outline) for bndbuild files: one symbol per rule/target,
//! Jinja-aware via the source map.

use tower_lsp::lsp_types::*;

use super::BuildFileAnalyzer;
use super::token::Collecting;
use crate::common::document::Document;

impl BuildFileAnalyzer {
    /// Outline for the editor: two top-level groups, "Variables" (the
    /// `{% set %}` definitions, each showing its value) and "Artifacts" (the
    /// rules/targets, as before). Either group is omitted when empty.
    pub fn document_symbols(&self, document: &Document) -> Vec<DocumentSymbol> {
        let variables = self.variable_symbols(document);
        let targets = self.target_symbols(document);

        let mut root = Vec::new();
        if !variables.is_empty() {
            root.push(Self::container_symbol("Variables", variables));
        }
        if !targets.is_empty() {
            root.push(Self::container_symbol("Artifacts", targets));
        }
        root
    }

    /// Flat list of target/rule symbols only, with no "Variables"/"Artifacts"
    /// grouping — used by callers that need one entry per actual buildable
    /// target (e.g. `code_lens`'s "▶ Run" buttons, `cpclib.getTargets`).
    pub(crate) fn target_symbols(&self, document: &Document) -> Vec<DocumentSymbol> {
        // Try Jinja expansion so loop-generated rules appear in the outline.
        // Fall back to raw text when expansion fails (missing variables, syntax
        // errors, etc.) so the outline still works on template-only edits.
        let expand_result = self.expand_or_identity(document);
        let (expanded_text, source_map) = (&expand_result.0, &expand_result.1);

        self.scan_symbols_from_text(expanded_text, source_map, &super::token::TGT_KEY_NAMES)
    }

    /// One symbol per `{% set NAME = VALUE %}`, `detail` set to the value's
    /// source text so it's visible directly in the outline.
    fn variable_symbols(&self, document: &Document) -> Vec<DocumentSymbol> {
        super::jinja::collect_jinja_variables(document)
            .into_iter()
            .map(|(name, value, location)| {
                #[allow(deprecated)]
                DocumentSymbol {
                    name,
                    detail: (!value.is_empty()).then_some(value),
                    kind: SymbolKind::VARIABLE,
                    tags: None,
                    deprecated: None,
                    range: location.range,
                    selection_range: location.range,
                    children: None
                }
            })
            .collect()
    }

    /// Group `children` under a synthetic namespace symbol spanning their
    /// combined range. `children` must be non-empty.
    fn container_symbol(name: &str, children: Vec<DocumentSymbol>) -> DocumentSymbol {
        let mut start = children[0].range.start;
        let mut end = children[0].range.end;
        for child in &children[1..] {
            if (child.range.start.line, child.range.start.character) < (start.line, start.character)
            {
                start = child.range.start;
            }
            if (child.range.end.line, child.range.end.character) > (end.line, end.character) {
                end = child.range.end;
            }
        }
        let range = Range { start, end };

        #[allow(deprecated)]
        DocumentSymbol {
            name: name.to_string(),
            detail: None,
            kind: SymbolKind::NAMESPACE,
            tags: None,
            deprecated: None,
            range,
            selection_range: range,
            children: Some(children)
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::document::Document;

    fn symbols_for(text: &str) -> Vec<DocumentSymbol> {
        let uri = Url::parse("file:///t.bnd").unwrap();
        let document = Document::new(uri, text.to_string(), 1);
        BuildFileAnalyzer::new().document_symbols(&document)
    }

    #[test]
    fn groups_variables_and_artifacts_separately() {
        let text = "{% set root = \"src\" %}\n- tgt: out.bin\n  cmd: basm {{root}}/main.asm\n";
        let symbols = symbols_for(text);
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["Variables", "Artifacts"], "{symbols:?}");

        let variables = &symbols[0];
        let children = variables.children.as_ref().expect("Variables has children");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name, "root");
        assert_eq!(children[0].detail.as_deref(), Some("\"src\""));
        assert_eq!(children[0].kind, SymbolKind::VARIABLE);

        let artifacts = &symbols[1];
        let children = artifacts.children.as_ref().expect("Artifacts has children");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name, "out.bin");
    }

    #[test]
    fn variables_group_omitted_when_there_are_no_set_statements() {
        let text = "- tgt: out.bin\n  cmd: basm main.asm\n";
        let symbols = symbols_for(text);
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["Artifacts"], "{symbols:?}");
    }

    #[test]
    fn target_symbols_stays_flat_for_code_lens_and_get_targets() {
        let uri = Url::parse("file:///t.bnd").unwrap();
        let document = Document::new(
            uri,
            "{% set root = \"src\" %}\n- tgt: out.bin\n  cmd: basm {{root}}/main.asm\n".to_string(),
            1
        );
        let flat = BuildFileAnalyzer::new().target_symbols(&document);
        let names: Vec<&str> = flat.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["out.bin"], "{flat:?}");
    }
}
