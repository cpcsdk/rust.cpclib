//! Shared `DocumentSymbol`-tree building helpers, used by every language
//! module that groups a flat symbol list under synthetic parent headings
//! (bndbuild's "Variables"/"Artifacts" split, basm's nested outline).

use tower_lsp::lsp_types::{DocumentSymbol, Range, SymbolKind};

/// Grow `range` so it also covers `other` — the min of both starts, the max
/// of both ends. Positions compare lexicographically (`(line, character)`),
/// matching `Range`'s own natural document-order semantics.
pub fn extend_range(range: &mut Range, other: &Range) {
    if (other.start.line, other.start.character) < (range.start.line, range.start.character) {
        range.start = other.start;
    }
    if (other.end.line, other.end.character) > (range.end.line, range.end.character) {
        range.end = other.end;
    }
}

/// Group `children` under a synthetic namespace symbol spanning their
/// combined range. `children` must be non-empty.
pub fn container_symbol(name: &str, children: Vec<DocumentSymbol>) -> DocumentSymbol {
    let mut range = children[0].range;
    for child in &children[1..] {
        extend_range(&mut range, &child.range);
    }

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
