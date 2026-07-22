//! Hover for Locomotive BASIC: keyword documentation and numeric literals.
//!
//! `locomotive_basic_hover` is `pub(crate)` because the basm module reuses it
//! for BASIC blocks embedded in assembly.

use cpclib_basic::BasicProgram;
use cpclib_basic::located::{LocatedBasicProgram, LocatedTokenKind};
use cpclib_basic::tokens::{BasicToken, BasicTokenNoPrefix};
use tower_lsp::lsp_types::*;

use super::BasicAnalyzer;
use super::token::*;
use crate::common::document::Document;

impl BasicAnalyzer {
    pub fn hover(&self, document: &Document, position: Position) -> Option<Hover> {
        let source_line = document.line(position.line as usize)?;
        let col = position.character as usize;
        let line = source_line.trim_end_matches(|c| c == '\n' || c == '\r');

        let text = document.text();
        let prog = self.parse_cached(document).ok();
        let tok = prog.as_ref().and_then(|p| token_at_position(p, position));

        // Try keyword hover first (alphabetic words), enriched with the
        // token's own encoded byte(s) when the document parses cleanly.
        if let Some(word_upper) = alpha_word_at(line, col).map(|w| w.to_uppercase()) {
            if let Some(&(_, doc)) = KEYWORD_DOCS
                .iter()
                .find(|(kw, _)| kw.to_uppercase() == word_upper)
            {
                let mut md = doc.to_string();
                if let Some((label, bytes)) = tok.map(|t| &t.kind).and_then(token_bytes) {
                    md.push_str("\n\n");
                    md.push_str(&crate::common::render::format_labeled_bytes(&[(
                        label.as_str(),
                        bytes.as_slice()
                    )]));
                }
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: md
                    }),
                    range: None
                });
            }
        }

        // Number hover: find a Number token at the cursor and show base conversions.
        let tok = tok?;
        match &tok.kind {
            // Line number at the start of a line → show byte size of that
            // line, split into header / each token / end marker.
            LocatedTokenKind::LineNumber(n) => {
                // Look up the compiled line from BasicProgram (binary encoding).
                let mut md = format!("**Line {}**", n);
                if let Ok(mut compiled) = BasicProgram::parse(&text) {
                    use cpclib_basic::BasicProgramLineIdx;
                    if let Some(line) = compiled.get_line(BasicProgramLineIdx::Number(*n)) {
                        md.push_str(&format!(
                            "\n\n**Encoded size:** {} bytes\n\n",
                            line.complete_bytes_length()
                        ));

                        let size = line.public_bytes_length();
                        let header = [
                            (size % 256) as u8,
                            (size / 256) as u8,
                            (*n % 256) as u8,
                            (*n / 256) as u8
                        ];

                        let mut groups: Vec<(String, Vec<u8>)> =
                            vec![("Header".to_string(), header.to_vec())];
                        for (i, token) in line.tokens().iter().enumerate() {
                            let text = token.to_string();
                            let text = if text.trim().is_empty() && !text.is_empty() {
                                "(space)".to_string()
                            }
                            else {
                                text
                            };
                            groups.push((format!("Token {}: {text}", i + 1), token.as_bytes()));
                        }
                        if line.tokens().last()
                            != Some(&BasicToken::SimpleToken(
                                BasicTokenNoPrefix::EndOfTokenisedLine
                            ))
                        {
                            groups.push(("End".to_string(), vec![0]));
                        }

                        let group_refs: Vec<(&str, &[u8])> = groups
                            .iter()
                            .map(|(label, bytes)| (label.as_str(), bytes.as_slice()))
                            .collect();
                        md.push_str(&crate::common::render::format_labeled_bytes(&group_refs));
                    }
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

            // Space, statement separator, operator, or any other raw
            // passthrough character → show its own byte(s) directly, same
            // as its row in the line-level byte breakdown.
            LocatedTokenKind::Operator(_)
            | LocatedTokenKind::Space
            | LocatedTokenKind::Separator
            | LocatedTokenKind::Other(_) => {
                if let Some((label, bytes)) = token_bytes(&tok.kind) {
                    return Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: crate::common::render::format_labeled_bytes(&[(
                                label.as_str(),
                                bytes.as_slice()
                            )])
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
    let position = Position {
        line: basic_line,
        character: col
    };
    let prog = LocatedBasicProgram::parse(basic_text).ok();
    let tok = prog.as_ref().and_then(|p| token_at_position(p, position));

    // 1. Keyword hover, enriched with the token's own encoded byte(s) when
    // the block parses cleanly.
    if let Some(word_upper) = alpha_word_at(line_text, col_usize).map(|w| w.to_uppercase()) {
        if let Some(&(_, doc)) = KEYWORD_DOCS
            .iter()
            .find(|(kw, _)| kw.to_uppercase() == word_upper)
        {
            let mut md = doc.to_string();
            if let Some((label, bytes)) = tok.map(|t| &t.kind).and_then(token_bytes) {
                md.push_str("\n\n");
                md.push_str(&crate::common::render::format_labeled_bytes(&[(
                    label.as_str(),
                    bytes.as_slice()
                )]));
            }
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: md
                }),
                range: None
            });
        }
    }

    // 2. Number hover (token already resolved above)
    if let Some(tok) = tok {
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

        // 3. Space, statement separator, operator, or any other raw
        // passthrough character → show its own byte(s) directly.
        if let Some((label, bytes)) = token_bytes(&tok.kind) {
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: crate::common::render::format_labeled_bytes(&[(
                        label.as_str(),
                        bytes.as_slice()
                    )])
                }),
                range: None
            });
        }
    }

    None
}

#[cfg(test)]
mod byte_hover_tests {
    use super::*;
    use crate::common::document::Document;

    fn doc(text: &str) -> Document {
        let uri = Url::parse("file:///t.bas").unwrap();
        Document::new(uri, text.to_string(), 1)
    }

    fn hover_md(text: &str, line: u32, character: u32) -> String {
        let d = doc(text);
        let analyzer = BasicAnalyzer::new();
        let hover = analyzer
            .hover(&d, Position { line, character })
            .expect("expected a hover result");
        match hover.contents {
            HoverContents::Markup(MarkupContent { value, .. }) => value,
            _ => panic!("expected markdown hover contents")
        }
    }

    #[test]
    fn line_number_hover_splits_header_tokens_and_end_marker() {
        // "10 CLS" -> header (size,size,10,0) + token CLS (single byte) + end (0x00).
        let text = "10 CLS\n";
        let md = hover_md(text, 0, 0);
        assert!(md.contains("**Line 10**"), "{md}");
        assert!(md.contains("| Bytes | Token |"), "{md}");
        assert!(md.contains("| Header |"), "{md}");
        assert!(md.contains("Token 1: CLS"), "{md}");
        assert!(md.contains("| End |"), "{md}");
        assert!(md.contains("00"), "{md}");
    }

    #[test]
    fn line_number_hover_shows_a_byte_group_per_token() {
        // Two tokens on the line: CLS and CLG, each its own labeled group.
        let text = "10 CLS:CLG\n";
        let md = hover_md(text, 0, 0);
        assert!(md.contains("Token 1: CLS"), "{md}");
        assert!(md.contains("Token 2:"), "{md}");
        assert!(md.contains("CLG"), "{md}");
    }

    #[test]
    fn keyword_token_hover_shows_its_single_byte() {
        // CLS is a plain (unprefixed) keyword -> one byte.
        let text = "10 CLS\n";
        let md = hover_md(text, 0, 3);
        assert!(md.contains("| Bytes | Token |"), "{md}");
        let cls_value = BasicTokenNoPrefix::Cls.value();
        assert!(md.contains(&format!("{cls_value:02X}")), "{md}");
    }

    #[test]
    fn function_token_hover_shows_the_additional_token_marker_and_its_byte() {
        // ABS is a prefixed ("additional") token -> 0xFF + its own value.
        let text = "10 PRINT ABS(1)\n";
        let md = hover_md(text, 0, 9);
        assert!(md.contains("| Bytes | Token |"), "{md}");
        assert!(md.contains("FF"), "{md}");
    }

    #[test]
    fn space_token_hover_shows_its_own_byte() {
        let text = "10 PRINT 1\n";
        // Column 8 is the space between PRINT and 1.
        let md = hover_md(text, 0, 8);
        assert!(md.contains("(space)"), "{md}");
        let space_value = BasicTokenNoPrefix::CharSpace.value();
        assert!(md.contains(&format!("{space_value:02X}")), "{md}");
    }

    #[test]
    fn separator_token_hover_shows_its_own_byte() {
        let text = "10 CLS:CLG\n";
        // Column 6 is the ':' separating the two statements.
        let md = hover_md(text, 0, 6);
        assert!(md.contains(":"), "{md}");
        let colon_value = BasicTokenNoPrefix::CharColon.value();
        assert!(md.contains(&format!("{colon_value:02X}")), "{md}");
    }

    #[test]
    fn operator_token_hover_shows_its_own_byte() {
        let text = "10 A=1-2\n";
        // Column 6 is the '-' operator.
        let md = hover_md(text, 0, 6);
        assert!(md.contains('-'), "{md}");
        let minus_value = BasicTokenNoPrefix::SubstractionOrUnaryMinus.value();
        assert!(md.contains(&format!("{minus_value:02X}")), "{md}");
    }
}
