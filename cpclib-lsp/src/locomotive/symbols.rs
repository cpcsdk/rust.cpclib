//! Document symbols (outline) for Locomotive BASIC programs.

use cpclib_basic::located::LocatedBasicProgram;
use tower_lsp::lsp_types::*;

use super::BasicAnalyzer;
use super::token::collect_variable_occurrences;
use crate::common::document::Document;

impl BasicAnalyzer {
    pub fn document_symbols(&self, document: &Document) -> Vec<DocumentSymbol> {
        let text = document.text();
        let prog = match LocatedBasicProgram::parse(&text) {
            Ok(p) => p,
            Err(_) => return vec![]
        };

        // key → (original_name, line, col) of the first "assignment context"
        // occurrence: LET/FOR target, INPUT/READ target, or bare `NAME = ...`.
        let seen = collect_variable_occurrences(&prog);

        let mut entries: Vec<(String, String, u32, u32)> = seen
            .into_values()
            .map(|(orig, line, col)| (orig.to_uppercase(), orig, line, col))
            .collect();
        entries.sort_by(|a, b| a.2.cmp(&b.2).then(a.3.cmp(&b.3)));

        entries
            .into_iter()
            .map(|(_, name, line_idx, col)| {
                let end_char = col + name.len() as u32;
                let pos = Range {
                    start: Position {
                        line: line_idx,
                        character: col
                    },
                    end: Position {
                        line: line_idx,
                        character: end_char
                    }
                };
                #[allow(deprecated)]
                DocumentSymbol {
                    name,
                    detail: None,
                    kind: SymbolKind::VARIABLE,
                    tags: None,
                    deprecated: None,
                    range: pos,
                    selection_range: pos,
                    children: None
                }
            })
            .collect()
    }
}
