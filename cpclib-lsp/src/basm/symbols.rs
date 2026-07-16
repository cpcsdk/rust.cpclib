//! Document symbols (outline) for assembly files: labels, EQU/assign
//! constants, macros, modules; local labels nested under their parent.

use cpclib_asm::parser::obtained::MayHaveSpan;
use cpclib_tokens::ListingElement;
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use crate::common::document::Document;

impl AssemblyAnalyzer {
    pub fn document_symbols(&self, document: &Document) -> Vec<DocumentSymbol> {
        let mut symbols = Vec::new();

        let Ok(listing) = self.parse_document(document)
        else {
            return symbols;
        };

        // Track the last seen global label to qualify local labels (`.foo` → `parent.foo`)
        let mut current_global: Option<String> = None;

        for token in listing.iter() {
            // source_name: as it appears in source (for range length)
            // display_name: what the outline shows
            let (source_name, display_name, kind, detail): (
                &str,
                String,
                SymbolKind,
                Option<String>
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
                (raw, display, SymbolKind::FUNCTION, None)
            }
            else if token.is_equ() {
                let sym = token.equ_symbol();
                (
                    sym,
                    sym.to_string(),
                    SymbolKind::CONSTANT,
                    Some(format!("= {}", token.equ_value()))
                )
            }
            else if token.is_assign() {
                let sym = token.assign_symbol();
                (
                    sym,
                    sym.to_string(),
                    SymbolKind::VARIABLE,
                    Some(format!("= {}", token.assign_value()))
                )
            }
            else if token.is_macro_definition() {
                let name = token.macro_definition_name();
                current_global = Some(name.to_string());
                (
                    name,
                    name.to_string(),
                    SymbolKind::FUNCTION,
                    Some("MACRO".to_string())
                )
            }
            else if token.is_module() {
                let name = token.module_name();
                current_global = Some(name.to_string());
                (name, name.to_string(), SymbolKind::MODULE, None)
            }
            else {
                continue;
            };

            let span = token.span();
            let (line_1based, col_1based) = span.relative_line_and_column();
            let lsp_line = line_1based.saturating_sub(1) as u32;
            let lsp_char = col_1based.saturating_sub(1) as u32;
            // Range covers the source token, not the (potentially longer) display name
            let range = Range {
                start: Position {
                    line: lsp_line,
                    character: lsp_char
                },
                end: Position {
                    line: lsp_line,
                    character: lsp_char + source_name.len() as u32
                }
            };

            #[allow(deprecated)]
            symbols.push(DocumentSymbol {
                name: display_name,
                detail,
                kind,
                tags: None,
                deprecated: None,
                range: range.clone(),
                selection_range: range,
                children: None
            });
        }

        symbols
    }
}
