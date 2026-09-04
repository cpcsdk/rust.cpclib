//! Document symbols (outline) for assembly files: a flat, document-order
//! top-level list (modules, sections, labels, macros, functions, EQU
//! constants, assign variables all mixed together in the order they appear
//! in the source) — *not* grouped under synthetic "Macros"/"Constants"/etc.
//! category headers the way earlier versions of this module were. Local
//! labels still nest under the global label whose scope contains them (a
//! real containment relationship, unlike a category grouping).
//!
//! The category-header grouping was removed after a real, user-reported
//! Sticky Scroll regression: a synthetic header's own `range` either had to
//! (a) span from its first child to its last (wrong - VS Code then treats
//! the *header itself* as a real, wide container to pin while scrolled
//! anywhere between two unrelated, far-apart children - see
//! `common::symbols::container_symbol`'s own history), or (b) collapse to a
//! narrow/zero-width point (also wrong - VS Code's Sticky Scroll only
//! recurses into a symbol's `children` when the *parent's* range already
//! contains the scroll position, so a header whose range doesn't actually
//! contain its real children makes Sticky Scroll stop dead at the header
//! and never reach them at all - reported as "nothing shows anymore").
//! There is no `range` a purely-organizational grouping node can have that
//! satisfies both "don't falsely claim ownership of the gaps between
//! children" and "let Sticky Scroll actually recurse into the children" -
//! so this module stopped introducing that grouping node in the first
//! place. Each symbol still keeps its own `kind` (Function/Constant/
//! Variable/Namespace/...), so the Outline panel still shows a distinct
//! icon per entry, just not bucketed under a header anymore.

use std::collections::HashMap;

use cpclib_asm::parser::obtained::MayHaveSpan;
use cpclib_tokens::{ListingElement, Token};
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use crate::common::document::Document;

/// Grouping key for the outline — see the module doc comment for the order.
/// Not exposed over LSP; only used to decide which top-level container each
/// symbol lands in, and the order those containers themselves appear in.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum SymbolCategory {
    Module,
    Section,
    Label,
    Macro,
    Function,
    Constant,
    Variable
}

/// A label symbol plus whether it's a bare (dotted) local reference —
/// tracked alongside the `DocumentSymbol` since `nest_local_labels` needs it
/// and it isn't otherwise recoverable from the symbol's own (already
/// display-qualified) `name`.
enum LabelEntry {
    Global(DocumentSymbol),
    Local(DocumentSymbol)
}

/// Per-token symbol facts extracted before building its `DocumentSymbol`:
/// `(source_len, display_name, category, kind, detail, pos_override, is_local_label)`.
type TokenSymbolFacts = (
    usize,
    String,
    SymbolCategory,
    SymbolKind,
    Option<String>,
    Option<(u32, u32)>,
    bool
);

impl AssemblyAnalyzer {
    pub fn document_symbols(&self, document: &Document) -> Vec<DocumentSymbol> {
        let Ok(listing) = self.parse_document(document)
        else {
            return Vec::new();
        };
        let text = document.text();

        let mut modules: Vec<DocumentSymbol> = Vec::new();
        let mut sections: Vec<DocumentSymbol> = Vec::new();
        let mut labels: Vec<LabelEntry> = Vec::new();
        let mut macros: Vec<DocumentSymbol> = Vec::new();
        let mut functions: Vec<DocumentSymbol> = Vec::new();
        let mut constants: Vec<DocumentSymbol> = Vec::new();
        let mut variables: Vec<DocumentSymbol> = Vec::new();

        // Track the last seen global label to qualify local labels (`.foo` → `parent.foo`)
        let mut current_global: Option<String> = None;

        // Each global label's own line range (through the next global label,
        // or end of file) - reused to extend a global label symbol's `range`
        // for Sticky Scroll, the same way `scope_containing` already backs
        // local-label scope confinement elsewhere.
        let global_scopes = super::token::global_label_scopes(listing.iter());
        // The real last line the editor can address - not just
        // `text.lines().count()`, which under-counts by one whenever the
        // file has no trailing newline (see
        // `token::clamp_to_last_addressable_line`'s own doc comment for the
        // Sticky Scroll bug this caused for the very last symbol in such a
        // file).
        let total_lines = super::token::clamp_to_last_addressable_line(&text, u32::MAX);

        for token in super::token::flatten_listing(listing.iter()) {
            // source_len: byte length of the token as it appears in source
            // (for the selection range) — display_name: what the outline shows
            // pos_override: set only when the definition's own position isn't
            // `token.span()` (see the RANGE/DEFSECTION arm below)
            let (source_len, display_name, category, kind, detail, pos_override, is_local_label):
                TokenSymbolFacts = if token.is_label() {
                let raw = token.label_symbol();
                let is_local = raw.starts_with('.');
                let display = if is_local {
                    match &current_global {
                        Some(g) => format!("{}{}", g, raw),
                        None => raw.to_string()
                    }
                }
                else {
                    current_global = Some(raw.to_string());
                    raw.to_string()
                };
                (
                    raw.len(),
                    display,
                    SymbolCategory::Label,
                    SymbolKind::FUNCTION,
                    None,
                    None,
                    is_local
                )
            }
            else if token.is_equ() {
                let sym = token.equ_symbol();
                (
                    sym.len(),
                    sym.to_string(),
                    SymbolCategory::Constant,
                    SymbolKind::CONSTANT,
                    Some(format!("= {}", token.equ_value())),
                    None,
                    false
                )
            }
            else if token.is_assign() {
                let sym = token.assign_symbol();
                (
                    sym.len(),
                    sym.to_string(),
                    SymbolCategory::Variable,
                    SymbolKind::VARIABLE,
                    Some(format!("= {}", token.assign_value())),
                    None,
                    false
                )
            }
            else if token.is_macro_definition() {
                let name = token.macro_definition_name();
                current_global = Some(name.to_string());
                (
                    name.len(),
                    name.to_string(),
                    SymbolCategory::Macro,
                    SymbolKind::FUNCTION,
                    Some("MACRO".to_string()),
                    None,
                    false
                )
            }
            else if token.is_function_definition() {
                let name = token.function_definition_name();
                current_global = Some(name.to_string());
                (
                    name.len(),
                    name.to_string(),
                    SymbolCategory::Function,
                    SymbolKind::FUNCTION,
                    Some("FUNCTION".to_string()),
                    None,
                    false
                )
            }
            else if token.is_module() {
                let name = token.module_name();
                current_global = Some(name.to_string());
                (
                    name.len(),
                    name.to_string(),
                    SymbolCategory::Module,
                    SymbolKind::MODULE,
                    None,
                    None,
                    false
                )
            }
            else if token.is_directive() && super::token::starts_with_range_keyword(token) {
                // A section's *definition*: `RANGE start, stop, name` (or the
                // `DEFSECTION` alias) — the name is the last argument. Bare
                // `SECTION name` only *uses* an already-defined section (it
                // switches the current section for subsequent code), so it
                // doesn't get its own outline entry — same as how a `CALL`
                // to a label isn't a second definition of that label.
                //
                // `ListingElement` has no dedicated `is_range`/`range_*`
                // accessors, so this goes through `to_token()` instead;
                // `starts_with_range_keyword` keeps that call off the hot
                // path of ordinary directives — see its doc comment.
                match token.to_token().into_owned() {
                    Token::Range(name, start, stop) => {
                        // The token's own span points at the `RANGE`/
                        // `DEFSECTION` keyword, not at `name` (the last
                        // argument) — locate it within the statement so the
                        // outline entry (and goto-definition, which reuses
                        // this same extraction) highlights the actual name.
                        let pos = super::token::locate_name_in_statement(token, &name);
                        (
                            name.len(),
                            String::from(name),
                            SymbolCategory::Section,
                            SymbolKind::NAMESPACE,
                            Some(format!("{start}..{stop}")),
                            Some(pos),
                            false
                        )
                    },
                    _ => continue
                }
            }
            else {
                continue;
            };

            let (lsp_line, lsp_char) = pos_override.unwrap_or_else(|| {
                let span = token.span();
                let (line_1based, col_1based) = span.relative_line_and_column();
                (
                    line_1based.saturating_sub(1) as u32,
                    col_1based.saturating_sub(1) as u32
                )
            });
            // `selection_range` covers just the source token (the name),
            // not the (potentially longer) display name.
            let selection_range = Range {
                start: Position {
                    line: lsp_line,
                    character: lsp_char
                },
                end: Position {
                    line: lsp_line,
                    character: lsp_char + source_len as u32
                }
            };
            // `range` is the symbol's *full* extent - for a MACRO/FUNCTION/
            // MODULE, that's the whole body through its closing keyword, and
            // for a global (non-dotted) label, its whole scope through the
            // next global label. Editor features that key off a symbol's
            // real scope (VS Code's Sticky Scroll pins whichever symbol's
            // `range` contains the current scroll position) were badly
            // broken by `range == selection_range` here: every multi-line
            // symbol looked like a single-line one, so nothing ever
            // correctly matched the cursor's real position within one.
            // Local (dotted) labels and EQU/ASSIGN entries deliberately
            // keep the narrow, single-line `range == selection_range` -
            // they're not the kind of "container" Sticky Scroll should ever
            // pin. REPEAT/ITERATE/FOR loop bodies aren't tracked as outline
            // symbols at all (they have no user-given name the way MACRO/
            // FUNCTION/MODULE/global labels do, only an optional loop
            // counter variable) - so unlike those four, they were never
            // stuck with a *wrong* range; there's simply nothing there yet
            // to be wrong. Adding them would be new outline scope, not a
            // bug fix - not done here.
            let range = match category {
                SymbolCategory::Macro => {
                    let end_line = super::token::macro_body_end_line(&text, lsp_line);
                    Range {
                        start: selection_range.start,
                        end: Position {
                            line: end_line,
                            character: 0
                        }
                    }
                },
                SymbolCategory::Function => {
                    let end_line = super::token::function_body_end_line(&text, lsp_line);
                    Range {
                        start: selection_range.start,
                        end: Position {
                            line: end_line,
                            character: 0
                        }
                    }
                },
                SymbolCategory::Module => {
                    let end_line = super::token::module_body_end_line(&text, lsp_line);
                    Range {
                        start: selection_range.start,
                        end: Position {
                            line: end_line,
                            character: 0
                        }
                    }
                },
                SymbolCategory::Label if !is_local_label => {
                    let end_line = super::token::scope_containing(&global_scopes, lsp_line)
                        .map(|(_, scope)| scope.end.min(total_lines))
                        .unwrap_or(lsp_line + 1);
                    Range {
                        start: selection_range.start,
                        end: Position {
                            line: end_line,
                            character: 0
                        }
                    }
                },
                _ => selection_range
            };

            #[allow(deprecated)]
            let symbol = DocumentSymbol {
                name: display_name,
                detail,
                kind,
                tags: None,
                deprecated: None,
                range,
                selection_range,
                children: None
            };

            match category {
                SymbolCategory::Module => modules.push(symbol),
                SymbolCategory::Section => sections.push(symbol),
                SymbolCategory::Label => {
                    labels.push(if is_local_label {
                        LabelEntry::Local(symbol)
                    }
                    else {
                        LabelEntry::Global(symbol)
                    });
                },
                SymbolCategory::Macro => macros.push(symbol),
                SymbolCategory::Function => functions.push(symbol),
                SymbolCategory::Constant => constants.push(symbol),
                SymbolCategory::Variable => variables.push(symbol)
            }
        }

        let labels = nest_local_labels(listing.iter(), labels);

        let mut root = Vec::new();
        root.extend(modules);
        root.extend(sections);
        root.extend(labels);
        root.extend(macros);
        root.extend(functions);
        root.extend(constants);
        root.extend(variables);
        // Document order, not insertion order (which was kind-by-kind) - a
        // flat top-level list reads naturally in the Outline panel this way,
        // and Sticky Scroll has no grouping node left to get confused by.
        root.sort_by_key(|s| {
            (
                s.selection_range.start.line,
                s.selection_range.start.character
            )
        });
        root
    }
}

/// Restructure a flat, document-order list of label entries into a tree:
/// each local (dotted) label becomes a `children` entry of the global label
/// whose scope (per `token::global_label_scopes`) contains its own line,
/// extending that parent's `range` (never its `selection_range`, which must
/// stay the label's own tight name-span) to cover it.
///
/// A local with no enclosing global-label scope — e.g. the nearest
/// preceding entity is a `MACRO`/`MODULE` rather than a real label, or
/// there's no enclosing global at all — is left as a flat top-level entry
/// instead: `global_label_scopes` only treats non-dotted *labels* as scope
/// boundaries (matching `label_scope_at_line`, which backs rename's own
/// local-scope confinement), narrower than this module's own
/// `current_global` display-qualification tracker above (which also updates
/// on `MACRO`/`MODULE` names). Same known, deliberately accepted gap already
/// documented for `autocomplete.rs::scope_filtered_symbols`.
fn nest_local_labels<'a, T>(
    listing: impl IntoIterator<Item = &'a T> + 'a,
    labels: Vec<LabelEntry>
) -> Vec<DocumentSymbol>
where
    T: MayHaveSpan + ListingElement + 'a
{
    let scopes = super::token::global_label_scopes(listing);
    let mut top_level: Vec<DocumentSymbol> = Vec::new();
    let mut owner_index: HashMap<String, usize> = HashMap::new();

    for entry in labels {
        match entry {
            LabelEntry::Global(symbol) => {
                owner_index.insert(symbol.name.clone(), top_level.len());
                top_level.push(symbol);
            },
            LabelEntry::Local(symbol) => {
                let owner = super::token::scope_containing(&scopes, symbol.range.start.line)
                    .and_then(|(owner, _)| owner_index.get(&owner).copied());
                match owner {
                    Some(idx) => {
                        let parent = &mut top_level[idx];
                        crate::common::symbols::extend_range(&mut parent.range, &symbol.range);
                        parent.children.get_or_insert_with(Vec::new).push(symbol);
                    },
                    None => top_level.push(symbol)
                }
            }
        }
    }

    top_level
}

/// Recursively find a symbol named `name` anywhere in the tree (including
/// nested `children`) — every test below cares whether a symbol exists and
/// what it looks like, not which container/nesting depth it landed at.
#[cfg(test)]
fn find_symbol<'a>(symbols: &'a [DocumentSymbol], name: &str) -> Option<&'a DocumentSymbol> {
    for s in symbols {
        if s.name == name {
            return Some(s);
        }
        if let Some(children) = &s.children
            && let Some(found) = find_symbol(children, name)
        {
            return Some(found);
        }
    }
    None
}

/// Recursively count symbols named `name` anywhere in the tree.
#[cfg(test)]
fn count_symbol(symbols: &[DocumentSymbol], name: &str) -> usize {
    symbols
        .iter()
        .map(|s| {
            (s.name == name) as usize
                + s.children
                    .as_deref()
                    .map(|c| count_symbol(c, name))
                    .unwrap_or(0)
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::document::Document;

    fn symbols_for(text: &str) -> Vec<DocumentSymbol> {
        let uri = Url::parse("file:///t.asm").unwrap();
        let doc = Document::new(uri, text.to_string(), 1);
        AssemblyAnalyzer::new().document_symbols(&doc)
    }

    #[test]
    fn range_directive_defines_a_section_symbol() {
        let text = "RANGE 0x4000, 0x8000, MY_SECTION\n    ret\n";
        let symbols = symbols_for(text);
        let section = find_symbol(&symbols, "MY_SECTION").expect("section symbol");
        assert_eq!(section.kind, SymbolKind::NAMESPACE);
        assert!(
            section.detail.as_deref().unwrap_or("").contains("4000"),
            "{:?}",
            section.detail
        );
    }

    #[test]
    fn bare_section_usage_is_not_a_second_definition() {
        let text = "RANGE 0x4000, 0x8000, MY_SECTION\nSECTION MY_SECTION\n    ret\n";
        let symbols = symbols_for(text);
        assert_eq!(count_symbol(&symbols, "MY_SECTION"), 1, "{symbols:?}");
    }

    #[test]
    fn top_level_symbols_appear_flat_in_document_order() {
        // A variable, then a module (with a top-level label inside it -
        // labels nest under their *global label* parent, never under an
        // enclosing MODULE, so `my_label` still lands as a top-level entry
        // here, right after `mymod`).
        let text = "SOME_VAR = 1\nMODULE mymod\nmy_label:\n    ret\nENDMODULE\n";
        let symbols = symbols_for(text);
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["SOME_VAR", "mymod", "my_label"], "{symbols:?}");
        // No synthetic grouping node wraps any of them (a grouping node's
        // own `range` broke Sticky Scroll no matter how it was computed -
        // see the module doc comment for the full story).
        assert!(symbols.iter().all(|s| s.kind != SymbolKind::NAMESPACE));
    }

    /// Regression test for a real, user-reported Sticky Scroll bug: two EQU
    /// constants far apart in the document used to be wrapped in a synthetic
    /// "Constants" grouping node whose own `range` spanned the whole
    /// distance between them, making VS Code treat *the grouping itself* as
    /// a real, wide container to pin while scrolled anywhere in between -
    /// showing the first constant's own line stuck long after it (and its
    /// real enclosing symbol, if any) had scrolled off screen. Fixed by not
    /// introducing that grouping node at all - each constant is a direct,
    /// narrow-ranged top-level entry.
    #[test]
    fn far_apart_constants_never_share_a_wide_synthetic_container() {
        let text = "FOO equ 1\nmain:\n    nop\n    nop\n    nop\nBAR equ 2\n";
        let symbols = symbols_for(text);
        assert!(
            symbols.iter().all(|s| s.kind != SymbolKind::NAMESPACE),
            "{symbols:?}"
        );

        let foo = find_symbol(&symbols, "FOO").expect("FOO");
        assert_eq!(foo.range, foo.selection_range, "{foo:?}");
        assert_eq!(foo.range.start.line, 0, "{foo:?}");

        let bar = find_symbol(&symbols, "BAR").expect("BAR");
        assert_eq!(bar.range, bar.selection_range, "{bar:?}");
        assert_eq!(bar.range.start.line, 5, "{bar:?}");
    }

    /// Regression test: `document_symbols` used to call `to_token()` for
    /// *every* directive once it fell through the label/equ/assign/macro/
    /// module checks, and `to_token()` has an unimplemented (`todo!()`)
    /// fallback for several directive variants — so any real file mixing
    /// ordinary directives like `ORG`/`DB`/`IF`/`INCLUDE` with labels would
    /// panic and the outline would come back empty. This must not panic and
    /// must still find `main`.
    #[test]
    fn ordinary_directives_do_not_break_the_outline() {
        let text = "    org 0x8000\nmain:\n    ifdef DEBUG\n    db 1,2,3\n    endif\n    include \"foo.asm\"\n    ret\n";
        let symbols = symbols_for(text);
        assert!(find_symbol(&symbols, "main").is_some(), "{symbols:?}");
    }

    /// Regression test for a real user report: VS Code's Sticky Scroll (and
    /// any other editor feature that keys off `DocumentSymbol.range` to
    /// know a symbol's real extent) was showing the wrong line, nothing at
    /// all for the last macro in a file, or a line even when the cursor
    /// wasn't inside any macro - all symptoms of `range` being collapsed to
    /// just the declaration line (identical to `selection_range`) instead
    /// of spanning the whole macro body through `ENDM`.

    #[test]
    fn macro_symbol_range_spans_the_whole_body_not_just_the_declaration_line() {
        let text = "MACRO foo\n    nop\n    nop\nENDM\n\nMACRO bar\n    ret\nENDM\n";
        let symbols = symbols_for(text);

        let foo = find_symbol(&symbols, "foo").unwrap();
        assert_eq!(foo.range.start.line, 0);
        assert_eq!(foo.range.end.line, 4, "{foo:?}"); // just past the "ENDM" line (3)
        // `selection_range` stays the narrow name-only span.
        assert_eq!(foo.selection_range.start.line, 0);
        assert_eq!(foo.selection_range.end.line, 0);

        // The *last* macro in the file must also get a real range, not an
        // empty/missing one.
        let bar = find_symbol(&symbols, "bar").unwrap();
        assert_eq!(bar.range.start.line, 5);
        assert_eq!(bar.range.end.line, 8, "{bar:?}");
    }

    /// Regression test for a real, user-reported Sticky Scroll bug: the
    /// *last* macro in a file that doesn't end with a trailing newline
    /// (`macros.asm` from a real project - a common, unremarkable file
    /// shape, not an edge case anyone deliberately created) used to get a
    /// `range.end` one line *past* what the editor considers to exist -
    /// VS Code's Sticky Scroll silently failed for it, while the Outline
    /// panel (which only needs the much narrower `selection_range`) showed
    /// it just fine, which was the confusing part of the report.
    #[test]
    fn last_macro_range_stays_in_bounds_when_the_file_has_no_trailing_newline() {
        let text = "MACRO foo\n    nop\nENDM"; // deliberately no trailing "\n"
        let symbols = symbols_for(text);
        let foo = find_symbol(&symbols, "foo").unwrap();
        // 3 lines total (0, 1, 2) - `range.end.line` must never reach 3,
        // which the editor has no line at.
        assert_eq!(foo.range.end.line, 2, "{foo:?}");
    }

    /// MODULE gets the same real-range treatment as MACRO/FUNCTION (Sticky
    /// Scroll needs to pin its declaration line the same way) - a real bug
    /// found while checking whether this same `range == selection_range`
    /// pattern also affected MODULE/REPEAT/ITERATE: MODULE did (it's a
    /// tracked outline symbol, same as MACRO/FUNCTION, just missing the
    /// range-widening fix); REPEAT/ITERATE don't (they aren't tracked as
    /// outline symbols at all - see the module doc comment above the
    /// `range` match for why that's not a bug to fix here).
    #[test]
    fn module_symbol_range_spans_the_whole_body_not_just_the_declaration_line() {
        let text = "MODULE foo\n    nop\n    nop\nENDMODULE\n\nMODULE bar\n    ret\nENDMODULE\n";
        let symbols = symbols_for(text);

        let foo = find_symbol(&symbols, "foo").unwrap();
        assert_eq!(foo.range.start.line, 0);
        assert_eq!(foo.range.end.line, 4, "{foo:?}"); // just past "ENDMODULE"
        assert_eq!(foo.selection_range.start.line, 0);
        assert_eq!(foo.selection_range.end.line, 0);

        // The *last* module in the file must also get a real range.
        let bar = find_symbol(&symbols, "bar").unwrap();
        assert_eq!(bar.range.start.line, 5);
        assert_eq!(bar.range.end.line, 8, "{bar:?}");
    }

    /// FUNCTION gets the same real-range treatment as MACRO (Sticky Scroll
    /// needs to pin its declaration line the same way), including the
    /// `ENDF` closing alias alongside `ENDFUNCTION`.
    #[test]
    fn function_symbol_range_spans_the_whole_body_not_just_the_declaration_line() {
        let text = "FUNCTION sq(x)\n    RETURN x*x\nENDFUNCTION\n\nFUNCTION cube(x)\n    RETURN x*x*x\nENDF\n";
        let symbols = symbols_for(text);

        let sq = find_symbol(&symbols, "sq").unwrap();
        assert_eq!(sq.range.start.line, 0);
        assert_eq!(sq.range.end.line, 3, "{sq:?}"); // just past "ENDFUNCTION"
        assert_eq!(sq.selection_range.start.line, 0);
        assert_eq!(sq.selection_range.end.line, 0);

        let cube = find_symbol(&symbols, "cube").unwrap();
        assert_eq!(cube.range.start.line, 4);
        assert_eq!(cube.range.end.line, 7, "{cube:?}"); // just past "FEND"
    }

    /// A global label's `range` extends through the next global label (or
    /// end of file for the last one) so Sticky Scroll can pin "the latest
    /// global label" while scrolled through its body - but local (dotted)
    /// labels and EQU/ASSIGN entries must keep their own narrow,
    /// single-line range: they aren't containers Sticky Scroll should pin.
    #[test]
    fn global_label_range_extends_to_the_next_global_label_or_eof_but_locals_and_equ_stay_narrow() {
        let text = "start:\n  FOO equ 1\n  BAR = 2\n  nop\nfinish:\n  ret\n  .loop\n  jr .loop\n";
        let symbols = symbols_for(text);

        let start = find_symbol(&symbols, "start").unwrap();
        assert_eq!(start.range.start.line, 0);
        assert_eq!(start.range.end.line, 4, "{start:?}"); // up to "finish:"

        // The *last* global label extends to end of file, not nothing/zero.
        let finish = find_symbol(&symbols, "finish").unwrap();
        assert_eq!(finish.range.start.line, 4);
        assert_eq!(finish.range.end.line, 8, "{finish:?}"); // total line count

        let loop_local = finish
            .children
            .as_ref()
            .expect("finish has locals")
            .iter()
            .find(|s| s.name == "finish.loop")
            .unwrap();
        assert_eq!(
            loop_local.range, loop_local.selection_range,
            "a local label must keep a narrow, single-line range: {loop_local:?}"
        );

        let foo = find_symbol(&symbols, "FOO").unwrap();
        assert_eq!(
            foo.range, foo.selection_range,
            "an EQU constant must keep a narrow, single-line range: {foo:?}"
        );

        let bar = find_symbol(&symbols, "BAR").unwrap();
        assert_eq!(
            bar.range, bar.selection_range,
            "an ASSIGN variable must keep a narrow, single-line range: {bar:?}"
        );
    }

    /// Same real-world bug as
    /// `last_macro_range_stays_in_bounds_when_the_file_has_no_trailing_newline`,
    /// for the *last global label* case instead of a macro: without a
    /// trailing newline, its "extends to end of file" range must stay in
    /// bounds rather than reaching a line the editor has no line at.
    #[test]
    fn last_global_label_range_stays_in_bounds_when_the_file_has_no_trailing_newline() {
        let text = "start:\n  nop\n  ret"; // deliberately no trailing "\n"
        let symbols = symbols_for(text);
        let start = find_symbol(&symbols, "start").unwrap();
        // 3 lines total (0, 1, 2) - `range.end.line` must never reach 3.
        assert_eq!(start.range.end.line, 2, "{start:?}");
    }

    #[test]
    fn range_directive_alongside_ordinary_directives_still_defines_a_section() {
        let text =
            "    org 0x8000\nRANGE 0x4000, 0x8000, MY_SECTION\nmain:\n    db 1,2,3\n    ret\n";
        let symbols = symbols_for(text);
        assert!(find_symbol(&symbols, "main").is_some(), "{symbols:?}");
        let section = find_symbol(&symbols, "MY_SECTION").expect("section symbol");
        assert_eq!(section.kind, SymbolKind::NAMESPACE);
    }

    #[test]
    fn local_labels_nest_under_their_owning_global_label_without_cross_contamination() {
        let text = "global1\n.g1l1\n.g1l2\nglobal2\n.g2l1\n";
        let symbols = symbols_for(text);
        // Locals are nested under their own global, not flat top-level
        // entries.
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["global1", "global2"], "{symbols:?}");

        let global1 = &symbols[0];
        let g1_children = global1.children.as_ref().expect("global1 has locals");
        let g1_names: Vec<&str> = g1_children.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            g1_names,
            vec!["global1.g1l1", "global1.g1l2"],
            "{g1_children:?}"
        );
        // A global label's range now extends through its whole scope (up
        // to the next global label), not just far enough to cover its own
        // locals.
        assert_eq!(
            global1.range.start,
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            global1.range.end,
            Position {
                line: 3,
                character: 0
            }
        );

        let global2 = &symbols[1];
        let g2_children = global2.children.as_ref().expect("global2 has locals");
        let g2_names: Vec<&str> = g2_children.iter().map(|s| s.name.as_str()).collect();
        // global1's locals never leak into global2's own children.
        assert_eq!(g2_names, vec!["global2.g2l1"], "{g2_children:?}");
    }

    #[test]
    fn a_local_label_with_no_enclosing_global_scope_is_not_nested_but_still_appears() {
        // `.foo` appears before any global label at all - `scope_containing`
        // returns `None` here (matching `token.rs`'s own
        // `scope_containing_returns_none_before_the_first_label` test), so
        // it must still show up, flat, at the top level - not silently
        // dropped.
        let text = ".foo\nglobal1\n";
        let symbols = symbols_for(text);
        assert!(symbols.iter().any(|s| s.name == ".foo"), "{symbols:?}");
        // Not nested under global1 either, even though it's the only other
        // label in the file - global1 comes textually *after* it, so can't
        // be its owner.
        let global1 = symbols.iter().find(|s| s.name == "global1").unwrap();
        assert!(
            global1
                .children
                .as_ref()
                .is_none_or(|c| !c.iter().any(|s| s.name == ".foo")),
            "{global1:?}"
        );
    }

    #[test]
    fn only_the_symbols_that_actually_exist_appear() {
        let text = "main:\n    ret\n";
        let symbols = symbols_for(text);
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["main"], "{symbols:?}");
    }
}

#[cfg(test)]
mod multi_statement_line_tests {
    use super::*;
    use crate::common::document::Document;

    fn symbols_for(text: &str) -> Vec<DocumentSymbol> {
        let uri = Url::parse("file:///t.asm").unwrap();
        let doc = Document::new(uri, text.to_string(), 1);
        AssemblyAnalyzer::new().document_symbols(&doc)
    }

    /// A `RANGE`/`DEFSECTION` (or any directive) doesn't necessarily start at
    /// column 0 — several statements can share one physical line via `:`,
    /// and a `/* ... */` block comment can precede it too. `token.span()`
    /// (used by `starts_with_range_keyword`/`locate_name_in_statement`) is
    /// anchored to the token's own real start position, as parsed, not to
    /// the start of its physical line, so both must still work.
    #[test]
    fn range_after_a_colon_separated_statement_is_still_found() {
        let text = "    LD A,1 : RANGE 0x4000, 0x8000, MY_SECTION\n    ret\n";
        let symbols = symbols_for(text);
        let section = find_symbol(&symbols, "MY_SECTION")
            .expect("section symbol after a `:`-separated statement");
        assert_eq!(section.kind, SymbolKind::NAMESPACE);
        // The highlighted range must be "MY_SECTION" itself, not swallow the
        // "LD A,1 : RANGE ..." prefix that precedes it on the line.
        assert_eq!(
            section.range.end.character - section.range.start.character,
            "MY_SECTION".len() as u32
        );
    }

    #[test]
    fn range_after_a_block_comment_is_still_found() {
        let text = "/* comment */ RANGE 0x4000, 0x8000, OTHER_SECTION\n";
        let symbols = symbols_for(text);
        let section =
            find_symbol(&symbols, "OTHER_SECTION").expect("section symbol after a block comment");
        assert_eq!(section.kind, SymbolKind::NAMESPACE);
        assert_eq!(
            section.range.end.character - section.range.start.character,
            "OTHER_SECTION".len() as u32
        );
    }
}
