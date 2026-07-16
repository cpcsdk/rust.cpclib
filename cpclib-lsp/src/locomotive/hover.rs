//! Hover for Locomotive BASIC: keyword documentation and numeric literals.
//!
//! `locomotive_basic_hover` is `pub(crate)` because the basm module reuses it
//! for BASIC blocks embedded in assembly.

use cpclib_basic::BasicProgram;
use cpclib_basic::located::{LocatedBasicProgram, LocatedTokenKind};
use tower_lsp::lsp_types::*;

use super::BasicAnalyzer;
use super::token::*;
use crate::common::document::Document;

impl BasicAnalyzer {
    pub fn hover(&self, document: &Document, position: Position) -> Option<Hover> {
        let source_line = document.line(position.line as usize)?;
        let col = position.character as usize;
        let line = source_line.trim_end_matches(|c| c == '\n' || c == '\r');

        // Try keyword hover first (alphabetic words).
        if let Some(word_upper) = alpha_word_at(line, col).map(|w| w.to_uppercase()) {
            if let Some(&(_, doc)) = KEYWORD_DOCS
                .iter()
                .find(|(kw, _)| kw.to_uppercase() == word_upper)
            {
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: doc.to_string()
                    }),
                    range: None
                });
            }
        }

        // Number hover: find a Number token at the cursor and show base conversions.
        let text = document.text();
        let prog = LocatedBasicProgram::parse(&text).ok()?;
        let bline = prog.lines.iter().find(|l| l.source_line == position.line)?;
        let tok = bline.tokens.iter().find(|t| {
            t.span.col <= position.character && position.character < t.span.col + t.span.len
        })?;
        match &tok.kind {
            // Line number at the start of a line → show byte size of that line.
            LocatedTokenKind::LineNumber(n) => {
                // Look up the compiled byte length from BasicProgram (binary encoding).
                let byte_size: Option<u16> =
                    BasicProgram::parse(&text).ok().and_then(|mut compiled| {
                        use cpclib_basic::BasicProgramLineIdx;
                        let line = compiled.get_line(BasicProgramLineIdx::Number(*n))?;
                        Some(line.complete_bytes_length())
                    });

                let mut md = format!("**Line {}**", n);
                if let Some(sz) = byte_size {
                    md.push_str(&format!("\n\n**Encoded size:** {} bytes", sz));
                }
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: md
                    }),
                    range: None
                });
            },

            // Any other number → show base conversions.
            LocatedTokenKind::Number(num_text) => {
                if let Some(value) = parse_basic_integer(num_text) {
                    return Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: crate::common::render::format_number_hover(num_text, value)
                        }),
                        range: None
                    });
                }
            },

            _ => {}
        }

        None
    }
}

pub(crate) fn locomotive_basic_hover(
    line_text: &str,
    basic_text: &str,
    basic_line: u32,
    col: u32
) -> Option<Hover> {
    let col_usize = col as usize;

    // 1. Keyword hover (text-based, no parse needed)
    if let Some(word_upper) = alpha_word_at(line_text, col_usize).map(|w| w.to_uppercase()) {
        if let Some(&(_, doc)) = KEYWORD_DOCS
            .iter()
            .find(|(kw, _)| kw.to_uppercase() == word_upper)
        {
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: doc.to_string()
                }),
                range: None
            });
        }
    }

    // 2. Number hover (parse the BASIC block, find token under cursor)
    if let Ok(prog) = LocatedBasicProgram::parse(basic_text) {
        if let Some(bline) = prog.lines.iter().find(|l| l.source_line == basic_line) {
            if let Some(tok) = bline
                .tokens
                .iter()
                .find(|t| t.span.col <= col && col < t.span.col + t.span.len)
            {
                if let LocatedTokenKind::Number(num_text) = &tok.kind {
                    if let Some(value) = parse_basic_integer(num_text) {
                        return Some(Hover {
                            contents: HoverContents::Markup(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: crate::common::render::format_number_hover(num_text, value)
                            }),
                            range: None
                        });
                    }
                }
            }
        }
    }

    None
}
