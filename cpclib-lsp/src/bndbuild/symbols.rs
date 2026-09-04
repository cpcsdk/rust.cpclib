//! Document symbols (outline) for bndbuild files: a flat, document-order
//! top-level list mixing `{% set %}` variables and rule/target symbols -
//! *not* grouped under synthetic "Variables"/"Artifacts" category headers
//! the way this module used to. A synthetic header's own `range` has no
//! value that's both safe and useful: spanning from its first child to its
//! last (the old behavior) makes VS Code treat the header itself as a real,
//! wide container to pin while scrolled anywhere between two unrelated,
//! far-apart rules; collapsing it to a narrow/zero-width point instead (a
//! later attempt) stops Sticky Scroll from ever recursing into the real
//! children at all, since it only descends into a symbol's `children` when
//! the *parent's* range already contains the scroll position - see
//! `common::symbols::container_symbol`'s own history and
//! `basm::symbols`'s matching fix for the full story. Each symbol keeps its
//! own `kind` (Variable vs. File), so the Outline panel still shows a
//! distinct icon per entry, just not bucketed under a header.
use tower_lsp::lsp_types::*;

use super::BuildFileAnalyzer;
use super::token::Collecting;
use crate::common::document::Document;

/// Mutable rule-accumulation state threaded through `process_key_value`,
/// borrowed from the caller's locals rather than owned.
struct RuleParseState<'a> {
    rule_tgt: &'a mut Option<(String, u32, u32)>,
    rule_help: &'a mut Option<String>,
    collecting: &'a mut Collecting,
    block_base: &'a mut Option<usize>,
    block_buf: &'a mut String
}

impl BuildFileAnalyzer {
    /// Outline for the editor: every `{% set %}` variable and every
    /// rule/target, flat, in document order.
    pub fn document_symbols(&self, document: &Document) -> Vec<DocumentSymbol> {
        let mut root = self.variable_symbols(document);
        root.extend(self.target_symbols(document));
        root.sort_by_key(|s| {
            (
                s.selection_range.start.line,
                s.selection_range.start.character
            )
        });
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

    // `finalize_block!`/`flush_rule!` reset `collecting`/`block_base`/
    // `rule_help` to their "nothing pending" state after consuming them -
    // correct and needed for every mid-loop invocation, but on each macro's
    // *final* call (right before `symbols` is returned) that reset is
    // provably never read again. Real state resets, not dead logic - not
    // worth complicating the shared macros to special-case the last call.
    #[allow(unused_assignments)]
    pub(super) fn scan_symbols_from_text(
        &self,
        text: &str,
        source_map: &super::sourcemap::SourceMap,
        target_names: &[&'static str]
    ) -> Vec<DocumentSymbol> {
        let mut symbols = Vec::new();
        // (value text, line, column where the value text starts on that line)
        let mut rule_tgt: Option<(String, u32, u32)> = None;
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
                            // Block-scalar continuation lines don't map
                            // cleanly to a single source column (the value
                            // spans several lines) - column 0 preserves the
                            // pre-existing (imperfect but harmless) behavior
                            // for this rarer case.
                            rule_tgt = Some((val, tgt_line, 0));
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
                if let Some((tgt_str, tgt_line, base_col)) = rule_tgt.take() {
                    // Track each target's own position within `tgt_str` -
                    // several names can share one `tgt:`/`targets:` line
                    // (e.g. `- tgt: a.asm b.asm`) and each must get its own,
                    // distinct column range rather than all collapsing onto
                    // column 0 (which made every extra target invisible/
                    // indistinguishable in the outline).
                    let mut search_from = 0usize;
                    for target in tgt_str.split_whitespace() {
                        let rel_start = tgt_str[search_from..]
                            .find(target)
                            .map(|i| i + search_from)
                            .unwrap_or(search_from);
                        let rel_end = rel_start + target.len();
                        search_from = rel_end;

                        let sel_start_char = base_col + rel_start as u32;
                        let sel_end_char = base_col + rel_end as u32;

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
                                character: sel_start_char
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
            // A line spliced in by `{% include %}` has no original line of
            // its own - falls forward to the line the `{% include %}`
            // directive itself was written on, so an included rule's own
            // code-lens/symbol lands on top of the directive that pulled it
            // in, not at a meaningless raw expanded-line index.
            let orig = source_map.to_original_or_nearest_following(exp_idx as u32);

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
            if let Some(after_dash) = trimmed.strip_prefix("- ") {
                if in_rule {
                    finalize_block!();
                    flush_rule!();
                }
                in_rule = true;
                rule_start = orig;
                rule_end = orig;

                let rest = after_dash.trim_start();
                let rest_col = indent + 2 + (after_dash.len() - rest.len());
                self.process_key_value(
                    rest,
                    rest_col as u32,
                    orig,
                    target_names,
                    &mut RuleParseState {
                        rule_tgt: &mut rule_tgt,
                        rule_help: &mut rule_help,
                        collecting: &mut collecting,
                        block_base: &mut block_base,
                        block_buf: &mut block_buf
                    }
                );
            }
            else if in_rule {
                rule_end = orig;
                self.process_key_value(
                    trimmed,
                    indent as u32,
                    orig,
                    target_names,
                    &mut RuleParseState {
                        rule_tgt: &mut rule_tgt,
                        rule_help: &mut rule_help,
                        collecting: &mut collecting,
                        block_base: &mut block_base,
                        block_buf: &mut block_buf
                    }
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
        line_col: u32,
        orig: u32,
        target_names: &[&'static str],
        state: &mut RuleParseState<'_>
    ) {
        let rule_tgt = &mut *state.rule_tgt;
        let rule_help = &mut *state.rule_help;
        let collecting = &mut *state.collecting;
        let block_base = &mut *state.block_base;
        let block_buf = &mut *state.block_buf;

        let colon = match line.find(':') {
            Some(i) => i,
            None => return
        };
        let key = line[..colon].trim();
        let after_colon = line[colon + 1..].split('#').next().unwrap_or("");
        let value = after_colon.trim();
        let value_col = line_col
            + colon as u32
            + 1
            + (after_colon.len() - after_colon.trim_start().len()) as u32;

        if rule_tgt.is_none() && target_names.contains(&key) {
            match value {
                ">" | "|" => {
                    *collecting = Collecting::Target(orig);
                    *block_base = None;
                    block_buf.clear();
                },
                "" => {},
                v => {
                    *rule_tgt = Some((v.to_string(), orig, value_col));
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
    fn variables_and_artifacts_appear_flat_in_document_order() {
        let text = "{% set root = \"src\" %}\n- tgt: out.bin\n  cmd: basm {{root}}/main.asm\n";
        let symbols = symbols_for(text);
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["root", "out.bin"], "{symbols:?}");

        assert_eq!(symbols[0].detail.as_deref(), Some("\"src\""));
        assert_eq!(symbols[0].kind, SymbolKind::VARIABLE);
        assert_eq!(symbols[1].kind, SymbolKind::FILE);
        // No synthetic grouping node wraps them - each is a real, direct
        // top-level entry (a grouping node's own `range` broke Sticky
        // Scroll no matter how it was computed - see the module doc
        // comment).
        assert!(symbols[0].children.is_none());
        assert!(symbols[1].children.is_none());
    }

    #[test]
    fn no_variables_means_just_the_targets() {
        let text = "- tgt: out.bin\n  cmd: basm main.asm\n";
        let symbols = symbols_for(text);
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["out.bin"], "{symbols:?}");
    }

    /// Regression test for a real report: `- tgt: a.asm b.asm` used to give
    /// *every* target name the same `selection_range` (always starting at
    /// column 0), so the outline only ever showed the first one distinctly.
    /// Each name must get its own, correctly-positioned column range.
    #[test]
    fn multi_target_rule_gives_each_name_its_own_column_range() {
        let text = "- tgt: a.asm b.asm\n  cmd: echo one\n";
        let symbols = symbols_for(text);
        assert_eq!(symbols.len(), 2, "{symbols:?}");

        assert_eq!(symbols[0].name, "a.asm");
        assert_eq!(symbols[0].selection_range.start.character, 7);
        assert_eq!(symbols[0].selection_range.end.character, 12);

        assert_eq!(symbols[1].name, "b.asm");
        assert_eq!(symbols[1].selection_range.start.character, 13);
        assert_eq!(symbols[1].selection_range.end.character, 18);
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

    /// Regression test for a real report: a rule pulled in via
    /// `{% include %}` got a symbol/code-lens position at a meaningless raw
    /// expanded-line index (since spliced content has no source-map
    /// counterpart of its own - see `SourceMap::
    /// to_original_or_nearest_following`'s own doc comment) instead of
    /// landing on the `{% include %}` line itself. Shared fix in
    /// `scan_symbols_from_text`'s use of the source map - benefits a real
    /// standalone `.bnd` file's own `target_symbols`/`code_lens`, not just
    /// the `#!bndbuild`-embedded case this was first reported against.
    #[test]
    fn an_included_rule_s_symbol_lands_on_the_include_line() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("build.bnd"),
            "- tgt: imported\n  cmd: echo hi\n"
        )
        .unwrap();
        let uri = Url::from_file_path(tmp.path().join("host.bnd")).unwrap();
        let text = "{% include \"build.bnd\" %}\n- tgt: local\n  cmd: echo local\n";
        let document = Document::new(uri, text.to_string(), 1);

        let flat = BuildFileAnalyzer::new().target_symbols(&document);
        let imported = flat
            .iter()
            .find(|s| s.name == "imported")
            .expect("expected a symbol for the included rule");
        let local = flat
            .iter()
            .find(|s| s.name == "local")
            .expect("expected a symbol for the local rule");
        assert_eq!(
            imported.selection_range.start.line, 0,
            "should land on the include line: {imported:?}"
        );
        assert_eq!(local.selection_range.start.line, 1, "{local:?}");
    }
}
