//! "Highlight matching push/pop pairs" - `textDocument/documentHighlight`
//! for `PUSH`/`POP` statements. The LSP-native equivalent of asm-code-lens's
//! own `enablePushPopMatching` setting (present in its config since v2.6,
//! though no longer called out in its current README's headline feature
//! list - a legacy x86-era addition it still ships).
//!
//! Pairing is LIFO by *position*, not by register name: `push hl` followed
//! later by `pop de` is a completely ordinary idiom (moving a value through
//! a different register, or just discarding it while balancing the stack),
//! and the real Z80 stack itself has no notion of which register a slot
//! "belongs to" - only push/pop *order*. Matching by name would both miss
//! real pairs and invent fake ones.
//!
//! `PUSH`/`POP` naming several registers at once (`push af, bc, hl` -
//! basm's `MultiPush`/`MultiPop`, expanding to several single-register
//! push/pops in the order given - see
//! `ListingElement::multi_push_pop_to_listing`) contributes one stack slot
//! per register, in that same order - the order the assembler itself
//! actually emits them in. Every slot from one such statement shares that
//! statement's own source range (there is no per-register span to give
//! each one its own highlight), so a multi-register statement can appear
//! paired with several distinct statements at once.
//!
//! Scoped per global label - the same routine boundaries `symbols.rs`'s own
//! outline uses, via `token::global_label_scopes`: stack balance is a
//! per-routine invariant, and resetting the virtual stack at each boundary
//! keeps one routine's own imbalance from producing a nonsensical pairing
//! into the next routine.

use cpclib_asm::parser::obtained::MayHaveSpan;
use cpclib_tokens::{ListingElement, Mnemonic};
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use crate::common::document::Document;

impl AssemblyAnalyzer {
    pub fn push_pop_highlights(
        &self,
        document: &Document,
        position: Position
    ) -> Vec<DocumentHighlight> {
        let Ok(listing) = self.parse_document(document)
        else {
            return Vec::new();
        };
        let scopes = super::token::global_label_scopes(listing.iter());

        // Every (push_statement_range, pop_statement_range) pairing found
        // anywhere in the document - filtered down to the ones touching
        // `position` only at the very end, since a statement can be a
        // multi-register one paired with several different partners.
        let mut pairs: Vec<(Range, Range)> = Vec::new();
        let mut stack: Vec<Range> = Vec::new();
        let mut current_scope_start: Option<u32> = None;

        for token in super::token::flatten_listing(listing.iter()) {
            let (mnemonic, slots) = if let Some(expanded) = token.multi_push_pop_to_listing() {
                match expanded.first() {
                    Some((m, ..)) => (*m, expanded.len()),
                    None => continue
                }
            }
            else {
                match token.mnemonic() {
                    Some(Mnemonic::Push) => (Mnemonic::Push, 1),
                    Some(Mnemonic::Pop) => (Mnemonic::Pop, 1),
                    _ => continue
                }
            };

            let (line_1based, _col) = token.span().relative_line_and_column();
            let line = line_1based.saturating_sub(1) as u32;

            // A new routine (or falling out of every known one) starts a
            // fresh stack - see the module doc comment for why.
            let scope_start = super::token::scope_containing(&scopes, line).map(|(_, r)| r.start);
            if scope_start != current_scope_start {
                stack.clear();
                current_scope_start = scope_start;
            }

            let range = whole_line_range(document, line);
            match mnemonic {
                Mnemonic::Push => {
                    for _ in 0..slots {
                        stack.push(range);
                    }
                },
                Mnemonic::Pop => {
                    for _ in 0..slots {
                        if let Some(push_range) = stack.pop() {
                            pairs.push((push_range, range));
                        }
                    }
                },
                _ => unreachable!("multi_push_pop_to_listing/mnemonic only ever yield Push/Pop")
            }
        }

        let mut ranges: Vec<Range> = pairs
            .iter()
            .filter(|(push_range, pop_range)| {
                contains(*push_range, position) || contains(*pop_range, position)
            })
            .flat_map(|(a, b)| [*a, *b])
            .collect();
        ranges.sort_by_key(|r| (r.start.line, r.start.character));
        ranges.dedup();

        ranges
            .into_iter()
            .map(|range| {
                DocumentHighlight {
                    range,
                    kind: Some(DocumentHighlightKind::TEXT)
                }
            })
            .collect()
    }
}

fn contains(range: Range, position: Position) -> bool {
    (range.start.line, range.start.character) <= (position.line, position.character)
        && (position.line, position.character) <= (range.end.line, range.end.character)
}

/// The full extent of source line `line` in `document`, trimmed of its
/// trailing newline - same convention as `diagnostics.rs`'s own
/// whole-line ranges.
fn whole_line_range(document: &Document, line: u32) -> Range {
    let len = document
        .line(line as usize)
        .map(|l| l.trim_end_matches(['\r', '\n']).chars().count() as u32)
        .unwrap_or(0);
    Range {
        start: Position { line, character: 0 },
        end: Position {
            line,
            character: len
        }
    }
}

#[cfg(test)]
mod tests {
    use tower_lsp::lsp_types::Url;

    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///main.asm").unwrap(), text.to_string(), 0)
    }

    #[test]
    fn a_simple_push_and_pop_pair_highlight_each_other() {
        let analyzer = AssemblyAnalyzer::new();
        let document = doc("start:\n    push hl\n    nop\n    pop hl\n    ret\n");
        let highlights = analyzer.push_pop_highlights(
            &document,
            Position {
                line: 1,
                character: 8
            }
        );
        let lines: Vec<u32> = highlights.iter().map(|h| h.range.start.line).collect();
        assert_eq!(lines, vec![1, 3], "{highlights:?}");
    }

    #[test]
    fn pairing_is_by_position_not_by_register_name() {
        // `push hl` / `pop de` is an ordinary idiom (moving a value through
        // a different register) - it must still pair, proving matching is
        // purely LIFO-by-position.
        let analyzer = AssemblyAnalyzer::new();
        let document = doc("start:\n    push hl\n    pop de\n    ret\n");
        let highlights = analyzer.push_pop_highlights(
            &document,
            Position {
                line: 1,
                character: 8
            }
        );
        let lines: Vec<u32> = highlights.iter().map(|h| h.range.start.line).collect();
        assert_eq!(lines, vec![1, 2], "{highlights:?}");
    }

    #[test]
    fn a_multi_register_push_pairs_with_each_of_its_reverse_order_pops() {
        let analyzer = AssemblyAnalyzer::new();
        let document = doc(
            "start:\n    push af, bc, hl\n    pop hl\n    pop bc\n    pop af\n    ret\n"
        );
        // Cursor on the multi-push: it contributed 3 slots, so it must be
        // paired with all 3 single-register pops that drained them.
        let highlights = analyzer.push_pop_highlights(
            &document,
            Position {
                line: 1,
                character: 8
            }
        );
        let lines: Vec<u32> = highlights.iter().map(|h| h.range.start.line).collect();
        assert_eq!(lines, vec![1, 2, 3, 4], "{highlights:?}");
    }

    #[test]
    fn an_unbalanced_push_has_no_match() {
        let analyzer = AssemblyAnalyzer::new();
        let document = doc("start:\n    push hl\n    ret\n");
        let highlights = analyzer.push_pop_highlights(
            &document,
            Position {
                line: 1,
                character: 8
            }
        );
        assert!(highlights.is_empty(), "{highlights:?}");
    }

    #[test]
    fn stack_resets_at_each_global_label_so_routines_do_not_bleed_into_each_other() {
        // `first` pushes without a matching pop (a bug, or an intentional
        // non-local exit) - `second`'s own pop must not be silently matched
        // against it just because it's the next one in the file.
        let analyzer = AssemblyAnalyzer::new();
        let document = doc(
            "first:\n    push hl\n    ret\nsecond:\n    push de\n    pop de\n    ret\n"
        );
        let highlights = analyzer.push_pop_highlights(
            &document,
            Position {
                line: 1,
                character: 8
            }
        );
        assert!(highlights.is_empty(), "{highlights:?}");

        let highlights = analyzer.push_pop_highlights(
            &document,
            Position {
                line: 4,
                character: 8
            }
        );
        let lines: Vec<u32> = highlights.iter().map(|h| h.range.start.line).collect();
        assert_eq!(lines, vec![4, 5], "{highlights:?}");
    }
}
