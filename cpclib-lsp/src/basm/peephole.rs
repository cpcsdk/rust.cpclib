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

use std::sync::Arc;

use cpclib_asm::assembler::Env;
use cpclib_asm::flatten::flatten_for_analysis;
use cpclib_asm::parser::obtained::{LocatedListing, LocatedToken, MayHaveSpan};
use cpclib_asm::parser::source::{SourceString, Z80Span};
use cpclib_asmoptim::engine::{PeepholeMatch, find_matches, find_matches_with_resolver};
use cpclib_asmoptim::{
    EnvAddressResolver, OptimizationGoal, ProjectAddressResolver, builtin_rules
};
use cpclib_project::entry;
use cpclib_tokens::{DataAccessElem, ListingElement};
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use super::command::{single_file_edit, single_file_multi_edit};
use crate::common::document::Document;

/// Diagnostic `source` tag for every peephole warning, distinct from plain
/// `"basm"` (used for real parser/assembler diagnostics) so a user can tell
/// at a glance which findings are advisory-only.
/// `Diagnostic::source` for everything this module reports, so the on-demand
/// commands can count their own findings among a document's diagnostics.
pub(crate) const DIAGNOSTIC_SOURCE: &str = "basm-peephole";
const SOURCE: &str = DIAGNOSTIC_SOURCE;

/// The command name a "Fix All" CodeLens (see [`AssemblyAnalyzer::peephole_code_lenses`])
/// invokes - handled in `server/backend.rs`'s `execute_command`. `pub(crate)`,
/// not `pub(super)`: `server/backend.rs` is a sibling of `basm`, not a
/// descendant, so it needs crate-wide visibility to reference this same
/// constant rather than duplicating the literal string.
pub(crate) const FIX_ALL_COMMAND: &str = "cpclib.fixAllPeephole";

/// Ask for peephole analysis on demand: `[uri]`, or `[uri, range]` to narrow
/// the report to a selection.
///
/// One file per call, on purpose. Scanning a whole project used to be a single
/// server-side command, which meant the user got no progress, no way to stop
/// it, and no say in how many files it was about to assemble - on a workspace
/// holding several demos that is an hour of silence. The client drives the
/// loop instead, so it owns the count, the progress bar and the cancel button.
///
/// The automatic pass is off by default (see
/// `AsmWarningClasses::peephole_optimizer`), so this is how a user gets an
/// answer without turning it on for everything. Handled in `server/backend.rs`.
pub(crate) const ANALYZE_COMMAND: &str = "cpclib.analyzePeephole";

/// Undo an [`ANALYZE_COMMAND`]: `[uri]` to stop reporting for one document,
/// or no arguments at all to stop everywhere.
pub(crate) const CLEAR_COMMAND: &str = "cpclib.clearPeephole";

/// Flatten `listing` and match it against `goal`'s built-in rules, using
/// `env` (see [`AssemblyAnalyzer::peephole_quickfix_action`]'s own doc
/// comment on why it must be the same parse `env` was assembled from) to
/// evaluate address-aware rules like `jp2jr`. Shared by every entry point in
/// this file so they can never disagree with each other about what matches.
fn peephole_matches<'a>(
    listing: &'a LocatedListing,
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
        Addresses::OwnAssemble(env) => {
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

/// Do two ranges share at least one position?
///
/// Used to narrow an explicitly-requested analysis to a selection. A match
/// straddling the edge of the selection counts as inside it: the user pointed
/// at part of it, and reporting half an instruction sequence would be worse
/// than reporting one they did not fully select.
pub(super) fn overlaps(a: &Range, b: &Range) -> bool {
    a.start <= b.end && b.start <= a.end
}

/// [`AssemblyAnalyzer::peephole_inputs`], reachable from sibling modules.
pub(super) fn address_source(
    analyzer: &AssemblyAnalyzer,
    document: &Document,
    listing: &LocatedListing
) -> (Arc<AddressSource>, Option<Env>) {
    analyzer.peephole_inputs(document, listing)
}

/// Owned form of [`Addresses`], so a caller can hold the project assemble
/// alive across the call.
pub(super) enum AddressSource {
    OwnAssemble,
    Project(entry::ProjectAddresses),
    None
}

impl AddressSource {
    /// Borrow this as an [`Addresses`], supplying the document's own `Env`.
    ///
    /// `own_env` is only read for [`AddressSource::OwnAssemble`], and
    /// [`AssemblyAnalyzer::peephole_inputs`] is what produces the pair: it
    /// computes the own assemble exactly when this variant turns out to be
    /// the answer. `None` here degrades to [`Addresses::None`] rather than
    /// pretending - an address-aware rule with no addresses must stay quiet.
    pub(super) fn as_addresses<'a>(&'a self, own_env: Option<&'a Env>) -> Addresses<'a> {
        match self {
            Self::OwnAssemble => {
                match own_env {
                    Some(env) => Addresses::OwnAssemble(env),
                    None => Addresses::None
                }
            },
            Self::Project(p) => Addresses::Project(p),
            Self::None => Addresses::None
        }
    }
}

impl AssemblyAnalyzer {
    /// The assembled project `Env` for `entry`, cached.
    ///
    /// See `cpclib_project::cache::ProjectCache` for why the key is a
    /// fingerprint rather than a timestamp or a hash of the sources.
    fn project_env_cached(
        &self,
        entry: &std::path::Path,
        fingerprint: u128,
        config: &crate::common::config::AsmConfig
    ) -> Option<std::sync::Arc<Env>> {
        // Timed unconditionally, same reasoning as `workspace_fingerprint_of`:
        // `ProjectCache::env_for`'s own module doc comment says a real demo's
        // full assemble "takes tens of seconds" - true, but until now that
        // cost was invisible in the log, swallowed inside whichever request's
        // outer "took ..." line happened to trigger the cache miss (a
        // misleadingly generic `code_action`/CodeLens/diagnostics duration,
        // indistinguishable from every other kind of slowness). This line is
        // what lets "was that 40 real seconds of assembling, or 40 seconds
        // lost to CPU contention with something else on the machine" be
        // answered directly from the log instead of guessed at - a cache hit
        // logs near-instantly, so a slow line here specifically means a real,
        // uncached project assemble ran.
        let start = std::time::Instant::now();
        let result = self.projects.env_for(entry, fingerprint, config);
        tracing::debug!(
            "project_env_cached for {} took {:?}",
            entry.display(),
            start.elapsed()
        );
        result
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
    /// The project's include graph, rebuilt only when the project changed.
    fn project_graph_cached(&self, root: &std::path::Path) -> (u128, Arc<entry::ProjectGraph>) {
        self.projects.graph_for(root)
    }

    /// Everything the matcher needs for `document`: where its real addresses
    /// come from, and - only when that turns out to be this file's own
    /// assemble - the `Env` that assemble produced.
    ///
    /// The own assemble is deferred rather than done up front because a file
    /// belonging to a project never needs it: its addresses come from the
    /// project's `Env`. Skipping it is what makes scanning a whole project
    /// affordable, since assembling every file in one separately is most of
    /// the cost.
    pub(super) fn peephole_inputs(
        &self,
        document: &Document,
        listing: &LocatedListing
    ) -> (Arc<AddressSource>, Option<Env>) {
        let mut own: Option<Env> = None;
        let source = self.peephole_addresses(document, || {
            let (env, complete) = self.dry_run_env_cached_checked(document, listing);
            own = Some(env);
            complete
        });
        // A cache hit answers without ever running the closure, so the `Env`
        // still has to be fetched when the answer turns out to need one.
        if matches!(&*source, AddressSource::OwnAssemble) && own.is_none() {
            own = Some(self.dry_run_env_cached_checked(document, listing).0);
        }
        (source, own)
    }

    /// [`Self::resolve_peephole_addresses`], memoised per
    /// `(document version, workspace fingerprint)`.
    ///
    /// Four entry points ask this during a single editor interaction -
    /// diagnostics, the quickfix, the CodeLens and Fix All - and resolving it
    /// means walking the project and reading every source in it. On a hit, all
    /// that runs is [`entry::fingerprint_of`]: one `stat` per candidate file,
    /// no reads.
    ///
    /// The fingerprint has to be in the key, not just the version: an edit to
    /// an *included* file changes this document's addresses without touching
    /// its version.
    fn peephole_addresses(
        &self,
        document: &Document,
        own_assemble_complete: impl FnOnce() -> bool
    ) -> Arc<AddressSource> {
        let key = (document.version, super::workspace_fingerprint_of(&document.uri));

        if let Some(cached) = self.address_source_cache.get(&document.uri)
            && cached.0 == key
        {
            return cached.1.clone();
        }
        let resolved = Arc::new(self.resolve_peephole_addresses(document, own_assemble_complete));
        self.address_source_cache
            .insert(document.uri.clone(), (key, resolved.clone()));
        resolved
    }

    fn resolve_peephole_addresses(
        &self,
        document: &Document,
        own_assemble_complete: impl FnOnce() -> bool
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
        let matches_disk = fs_err::read_to_string(&path).is_ok_and(|disk| disk == buffer);
        if !matches_disk {
            return if own_assemble_complete() {
                AddressSource::OwnAssemble
            }
            else {
                AddressSource::None
            };
        }

        let config = self.config();
        let Some(document_path) = document.uri.to_file_path().ok()
        else {
            return AddressSource::None;
        };
        let Some(root) = entry::root_of(&document_path)
        else {
            return AddressSource::None;
        };
        // Shared across every document in the project: reading and parsing
        // the sources happens once per change, not once per document.
        let (fingerprint, graph) = self.project_graph_cached(&root);
        let configured_entry = (!config.entry.is_empty()).then(|| config.entry.as_str());
        match entry::entry_in_graph(&document_path, configured_entry, &root, &graph) {
            entry::Entry::Standalone => {
                if own_assemble_complete() {
                    AddressSource::OwnAssemble
                }
                else {
                    AddressSource::None
                }
            },
            entry::Entry::Project(entry) => {
                let Some(env) = self.project_env_cached(&entry, fingerprint, &config)
                else {
                    return AddressSource::None;
                };
                match fs_err::canonicalize(&path) {
                    Ok(document) => {
                        AddressSource::Project(entry::ProjectAddresses { env, document })
                    },
                    Err(_) => AddressSource::None
                }
            },
            entry::Entry::Unknown => AddressSource::None
        }
    }
}

/// Where a document's real addresses come from, if anywhere.
pub(super) enum Addresses<'a> {
    /// This document *is* the program - its own assemble is the real one, and
    /// the `Env` it produced is carried here.
    ///
    /// The `Env` lives in this variant rather than beside it because the
    /// project case has no use for it at all: carrying it separately meant
    /// every caller paid for this file's own assemble before finding out it
    /// would be ignored, which is most of what a whole-project scan spent its
    /// time on.
    OwnAssemble(&'a Env),
    /// This document is only part of a program; addresses come from assembling
    /// the entry that contains it.
    Project(&'a entry::ProjectAddresses),
    /// Nothing trustworthy. Address-aware rules must stay quiet: an
    /// incomplete assemble, an ambiguous entry, or a buffer that no longer
    /// matches disk (which shifts every recorded offset).
    None
}

/// Match `listing` against the built-in peephole rules and push one
/// `WARNING` `Diagnostic` per finding into `out`.
///
/// The `Env` inside `addresses` (when there is one) must come from the same
/// real (dry-run) assemble already computed for this document version -
/// `dry_run_env_cached`'s `Env` has `record_token_addresses` enabled
/// unconditionally (see that function's own doc comment), which is what lets
/// address-aware rules like `jp2jr` (`reachableByJr`) actually fire instead of
/// silently reporting unknown.
pub(super) fn collect_peephole_warnings(
    listing: &LocatedListing,
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

    let (tokens, matches) = peephole_matches(listing, addresses, goal);
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
        let (addresses, own_env) = self.peephole_inputs(document, &listing);
        let goal = self.config().peephole_goal.into();
        let (tokens, matches) =
            peephole_matches(&listing, addresses.as_addresses(own_env.as_ref()), goal);

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
            edit: Some(single_file_edit(
                document.uri.clone(),
                edit.range,
                edit.new_text
            )),
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
        // Gated like the diagnostic, and for the same reason: a `codeLens`
        // request re-derives every match, so it costs exactly what the
        // diagnostic costs. Both are things the editor asks for on its own
        // schedule rather than things the user asked for - so both wait for
        // the warning class, or for an explicit `cpclib.analyzePeephole`.
        if !self.peephole_wanted(&document.uri) {
            return Vec::new();
        }
        let Ok(listing) = self.parse_document(document)
        else {
            return Vec::new();
        };
        let (addresses, own_env) = self.peephole_inputs(document, &listing);
        let goal = self.config().peephole_goal.into();
        let (_tokens, matches) =
            peephole_matches(&listing, addresses.as_addresses(own_env.as_ref()), goal);
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

    /// Peephole diagnostics for `document`, and nothing else.
    ///
    /// What a whole-project scan publishes for files the editor does not have
    /// open. The saving over [`AssemblyAnalyzer::analyze`] is not the skipped
    /// warnings - it is that a file belonging to a project never gets
    /// assembled on its own here, because [`Self::peephole_inputs`] only
    /// reaches for that when the file *is* the program. `analyze` must do it
    /// regardless, to report assembler warnings.
    pub(crate) fn peephole_scan(&self, document: &Document) -> Vec<Diagnostic> {
        let Ok(listing) = self.parse_document(document)
        else {
            return Vec::new();
        };
        let (addresses, own_env) = self.peephole_inputs(document, &listing);

        let mut out = Vec::new();
        collect_peephole_warnings(
            &listing,
            self.config().peephole_goal.into(),
            addresses.as_addresses(own_env.as_ref()),
            &document.uri,
            &mut out
        );
        if let Some(scope) = self.peephole_scope(&document.uri) {
            out.retain(|d| overlaps(&d.range, &scope));
        }
        out
    }

    /// Build one `WorkspaceEdit` applying every peephole match in
    /// `document` at once (the [`FIX_ALL_COMMAND`] handler's job), alongside
    /// how many matches it covers (for the confirmation message). `None`
    /// when there is nothing to fix. `TextEdit`s within one document are
    /// resolved against the *original* text by the client
    /// (`single_file_multi_edit`'s own doc comment), so this doesn't need to
    /// worry about one match's edit shifting another's offsets - matches
    /// never overlap in the first place (`find_matches`'s own guarantee).
    pub(crate) fn fix_all_peephole_edit(
        &self,
        document: &Document
    ) -> Option<(WorkspaceEdit, usize)> {
        let listing = self.parse_document(document).ok()?;
        let (addresses, own_env) = self.peephole_inputs(document, &listing);
        let goal = self.config().peephole_goal.into();
        let (tokens, matches) =
            peephole_matches(&listing, addresses.as_addresses(own_env.as_ref()), goal);
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
        character: crate::common::document::byte_offset_to_utf16_col(line_text, offset - line_start)
            as u32
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
                start: Position {
                    line: 1,
                    character: 0
                },
                end: Position {
                    line: 2,
                    character: 0
                }
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
            let before: usize = lines.iter().take(p.line as usize).map(|l| l.len()).sum();
            before + p.character as usize
        };
        let mut out = text.clone();
        out.replace_range(
            offset(edit.range.start)..offset(edit.range.end),
            &edit.new_text
        );
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

        let (_tokens, matches) =
            peephole_matches(&listing, Addresses::None, OptimizationGoal::Neutral);
        assert!(
            !matches
                .iter()
                .any(|m| m.rule_name.as_deref() == Some("jp2jr")),
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
            Addresses::OwnAssemble(&env),
            OptimizationGoal::Neutral
        );
        assert!(
            matches
                .iter()
                .any(|m| m.rule_name.as_deref() == Some("jp2jr")),
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
        let (addresses, own_env) = analyzer.peephole_inputs(&d, &listing);

        assert!(
            matches!(*addresses, AddressSource::Project(_)),
            "an included file must be measured through its entry"
        );

        let (_tokens, matches) = peephole_matches(
            &listing,
            addresses.as_addresses(own_env.as_ref()),
            OptimizationGoal::Size
        );
        assert!(
            !matches
                .iter()
                .any(|m| m.rule_name.as_deref() == Some("jp2jr")),
            "the target is >127 bytes away in the real program, so jp2jr must \
             not fire: {matches:?}"
        );
    }

    /// Resolving where the addresses come from is the expensive step - it
    /// walks the project and reads every source in it - and four entry points
    /// ask for it during one editor interaction. So it must be computed once
    /// and shared, and it must stop being shared the moment the project could
    /// have moved underneath it.
    #[test]
    fn the_resolved_address_source_is_reused_until_the_project_changes() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let root = tmp.path().as_std_path();
        std::fs::create_dir_all(root.join(".git")).unwrap();

        let body = String::from("start\n    jp target\n    nop\ntarget\n    ret\n");
        let main = root.join("main.asm");
        std::fs::write(root.join("code.asm"), &body).unwrap();
        std::fs::write(
            &main,
            "    org 0x4000\n    run start\n    include \"code.asm\"\n"
        )
        .unwrap();

        let code = std::fs::canonicalize(root.join("code.asm")).unwrap();
        let uri = Url::from_file_path(&code).unwrap();
        let d = Document::new(uri.clone(), body.clone(), 1);
        let analyzer = AssemblyAnalyzer::new();

        let first = analyzer.peephole_addresses(&d, || false);
        let second = analyzer.peephole_addresses(&d, || false);
        assert!(
            Arc::ptr_eq(&first, &second),
            "the second ask must be served from the cache, not walked again"
        );

        // Touching an *included* file - not this document - has to invalidate
        // it. The document's version is unchanged, so a version-only key would
        // wrongly keep serving the old answer.
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(
            &main,
            "    org 0x5000\n    run start\n    include \"code.asm\"\n"
        )
        .unwrap();
        // `cpclib_project::entry::fingerprint_of` now memoizes its own
        // result for a short while (`FINGERPRINT_CACHE_TTL`, currently
        // 300ms), so a lookup immediately after the write above could still
        // observe the pre-change fingerprint from the `second` call just
        // above. Wait past it so this test's own change is what gets
        // observed, not a still-warm memo.
        std::thread::sleep(std::time::Duration::from_millis(350));
        let third = analyzer.peephole_addresses(&d, || false);
        assert!(
            !Arc::ptr_eq(&first, &third),
            "a changed include must invalidate the cached answer"
        );

        // And closing the document must not leave it behind.
        let fourth = analyzer.peephole_addresses(&d, || false);
        assert!(Arc::ptr_eq(&third, &fourth));
        analyzer.evict(&uri);
        let after_close = analyzer.peephole_addresses(&d, || false);
        assert!(
            !Arc::ptr_eq(&fourth, &after_close),
            "evict() must drop the cached address source"
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
        let (addresses, own_env) = analyzer.peephole_inputs(&d, &listing);
        assert!(matches!(*addresses, AddressSource::Project(_)));

        let (_tokens, matches) = peephole_matches(
            &listing,
            addresses.as_addresses(own_env.as_ref()),
            OptimizationGoal::Size
        );
        assert!(
            matches
                .iter()
                .any(|m| m.rule_name.as_deref() == Some("jp2jr")),
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
        std::fs::write(
            root.join("code.asm"),
            "start\n    jp target\ntarget\n    ret\n"
        )
        .unwrap();
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
                *analyzer.peephole_addresses(&d, || own_complete),
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
        assert!(actions.iter().any(|a| {
            a.title == "Peephole: Remove ld b,b" && a.kind == Some(CodeActionKind::QUICKFIX)
        }));
    }

    #[test]
    fn no_quickfix_when_the_cursor_is_not_on_a_matched_line() {
        let d = doc("start:\n    ld b, b\n    ret\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .peephole_quickfix_action(&d, cursor(2, 4))
                .is_none()
        );
    }

    #[test]
    fn no_quickfix_for_already_optimal_source() {
        let d = doc("start:\n    xor a\n    ret\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .peephole_quickfix_action(&d, cursor(1, 4))
                .is_none()
        );
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
        // The lens costs what the diagnostic costs, so it is gated the same
        // way and has to be asked for.
        let mut config = crate::common::config::AsmConfig::default();
        config.warnings.peephole_optimizer = true;
        analyzer.set_config(config);
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
        assert!(
            text_edits.iter().any(|e| e.new_text.is_empty()),
            "{text_edits:?}"
        );
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
        let mut out = Vec::new();
        collect_peephole_warnings(
            &listing,
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
        let mut out = Vec::new();
        collect_peephole_warnings(
            &listing,
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
            OptimizationGoal::Neutral,
            Addresses::OwnAssemble(&env),
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
