//! Document symbols (outline) for assembly files: a real nested tree, one
//! top-level container per symbol kind present (modules, sections, labels,
//! macros, EQU constants, assign variables, in that order — empty kinds are
//! omitted), preserving document order within each. Local labels nest under
//! the global label whose scope contains them.

use std::collections::HashMap;

use cpclib_asm::parser::obtained::MayHaveSpan;
use cpclib_tokens::{ListingElement, Token};
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use crate::common::document::Document;
use crate::common::symbols::container_symbol;

/// Grouping key for the outline — see the module doc comment for the order.
/// Not exposed over LSP; only used to decide which top-level container each
/// symbol lands in, and the order those containers themselves appear in.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum SymbolCategory {
    Module,
    Section,
    Label,
    Macro,
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

impl AssemblyAnalyzer {
    pub fn document_symbols(&self, document: &Document) -> Vec<DocumentSymbol> {
        let Ok(listing) = self.parse_document(document)
        else {
            return Vec::new();
        };

        let mut modules: Vec<DocumentSymbol> = Vec::new();
        let mut sections: Vec<DocumentSymbol> = Vec::new();
        let mut labels: Vec<LabelEntry> = Vec::new();
        let mut macros: Vec<DocumentSymbol> = Vec::new();
        let mut constants: Vec<DocumentSymbol> = Vec::new();
        let mut variables: Vec<DocumentSymbol> = Vec::new();

        // Track the last seen global label to qualify local labels (`.foo` → `parent.foo`)
        let mut current_global: Option<String> = None;

        for token in super::token::flatten_listing(listing.iter()) {
            // source_len: byte length of the token as it appears in source
            // (for the selection range) — display_name: what the outline shows
            // pos_override: set only when the definition's own position isn't
            // `token.span()` (see the RANGE/DEFSECTION arm below)
            let (source_len, display_name, category, kind, detail, pos_override, is_local_label): (
                usize,
                String,
                SymbolCategory,
                SymbolKind,
                Option<String>,
                Option<(u32, u32)>,
                bool
            ) = if token.is_label() {
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
                            name,
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
            // Range covers the source token, not the (potentially longer) display name
            let range = Range {
                start: Position {
                    line: lsp_line,
                    character: lsp_char
                },
                end: Position {
                    line: lsp_line,
                    character: lsp_char + source_len as u32
                }
            };

            #[allow(deprecated)]
            let symbol = DocumentSymbol {
                name: display_name,
                detail,
                kind,
                tags: None,
                deprecated: None,
                range,
                selection_range: range,
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
                SymbolCategory::Constant => constants.push(symbol),
                SymbolCategory::Variable => variables.push(symbol)
            }
        }

        let labels = nest_local_labels(listing.iter(), labels);

        let mut root = Vec::new();
        if !modules.is_empty() {
            root.push(container_symbol("Modules", modules));
        }
        if !sections.is_empty() {
            root.push(container_symbol("Sections", sections));
        }
        if !labels.is_empty() {
            root.push(container_symbol("Labels", labels));
        }
        if !macros.is_empty() {
            root.push(container_symbol("Macros", macros));
        }
        if !constants.is_empty() {
            root.push(container_symbol("Constants", constants));
        }
        if !variables.is_empty() {
            root.push(container_symbol("Variables", variables));
        }
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
    fn outline_is_grouped_by_kind_not_document_order() {
        // Deliberately out of "natural" order: variable, then module (with a
        // top-level label — labels nest under their *global label* parent,
        // never under an enclosing MODULE), so a plain document-order walk
        // would list top-level containers VARIABLE, MODULE, LABEL —
        // kind-grouping must reorder the containers themselves.
        let text = "SOME_VAR = 1\nMODULE mymod\nmy_label:\n    ret\nENDMODULE\n";
        let symbols = symbols_for(text);
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["Modules", "Labels", "Variables"], "{symbols:?}");
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
        let labels = find_symbol(&symbols, "Labels").expect("Labels container");
        let top_level = labels.children.as_ref().expect("Labels has children");

        // Locals are nested under their own global, not flat siblings of it.
        let names: Vec<&str> = top_level.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["global1", "global2"], "{top_level:?}");

        let global1 = &top_level[0];
        let g1_children = global1.children.as_ref().expect("global1 has locals");
        let g1_names: Vec<&str> = g1_children.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            g1_names,
            vec!["global1.g1l1", "global1.g1l2"],
            "{g1_children:?}"
        );
        // The parent's range grew to cover both its own locals.
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
                line: 2,
                character: 5
            }
        );

        let global2 = &top_level[1];
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
        // it must still show up, flat, in "Labels" - not silently dropped.
        let text = ".foo\nglobal1\n";
        let symbols = symbols_for(text);
        let labels = find_symbol(&symbols, "Labels").expect("Labels container");
        let top_level = labels.children.as_ref().expect("Labels has children");
        assert!(top_level.iter().any(|s| s.name == ".foo"), "{top_level:?}");
        // Not nested under global1 either, even though it's the only other
        // label in the file - global1 comes textually *after* it, so can't
        // be its owner.
        let global1 = top_level.iter().find(|s| s.name == "global1").unwrap();
        assert!(
            global1
                .children
                .as_ref()
                .is_none_or(|c| !c.iter().any(|s| s.name == ".foo")),
            "{global1:?}"
        );
    }

    #[test]
    fn a_category_with_no_symbols_is_omitted_from_the_tree() {
        let text = "main:\n    ret\n";
        let symbols = symbols_for(text);
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["Labels"], "{symbols:?}");
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
