//! Document symbols (outline) for assembly files: labels, EQU/assign
//! constants, macros, modules, sections; local labels nested under their
//! parent. The outline is grouped by kind (modules, sections, labels,
//! macros, EQU constants, assign variables, in that order), preserving
//! document order within each group.

use cpclib_asm::parser::obtained::MayHaveSpan;
use cpclib_tokens::{ListingElement, Token};
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use crate::common::document::Document;

/// Grouping key for the outline — see the module doc comment for the order.
/// Not exposed over LSP; only used to sort the flat symbol list before
/// returning it (the sort is stable, so document order survives within a
/// group).
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum SymbolCategory {
    Module,
    Section,
    Label,
    Macro,
    Constant,
    Variable
}

impl AssemblyAnalyzer {
    pub fn document_symbols(&self, document: &Document) -> Vec<DocumentSymbol> {
        let mut symbols: Vec<(SymbolCategory, DocumentSymbol)> = Vec::new();

        let Ok(listing) = self.parse_document(document)
        else {
            return Vec::new();
        };

        // Track the last seen global label to qualify local labels (`.foo` → `parent.foo`)
        let mut current_global: Option<String> = None;

        for token in super::token::flatten_listing(listing.iter()) {
            // source_len: byte length of the token as it appears in source
            // (for the selection range) — display_name: what the outline shows
            // pos_override: set only when the definition's own position isn't
            // `token.span()` (see the RANGE/DEFSECTION arm below)
            let (source_len, display_name, category, kind, detail, pos_override): (
                usize,
                String,
                SymbolCategory,
                SymbolKind,
                Option<String>,
                Option<(u32, u32)>
            ) = if token.is_label() {
                let raw = token.label_symbol();
                let display = if raw.starts_with('.') {
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
                    None
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
                    None
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
                    None
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
                    None
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
                    None
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
                            Some(pos)
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
            symbols.push((
                category,
                DocumentSymbol {
                    name: display_name,
                    detail,
                    kind,
                    tags: None,
                    deprecated: None,
                    range: range.clone(),
                    selection_range: range,
                    children: None
                }
            ));
        }

        symbols.sort_by(|a, b| a.0.cmp(&b.0));
        symbols.into_iter().map(|(_, sym)| sym).collect()
    }
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
        let section = symbols
            .iter()
            .find(|s| s.name == "MY_SECTION")
            .expect("section symbol");
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
        let count = symbols.iter().filter(|s| s.name == "MY_SECTION").count();
        assert_eq!(count, 1, "{symbols:?}");
    }

    #[test]
    fn outline_is_grouped_by_kind_not_document_order() {
        // Deliberately out of "natural" order: variable, then module (with a
        // nested label), so a plain document-order walk would list them
        // VARIABLE, MODULE, LABEL — kind-grouping must reorder them.
        let text = "SOME_VAR = 1\nMODULE mymod\nmy_label:\n    ret\nENDMODULE\n";
        let symbols = symbols_for(text);
        let kinds: Vec<SymbolKind> = symbols.iter().map(|s| s.kind).collect();
        assert!(kinds.contains(&SymbolKind::MODULE), "{symbols:?}");
        assert!(kinds.contains(&SymbolKind::FUNCTION), "{symbols:?}");
        assert!(kinds.contains(&SymbolKind::VARIABLE), "{symbols:?}");

        let module_idx = kinds.iter().position(|k| *k == SymbolKind::MODULE).unwrap();
        let label_idx = kinds
            .iter()
            .position(|k| *k == SymbolKind::FUNCTION)
            .unwrap();
        let variable_idx = kinds
            .iter()
            .position(|k| *k == SymbolKind::VARIABLE)
            .unwrap();
        assert!(module_idx < label_idx, "{symbols:?}");
        assert!(label_idx < variable_idx, "{symbols:?}");
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
        assert!(symbols.iter().any(|s| s.name == "main"), "{symbols:?}");
    }

    #[test]
    fn range_directive_alongside_ordinary_directives_still_defines_a_section() {
        let text =
            "    org 0x8000\nRANGE 0x4000, 0x8000, MY_SECTION\nmain:\n    db 1,2,3\n    ret\n";
        let symbols = symbols_for(text);
        assert!(symbols.iter().any(|s| s.name == "main"), "{symbols:?}");
        let section = symbols
            .iter()
            .find(|s| s.name == "MY_SECTION")
            .expect("section symbol");
        assert_eq!(section.kind, SymbolKind::NAMESPACE);
    }
}
