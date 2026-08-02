//! Shared `DocumentSymbol`-tree building helpers.

use tower_lsp::lsp_types::Range;

/// Grow `range` so it also covers `other` — the min of both starts, the max
/// of both ends. Positions compare lexicographically (`(line, character)`),
/// matching `Range`'s own natural document-order semantics. Used to extend a
/// *real* containment relationship (e.g. a global label's range growing to
/// cover a local label nested inside it) - not for synthetic UI groupings,
/// which must never claim a range wider than what's really theirs (see
/// `basm::symbols`/`bndbuild::symbols`'s own module doc comments for why a
/// prior version of this file's now-removed `container_symbol` helper broke
/// Sticky Scroll).
pub fn extend_range(range: &mut Range, other: &Range) {
    if (other.start.line, other.start.character) < (range.start.line, range.start.character) {
        range.start = other.start;
    }
    if (other.end.line, other.end.character) > (range.end.line, range.end.character) {
        range.end = other.end;
    }
}
