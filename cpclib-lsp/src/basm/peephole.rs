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
use cpclib_asm::flatten::flatten_for_analysis;
use cpclib_asm::parser::obtained::{LocatedListing, LocatedToken, MayHaveSpan};
use cpclib_asm::parser::source::{SourceString, Z80Span};
use cpclib_asmoptim::engine::{PeepholeMatch, find_matches, find_matches_with_resolver};
use cpclib_tokens::{DataAccessElem, ListingElement};
use cpclib_asmoptim::{EnvAddressResolver, OptimizationGoal, ProjectAddressResolver, builtin_rules};

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
    addresses: Addresses<'_>,
    goal: OptimizationGoal
) -> (Vec<&'a LocatedToken>, Vec<PeepholeMatch>) {
    let tokens: Vec<&LocatedToken> = flatten_for_analysis(listing.iter()).collect();
    if tokens.is_empty() {
        return (tokens, Vec::new());
    }
    let rules = builtin_rules(goal);

    // Without a *complete* assemble the recorded addresses describe a program
    // that was never finished being laid out, so no address-aware rule may
    // read them. Matching with no resolver at all is the existing, tested way
    // to say that: `reachableByJr` reports `Unknown`, and `jp2jr` stays quiet
    // rather than quoting a distance from a half-built program.
    //
    // `cpclib-basmopt::analyze_file` has always done exactly this on a failed
    // assemble; only this path did not, which is how a user was told a target
    // was 127 bytes away when the real build measured 146.
    let matches = match addresses {
        // The document is its own program: its own assemble is the real one.
        Addresses::OwnAssemble => {
            let resolver = EnvAddressResolver::new(env);
            find_matches_with_resolver(&tokens, rules, &resolver)
        },
        // The document is part of a larger program, so the addresses come from
        // assembling *that* - see `entry::project_addresses`.
        Addresses::Project(project) => {
            let resolver = ProjectAddressResolver::new(&project.env, project.document.clone());
            find_matches_with_resolver(&tokens, rules, &resolver)
        },
        Addresses::None => find_matches(&tokens, rules)
    };
    (tokens, matches)
}


/// The span of the first unconditional `jp <label>` in `listing`, if any.
///
/// Used to decide whether the "addresses unavailable" notice is worth showing:
/// without such a jump there is no address-aware suggestion to be missing, and
/// the notice would be pure noise.
fn unconditional_jump_span(listing: &LocatedListing) -> Option<&Z80Span> {
    flatten_for_analysis(listing.iter())
        .find(|t: &&LocatedToken| {
            if t.mnemonic() != Some(&cpclib_tokens::Mnemonic::Jp) {
                return false;
            }
            // Unconditional only: `jp nz, x` cannot become a `jr` for every
            // condition anyway, and it is not what `jp2jr` matches. Which slot
            // holds the target differs between the conditional and
            // unconditional forms, so this counts operands rather than
            // assuming one.
            let operands: Vec<_> = [t.mnemonic_arg1(), t.mnemonic_arg2()]
                .into_iter()
                .flatten()
                .collect();
            operands.len() == 1 && operands[0].get_expression().is_some()
        })
        .map(|t| t.span())
}

/// [`AssemblyAnalyzer::peephole_addresses`], reachable from sibling modules.
pub(super) fn address_source(
    analyzer: &AssemblyAnalyzer,
    document: &Document,
    own_assemble_complete: bool
) -> AddressSource {
    analyzer.peephole_addresses(document, own_assemble_complete)
}

/// Owned form of [`Addresses`], so a caller can hold the project assemble
/// alive across the call.
pub(super) enum AddressSource {
    OwnAssemble,
    Project(super::entry::ProjectAddresses),
    None
}

impl AddressSource {
    pub(super) fn as_addresses(&self) -> Addresses<'_> {
        match self {
            Self::OwnAssemble => Addresses::OwnAssemble,
            Self::Project(p) => Addresses::Project(p),
            Self::None => Addresses::None
        }
    }
}

impl AssemblyAnalyzer {
    /// The assembled project `Env` for `entry`, cached.
    ///
    /// Assembling a whole demo takes tens of seconds, so this must not happen
    /// per request. The cache key is the newest modification time across the
    /// project's sources: it changes exactly when a rebuild would lay code out
    /// differently, and costs a `stat` per file instead of an assemble.
    fn project_env_cached(
        &self,
        entry: &std::path::Path,
        doc_uri: &Url,
        config: &crate::common::config::AsmConfig
    ) -> Option<std::sync::Arc<Env>> {
        let root = super::entry::root_of(doc_uri)?;
        let fingerprint = super::entry::sources_fingerprint(&root);
        if let Some(entry_cache) = self.project_env_cache.get(entry)
            && entry_cache.0 == fingerprint
        {
            return Some(entry_cache.1.clone());
        }

        let disabled = super::parse::disabled_assembling_warning_categories(&config.warnings);
        let env = std::sync::Arc::new(super::entry::assemble_entry(
            entry,
            config.case_sensitive,
            disabled
        )?);
        self.project_env_cache
            .insert(entry.to_path_buf(), (fingerprint, env.clone()));
        Some(env)
    }

    /// Where this document's real addresses come from.
    ///
    /// `own_assemble_complete` is whether assembling the document by itself
    /// finished - necessary but not sufficient, because a file that is only
    /// ever `include`d assembles into a *different program* than the one it
    /// really belongs to, quite possibly without erroring at all.
    ///
    /// Ordered so the cheap test comes first: an unsaved buffer disqualifies
    /// the project route immediately (recorded addresses are keyed by byte
    /// offsets in the file *as assembled*), which means the expensive work -
    /// walking the workspace for the include graph, then assembling the entry
    /// - only ever happens for a document that matches disk, i.e. just after a
    /// save rather than on every keystroke.
    fn peephole_addresses(
        &self,
        document: &Document,
        own_assemble_complete: bool
    ) -> AddressSource {
        let Ok(path) = document.uri.to_file_path()
        else {
            return AddressSource::None;
        };
        // Cheap gate, before anything expensive. Two ways to fail it, both
        // meaning "the project route cannot apply here":
        //
        // * the buffer has unsaved edits, so every offset the project assemble
        //   recorded for this file has shifted;
        // * there is no file on disk at all (an unsaved or synthetic
        //   document), so it belongs to no project we can see.
        //
        // Either way the document's *own* assemble still describes the buffer
        // faithfully, which is what the LSP has always used.
        let buffer = document.text();
        let matches_disk = std::fs::read_to_string(&path).is_ok_and(|disk| disk == buffer);
        if !matches_disk {
            return if own_assemble_complete {
                AddressSource::OwnAssemble
            }
            else {
                AddressSource::None
            };
        }

        let config = self.config();
        match super::entry::entry_for(&document.uri, config.entry.as_deref()) {
            super::entry::Entry::Standalone => {
                if own_assemble_complete {
                    AddressSource::OwnAssemble
                }
                else {
                    AddressSource::None
                }
            },
            super::entry::Entry::Project(entry) => {
                let Some(env) = self.project_env_cached(&entry, &document.uri, &config)
                else {
                    return AddressSource::None;
                };
                match std::fs::canonicalize(&path) {
                    Ok(document) => {
                        AddressSource::Project(super::entry::ProjectAddresses { env, document })
                    },
                    Err(_) => AddressSource::None
                }
            },
            super::entry::Entry::Unknown => AddressSource::None
        }
    }
}

/// Where a document's real addresses come from, if anywhere.
pub(super) enum Addresses<'a> {
    /// This document *is* the program - assemble it directly, as the LSP has
    /// always done.
    OwnAssemble,
    /// This document is only part of a program; addresses come from assembling
    /// the entry that contains it.
    Project(&'a super::entry::ProjectAddresses),
    /// Nothing trustworthy. Address-aware rules must stay quiet: an
    /// incomplete assemble, an ambiguous entry, or a buffer that no longer
    /// matches disk (which shifts every recorded offset).
    None
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
    addresses: Addresses<'_>,
    uri: &Url,
    out: &mut Vec<Diagnostic>
) {
    // Say so when address-aware rules had to sit out. Silence is
    // indistinguishable from "nothing to suggest", and a user who has just
    // been told a `jp` is fine has no way to know the question was never
    // actually asked.
    if matches!(addresses, Addresses::None)
        && let Some(first) = unconditional_jump_span(listing)
    {
        out.push(Diagnostic {
            range: match_range(first, first),
            severity: Some(DiagnosticSeverity::INFORMATION),
            source: Some(SOURCE.to_string()),
            message: "jp/jr analysis skipped: this file's real addresses are not \
                      available. It is assembled as part of a larger program whose \
                      entry could not be determined or assembled (unsaved edits, an \
                      ambiguous entry, or missing -D definitions from the build \
                      rule). Set [asm] entry in cpclib-lsp.toml to name it."
                .to_string(),
            ..Default::default()
        });
    }

    let (tokens, matches) = peephole_matches(listing, env, addresses, goal);
    for m in &matches {
        let start_span = tokens[m.start].span();
        let end_span = tokens[m.end - 1].span();
        // The evidence goes in `relatedInformation` rather than the message:
        // the squiggle stays readable, and the editor turns each reason into a
        // clickable jump to the instruction that proves the suggestion safe.
        // Without it a reader has no way to tell whether a register is
        // clobbered two instructions later or inside a routine three calls
        // deep.
        let related: Vec<DiagnosticRelatedInformation> = m
            .reasons
            .iter()
            .map(|reason| {
                let range = reason
                    .witness
                    .and_then(|i| tokens.get(i))
                    .map(|t| {
                        let span = t.span();
                        match_range(span, span)
                    })
                    .unwrap_or_else(|| match_range(start_span, end_span));
                DiagnosticRelatedInformation {
                    location: Location {
                        uri: uri.clone(),
                        range
                    },
                    message: reason.text.clone()
                }
            })
            .collect();

        out.push(Diagnostic {
            range: match_range(start_span, end_span),
            severity: Some(DiagnosticSeverity::WARNING),
            source: Some(SOURCE.to_string()),
            message: m.message.clone(),
            related_information: if related.is_empty() {
                None
            }
            else {
                Some(related)
            },
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
        let (env, own_complete) = self.dry_run_env_cached_checked(document, &listing);
        let addresses = self.peephole_addresses(document, own_complete);
        let goal = self.config().peephole_goal.into();
        let (tokens, matches) = peephole_matches(&listing, &env, addresses.as_addresses(), goal);

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
        let (env, own_complete) = self.dry_run_env_cached_checked(document, &listing);
        let addresses = self.peephole_addresses(document, own_complete);
        let goal = self.config().peephole_goal.into();
        let (_tokens, matches) = peephole_matches(&listing, &env, addresses.as_addresses(), goal);
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
        let (env, own_complete) = self.dry_run_env_cached_checked(document, &listing);
        let addresses = self.peephole_addresses(document, own_complete);
        let goal = self.config().peephole_goal.into();
        let (tokens, matches) = peephole_matches(&listing, &env, addresses.as_addresses(), goal);
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
    let text = document.text();
    // Shared with `cpclib-basmopt` so the editor and the CLI never disagree
    // about what applying a suggestion means - and so the awkward parts (a
    // line holding several instructions, the `:` separators, a comment
    // running to end of line) are solved once. See `cpclib_asmoptim::edit`.
    match cpclib_asmoptim::edit::edit_for_match(&text, tokens, m) {
        Some(edit) => {
            TextEdit {
                range: byte_range_to_lsp(&text, &edit.range),
                new_text: edit.text
            }
        },
        // No span to anchor an edit to. Falling back to the matched
        // instructions' own range with no text would delete them, so the
        // safe degenerate edit is one that changes nothing.
        None => {
            let here = Position {
                line: super::token::span_line(tokens[m.start]),
                character: 0
            };
            TextEdit {
                range: Range {
                    start: here,
                    end: here
                },
                new_text: String::new()
            }
        }
    }
}

/// LSP `Range` covering everything from `start_span`'s own start to
/// `end_span`'s own end - the whole matched instruction sequence, which may
/// span several lines (e.g. a `push`/`pop` pair). Used to underline a
/// diagnostic; the *edit* that fixes it comes from `cpclib_asmoptim::edit`
/// instead, which has to be far more careful about exactly which bytes it
/// claims.
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

/// Convert a byte range in the document's text into the LSP `Range` a client
/// expects, with UTF-16 columns.
fn byte_range_to_lsp(text: &str, range: &std::ops::Range<usize>) -> Range {
    Range {
        start: byte_offset_to_position(text, range.start),
        end: byte_offset_to_position(text, range.end)
    }
}

fn byte_offset_to_position(text: &str, offset: usize) -> Position {
    let offset = offset.min(text.len());
    let line = text[..offset].matches('\n').count();
    let line_start = text[..offset].rfind('\n').map_or(0, |i| i + 1);
    let line_text = &text[line_start..cpclib_asmoptim::edit::line_end(text, line_start)];
    Position {
        line: line as u32,
        character: crate::common::document::byte_offset_to_utf16_col(
            line_text,
            offset - line_start
        ) as u32
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

    /// Apply a quickfix's edit to the document, so a test can assert on the
    /// file the user ends up with rather than on how the edit happens to be
    /// encoded. The range/text split is an implementation detail - replacing
    /// `    jp x` with `    jr x` and replacing `jp x` with `jr x` are the
    /// same fix - and pinning it made these tests fail when the edit logic
    /// moved into `cpclib_asmoptim::edit` without the result changing at all.
    fn apply(d: &Document, edit: &TextEdit) -> String {
        let text = d.text();
        let lines: Vec<&str> = text.split_inclusive('\n').collect();
        let offset = |p: Position| -> usize {
            let before: usize = lines
                .iter()
                .take(p.line as usize)
                .map(|l| l.len())
                .sum();
            before + p.character as usize
        };
        let mut out = text.clone();
        out.replace_range(offset(edit.range.start)..offset(edit.range.end), &edit.new_text);
        out
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
        assert_eq!(
            apply(&d, &text_edits[0]),
            "start:\n    ld d, h\n    ld e, l\n    ret\n",
            "both replacement instructions must land, indented like the code they replace"
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
        assert_eq!(
            apply(&d, &text_edits[0]),
            "SomeLabel:\n    jr SomeLabel\n",
            "the label's own spelling must survive the rewrite"
        );
    }

    /// A failed assemble must never produce an address-aware suggestion.
    ///
    /// The reported bug: `demo_code.asm` is not the program (`sna.asm` is), so
    /// assembling it alone fails on an unresolvable include - and the LSP used
    /// the half-built `Env`'s addresses anyway, telling the user a `jp` target
    /// was 127 bytes away when the real build measured 146.
    #[test]
    fn an_incomplete_assemble_yields_no_address_aware_suggestion() {
        // `include` of a file that does not exist: the assemble cannot finish,
        // exactly like `include MUSIC_CFG` in the real project.
        let d = doc("    include \"there-is-no-such-file.asm\"\nSomeLabel:\n    JP SomeLabel\n");
        let analyzer = AssemblyAnalyzer::new();
        let listing = analyzer.parse_document(&d).expect("must parse");
        let (env, own_complete) = analyzer.dry_run_env_cached_checked(&d, &listing);
        assert!(
            !own_complete,
            "an unresolvable include must mark the assemble incomplete"
        );

        let (_tokens, matches) = peephole_matches(
            &listing,
            &env,
            Addresses::None,
            OptimizationGoal::Neutral
        );
        assert!(
            !matches.iter().any(|m| m.rule_name.as_deref() == Some("jp2jr")),
            "jp2jr must not fire off a half-built program: {matches:?}"
        );
    }

    /// The control: the same jump in a file that assembles cleanly still gets
    /// its suggestion, so the guard above is about completeness and not about
    /// jp2jr having been switched off.
    #[test]
    fn a_complete_assemble_still_gets_its_address_aware_suggestion() {
        let d = doc("SomeLabel:\n    JP SomeLabel\n");
        let analyzer = AssemblyAnalyzer::new();
        let listing = analyzer.parse_document(&d).expect("must parse");
        let (env, own_complete) = analyzer.dry_run_env_cached_checked(&d, &listing);
        assert!(own_complete, "this file assembles fine");

        let (_tokens, matches) = peephole_matches(
            &listing,
            &env,
            Addresses::OwnAssemble,
            OptimizationGoal::Neutral
        );
        assert!(
            matches.iter().any(|m| m.rule_name.as_deref() == Some("jp2jr")),
            "jp2jr should fire when the addresses are real: {matches:?}"
        );
    }

    /// The reported bug, against the real project layout in miniature: the
    /// document is only ever `include`d, and the file including it is the one
    /// that carries `RUN` *and* the memory map. Assembled alone the document
    /// starts at 0 and the jump looks near; through the entry it starts where
    /// the `org` puts it and the same jump is far out of `jr` range.
    #[test]
    fn an_included_file_is_measured_through_its_entry_not_on_its_own() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let root = tmp.path().as_std_path();
        std::fs::create_dir_all(root.join(".git")).unwrap();

        // 200 bytes of padding puts `target` out of `jr` reach *only* once the
        // entry's `org` and preceding data are taken into account.
        let mut body = String::from("start\n    jp target\n");
        for _ in 0..200 {
            body.push_str("    nop\n");
        }
        body.push_str("target\n    ret\n");
        std::fs::write(root.join("code.asm"), &body).unwrap();
        std::fs::write(
            root.join("main.asm"),
            "    org 0x4000\n    run start\n    include \"code.asm\"\n"
        )
        .unwrap();

        let code = std::fs::canonicalize(root.join("code.asm")).unwrap();
        let uri = Url::from_file_path(&code).unwrap();
        let d = Document::new(uri, body.clone(), 1);

        let analyzer = AssemblyAnalyzer::new();
        let listing = analyzer.parse_document(&d).expect("must parse");
        let (env, own_complete) = analyzer.dry_run_env_cached_checked(&d, &listing);
        let addresses = analyzer.peephole_addresses(&d, own_complete);

        assert!(
            matches!(addresses, AddressSource::Project(_)),
            "an included file must be measured through its entry"
        );

        let (_tokens, matches) =
            peephole_matches(&listing, &env, addresses.as_addresses(), OptimizationGoal::Size);
        assert!(
            !matches.iter().any(|m| m.rule_name.as_deref() == Some("jp2jr")),
            "the target is >127 bytes away in the real program, so jp2jr must \
             not fire: {matches:?}"
        );
    }

    /// The control: the very same file, with the jump close enough that it is
    /// in range even through the entry. Without this, the test above would
    /// pass just as well if the project route were silently broken.
    #[test]
    fn a_near_jump_in_an_included_file_still_gets_its_suggestion() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let root = tmp.path().as_std_path();
        std::fs::create_dir_all(root.join(".git")).unwrap();

        let body = String::from("start\n    jp target\n    nop\ntarget\n    ret\n");
        std::fs::write(root.join("code.asm"), &body).unwrap();
        std::fs::write(
            root.join("main.asm"),
            "    org 0x4000\n    run start\n    include \"code.asm\"\n"
        )
        .unwrap();

        let code = std::fs::canonicalize(root.join("code.asm")).unwrap();
        let uri = Url::from_file_path(&code).unwrap();
        let d = Document::new(uri, body.clone(), 1);

        let analyzer = AssemblyAnalyzer::new();
        let listing = analyzer.parse_document(&d).expect("must parse");
        let (env, own_complete) = analyzer.dry_run_env_cached_checked(&d, &listing);
        let addresses = analyzer.peephole_addresses(&d, own_complete);
        assert!(matches!(addresses, AddressSource::Project(_)));

        let (_tokens, matches) =
            peephole_matches(&listing, &env, addresses.as_addresses(), OptimizationGoal::Size);
        assert!(
            matches.iter().any(|m| m.rule_name.as_deref() == Some("jp2jr")),
            "a genuinely near jump must still be offered: {matches:?}"
        );
    }

    /// Unsaved edits shift every offset the project assemble recorded, so the
    /// project route must not be used for a dirty buffer.
    #[test]
    fn an_unsaved_buffer_falls_back_rather_than_using_stale_offsets() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let root = tmp.path().as_std_path();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join("code.asm"), "start\n    jp target\ntarget\n    ret\n").unwrap();
        std::fs::write(
            root.join("main.asm"),
            "    org 0x4000\n    run start\n    include \"code.asm\"\n"
        )
        .unwrap();

        let code = std::fs::canonicalize(root.join("code.asm")).unwrap();
        let uri = Url::from_file_path(&code).unwrap();
        // The buffer has an extra line the file on disk does not.
        let d = Document::new(
            uri,
            "start\n    nop\n    jp target\ntarget\n    ret\n".to_string(),
            2
        );

        let analyzer = AssemblyAnalyzer::new();
        let listing = analyzer.parse_document(&d).expect("must parse");
        let (_env, own_complete) = analyzer.dry_run_env_cached_checked(&d, &listing);
        assert!(
            !matches!(
                analyzer.peephole_addresses(&d, own_complete),
                AddressSource::Project(_)
            ),
            "a dirty buffer must not be measured against the assembled file"
        );
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

#[cfg(test)]
mod skipped_notice_tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///main.asm").unwrap(), text.to_string(), 1)
    }

    /// Going quiet is indistinguishable from "nothing to suggest", so when the
    /// addresses are unavailable the user is told - otherwise a `jp` that was
    /// never actually examined looks like a `jp` that was examined and passed.
    #[test]
    fn a_file_whose_addresses_are_unavailable_says_so() {
        let d = doc("start\n    jp target\ntarget\n    ret\n");
        let analyzer = AssemblyAnalyzer::new();
        let listing = analyzer.parse_document(&d).expect("parses");
        let (env, _) = analyzer.dry_run_env_cached_checked(&d, &listing);

        let mut out = Vec::new();
        collect_peephole_warnings(
            &listing,
            &env,
            OptimizationGoal::Neutral,
            Addresses::None,
            &d.uri,
            &mut out
        );
        let notice = out
            .iter()
            .find(|d| d.severity == Some(DiagnosticSeverity::INFORMATION))
            .expect("expected the skipped-analysis notice");
        assert!(notice.message.contains("addresses"), "{}", notice.message);
        assert!(notice.message.contains("entry"), "{}", notice.message);
    }

    /// ...but only when there is something it could have said. A file with no
    /// unconditional jump loses nothing by the analysis being skipped, so the
    /// notice would be pure noise.
    #[test]
    fn a_file_with_no_jump_gets_no_notice() {
        let d = doc("start\n    nop\n    ret\n");
        let analyzer = AssemblyAnalyzer::new();
        let listing = analyzer.parse_document(&d).expect("parses");
        let (env, _) = analyzer.dry_run_env_cached_checked(&d, &listing);

        let mut out = Vec::new();
        collect_peephole_warnings(
            &listing,
            &env,
            OptimizationGoal::Neutral,
            Addresses::None,
            &d.uri,
            &mut out
        );
        assert!(
            !out.iter()
                .any(|d| d.severity == Some(DiagnosticSeverity::INFORMATION)),
            "{out:?}"
        );
    }

    /// And nothing is said when the analysis did run.
    #[test]
    fn a_file_with_real_addresses_gets_no_notice() {
        let d = doc("start\n    jp target\ntarget\n    ret\n");
        let analyzer = AssemblyAnalyzer::new();
        let listing = analyzer.parse_document(&d).expect("parses");
        let (env, _) = analyzer.dry_run_env_cached_checked(&d, &listing);

        let mut out = Vec::new();
        collect_peephole_warnings(
            &listing,
            &env,
            OptimizationGoal::Neutral,
            Addresses::OwnAssemble,
            &d.uri,
            &mut out
        );
        assert!(
            !out.iter()
                .any(|d| d.severity == Some(DiagnosticSeverity::INFORMATION)),
            "{out:?}"
        );
    }
}
