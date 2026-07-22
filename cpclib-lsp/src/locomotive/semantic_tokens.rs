//! Semantic tokens (syntax highlighting) for Locomotive BASIC.

use cpclib_basic::located::LocatedTokenKind;
use tower_lsp::lsp_types::*;

use super::BasicAnalyzer;
use super::token::*;
use crate::common::document::Document;

impl BasicAnalyzer {
    pub fn semantic_tokens(&self, document: &Document) -> Vec<SemanticToken> {
        let prog = match self.parse_cached(document) {
            Ok(p) => p,
            Err(_) => return vec![]
        };

        let mut result: Vec<SemanticToken> = Vec::new();
        let mut prev_line: u32 = 0;
        let mut prev_col: u32 = 0;

        for bline in &prog.lines {
            for tok in &bline.tokens {
                let tt = match &tok.kind {
                    LocatedTokenKind::Keyword(_) => TT_KEYWORD,
                    LocatedTokenKind::Function(_) => TT_FUNCTION,
                    LocatedTokenKind::Variable(_) => TT_VARIABLE,
                    LocatedTokenKind::Number(_) => TT_NUMBER,
                    LocatedTokenKind::StringLit(_) => TT_STRING,
                    LocatedTokenKind::Comment(_) => TT_COMMENT,
                    LocatedTokenKind::Operator(_) => TT_OPERATOR,
                    LocatedTokenKind::LineNumber(_) => TT_NUMBER,
                    // Skip Space, Separator, Other
                    _ => continue
                };

                if tok.span.len == 0 {
                    continue;
                }

                let (delta_line, delta_start) = if tok.span.line == prev_line {
                    (0, tok.span.col - prev_col)
                }
                else {
                    (tok.span.line - prev_line, tok.span.col)
                };

                result.push(SemanticToken {
                    delta_line,
                    delta_start,
                    length: tok.span.len,
                    token_type: tt,
                    token_modifiers_bitset: 0
                });

                prev_line = tok.span.line;
                prev_col = tok.span.col;
            }
        }

        result
    }
}
