//! Peephole-optimization diagnostics, quickfix, and "Fix All" CodeLens: walk
//! `cpclib-asmoptim`'s matching engine over a parsed document and turn each
//! match into a `WARNING` `Diagnostic` (the same "structured finding → LSP
//! diagnostic" glue `diagnostics.rs`'s `collect_assembler_warnings`/
//! `collect_unused_binding_warnings` provide for their own sources), a
//! quickfix `CodeAction`, or one combined `WorkspaceEdit` applying every
//! match at once.
//!
//! Advisory only, like `unused_bindings` - this never runs as part of a real
//! `basm` assemble and never changes what gets assembled (see
//! `cpclib-asmoptim`'s own crate doc comment).

use tower_lsp::lsp_types::*;

use cpclib_asm::assembler::Env;
use cpclib_asm::flatten::flatten_listing;
use cpclib_asm::parser::obtained::{LocatedListing, LocatedToken, MayHaveSpan};
use cpclib_asm::parser::source::{SourceString, Z80Span};
use cpclib_asmoptim::engine::{PeepholeMatch, find_matches_with_resolver};
use cpclib_asmoptim::{EnvAddressResolver, OptimizationGoal, builtin_rules};

use super::AssemblyAnalyzer;
use super::command::{single_file_edit, single_file_multi_edit};
use crate::common::document::Document;

/// Diagnostic `source` tag for every peephole warning, distinct from plain
/// `"basm"` (used for real parser/assembler diagnostics) so a user can tell
/// at a glance which findings are advisory-only.
const SOURCE: &str = "basm-peephole";

/// The command name a "Fix All" CodeLens (see [`AssemblyAnalyzer::peephole_code_lenses`])
/// invokes - handled in `server/backend.rs`'s `execute_command`. `pub(crate)`,
/// not `pub(super)`: `server/backend.rs` is a sibling of `basm`, not a
/// descendant, so it needs crate-wide visibility to reference this same
/// constant rather than duplicating the literal string.
pub(crate) const FIX_ALL_COMMAND: &str = "cpclib.fixAllPeephole";

/// Flatten `listing` and match it against `goal`'s built-in rules, using
/// `env` (see [`AssemblyAnalyzer::peephole_quickfix_action`]'s own doc
/// comment on why it must be the same parse `env` was assembled from) to
/// evaluate address-aware rules like `jp2jr`. Shared by every entry point in
/// this file so they can never disagree with each other about what matches.
fn peephole_matches<'a>(
    listing: &'a LocatedListing,
    env: &Env,
    goal: OptimizationGoal
) -> (Vec<&'a LocatedToken>, Vec<PeepholeMatch>) {
    let tokens: Vec<&LocatedToken> = flatten_listing(listing.iter()).collect();
    if tokens.is_empty() {
        return (tokens, Vec::new());
    }
    let rules = builtin_rules(goal);
    let resolver = EnvAddressResolver::new(env);
    let matches = find_matches_with_resolver(&tokens, rules, &resolver);
    (tokens, matches)
}

/// Match `listing` against the built-in peephole rules and push one
/// `WARNING` `Diagnostic` per finding into `out`.
///
/// `env` should come from the same real (dry-run) assemble already computed
/// for this document version - `dry_run_env_cached`'s `Env` has
/// `record_token_addresses` enabled unconditionally (see that function's own
/// doc comment), which is what lets address-aware rules like `jp2jr`
/// (`reachableByJr`) actually fire instead of silently reporting unknown.
pub(super) fn collect_peephole_warnings(
    listing: &LocatedListing,
    env: &Env,
    goal: OptimizationGoal,
    out: &mut Vec<Diagnostic>
) {
    let (tokens, matches) = peephole_matches(listing, env, goal);
    for m in &matches {
        let start_span = tokens[m.start].span();
        let end_span = tokens[m.end - 1].span();
        out.push(Diagnostic {
            range: match_range(start_span, end_span),
            severity: Some(DiagnosticSeverity::WARNING),
            source: Some(SOURCE.to_string()),
            message: m.message.clone(),
            ..Default::default()
        });
    }
}

impl AssemblyAnalyzer {
    /// Offer to apply a peephole-optimization suggestion when the
    /// cursor/selection sits on any line the match covers - same re-derive-
    /// fresh-from-`(document, range)` shape every other quickfix in this
    /// crate uses (no diagnostic↔code-action pairing mechanism exists here,
    /// see `refactor.rs`'s own doc comments on its sibling actions).
    ///
    /// Deliberately does **not** case-adapt the replacement to the
    /// surrounding file's upper/lowercase style the way
    /// `no_op_or_improvable_instruction_action` does via `match_case_like` -
    /// that helper's own doc comment restricts it to a "fixed, static
    /// keyword string with no embedded user expression/symbol", and a
    /// peephole replacement can embed a real label (`jr target`). Blindly
    /// folding the whole string's case would silently rewrite the label's
    /// own spelling - exactly the symbolic-operand-corruption bug class
    /// this codebase has already been burned by once (see `jp2jr`'s own
    /// verbatim-label-preservation tests in `cpclib-asmoptim`).
    /// `PeepholeMatch::replacement` already renders keywords in canonical
    /// lower case and symbols verbatim (`Captures::rendered_text` in
    /// `cpclib-asmoptim`), which is correct as-is; matching the surrounding
    /// file's own case *style* is a cosmetic nicety left for later, not
    /// attempted here at the risk of corrupting a label.
    pub(super) fn peephole_quickfix_action(
        &self,
        document: &Document,
        range: Range
    ) -> Option<CodeAction> {
        let listing = self.parse_document(document).ok()?;
        let env = self.dry_run_env_cached(document, &listing);
        let goal = self.config().peephole_goal.into();
        let (tokens, matches) = peephole_matches(&listing, &env, goal);

        let cursor_line = range.start.line;
        let m = matches.iter().find(|m| {
            let start_line = super::token::span_line(tokens[m.start]);
            let end_line = super::token::span_line(tokens[m.end - 1]);
            (start_line..=end_line).contains(&cursor_line)
        })?;

        let edit = edit_for_match(document, &tokens, m);
        Some(CodeAction {
            title: format!("Peephole: {}", m.message),
            kind: Some(CodeActionKind::QUICKFIX),
            edit: Some(single_file_edit(document.uri.clone(), edit.range, edit.new_text)),
            ..Default::default()
        })
    }

    /// A single "⚡ N optimization opportunities - Fix All" `CodeLens` at the
    /// top of the document whenever at least one peephole match exists -
    /// empty `Vec` otherwise, matching every other `code_lens` provider in
    /// this crate's own convention (`embedded_bndbuild.rs`,
    /// `bndbuild::BuildFileAnalyzer`). Clicking it invokes
    /// [`FIX_ALL_COMMAND`], handled in `server/backend.rs`.
    pub(super) fn peephole_code_lenses(&self, document: &Document) -> Vec<CodeLens> {
        let Ok(listing) = self.parse_document(document)
        else {
            return Vec::new();
        };
        let env = self.dry_run_env_cached(document, &listing);
        let goal = self.config().peephole_goal.into();
        let (_tokens, matches) = peephole_matches(&listing, &env, goal);
        if matches.is_empty() {
            return Vec::new();
        }

        let count = matches.len();
        vec![CodeLens {
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
            command: Some(Command {
                title: format!(
                    "⚡ {count} optimization opportunit{} - Fix All",
                    if count == 1 { "y" } else { "ies" }
                ),
                command: FIX_ALL_COMMAND.to_string(),
                arguments: Some(vec![serde_json::json!(document.uri.to_string())])
            }),
            data: None
        }]
    }

    /// Build one `WorkspaceEdit` applying every peephole match in
    /// `document` at once (the [`FIX_ALL_COMMAND`] handler's job), alongside
    /// how many matches it covers (for the confirmation message). `None`
    /// when there is nothing to fix. `TextEdit`s within one document are
    /// resolved against the *original* text by the client
    /// (`single_file_multi_edit`'s own doc comment), so this doesn't need to
    /// worry about one match's edit shifting another's offsets - matches
    /// never overlap in the first place (`find_matches`'s own guarantee).
    pub(crate) fn fix_all_peephole_edit(&self, document: &Document) -> Option<(WorkspaceEdit, usize)> {
        let listing = self.parse_document(document).ok()?;
        let env = self.dry_run_env_cached(document, &listing);
        let goal = self.config().peephole_goal.into();
        let (tokens, matches) = peephole_matches(&listing, &env, goal);
        if matches.is_empty() {
            return None;
        }

        let edits: Vec<TextEdit> = matches
            .iter()
            .map(|m| edit_for_match(document, &tokens, m))
            .collect();
        let count = edits.len();
        Some((single_file_multi_edit(document.uri.clone(), edits), count))
    }
}

/// The `TextEdit` that applies one match - shared by the quickfix and "Fix
/// All", so they can never disagree about what a given match's edit looks
/// like.
fn edit_for_match(document: &Document, tokens: &[&LocatedToken], m: &PeepholeMatch) -> TextEdit {
    let first_line = super::token::span_line(tokens[m.start]);
    let last_line = super::token::span_line(tokens[m.end - 1]);

    if m.replacement.is_empty() {
        // Delete the whole matched line(s), trailing newline included, so
        // removing an instruction doesn't leave a blank line behind (same
        // fix `cpclib-basmopt::apply_fixes` already applies).
        TextEdit {
            range: Range {
                start: Position {
                    line: first_line,
                    character: 0
                },
                end: Position {
                    line: last_line + 1,
                    character: 0
                }
            },
            new_text: String::new()
        }
    }
    else {
        let first_span = tokens[m.start].span();
        let last_span = tokens[m.end - 1].span();
        let indent = leading_whitespace(&document.line(first_line as usize).unwrap_or_default());
        let new_text = m.replacement.join(&format!("\n{indent}"));
        TextEdit {
            range: match_range(first_span, last_span),
            new_text
        }
    }
}

/// The leading spaces/tabs of `line` - reused to indent every continuation
/// line of a multi-instruction replacement the same way the line it's
/// replacing was indented.
fn leading_whitespace(line: &str) -> String {
    line.chars().take_while(|c| *c == ' ' || *c == '\t').collect()
}

/// LSP `Range` covering everything from `start_span`'s own start to
/// `end_span`'s own end - the whole matched instruction sequence, which may
/// span several lines (e.g. a `push`/`pop` pair), not just `start_span`'s
/// single line. Same simple, always-ASCII-Z80-source assumption
/// `call_hierarchy.rs`'s `label_span_to_range` makes (no UTF-16 column
/// conversion): a matched instruction's own mnemonic/operand text is never
/// inside a string literal, so byte and UTF-16 columns coincide here.
fn match_range(start_span: &Z80Span, end_span: &Z80Span) -> Range {
    let (start_line_1, start_col_1) = start_span.relative_line_and_column();
    let (end_line_1, end_col_1) = end_span.relative_line_and_column();
    Range {
        start: Position {
            line: start_line_1.saturating_sub(1) as u32,
            character: start_col_1.saturating_sub(1) as u32
        },
        end: Position {
            line: end_line_1.saturating_sub(1) as u32,
            character: (end_col_1.saturating_sub(1) + end_span.as_str().len()) as u32
        }
    }
}

#[cfg(test)]
mod quickfix_tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///main.asm").unwrap(), text.to_string(), 1)
    }

    fn cursor(line: u32, character: u32) -> Range {
        Range {
            start: Position { line, character },
            end: Position { line, character }
        }
    }

    #[test]
    fn offers_a_deletion_quickfix_that_removes_the_whole_line() {
        let d = doc("start:\n    ld b, b\n    ret\n");
        let analyzer = AssemblyAnalyzer::new();
        let action = analyzer
            .peephole_quickfix_action(&d, cursor(1, 6))
            .expect("expected the quickfix");
        assert_eq!(action.title, "Peephole: Remove ld b,b");
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        let edit = action.edit.expect("expected an edit");
        let text_edits = &edit.changes.expect("expected changes")[&d.uri];
        assert_eq!(text_edits.len(), 1);
        assert_eq!(text_edits[0].new_text, "");
        assert_eq!(
            text_edits[0].range,
            Range {
                start: Position { line: 1, character: 0 },
                end: Position { line: 2, character: 0 }
            }
        );
    }

    #[test]
    fn offers_a_multi_line_replacement_quickfix_with_matching_indentation() {
        let d = doc("start:\n    push hl\n    pop de\n    ret\n");
        let analyzer = AssemblyAnalyzer::new();
        let action = analyzer
            .peephole_quickfix_action(&d, cursor(1, 6))
            .expect("expected the quickfix");
        assert_eq!(action.title, "Peephole: Replace push hl");
        let edit = action.edit.expect("expected an edit");
        let text_edits = &edit.changes.expect("expected changes")[&d.uri];
        assert_eq!(text_edits.len(), 1);
        assert_eq!(text_edits[0].new_text, "ld d, h\n    ld e, l");
        assert_eq!(
            text_edits[0].range,
            Range {
                start: Position { line: 1, character: 4 },
                end: Position { line: 2, character: 10 }
            }
        );
    }

    #[test]
    fn a_matched_symbolic_operand_survives_verbatim_in_the_quickfix() {
        // The exact historical bug class this quickfix must never
        // reintroduce: a label's own spelling must never be re-cased or
        // resolved away. `jp2jr` lives in the base rule set (used by every
        // goal, including the `Neutral` one this quickfix matches against -
        // see the fix note in memory about `jp2jr` not being Size-only), and
        // this target is trivially reachable by `jr`, so it fires here.
        let d = doc("SomeLabel:\n    JP SomeLabel\n");
        let analyzer = AssemblyAnalyzer::new();
        let action = analyzer
            .peephole_quickfix_action(&d, cursor(1, 6))
            .expect("expected jp2jr to fire");
        let edit = action.edit.expect("expected an edit");
        let text_edits = &edit.changes.expect("expected changes")[&d.uri];
        assert_eq!(text_edits[0].new_text, "jr SomeLabel");
    }

    #[test]
    fn is_wired_into_code_actions() {
        let d = doc("start:\n    ld b, b\n    ret\n");
        let analyzer = AssemblyAnalyzer::new();
        let actions = analyzer.code_actions(&d, cursor(1, 6));
        assert!(
            actions
                .iter()
                .any(|a| a.title == "Peephole: Remove ld b,b"
                    && a.kind == Some(CodeActionKind::QUICKFIX))
        );
    }

    #[test]
    fn no_quickfix_when_the_cursor_is_not_on_a_matched_line() {
        let d = doc("start:\n    ld b, b\n    ret\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(analyzer.peephole_quickfix_action(&d, cursor(2, 4)).is_none());
    }

    #[test]
    fn no_quickfix_for_already_optimal_source() {
        let d = doc("start:\n    xor a\n    ret\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(analyzer.peephole_quickfix_action(&d, cursor(1, 4)).is_none());
    }
}

#[cfg(test)]
mod code_lens_and_fix_all_tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///main.asm").unwrap(), text.to_string(), 1)
    }

    #[test]
    fn offers_a_fix_all_lens_with_the_right_count_when_matches_exist() {
        let d = doc("start:\n    ld b, b\n    push hl\n    pop de\n    ret\n");
        let analyzer = AssemblyAnalyzer::new();
        let lenses = analyzer.peephole_code_lenses(&d);
        assert_eq!(lenses.len(), 1, "{lenses:?}");
        let cmd = lenses[0].command.as_ref().unwrap();
        assert_eq!(cmd.title, "⚡ 2 optimization opportunities - Fix All");
        assert_eq!(cmd.command, FIX_ALL_COMMAND);
        assert_eq!(
            cmd.arguments.as_ref().unwrap()[0],
            serde_json::json!(d.uri.to_string())
        );
    }

    #[test]
    fn no_fix_all_lens_for_already_optimal_source() {
        let d = doc("start:\n    xor a\n    ret\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(analyzer.peephole_code_lenses(&d).is_empty());
    }

    #[test]
    fn fix_all_applies_every_match_in_one_combined_edit() {
        let d = doc("start:\n    ld b, b\n    push hl\n    pop de\n    ret\n");
        let analyzer = AssemblyAnalyzer::new();
        let (edit, count) = analyzer
            .fix_all_peephole_edit(&d)
            .expect("expected a combined edit");
        assert_eq!(count, 2);
        let text_edits = &edit.changes.expect("expected changes")[&d.uri];
        assert_eq!(text_edits.len(), 2);
        assert!(text_edits.iter().any(|e| e.new_text.is_empty()), "{text_edits:?}");
        assert!(
            text_edits
                .iter()
                .any(|e| e.new_text.contains("ld d, h") && e.new_text.contains("ld e, l")),
            "{text_edits:?}"
        );
    }

    #[test]
    fn fix_all_is_none_for_already_optimal_source() {
        let d = doc("start:\n    xor a\n    ret\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(analyzer.fix_all_peephole_edit(&d).is_none());
    }
}
