//! Cross-file call-site discovery and edit-span math for removing an unused
//! MACRO/FUNCTION parameter (`cpclib.removeUnusedParameter`). *Detection* of
//! which parameter is unused lives in `cpclib_asm::unused_bindings` and is
//! reused here, never reimplemented - this module is purely about finding
//! every *call site* of that macro/function and computing the edits needed
//! to remove one positional argument from each.
//!
//! This is a materially bigger problem than the earlier warning-only
//! feature: basm strictly enforces call arity (`MacroWithArgs::build`,
//! `AnyFunction::eval` both reject a mismatch outright), so a missed call
//! site would leave broken, non-assembling code behind, not just a stale
//! warning. Every function here either produces a real, safe edit or a
//! `RemovalBlocker` explaining why it couldn't - never a silent guess.
//!
//! `AssemblyAnalyzer::resolve_remove_parameter_target`/
//! `scan_document_for_parameter_removal` are the entry points
//! `server/backend.rs` actually calls; everything else is a pure,
//! `Document`-free core, directly unit-testable.

use std::ops::Range;

use cpclib_asm::ExprElement;
use cpclib_asm::parser::obtained::{
    LocatedDataAccess, LocatedExpr, LocatedListing, LocatedMacroParam, LocatedToken, MayHaveSpan
};
use cpclib_tokens::{ListingElement, TestKindElement};
use tower_lsp::lsp_types::{Position, TextEdit};

use super::AssemblyAnalyzer;
use super::format::strip_asm_comment;
use crate::common::document::{Document, byte_offset_to_utf16_col};

/// MACRO or FUNCTION - the only two kinds this refactor supports.
/// REPEAT/ITERATE/FOR counters already got their own, much simpler,
/// single-file quickfix (`refactor.rs::unused_repeat_counter_removal_action`),
/// since they aren't called by name elsewhere, so there's no call site to
/// rewrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoveParameterKind {
    Macro,
    Function
}

/// What to remove: `owner_name`'s declared parameter at `param_index`
/// (0-based). `param_name` is carried for messages/titles only - the index
/// is what's actually load-bearing, since basm's call arguments are always
/// positional (never named).
#[derive(Debug, Clone)]
pub(crate) struct RemoveParameterTarget {
    pub kind: RemoveParameterKind,
    pub owner_name: String,
    pub param_index: usize,
    pub param_name: String
}

/// Why the whole operation (a workspace-wide reason, `uri: None`) or one
/// call site within an otherwise-fine file couldn't be safely handled.
/// `line` is 1-based, matching `Z80Span::relative_line_and_column()`'s
/// convention used throughout `cpclib_asm::unused_bindings`. `uri` is filled
/// in by the caller once it knows which file this blocker came from - every
/// function in this module produces `uri: None`, since they work one
/// already-loaded file at a time.
#[derive(Debug, Clone)]
pub(crate) struct RemovalBlocker {
    pub uri: Option<tower_lsp::lsp_types::Url>,
    pub line: Option<usize>,
    pub reason: String
}

impl RemovalBlocker {
    fn here(line: usize, reason: impl Into<String>) -> Self {
        Self {
            uri: None,
            line: Some(line),
            reason: reason.into()
        }
    }
}

/// One already-parsed file's full contribution to a removal: every
/// same-named MACRO/FUNCTION/STRUCT definition found (deliberately
/// *inclusive* of the target's own definition - the caller filters that one
/// out when deciding whether the name is ambiguous) plus every call site's
/// outcome.
#[derive(Debug, Clone, Default)]
pub(crate) struct FileScan {
    /// One entry per same-named MACRO/FUNCTION/STRUCT definition found in
    /// this file - the target's own definition included. More than one
    /// entry anywhere across every scanned file means the name is
    /// ambiguous; the caller aborts the whole operation rather than risk
    /// rewriting a call that actually resolves to a different definition.
    pub matching_definitions: Vec<(usize, usize)>,
    pub edits: Vec<TextEdit>,
    pub blockers: Vec<RemovalBlocker>
}

/// Scan one already-parsed file for every call site of `target`, producing
/// the edits needed to remove its argument at `target.param_index` - or a
/// `RemovalBlocker` per call site (or for the whole file) that couldn't be
/// safely handled. Pure: no I/O, no `&self` - `text` is the *whole
/// document's own text* (the same one `listing` was parsed from), since the
/// span math below works in absolute byte offsets throughout, only
/// converting to an LSP `Position` at the very end.
pub(crate) fn scan_listing_for_parameter_removal(
    listing: &LocatedListing,
    text: &str,
    target: &RemoveParameterTarget
) -> FileScan {
    let mut scan = FileScan::default();

    for token in super::token::flatten_listing(listing.iter()) {
        let is_match = match target.kind {
            RemoveParameterKind::Macro => {
                token.is_macro_definition() && token.macro_definition_name() == target.owner_name
            },
            RemoveParameterKind::Function => {
                token.is_function_definition()
                    && token.function_definition_name() == target.owner_name
            },
        };
        if !is_match {
            continue;
        }
        let (line, column) = token.span().relative_line_and_column();
        scan.matching_definitions.push((line, column));
        record_definition_header_edit(token, target, text, line, &mut scan);
    }
    if target.kind == RemoveParameterKind::Macro
        && text_contains_struct_definition_named(text, &target.owner_name)
    {
        // Exact location doesn't matter here - only that a same-named
        // STRUCT exists at all, which alone makes the name ambiguous.
        scan.matching_definitions.push((0, 0));
    }

    match target.kind {
        RemoveParameterKind::Macro => macro_call_site_results(listing, text, target, &mut scan),
        RemoveParameterKind::Function => {
            function_call_site_results(listing, text, target, &mut scan)
        },
    }

    scan
}

// ─── MACRO call sites ───────────────────────────────────────────────────────

fn macro_call_site_results(
    listing: &LocatedListing,
    text: &str,
    target: &RemoveParameterTarget,
    scan: &mut FileScan
) {
    for call in super::token::flatten_listing(listing.iter())
        .filter(|t| t.is_call_macro_or_build_struct() && t.macro_call_name() == target.owner_name)
    {
        let args = call.macro_call_arguments();
        let arity = args.len();
        let (line, _column) = call.span().relative_line_and_column();

        if target.param_index >= arity {
            scan.blockers.push(RemovalBlocker::here(
                line,
                format!(
                    "this call to '{}' passes only {arity} argument(s), but the parameter \
                     being removed is at position {}",
                    target.owner_name,
                    target.param_index + 1
                )
            ));
            continue;
        }

        let Some(arg_span) = macro_arg_span(&args[target.param_index])
        else {
            scan.blockers.push(RemovalBlocker::here(
                line,
                format!(
                    "the argument at this call to '{}' is a bracketed list or an empty \
                     placeholder, which can't be safely rewritten automatically",
                    target.owner_name
                )
            ));
            continue;
        };

        match expand_to_removable_argument_span(text, arg_span, target.param_index, arity) {
            Some(removable) => scan.edits.push(byte_range_to_text_edit(text, removable)),
            None => {
                scan.blockers.push(RemovalBlocker::here(
                    line,
                    format!(
                        "couldn't locate the expected comma around argument {} of this call \
                         to '{}'",
                        target.param_index + 1,
                        target.owner_name
                    )
                ));
            }
        }
    }
}

/// The target argument's raw `[start, end)` absolute byte range, or `None`
/// for a `List`/`Empty`-shaped argument - `LocatedMacroParam::span()`
/// panics for both (`List(_) => todo!()`, `Empty => panic!()`, verified
/// directly against the parser), so this never calls it without matching on
/// the variant first.
fn macro_arg_span(param: &LocatedMacroParam) -> Option<Range<usize>> {
    match param {
        LocatedMacroParam::RawArgument(span) => {
            let start = span.offset_from_start();
            let text: &str = span.as_ref();
            Some(start..start + text.len())
        },
        LocatedMacroParam::EvaluatedArgument(span) => {
            // The span excludes its own "{eval}" prefix (verified directly
            // against `parse_macro_arg`'s grammar: the `opt(Caseless("{eval}")...)`
            // sits outside the `.take()`'d alternatives that become the
            // span) - back the start up so removal also deletes the
            // prefix, not just the value it wraps.
            const EVAL_PREFIX: &str = "{eval}";
            let value_start = span.offset_from_start();
            let start = value_start.saturating_sub(EVAL_PREFIX.len());
            let text: &str = span.as_ref();
            Some(start..value_start + text.len())
        },
        LocatedMacroParam::List(_) | LocatedMacroParam::Empty => None
    }
}

/// Whether `text` contains a `STRUCT <name>` definition header anywhere - a
/// lightweight whole-line textual scan, since `ListingElement` has no
/// accessor for STRUCT definitions at all (only for macro/struct *call*
/// sites), mirroring `token::starts_with_range_keyword`'s established
/// textual-fallback shape for the same kind of gap.
fn text_contains_struct_definition_named(text: &str, name: &str) -> bool {
    for line in text.lines() {
        let stripped = strip_asm_comment(line).trim_start();
        let Some(rest) = stripped
            .strip_prefix("STRUCT")
            .or_else(|| stripped.strip_prefix("struct"))
        else {
            continue;
        };
        let candidate = rest.trim_start();
        let candidate_name: String = candidate
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if candidate_name == name {
            return true;
        }
    }
    false
}

/// Computes `token`'s own definition-header removal edit (via
/// `definition_header_removable_span`) and pushes it to `scan.edits`, or
/// pushes a `RemovalBlocker` if the expected parameter structure can't be
/// found. `token` must be a MACRO/FUNCTION definition matching
/// `target.owner_name` already confirmed by the caller - this only extracts
/// its own parameter list and header text.
fn record_definition_header_edit(
    token: &LocatedToken,
    target: &RemoveParameterTarget,
    text: &str,
    line: usize,
    scan: &mut FileScan
) {
    let all_params: Vec<&str> = match target.kind {
        RemoveParameterKind::Macro => token.macro_definition_arguments().to_vec(),
        RemoveParameterKind::Function => token.function_definition_params().to_vec()
    };

    let header_start = token.span().offset_from_start();
    let header_text: &str = token.span().as_ref();
    let header_line = header_text.lines().next().unwrap_or("");

    match definition_header_removable_span(header_line, &all_params, target.param_index) {
        Some(local_range) => {
            let absolute = (header_start + local_range.start)..(header_start + local_range.end);
            scan.edits.push(byte_range_to_text_edit(text, absolute));
        },
        None => {
            scan.blockers.push(RemovalBlocker::here(
                line,
                format!(
                    "couldn't safely locate parameter '{}' in this definition's own header to \
                     remove it",
                    target.param_name
                )
            ));
        }
    }
}

// ─── FUNCTION call sites ────────────────────────────────────────────────────

fn function_call_site_results(
    listing: &LocatedListing,
    text: &str,
    target: &RemoveParameterTarget,
    scan: &mut FileScan
) {
    let mut found_spans: Vec<Range<usize>> = Vec::new();
    let mut definition_lines: Range<usize> = 0..0;

    for statement in super::token::flatten_listing(listing.iter()) {
        if statement.is_function_definition()
            && statement.function_definition_name() == target.owner_name
        {
            let (line, _col) = statement.span().relative_line_and_column();
            let body_text: &str = statement.span().as_ref();
            let body_lines = body_text.lines().count().max(1);
            let start_line0 = line - 1;
            definition_lines = start_line0..(start_line0 + body_lines);
        }

        for call in ast_reachable_function_calls(statement) {
            let call_span = call.span();
            let call_start = call_span.offset_from_start();
            let call_text: &str = call_span.as_ref();
            found_spans.push(call_start..call_start + call_text.len());

            if call.function_name() != target.owner_name {
                continue;
            }
            let args = call.function_args();
            let arity = args.len();
            let (line, _column) = call_span.relative_line_and_column();

            if target.param_index >= arity {
                scan.blockers.push(RemovalBlocker::here(
                    line,
                    format!(
                        "this call to '{}' passes only {arity} argument(s), but the \
                         parameter being removed is at position {}",
                        target.owner_name,
                        target.param_index + 1
                    )
                ));
                continue;
            }

            let arg_span = function_arg_span(&args[target.param_index]);
            match expand_to_removable_argument_span(text, arg_span, target.param_index, arity) {
                Some(removable) => scan.edits.push(byte_range_to_text_edit(text, removable)),
                None => {
                    scan.blockers.push(RemovalBlocker::here(
                        line,
                        format!(
                            "couldn't locate the expected comma around argument {} of this \
                             call to '{}'",
                            target.param_index + 1,
                            target.owner_name
                        )
                    ));
                }
            }
        }
    }

    scan.blockers
        .extend(uncovered_textual_function_call_occurrences(
            text,
            &target.owner_name,
            definition_lines,
            &found_spans
        ));
}

/// Every `AnyFunction` node reachable from `statement`'s own expression
/// fields via already-existing `ListingElement`/`DataAccess` accessors -
/// NOT a claim of exhaustiveness (see this module's own doc comment on
/// `uncovered_textual_function_call_occurrences`, the safety net for
/// whatever this walk can't reach, e.g. `PRINT`/`ASSERT`, which have no
/// expression accessor today, or a warning-wrapped "fake instruction" like
/// `LD HL, DE` - see below). Does not descend into nested *listings*
/// (`flatten_listing`, called by this module's own callers, already does
/// that at the statement level).
fn ast_reachable_function_calls(statement: &LocatedToken) -> Vec<&LocatedExpr> {
    let mut out = Vec::new();

    // `LocatedToken::inner` is `Either<LocatedTokenInner, (Box<LocatedToken>,
    // String)>` - the `Right` case wraps a "fake instruction" (e.g. `LD HL,
    // DE`, a real, accepted basm shorthand) together with its own warning
    // message. Every `is_*()` predicate used below is safe for this shape
    // (`is_stuff_delegate!`'s macro-generated impls all fall back to `false`
    // for `Right`), but `mnemonic_arg1()`/`mnemonic_arg2()` are NOT -
    // they're generated by the separate `any_delegate!` macro, which
    // unconditionally `.unwrap()`s the `Left` variant and panics on `Right`
    // (confirmed directly: this exact panic reproduced against a real
    // project file containing `ld hl, de`). Bailing out here entirely for a
    // warning-wrapped token is consistent with this walk's own established
    // non-exhaustiveness - the textual tripwire is the safety net for
    // exactly this kind of gap.
    if statement.is_warning() {
        return out;
    }

    if statement.is_db() || statement.is_dw() || statement.is_str() {
        for e in statement.data_exprs() {
            walk_expr_for_function_calls(e, &mut out);
        }
    }
    if statement.is_equ() {
        walk_expr_for_function_calls(statement.equ_value(), &mut out);
    }
    // `is_set()` is a plain alias for `is_assign()` (`fn is_set(&self) -> bool
    // { self.is_assign() }`), NOT a distinct variant with its own value -
    // verified directly after this exact call panicked (`unreachable!()`
    // inside `equ_value()`) against a real project file using `x = expr`
    // -style compile-time variables. The pre-existing `symbols()` walker in
    // `cpclib-tokens` has the identical `is_equ() || is_set() =>
    // equ_value()` pattern and would panic the same way if ever called on
    // one of these - a latent bug this investigation surfaced, not
    // something specific to this new code.
    if statement.is_assign() {
        walk_expr_for_function_calls(statement.assign_value(), &mut out);
    }
    if statement.is_return() {
        walk_expr_for_function_calls(statement.return_value(), &mut out);
    }
    if statement.is_iterate() {
        match statement.iterate_values() {
            either::Either::Left(exprs) => {
                for e in exprs {
                    walk_expr_for_function_calls(e, &mut out);
                }
            },
            either::Either::Right(e) => walk_expr_for_function_calls(e, &mut out)
        }
    }
    if statement.is_org() {
        walk_expr_for_function_calls(statement.org_first(), &mut out);
        if let Some(e) = statement.org_second() {
            walk_expr_for_function_calls(e, &mut out);
        }
    }
    if statement.is_for() {
        walk_expr_for_function_calls(statement.for_start(), &mut out);
        walk_expr_for_function_calls(statement.for_stop(), &mut out);
        if let Some(e) = statement.for_step() {
            walk_expr_for_function_calls(e, &mut out);
        }
    }
    if statement.is_repeat() {
        walk_expr_for_function_calls(statement.repeat_count(), &mut out);
        if let Some(e) = statement.repeat_counter_start() {
            walk_expr_for_function_calls(e, &mut out);
        }
        if let Some(e) = statement.repeat_counter_step() {
            walk_expr_for_function_calls(e, &mut out);
        }
    }
    if statement.is_repeat_until() {
        walk_expr_for_function_calls(statement.repeat_until_condition(), &mut out);
    }
    if statement.is_while() {
        walk_expr_for_function_calls(statement.while_expr(), &mut out);
    }
    if statement.is_switch() {
        walk_expr_for_function_calls(statement.switch_expr(), &mut out);
        for (case_expr, _listing, _fallthrough) in statement.switch_cases() {
            walk_expr_for_function_calls(case_expr, &mut out);
        }
    }
    if statement.is_rorg() {
        walk_expr_for_function_calls(statement.rorg_expr(), &mut out);
    }
    if statement.is_incbin() {
        walk_expr_for_function_calls(statement.incbin_fname(), &mut out);
        if let Some(e) = statement.incbin_offset() {
            walk_expr_for_function_calls(e, &mut out);
        }
        if let Some(e) = statement.incbin_length() {
            walk_expr_for_function_calls(e, &mut out);
        }
    }
    if statement.is_include() {
        walk_expr_for_function_calls(statement.include_fname(), &mut out);
    }
    if statement.is_run() {
        walk_expr_for_function_calls(statement.run_expr(), &mut out);
    }
    if statement.is_if() {
        for i in 0..statement.if_nb_tests() {
            let (test, _listing) = statement.if_test(i);
            if test.is_true_test() || test.is_false_test() {
                walk_expr_for_function_calls(test.expr_unchecked(), &mut out);
            }
        }
    }
    if let Some(access) = statement.mnemonic_arg1()
        && let Some(e) = data_access_expr(access)
    {
        walk_expr_for_function_calls(e, &mut out);
    }
    if let Some(access) = statement.mnemonic_arg2()
        && let Some(e) = data_access_expr(access)
    {
        walk_expr_for_function_calls(e, &mut out);
    }

    out
}

/// The nested expression inside a `LocatedDataAccess`, for the 4 of its 11
/// variants that carry one (`IndexRegister16WithIndex`/`Expression`/
/// `Memory`/`PortN`) - an exhaustive, panic-free match; every other variant
/// is a bare register/flag/port reference with nothing to recurse into.
fn data_access_expr(access: &LocatedDataAccess) -> Option<&LocatedExpr> {
    match access {
        LocatedDataAccess::IndexRegister16WithIndex(_, _, expr, _) => Some(expr),
        LocatedDataAccess::Expression(expr) => Some(expr),
        LocatedDataAccess::Memory(expr) => Some(expr),
        LocatedDataAccess::PortN(expr, _) => Some(expr),
        _ => None
    }
}

/// Recurse through `expr`'s own nested sub-expressions, collecting every
/// `AnyFunction` node found (including `expr` itself). Exhaustive over
/// `LocatedExpr`'s variants except `UnaryTokenOperation`, which wraps a
/// nested `LocatedToken` (not a `LocatedExpr`) and is out of reach for this
/// expr-only walk - an acknowledged, accepted gap covered by the textual
/// completeness tripwire, not silently unhandled.
fn walk_expr_for_function_calls<'a>(expr: &'a LocatedExpr, out: &mut Vec<&'a LocatedExpr>) {
    if matches!(expr, LocatedExpr::AnyFunction(..)) {
        out.push(expr);
    }
    match expr {
        LocatedExpr::List(items, _) => {
            for item in items {
                walk_expr_for_function_calls(item, out);
            }
        },
        LocatedExpr::Paren(inner, _) => walk_expr_for_function_calls(inner, out),
        LocatedExpr::UnaryOperation(_, inner, _) => walk_expr_for_function_calls(inner, out),
        LocatedExpr::BinaryOperation(_, lhs, rhs, _) => {
            walk_expr_for_function_calls(lhs, out);
            walk_expr_for_function_calls(rhs, out);
        },
        LocatedExpr::Ternary(cond, t, f, _) => {
            walk_expr_for_function_calls(cond, out);
            walk_expr_for_function_calls(t, out);
            walk_expr_for_function_calls(f, out);
        },
        LocatedExpr::AnyFunction(_, args, _) => {
            for arg in args {
                walk_expr_for_function_calls(arg, out);
            }
        },
        _ => {}
    }
}

/// `LocatedExpr::span()` has no panic case for any variant (confirmed
/// directly - unlike `LocatedMacroParam::span()`) - always safe to call
/// unconditionally.
fn function_arg_span(arg: &LocatedExpr) -> Range<usize> {
    let span = arg.span();
    let start = span.offset_from_start();
    let text: &str = span.as_ref();
    start..start + text.len()
}

/// Safety net for `ast_reachable_function_calls`'s known incompleteness:
/// every word-boundary `owner_name(` occurrence in `text` (comments
/// stripped, the definition's own header lines excluded) whose byte
/// position isn't covered by any span in `found_call_spans` becomes a
/// `RemovalBlocker` pinpointing that line - preserving the all-or-nothing
/// safety property even where the AST walk is known-incomplete (e.g. inside
/// `PRINT`/`ASSERT`, which have no expression accessor today), rather than
/// silently missing it.
fn uncovered_textual_function_call_occurrences(
    text: &str,
    owner_name: &str,
    definition_lines: Range<usize>,
    found_call_spans: &[Range<usize>]
) -> Vec<RemovalBlocker> {
    let mut out = Vec::new();
    let needle = format!("{owner_name}(");
    let mut line_start = 0usize;

    for (line_idx, line) in text.split('\n').enumerate() {
        if !definition_lines.contains(&line_idx) {
            let stripped = strip_asm_comment(line);
            let mut search_from = 0usize;
            while let Some(rel) = stripped.get(search_from..).and_then(|s| s.find(&needle)) {
                let match_start = search_from + rel;
                let prev_is_ident = match_start > 0 && {
                    let prev = stripped.as_bytes()[match_start - 1];
                    prev.is_ascii_alphanumeric() || prev == b'_'
                };
                if !prev_is_ident {
                    let abs_offset = line_start + match_start;
                    let covered = found_call_spans.iter().any(|s| s.contains(&abs_offset));
                    if !covered {
                        out.push(RemovalBlocker::here(
                            line_idx + 1,
                            format!(
                                "found a possible call to '{owner_name}' here that couldn't be \
                                 confirmed via the parsed structure (e.g. inside PRINT/ASSERT) \
                                 - refusing to remove the parameter without being able to check \
                                 every call site"
                            )
                        ));
                    }
                }
                search_from = match_start + needle.len();
            }
        }
        line_start += line.len() + 1; // +1 for the '\n' this split consumed
    }

    out
}

// ─── Shared span/position math ─────────────────────────────────────────────

/// The removable byte range for deleting argument `index` (0-based) of
/// `arity` total, given that argument's own precise `[start, end)` raw text
/// range - expands outward to also swallow exactly one adjoining comma,
/// handling first/middle/last/only position uniformly. Scans outward from
/// `arg_span`'s own boundary only - never inspects a neighboring argument's
/// own span/shape, so this is safe even when a neighbor is `List`/
/// `Empty`-shaped. Refuses (`None`) rather than guesses if the expected
/// comma isn't actually there.
///
/// For the *last* of several arguments, the backward scan trims *all*
/// whitespace (including newlines) before checking for a comma - needed to
/// actually locate it across a multi-line call - so the removed range
/// naturally swallows any intervening blank lines/indentation too. For a
/// *first or middle* argument, only a run of horizontal whitespace
/// (spaces/tabs) right after the following comma is also removed, not
/// newlines - a deliberate, documented asymmetry: it keeps the common case
/// (`foo(1, 2, 3)`) clean without needing to guess how much of a
/// multi-line call's own indentation structure is safe to collapse. The one
/// residual, minor artifact: on a one-argument-per-line multi-line call,
/// removing a first/middle argument can leave a blank line behind -
/// cosmetic, not a correctness bug.
fn expand_to_removable_argument_span(
    text: &str,
    arg_span: Range<usize>,
    index: usize,
    arity: usize
) -> Option<Range<usize>> {
    if index + 1 < arity {
        // Not the last: consume the following comma + trailing horizontal
        // whitespace.
        let after = text.get(arg_span.end..)?;
        let after_trimmed = after.trim_start();
        if !after_trimmed.starts_with(',') {
            return None;
        }
        let ws_before_comma = after.len() - after_trimmed.len();
        let mut end = arg_span.end + ws_before_comma + 1; // past the comma itself
        let rest = &text[end..];
        let rest_after_ws = rest.trim_start_matches([' ', '\t']);
        end += rest.len() - rest_after_ws.len();
        Some(arg_span.start..end)
    }
    else {
        // The last of several, or the only one: consume a preceding comma
        // if one exists. A genuinely sole argument with nothing before it
        // (e.g. a call's own "foo(1)") has none - that's fine, just remove
        // the argument's own span then, unlike a definition header's
        // "name, x" shape, which always has a preceding comma to clean up.
        let before = text.get(..arg_span.start)?;
        let trimmed = before.trim_end();
        if trimmed.ends_with(',') {
            let comma_byte = trimmed.len() - 1;
            Some(comma_byte..arg_span.end)
        }
        else {
            Some(arg_span.start..arg_span.end)
        }
    }
}

/// The definition's own header edit: the `[start, end)` byte range on
/// `header_line` (relative to `header_line`'s own start, e.g. matching a
/// token's own `span().as_ref()`'s first line) to delete to remove
/// `all_param_names[param_index]` from a MACRO/FUNCTION header - both use
/// the same paren-less `name, a, b, c` style in basm (confirmed via this
/// module's own test fixtures - no `kind` parameter needed here, unlike the
/// call-site path, since there's no MACRO-vs-STRUCT header shape
/// difference to account for). Handles first/middle/last/only position the
/// same way `expand_to_removable_argument_span` does for call sites,
/// generalizing `unused_bindings::repeat_counter_clause_location`'s
/// same-line clause-removal shape from "0-or-1 optional clause" to "N
/// parameters, remove index i." Parameter names are matched with any
/// enclosing `(`/`)` trimmed first (a real, already-worked-around grammar
/// quirk for a single-parameter MACRO/FUNCTION - see
/// `token::macro_scoped_symbol_at`).
fn definition_header_removable_span(
    header_line: &str,
    all_param_names: &[&str],
    param_index: usize
) -> Option<Range<usize>> {
    let mut search_from = 0usize;
    let mut spans = Vec::with_capacity(all_param_names.len());
    for raw_name in all_param_names {
        let name = raw_name.trim_start_matches('(').trim_end_matches(')');
        let rel = header_line.get(search_from..)?.find(name)?;
        let start = search_from + rel;
        let end = start + name.len();
        spans.push(start..end);
        search_from = end;
    }
    let arg_span = spans.get(param_index)?.clone();
    let arity = spans.len();
    expand_to_removable_argument_span(header_line, arg_span, param_index, arity)
}

fn byte_offset_to_position(text: &str, offset: usize) -> Position {
    let up_to = &text[..offset.min(text.len())];
    let line = up_to.matches('\n').count() as u32;
    let line_start = up_to.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_text = text[line_start..].split('\n').next().unwrap_or("");
    let character = byte_offset_to_utf16_col(line_text, offset - line_start) as u32;
    Position { line, character }
}

fn byte_range_to_text_edit(text: &str, range: Range<usize>) -> TextEdit {
    TextEdit {
        range: tower_lsp::lsp_types::Range {
            start: byte_offset_to_position(text, range.start),
            end: byte_offset_to_position(text, range.end)
        },
        new_text: String::new()
    }
}

// ─── `AssemblyAnalyzer` entry points ────────────────────────────────────────

impl AssemblyAnalyzer {
    /// Re-parses `document` and confirms a matching, *still-unused*
    /// `UnusedBinding` exists for `(kind, owner_name, param_index)` - never
    /// trusts a triggering `CodeAction`'s possibly-stale snapshot, since the
    /// file may have changed since the action was offered (body edited to
    /// now reference the param, param already removed by an earlier run
    /// shifting indices, definition renamed/deleted).
    pub(crate) fn resolve_remove_parameter_target(
        &self,
        document: &Document,
        kind: RemoveParameterKind,
        owner_name: &str,
        param_index: usize
    ) -> Result<RemoveParameterTarget, Box<RemovalBlocker>> {
        let listing = self.parse_document(document).map_err(|_| {
            Box::new(RemovalBlocker {
                uri: Some(document.uri.clone()),
                line: None,
                reason: "this document no longer parses cleanly".to_string()
            })
        })?;

        let expected_kind = match kind {
            RemoveParameterKind::Macro => {
                cpclib_asm::unused_bindings::UnusedBindingKind::MacroParameter
            },
            RemoveParameterKind::Function => {
                cpclib_asm::unused_bindings::UnusedBindingKind::FunctionParameter
            },
        };

        cpclib_asm::unused_bindings::find_unused_bindings(listing.iter())
            .into_iter()
            .find_map(|binding| {
                let owner = binding.owner.as_ref()?;
                if binding.kind == expected_kind
                    && owner.name == owner_name
                    && owner.param_index == param_index
                {
                    Some(RemoveParameterTarget {
                        kind,
                        owner_name: owner_name.to_string(),
                        param_index,
                        param_name: binding.name
                    })
                }
                else {
                    None
                }
            })
            .ok_or_else(|| {
                Box::new(RemovalBlocker {
                    uri: Some(document.uri.clone()),
                    line: None,
                    reason: format!(
                        "'{owner_name}' no longer has an unused parameter at position {} - it \
                         may already have been used, removed, or renamed since this action was \
                         offered",
                        param_index + 1
                    )
                })
            })
    }

    /// `scan_listing_for_parameter_removal`, parsing `document` first via
    /// the cached `parse_document` - reused uniformly for both the
    /// triggering document and every other candidate file.
    pub(crate) fn scan_document_for_parameter_removal(
        &self,
        document: &Document,
        target: &RemoveParameterTarget
    ) -> FileScan {
        let text = document.text();
        match self.parse_document(document) {
            Ok(listing) => scan_listing_for_parameter_removal(&listing, &text, target),
            Err(_) => {
                FileScan {
                    matching_definitions: Vec::new(),
                    edits: Vec::new(),
                    blockers: vec![RemovalBlocker {
                        uri: Some(document.uri.clone()),
                        line: None,
                        reason: "this file no longer parses cleanly".to_string()
                    }]
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use tower_lsp::lsp_types::Url;

    use super::*;

    fn doc(code: &str) -> Document {
        Document::new(Url::parse("file:///t.asm").unwrap(), code.to_string(), 1)
    }

    fn parse(code: &str) -> (Document, std::sync::Arc<LocatedListing>) {
        let d = doc(code);
        let analyzer = AssemblyAnalyzer::new();
        let listing = analyzer.parse_document(&d).expect("should parse");
        (d, listing)
    }

    fn macro_target(owner: &str, index: usize, name: &str) -> RemoveParameterTarget {
        RemoveParameterTarget {
            kind: RemoveParameterKind::Macro,
            owner_name: owner.to_string(),
            param_index: index,
            param_name: name.to_string()
        }
    }

    fn function_target(owner: &str, index: usize, name: &str) -> RemoveParameterTarget {
        RemoveParameterTarget {
            kind: RemoveParameterKind::Function,
            owner_name: owner.to_string(),
            param_index: index,
            param_name: name.to_string()
        }
    }

    /// Applies `edits` to `document`'s own text, converting each LSP
    /// `Position` back to a byte offset via the existing
    /// `Document::offset_from_position` and splicing from the end backward
    /// so earlier offsets stay valid.
    fn apply_edits(document: &Document, edits: &[TextEdit]) -> String {
        let mut byte_edits: Vec<(Range<usize>, String)> = edits
            .iter()
            .map(|e| {
                let start = document.offset_from_position(e.range.start);
                let end = document.offset_from_position(e.range.end);
                (start..end, e.new_text.clone())
            })
            .collect();
        byte_edits.sort_by(|a, b| b.0.start.cmp(&a.0.start));
        let mut text = document.text();
        for (range, new_text) in byte_edits {
            text.replace_range(range, &new_text);
        }
        text
    }

    #[test]
    fn a_compile_time_variable_assignment_inside_a_function_body_does_not_panic() {
        // Regression test for a real bug found against a real project file:
        // `is_set()` is a plain alias for `is_assign()` (confirmed directly
        // against its source), NOT a distinct variant with its own value -
        // `ast_reachable_function_calls` used to gate `equ_value()` (which
        // panics for anything but a real EQU) behind `is_equ() || is_set()`,
        // copied from the pre-existing `symbols()` walker's identical bug.
        // `x = expr` is basm's real syntax for a mutable, compile-time
        // variable (common inside FUNCTION/MACRO/REPEAT bodies).
        let code = "FUNCTION helper, x, y\n    v = {x} + 1\n    RETURN v\nENDFUNCTION\n\
                     FUNCTION outer, z\n    RETURN helper(z, 99)\nENDFUNCTION\n";
        let (d, listing) = parse(code);
        let target = function_target("helper", 1, "y");
        let scan = scan_listing_for_parameter_removal(&listing, &d.text(), &target);
        assert!(scan.blockers.is_empty(), "{:?}", scan.blockers);
        let result = apply_edits(&d, &scan.edits);
        assert!(result.contains("helper(z)"), "{result}");
    }

    #[test]
    fn a_warning_wrapped_fake_instruction_does_not_panic() {
        // Regression test for a real bug found against a real project file:
        // `LocatedToken::inner` is `Either<LocatedTokenInner, (Box<LocatedToken>,
        // String)>` - the `Right` case wraps a "fake instruction" (e.g. `LD
        // HL, DE`, a real, accepted basm shorthand) with its own warning
        // message. Every `is_*()` predicate is safe for this shape, but
        // `mnemonic_arg1()`/`mnemonic_arg2()` are not - they unconditionally
        // `.unwrap()` the `Left` variant and panic on `Right`.
        // `ast_reachable_function_calls` must bail out for a warning-wrapped
        // token (via `is_warning()`) before ever reaching them.
        let code = "FUNCTION helper, x, y\n    RETURN {x}\nENDFUNCTION\nld hl, de\n";
        let (d, listing) = parse(code);
        let target = function_target("helper", 1, "y");
        let scan = scan_listing_for_parameter_removal(&listing, &d.text(), &target);
        assert!(scan.blockers.is_empty(), "{:?}", scan.blockers);
    }

    // ── expand_to_removable_argument_span ──────────────────────────────────

    #[test]
    fn removable_span_for_the_only_argument_is_just_the_argument_itself() {
        let text = "foo(1)";
        let arg = 4..5; // "1"
        assert_eq!(
            expand_to_removable_argument_span(text, arg, 0, 1),
            Some(4..5)
        );
    }

    #[test]
    fn removable_span_for_the_first_of_several_swallows_its_own_trailing_comma() {
        let text = "foo(1, 2, 3)";
        let arg = 4..5; // "1"
        let range = expand_to_removable_argument_span(text, arg, 0, 3).unwrap();
        let mut s = text.to_string();
        s.replace_range(range, "");
        assert_eq!(s, "foo(2, 3)");
    }

    #[test]
    fn removable_span_for_a_middle_argument_swallows_its_own_trailing_comma() {
        let text = "foo(1, 2, 3)";
        let arg = 7..8; // "2"
        let range = expand_to_removable_argument_span(text, arg, 1, 3).unwrap();
        let mut s = text.to_string();
        s.replace_range(range, "");
        assert_eq!(s, "foo(1, 3)");
    }

    #[test]
    fn removable_span_for_the_last_argument_swallows_the_preceding_comma() {
        let text = "foo(1, 2, 3)";
        let arg = 10..11; // "3"
        let range = expand_to_removable_argument_span(text, arg, 2, 3).unwrap();
        let mut s = text.to_string();
        s.replace_range(range, "");
        assert_eq!(s, "foo(1, 2)");
    }

    #[test]
    fn removable_span_refuses_when_the_expected_comma_is_missing() {
        // A deliberately malformed "arg_span"/arity pairing (2 arguments
        // claimed, but no comma anywhere) - must refuse, not guess.
        let text = "foo(1)";
        let arg = 4..5;
        assert!(expand_to_removable_argument_span(text, arg, 0, 2).is_none());
    }

    // ── MACRO call sites ─────────────────────────────────────────────────

    #[test]
    fn macro_call_first_argument_is_removed_from_both_the_call_and_the_header() {
        let code = "MACRO foo, a, b, c\nENDM\nfoo(1, 2, 3)\n";
        let (d, listing) = parse(code);
        let target = macro_target("foo", 0, "a");
        let scan = scan_listing_for_parameter_removal(&listing, &d.text(), &target);
        assert!(scan.blockers.is_empty(), "{:?}", scan.blockers);
        assert_eq!(scan.edits.len(), 2, "{:?}", scan.edits);
        let result = apply_edits(&d, &scan.edits);
        assert!(result.contains("MACRO foo, b, c"), "{result}");
        assert!(result.contains("foo(2, 3)"), "{result}");
    }

    #[test]
    fn macro_call_last_argument_is_removed_from_both_the_call_and_the_header() {
        let code = "MACRO foo, a, b, c\nENDM\nfoo(1, 2, 3)\n";
        let (d, listing) = parse(code);
        let target = macro_target("foo", 2, "c");
        let scan = scan_listing_for_parameter_removal(&listing, &d.text(), &target);
        assert!(scan.blockers.is_empty(), "{:?}", scan.blockers);
        let result = apply_edits(&d, &scan.edits);
        assert!(result.contains("MACRO foo, a, b"), "{result}");
        assert!(result.contains("foo(1, 2)"), "{result}");
    }

    #[test]
    fn macro_call_argument_removed_across_a_multi_line_call() {
        let code = "MACRO foo, a, b, c\nENDM\nfoo(\n    1,\n    2,\n    3\n)\n";
        let (d, listing) = parse(code);
        let target = macro_target("foo", 0, "a");
        let scan = scan_listing_for_parameter_removal(&listing, &d.text(), &target);
        assert!(scan.blockers.is_empty(), "{:?}", scan.blockers);
        let result = apply_edits(&d, &scan.edits);
        assert!(!result.contains('1'), "{result}");
        assert!(result.contains('2') && result.contains('3'), "{result}");
    }

    #[test]
    fn evaluated_macro_argument_removal_also_deletes_the_eval_prefix() {
        let code = "MACRO foo, a, b\nENDM\nfoo({eval}1+1, 2)\n";
        let (d, listing) = parse(code);
        let target = macro_target("foo", 0, "a");
        let scan = scan_listing_for_parameter_removal(&listing, &d.text(), &target);
        assert!(scan.blockers.is_empty(), "{:?}", scan.blockers);
        let result = apply_edits(&d, &scan.edits);
        assert!(!result.contains("eval"), "{result}");
        assert!(result.contains("foo(2)"), "{result}");
    }

    #[test]
    fn a_list_shaped_call_argument_is_a_blocker_not_a_panic() {
        let code = "MACRO foo, a, b\nENDM\nfoo([1, 2, 3], 5)\n";
        let (d, listing) = parse(code);
        let target = macro_target("foo", 0, "a");
        let scan = scan_listing_for_parameter_removal(&listing, &d.text(), &target);
        // The definition header's own edit is still computed independently
        // of the call-site scan (it's the *caller*'s job - the cross-file
        // orchestrator in `server/backend.rs` - to discard every edit once
        // any blocker exists anywhere, not this per-file scan's job), so
        // only the blocker itself is asserted here.
        assert!(
            !scan.blockers.is_empty(),
            "expected a blocker for the List-shaped call argument"
        );
    }

    #[test]
    fn macro_arg_span_refuses_an_empty_placeholder_without_panicking() {
        // `Empty` is unreachable from a real parsed macro call (verified
        // directly - its only construction site pads a *struct* call's
        // local argument buffer, never a macro's own), so this is
        // constructed directly rather than via source.
        assert!(macro_arg_span(&LocatedMacroParam::Empty).is_none());
    }

    #[test]
    fn a_struct_sharing_the_macro_s_name_makes_it_ambiguous() {
        let code = "MACRO foo, a, b\nENDM\nSTRUCT foo\n    x db\nENDSTRUCT\n";
        let (d, listing) = parse(code);
        let target = macro_target("foo", 0, "a");
        let scan = scan_listing_for_parameter_removal(&listing, &d.text(), &target);
        assert!(
            scan.matching_definitions.len() > 1,
            "{:?}",
            scan.matching_definitions
        );
    }

    #[test]
    fn two_same_named_macros_are_both_counted_as_matching_definitions() {
        let code = "MACRO foo, a\nENDM\nMACRO foo, x\nENDM\n";
        let (d, listing) = parse(code);
        let target = macro_target("foo", 0, "a");
        let scan = scan_listing_for_parameter_removal(&listing, &d.text(), &target);
        assert_eq!(
            scan.matching_definitions.len(),
            2,
            "{:?}",
            scan.matching_definitions
        );
    }

    // ── FUNCTION call sites ─────────────────────────────────────────────

    #[test]
    fn function_call_argument_is_removed_from_both_the_call_and_the_header() {
        let code = "FUNCTION f, a, b\n    RETURN {a}\nENDFUNCTION\nDEFB f(1, 2)\n";
        let (d, listing) = parse(code);
        let target = function_target("f", 1, "b");
        let scan = scan_listing_for_parameter_removal(&listing, &d.text(), &target);
        assert!(scan.blockers.is_empty(), "{:?}", scan.blockers);
        let result = apply_edits(&d, &scan.edits);
        assert!(result.contains("FUNCTION f, a"), "{result}");
        assert!(result.contains("f(1)"), "{result}");
    }

    #[test]
    fn function_call_nested_in_an_opcode_operand_is_found() {
        let code = "FUNCTION double, x, y\n    RETURN {x} * 2\nENDFUNCTION\nld a, double(5, 9)\n";
        let (d, listing) = parse(code);
        let target = function_target("double", 1, "y");
        let scan = scan_listing_for_parameter_removal(&listing, &d.text(), &target);
        assert!(scan.blockers.is_empty(), "{:?}", scan.blockers);
        let result = apply_edits(&d, &scan.edits);
        assert!(result.contains("double(5)"), "{result}");
    }

    #[test]
    fn a_function_call_only_reachable_textually_inside_print_is_a_blocker() {
        // PRINT has no expression accessor today, so the AST walk can't
        // reach a call inside it - the textual tripwire must catch this,
        // not silently miss it.
        let code = "FUNCTION double, x, y\n    RETURN {x} * 2\nENDFUNCTION\nPRINT double(5, 9)\n";
        let (d, listing) = parse(code);
        let target = function_target("double", 1, "y");
        let scan = scan_listing_for_parameter_removal(&listing, &d.text(), &target);
        assert!(
            !scan.blockers.is_empty(),
            "expected a blocker for the PRINT call site"
        );
    }

    #[test]
    fn a_call_site_inside_another_function_s_body_is_found() {
        // Regression test for the `flatten_listing` FUNCTION-body gap fix:
        // before that fix, nothing inside `outer`'s own body was ever
        // visited at all, so this call site would have been silently
        // missed (not even reported as a blocker) rather than found.
        let code = "FUNCTION helper, x, y\n    RETURN {x} * 2\nENDFUNCTION\n\
                     FUNCTION outer, z\n    RETURN helper(z, 99)\nENDFUNCTION\n";
        let (d, listing) = parse(code);
        let target = function_target("helper", 1, "y");
        let scan = scan_listing_for_parameter_removal(&listing, &d.text(), &target);
        assert!(scan.blockers.is_empty(), "{:?}", scan.blockers);
        let result = apply_edits(&d, &scan.edits);
        assert!(result.contains("helper(z)"), "{result}");
    }

    // ── definition_header_removable_span ────────────────────────────────

    #[test]
    fn definition_header_span_handles_first_middle_last_position() {
        let header = "MACRO foo, a, b, c";
        let params = ["a", "b", "c"];
        for (index, expected) in [
            (0, "MACRO foo, b, c"),
            (1, "MACRO foo, a, c"),
            (2, "MACRO foo, a, b")
        ] {
            let range = definition_header_removable_span(header, &params, index).unwrap();
            let mut s = header.to_string();
            s.replace_range(range, "");
            assert_eq!(s, expected);
        }
    }

    #[test]
    fn definition_header_span_handles_a_single_parameter_with_the_stored_parens_quirk() {
        // A single-parameter MACRO/FUNCTION can store its own parameter
        // name with surrounding parens still attached (`"(x)"` not `"x"`) -
        // an existing, already-worked-around grammar quirk elsewhere in
        // this crate (`token::macro_scoped_symbol_at`).
        let header = "MACRO foo, x";
        let params = ["(x)"];
        let range = definition_header_removable_span(header, &params, 0).unwrap();
        let mut s = header.to_string();
        s.replace_range(range, "");
        assert_eq!(s, "MACRO foo");
    }

    // ── resolve_remove_parameter_target ──────────────────────────────────

    #[test]
    fn resolve_target_succeeds_for_a_genuinely_unused_parameter() {
        let code = "MACRO foo, a, b\n    ld a, {a}\nENDM\n";
        let d = doc(code);
        let analyzer = AssemblyAnalyzer::new();
        let target = analyzer
            .resolve_remove_parameter_target(&d, RemoveParameterKind::Macro, "foo", 1)
            .expect("should resolve");
        assert_eq!(target.param_name, "b");
    }

    #[test]
    fn resolve_target_refuses_a_parameter_that_is_actually_used() {
        let code = "MACRO foo, a, b\n    ld a, {a}\n    ld b, {b}\nENDM\n";
        let d = doc(code);
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .resolve_remove_parameter_target(&d, RemoveParameterKind::Macro, "foo", 1)
                .is_err()
        );
    }
}
