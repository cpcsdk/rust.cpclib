//! Call hierarchy for basm: `CALL`/`CALL cc,label` as the "call" relation,
//! with the existing `label_scope_at_line` heuristic (a global label's span
//! up to the next global label) standing in for "function boundary" - the
//! same heuristic already used to confine local-label renames, reused here
//! rather than inventing a new `RET`-based one. `JP`/`JR` are deliberately
//! not modeled - those are jumps, not calls.

use cpclib_asm::parser::obtained::{LocatedDataAccess, LocatedExpr, MayHaveSpan};
use cpclib_asm::preamble::SourceString;
use cpclib_tokens::{ListingElement, Mnemonic};
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use super::embedded_basic::{block_and_text_at, extract_locomotive_blocks, text_for_block};
use super::token::{flatten_listing, label_scope_at_line, span_line};
use crate::common::call_hierarchy::CallHierarchyData;
use crate::common::document::Document;

impl AssemblyAnalyzer {
    /// The `CallHierarchyItem` for the global label `name_upper` (already
    /// uppercased) if it is defined in `document` - `find_definition_in`
    /// gives the definition location (`selection_range`); `label_scope_at_line`
    /// on that location's own line gives the label's scope (`range`),
    /// clamping the "no next label" `u32::MAX` sentinel to `document`'s
    /// actual last line.
    ///
    /// A `name_upper` that resolves to a local `.foo` label's own
    /// definition still reports the *enclosing global*'s name/scope - call
    /// hierarchy here models function-level `CALL` boundaries, not
    /// local-label granularity.
    pub fn call_hierarchy_item_for_label(
        &self,
        document: &Document,
        name_upper: &str
    ) -> Option<CallHierarchyItem> {
        let loc = self.find_definition_in(document, name_upper)?;
        let listing = self.parse_document(document).ok()?;
        let (name, scope) = label_scope_at_line(listing.iter(), loc.range.start.line)?;
        let end = scope.end.min(document.line_count() as u32);
        Some(CallHierarchyItem {
            name: name.clone(),
            kind: SymbolKind::FUNCTION,
            tags: None,
            detail: None,
            uri: loc.uri.clone(),
            range: Range {
                start: Position {
                    line: scope.start,
                    character: 0
                },
                end: Position {
                    line: end,
                    character: 0
                }
            },
            selection_range: loc.range,
            data: Some(CallHierarchyData::AsmLabel { name }.to_json())
        })
    }

    /// The call-hierarchy item for whatever global label's scope contains
    /// `position` - the LSP-facing entry point for `textDocument/prepareCallHierarchy`.
    /// Delegates to BASIC call hierarchy first when the cursor is inside a
    /// `LOCOMOTIVE` block, exactly like `goto_definition`.
    pub fn prepare_call_hierarchy(
        &self,
        document: &Document,
        position: Position
    ) -> Option<CallHierarchyItem> {
        let text = document.text();
        let line_idx = position.line as usize;
        if let Some((block, basic_text)) = block_and_text_at(&text, line_idx) {
            return crate::locomotive::call_hierarchy::locomotive_basic_prepare_call_hierarchy(
                &basic_text,
                position,
                block.basic_range.start as u32,
                &document.uri
            );
        }

        let listing = self.parse_document(document).ok()?;
        let (name, _scope) = label_scope_at_line(listing.iter(), position.line)?;
        self.call_hierarchy_item_for_label(document, &name.to_uppercase())
    }

    /// Every `CALL name_upper` / `CALL cc,name_upper` call site in
    /// `document`, grouped by the call site's *enclosing function*
    /// (`label_scope_at_line` on the call's own line) into one
    /// `CallHierarchyIncomingCall` per caller, with every call site's range
    /// collected into `from_ranges`. Callers loop this over every open
    /// Assembly document, mirroring `references()`'s own cross-document
    /// loop in `backend.rs`.
    pub fn incoming_calls_in(
        &self,
        document: &Document,
        name_upper: &str
    ) -> Vec<CallHierarchyIncomingCall> {
        let Ok(listing) = self.parse_document(document)
        else {
            return Vec::new();
        };

        // Precomputed once (not once per matched call site, see
        // `global_label_scopes`'s own doc comment) - `incoming_calls_in`
        // gets worse exactly when it matters most, a widely-called
        // subroutine with many call sites to attribute.
        let scopes = super::token::global_label_scopes(listing.iter());

        // owner name -> call-site ranges
        let mut groups: Vec<(String, Vec<Range>)> = Vec::new();

        for token in flatten_listing(listing.iter()) {
            for call_range in call_targets_in_token(token, name_upper) {
                let call_line = call_range.start.line;
                let Some((owner, _)) = super::token::scope_containing(&scopes, call_line)
                else {
                    continue;
                };
                match groups
                    .iter_mut()
                    .find(|(o, _)| o.eq_ignore_ascii_case(&owner))
                {
                    Some(g) => g.1.push(call_range),
                    None => groups.push((owner, vec![call_range]))
                }
            }
        }

        groups
            .into_iter()
            .filter_map(|(owner, ranges)| {
                let from = self.call_hierarchy_item_for_label(document, &owner.to_uppercase())?;
                Some(CallHierarchyIncomingCall {
                    from,
                    from_ranges: ranges
                })
            })
            .collect()
    }

    /// Every distinct `CALL` target reachable from `name_upper`'s (already
    /// uppercased) own scope in `document` - that label's body, per
    /// `label_scope_at_line` - as `(target_name_as_written, call-site
    /// ranges)`; multiple calls to the same target within the scope
    /// collapse into one entry's ranges. Returns an empty `Vec` if
    /// `name_upper` isn't defined in `document` or has no calls. Target
    /// resolution to a `CallHierarchyItem` (possibly in another open
    /// document) happens in `backend.rs`, which alone has access to every
    /// open document.
    pub fn outgoing_call_targets(
        &self,
        document: &Document,
        name_upper: &str
    ) -> Vec<(String, Vec<Range>)> {
        let Ok(listing) = self.parse_document(document)
        else {
            return Vec::new();
        };
        let Some(loc) = self.find_definition_in(document, name_upper)
        else {
            return Vec::new();
        };
        let Some((_, scope)) = label_scope_at_line(listing.iter(), loc.range.start.line)
        else {
            return Vec::new();
        };

        let mut groups: Vec<(String, Vec<Range>)> = Vec::new();
        for token in flatten_listing(listing.iter()) {
            let line = span_line(token);
            if line < scope.start || line >= scope.end {
                continue;
            }
            for (target, call_range) in call_targets_with_names_in_token(token) {
                match groups
                    .iter_mut()
                    .find(|(t, _)| t.eq_ignore_ascii_case(&target))
                {
                    Some(g) => g.1.push(call_range),
                    None => groups.push((target, vec![call_range]))
                }
            }
        }
        groups
    }

    /// As [`Self::incoming_calls_in`], for a BASIC line embedded in a
    /// `LOCOMOTIVE` block at `block_start_line` inside `document`. The
    /// `LocomotiveBlock` re-derivation (rather than a cursor-based lookup,
    /// since `incoming_calls`/`outgoing_calls` only carry the item's stashed
    /// `data`, not a position) has to happen here, not in `backend.rs`:
    /// `LocomotiveBlock`/`basic_range` are `pub(super)` to this module.
    pub fn incoming_calls_for_embedded_basic_line(
        &self,
        document: &Document,
        line_number: u16,
        block_start_line: u32
    ) -> Vec<CallHierarchyIncomingCall> {
        let Some(basic_text) = embedded_basic_text_at(document, block_start_line)
        else {
            return Vec::new();
        };
        crate::locomotive::call_hierarchy::locomotive_basic_incoming_calls(
            &basic_text,
            line_number,
            block_start_line,
            &document.uri
        )
    }

    /// As [`Self::incoming_calls_for_embedded_basic_line`], for outgoing calls.
    pub fn outgoing_calls_for_embedded_basic_line(
        &self,
        document: &Document,
        line_number: u16,
        block_start_line: u32
    ) -> Vec<CallHierarchyOutgoingCall> {
        let Some(basic_text) = embedded_basic_text_at(document, block_start_line)
        else {
            return Vec::new();
        };
        crate::locomotive::call_hierarchy::locomotive_basic_outgoing_calls(
            &basic_text,
            line_number,
            block_start_line,
            &document.uri
        )
    }
}

/// Re-join the source text of the `LOCOMOTIVE` block starting at
/// `block_start_line` (0-based document line), or `None` if no such block
/// exists any more (the document changed since the call-hierarchy item was
/// prepared).
fn embedded_basic_text_at(document: &Document, block_start_line: u32) -> Option<String> {
    let text = document.text();
    let block = extract_locomotive_blocks(&text)
        .into_iter()
        .find(|b| b.basic_range.start as u32 == block_start_line)?;
    Some(text_for_block(&text, &block))
}

/// LSP `Range` for a label-name `Z80Span` (e.g. a `CALL` operand).
fn label_span_to_range(span: &cpclib_asm::parser::source::Z80Span) -> Range {
    let (line_1based, col_1based) = span.relative_line_and_column();
    let lsp_line = line_1based.saturating_sub(1) as u32;
    let lsp_char = col_1based.saturating_sub(1) as u32;
    Range {
        start: Position {
            line: lsp_line,
            character: lsp_char
        },
        end: Position {
            line: lsp_line,
            character: lsp_char + span.as_str().len() as u32
        }
    }
}

/// If `token` is a non-warning `CALL`/`CALL cc,label` whose label operand
/// (checked in both `mnemonic_arg1()`/`mnemonic_arg2()`, since the label is
/// in arg1 for unconditional `CALL label` and arg2 for conditional
/// `CALL cc,label`) matches `name_upper` case-insensitively, its call-site
/// range - there can be at most one match per token, but this returns a
/// `Vec` to keep call sites uniform with [`call_targets_with_names_in_token`].
fn call_targets_in_token<T>(token: &T, name_upper: &str) -> Vec<Range>
where T: ListingElement<DataAccess = cpclib_asm::parser::obtained::LocatedDataAccess> + MayHaveSpan
{
    call_targets_with_names_in_token(token)
        .into_iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case(name_upper))
        .map(|(_, range)| range)
        .collect()
}

/// Every `CALL`/`CALL cc,label` target named in `token`, as
/// `(target_name_as_written, call-site range)`. Warning-wrapped tokens
/// (e.g. fake instructions) are skipped - most `ListingElement` accessors,
/// including `mnemonic()`/`mnemonic_arg1()`/`mnemonic_arg2()`, panic on one
/// instead of returning `None` (see `overflow.rs`'s `overflow_candidates`
/// for the same guard, needed for the same reason).
fn call_targets_with_names_in_token<T>(token: &T) -> Vec<(String, Range)>
where T: ListingElement<DataAccess = cpclib_asm::parser::obtained::LocatedDataAccess> + MayHaveSpan
{
    let mut out = Vec::new();
    if token.is_warning() {
        return out;
    }
    if !matches!(token.mnemonic(), Some(Mnemonic::Call)) {
        return out;
    }
    for arg in [token.mnemonic_arg1(), token.mnemonic_arg2()]
        .into_iter()
        .flatten()
    {
        if let LocatedDataAccess::Expression(LocatedExpr::Label(span)) = arg {
            out.push((span.as_str().to_string(), label_span_to_range(span)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::document::Document;

    fn doc(text: &str) -> Document {
        let uri = Url::parse("file:///main.asm").unwrap();
        Document::new(uri, text.to_string(), 1)
    }

    /// Regression test for the LOCOMOTIVE-block dedup (`block_and_text_at`):
    /// preparing call hierarchy inside a `LOCOMOTIVE` block must still
    /// delegate to BASIC call hierarchy with the correctly-reconstructed
    /// block text.
    #[test]
    fn prepare_call_hierarchy_inside_a_locomotive_block_delegates_to_basic() {
        let text = "ORG 0x8000\nLOCOMOTIVE\n10 GOSUB 20\n20 RETURN\nENDLOCOMOTIVE\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let item = analyzer
            .prepare_call_hierarchy(
                &d,
                Position {
                    line: 2,
                    character: 0
                }
            )
            .expect("expected a BASIC call hierarchy item inside the LOCOMOTIVE block");
        assert_eq!(item.name, "Line 10");
    }

    #[test]
    fn prepare_call_hierarchy_finds_the_enclosing_label() {
        let text = "start:\n  call target\n  ret\ntarget:\n  ret\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let item = analyzer
            .prepare_call_hierarchy(
                &d,
                Position {
                    line: 3,
                    character: 0
                }
            )
            .expect("expected an item at the target: label");
        assert_eq!(item.name, "target");
    }

    #[test]
    fn incoming_calls_finds_the_caller() {
        let text = "start:\n  call target\n  ret\ntarget:\n  ret\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let calls = analyzer.incoming_calls_in(&d, "TARGET");
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert_eq!(calls[0].from.name, "start");
        assert_eq!(calls[0].from_ranges.len(), 1);
    }

    #[test]
    fn outgoing_calls_finds_the_callee() {
        let text = "start:\n  call target\n  ret\ntarget:\n  ret\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let targets = analyzer.outgoing_call_targets(&d, "START");
        assert_eq!(targets.len(), 1, "{targets:?}");
        assert_eq!(targets[0].0.to_uppercase(), "TARGET");
        assert_eq!(targets[0].1.len(), 1);
    }

    #[test]
    fn conditional_call_target_is_found_via_arg2() {
        let text = "start:\n  call nz,target\n  ret\ntarget:\n  ret\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let calls = analyzer.incoming_calls_in(&d, "TARGET");
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert_eq!(calls[0].from.name, "start");
    }

    #[test]
    fn label_with_no_callers_yields_no_incoming_calls() {
        let text = "start:\n  ret\ntarget:\n  ret\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        assert!(analyzer.incoming_calls_in(&d, "TARGET").is_empty());
    }

    #[test]
    fn two_call_sites_in_one_function_collapse_into_one_incoming_entry() {
        let text = "start:\n  call target\n  call target\n  ret\ntarget:\n  ret\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let calls = analyzer.incoming_calls_in(&d, "TARGET");
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert_eq!(calls[0].from_ranges.len(), 2);
    }

    #[test]
    fn two_calls_to_the_same_target_collapse_into_one_outgoing_entry() {
        let text = "start:\n  call target\n  call target\n  ret\ntarget:\n  ret\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let targets = analyzer.outgoing_call_targets(&d, "START");
        assert_eq!(targets.len(), 1, "{targets:?}");
        assert_eq!(targets[0].1.len(), 2);
    }

    #[test]
    fn last_label_scope_is_clamped_to_document_line_count() {
        let text = "start:\n  call target\n  ret\ntarget:\n  ret\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let item = analyzer
            .call_hierarchy_item_for_label(&d, "TARGET")
            .expect("target should resolve");
        assert_eq!(item.range.end.line, d.line_count() as u32);
    }

    #[test]
    fn a_fake_instruction_elsewhere_does_not_panic() {
        // `ld hl, sp` is assembled as several real opcodes and is
        // warning-wrapped by the parser - must not panic incoming/outgoing
        // call scanning (regression for the `is_warning()` guard).
        let text = "start:\n  ld hl, sp\n  call target\n  ret\ntarget:\n  ret\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let calls = analyzer.incoming_calls_in(&d, "TARGET");
        assert_eq!(calls.len(), 1, "{calls:?}");
        let targets = analyzer.outgoing_call_targets(&d, "START");
        assert_eq!(targets.len(), 1, "{targets:?}");
    }
}
