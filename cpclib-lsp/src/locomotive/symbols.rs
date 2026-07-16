//! Document symbols (outline) for Locomotive BASIC programs.

use std::collections::HashMap;

use cpclib_basic::located::{LocatedBasicProgram, LocatedTokenKind};
use cpclib_basic::tokens::BasicTokenNoPrefix;
use tower_lsp::lsp_types::*;

use super::BasicAnalyzer;
use super::diagnostics::{collect_comma_separated_vars, collect_vars_after_input, record_var};
use super::token::*;
use crate::common::document::Document;

impl BasicAnalyzer {
    pub fn document_symbols(&self, document: &Document) -> Vec<DocumentSymbol> {
        let text = document.text();
        let prog = match LocatedBasicProgram::parse(&text) {
            Ok(p) => p,
            Err(_) => return vec![]
        };

        // Track first-assignment location for each variable (key: uppercase).
        // "Assignment context" means:
        //   - After LET  → variable immediately follows
        //   - After FOR  → variable immediately follows
        //   - After INPUT / READ → one or more variables follow
        //   - Bare assignment: variable is followed (through spaces) by `=`
        let mut seen: HashMap<String, (String, u32, u32)> = HashMap::new(); // key→(original_name, line, col)

        for bline in &prog.lines {
            let toks = &bline.tokens;
            let n = toks.len();
            let mut i = 0;

            while i < n {
                let tok = &toks[i];
                match &tok.kind {
                    LocatedTokenKind::Keyword(BasicTokenNoPrefix::Let) => {
                        // Skip spaces, then expect variable.
                        if let Some(var_tok) = skip_spaces_then_var(toks, i + 1) {
                            record_var(&mut seen, var_tok);
                        }
                        i += 1;
                    },
                    LocatedTokenKind::Keyword(BasicTokenNoPrefix::For) => {
                        if let Some(var_tok) = skip_spaces_then_var(toks, i + 1) {
                            record_var(&mut seen, var_tok);
                        }
                        i += 1;
                    },
                    LocatedTokenKind::Keyword(BasicTokenNoPrefix::Input) => {
                        // Collect all variables that follow (skipping prompt string).
                        collect_vars_after_input(toks, i + 1, &mut seen);
                        i += 1;
                    },
                    LocatedTokenKind::Keyword(BasicTokenNoPrefix::Read) => {
                        collect_comma_separated_vars(toks, i + 1, &mut seen);
                        i += 1;
                    },
                    LocatedTokenKind::Variable(name) => {
                        // Bare assignment: var followed by optional spaces then `=`.
                        if is_followed_by_eq(toks, i + 1) {
                            let key = name.to_uppercase();
                            seen.entry(key)
                                .or_insert_with(|| (name.clone(), tok.span.line, tok.span.col));
                        }
                        i += 1;
                    },
                    _ => {
                        i += 1;
                    }
                }
            }
        }

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
