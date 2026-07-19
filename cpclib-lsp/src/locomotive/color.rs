//! `textDocument/documentColor` support for Locomotive BASIC: a color
//! swatch next to `INK pen, color[, color2]` / `BORDER color[, color2]`
//! ink-number arguments. Unlike basm's GA byte encoding, BASIC's ink
//! numbers are a direct 0-26 index into the CPC palette.
//!
//! A variable argument is resolved through a chain of simple
//! `NAME = literal-or-NAME` assignments (depth-capped, cycle-guarded) —
//! anything more complex than that (an expression, a function call) is
//! left uncolored rather than guessed at.

use std::collections::HashSet;

use cpclib_basic::located::{LocatedBasicProgram, LocatedBasicToken, LocatedTokenKind};
use cpclib_basic::tokens::BasicTokenNoPrefix;
use tower_lsp::lsp_types::*;

use super::BasicAnalyzer;
use super::token::parse_basic_integer;
use crate::common::colors::{from_lsp_color, ink_rgb, inks_by_distance, to_lsp_color};
use crate::common::document::Document;

impl BasicAnalyzer {
    pub fn document_colors(&self, document: &Document) -> Vec<ColorInformation> {
        let text = document.text();
        let Ok(prog) = LocatedBasicProgram::parse(&text)
        else {
            return Vec::new();
        };

        let mut colors = Vec::new();
        for bline in &prog.lines {
            let toks = &bline.tokens;
            for (i, tok) in toks.iter().enumerate() {
                // Which argument positions (0-based, after the keyword) hold
                // ink color numbers: INK's first argument is the *pen*, not
                // a color; BORDER's arguments are colors from the start.
                let color_args = match &tok.kind {
                    LocatedTokenKind::Keyword(BasicTokenNoPrefix::Ink) => 1..3,
                    LocatedTokenKind::Keyword(BasicTokenNoPrefix::Border) => 0..2,
                    _ => continue
                };

                for (arg_idx, arg_tok) in first_tokens_of_args(toks, i + 1).into_iter().enumerate()
                {
                    if !color_args.contains(&arg_idx) {
                        continue;
                    }
                    let Some(value) = resolve_color_number(&prog, arg_tok)
                    else {
                        continue;
                    };
                    let Some(rgb) = ink_rgb(value as usize)
                    else {
                        continue;
                    };
                    let len = token_text_len(arg_tok);
                    let start = Position {
                        line: arg_tok.span.line,
                        character: arg_tok.span.col
                    };
                    let end = Position {
                        line: arg_tok.span.line,
                        character: arg_tok.span.col + len
                    };
                    colors.push(ColorInformation {
                        range: Range { start, end },
                        color: to_lsp_color(rgb)
                    });
                }
            }
        }
        colors
    }

    /// Snap a client's (typically continuous) color picker to the nearest
    /// of the 27 CPC inks, offering all 27 sorted by proximity so the user
    /// can browse the exact discrete palette rather than an arbitrary RGB
    /// value with no meaning on real hardware. BASIC's ink numbers are a
    /// direct decimal index — no radix to preserve, unlike basm.
    pub fn color_presentations(&self, color: Color, range: Range) -> Vec<ColorPresentation> {
        let target = from_lsp_color(color);
        inks_by_distance(target)
            .into_iter()
            .map(|idx| {
                ColorPresentation {
                    label: format!("Ink {idx}"),
                    text_edit: Some(TextEdit {
                        range,
                        new_text: idx.to_string()
                    }),
                    additional_text_edits: None
                }
            })
            .collect()
    }
}

/// The first token of each comma-separated argument following a keyword,
/// stopping at the statement separator (`:`), a trailing comment, or
/// end-of-line.
fn first_tokens_of_args(toks: &[LocatedBasicToken], start: usize) -> Vec<&LocatedBasicToken> {
    let mut out = Vec::new();
    let mut expect_new_arg = true;
    let mut i = start;
    while i < toks.len() {
        match &toks[i].kind {
            LocatedTokenKind::Space => {},
            LocatedTokenKind::Separator | LocatedTokenKind::Comment(_) => break,
            LocatedTokenKind::Other(',') => expect_new_arg = true,
            _ => {
                if expect_new_arg {
                    out.push(&toks[i]);
                    expect_new_arg = false;
                }
            },
        }
        i += 1;
    }
    out
}

/// Source-text length of a token, for the swatch's end column.
fn token_text_len(tok: &LocatedBasicToken) -> u32 {
    match &tok.kind {
        LocatedTokenKind::Number(n) => n.len() as u32,
        LocatedTokenKind::Variable(name) => name.len() as u32,
        _ => 1
    }
}

/// A literal ink number for `tok`: direct if it's already a `Number`,
/// resolved through an assignment chain if it's a `Variable`.
fn resolve_color_number(prog: &LocatedBasicProgram, tok: &LocatedBasicToken) -> Option<u8> {
    match &tok.kind {
        LocatedTokenKind::Number(n) => parse_basic_integer(n).and_then(|v| u8::try_from(v).ok()),
        LocatedTokenKind::Variable(name) => resolve_variable_literal(prog, name),
        _ => None
    }
}

enum SimpleRhs {
    Number(String),
    Variable(String)
}

/// Resolve `var_name` to a literal `u8` by following a chain of
/// `NAME = literal-or-NAME` assignments, depth-capped and cycle-guarded.
fn resolve_variable_literal(prog: &LocatedBasicProgram, var_name: &str) -> Option<u8> {
    const MAX_DEPTH: usize = 8;
    let mut current = var_name.to_uppercase();
    let mut seen = HashSet::new();
    for _ in 0..MAX_DEPTH {
        if !seen.insert(current.clone()) {
            return None; // cycle
        }
        match find_simple_assignment(prog, &current)? {
            SimpleRhs::Number(n) => {
                return parse_basic_integer(&n).and_then(|v| u8::try_from(v).ok());
            },
            SimpleRhs::Variable(next) => current = next.to_uppercase()
        }
    }
    None
}

/// Find the first `NAME = value` assignment (bare or after `LET`) anywhere
/// in the program, where `value` is a bare literal or variable immediately
/// followed by end-of-statement — anything more (an operator, a function
/// call) makes the assignment too complex to treat as a simple alias, and
/// is skipped.
fn find_simple_assignment(prog: &LocatedBasicProgram, name_upper: &str) -> Option<SimpleRhs> {
    for bline in &prog.lines {
        let toks = &bline.tokens;
        let n = toks.len();
        for i in 0..n {
            let LocatedTokenKind::Variable(v) = &toks[i].kind
            else {
                continue;
            };
            if v.to_uppercase() != name_upper {
                continue;
            }

            let mut j = i + 1;
            while j < n && matches!(toks[j].kind, LocatedTokenKind::Space) {
                j += 1;
            }
            if !(j < n
                && matches!(
                    &toks[j].kind,
                    LocatedTokenKind::Operator(BasicTokenNoPrefix::Equal)
                ))
            {
                continue;
            }

            let mut k = j + 1;
            while k < n && matches!(toks[k].kind, LocatedTokenKind::Space) {
                k += 1;
            }
            let Some(rhs) = (k < n).then(|| &toks[k].kind).and_then(|kind| {
                match kind {
                    LocatedTokenKind::Number(num) => Some(SimpleRhs::Number(num.clone())),
                    LocatedTokenKind::Variable(v2) => Some(SimpleRhs::Variable(v2.clone())),
                    _ => None
                }
            })
            else {
                continue;
            };

            let mut m = k + 1;
            while m < n && matches!(toks[m].kind, LocatedTokenKind::Space) {
                m += 1;
            }
            let is_end_of_statement = m >= n
                || matches!(
                    toks[m].kind,
                    LocatedTokenKind::Separator | LocatedTokenKind::Comment(_)
                );
            if is_end_of_statement {
                return Some(rhs);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::document::Document;

    fn colors_for(text: &str) -> Vec<ColorInformation> {
        let uri = Url::parse("file:///t.bas").unwrap();
        let doc = Document::new(uri, text.to_string(), 1);
        BasicAnalyzer::new().document_colors(&doc)
    }

    #[test]
    fn ink_second_argument_yields_a_swatch_but_not_the_pen() {
        // pen=0, color=1 -> ink 1 is (0x00, 0x00, 0x80).
        let colors = colors_for("10 INK 0,1\n");
        assert_eq!(colors.len(), 1, "{colors:?}");
        assert_eq!(
            colors[0].color,
            Color {
                red: 0.0,
                green: 0.0,
                blue: 0x80 as f32 / 255.0,
                alpha: 1.0
            }
        );
    }

    #[test]
    fn ink_with_a_flash_second_color_yields_two_swatches() {
        let colors = colors_for("10 INK 0,1,2\n");
        assert_eq!(colors.len(), 2, "{colors:?}");
    }

    #[test]
    fn border_first_argument_yields_a_swatch() {
        let colors = colors_for("10 BORDER 26\n");
        assert_eq!(colors.len(), 1, "{colors:?}");
        assert_eq!(
            colors[0].color,
            Color {
                red: 1.0,
                green: 1.0,
                blue: 1.0,
                alpha: 1.0
            }
        );
    }

    #[test]
    fn variable_argument_resolves_through_a_simple_assignment() {
        let colors = colors_for("10 MYCOL = 26\n20 BORDER MYCOL\n");
        assert_eq!(colors.len(), 1, "{colors:?}");
        assert_eq!(
            colors[0].color,
            Color {
                red: 1.0,
                green: 1.0,
                blue: 1.0,
                alpha: 1.0
            }
        );
    }

    #[test]
    fn variable_argument_resolves_through_a_chain_of_aliases() {
        let colors = colors_for("10 BASE = 26\n20 MYCOL = BASE\n30 BORDER MYCOL\n");
        assert_eq!(colors.len(), 1, "{colors:?}");
    }

    #[test]
    fn variable_with_no_resolvable_assignment_yields_no_swatch() {
        let colors = colors_for("10 BORDER MYCOL\n");
        assert!(colors.is_empty(), "{colors:?}");
    }

    #[test]
    fn variable_assigned_a_complex_expression_yields_no_swatch() {
        let colors = colors_for("10 MYCOL = 1 + 2\n20 BORDER MYCOL\n");
        assert!(colors.is_empty(), "{colors:?}");
    }

    #[test]
    fn unrelated_keyword_yields_no_swatch() {
        let colors = colors_for("10 PRINT 26\n");
        assert!(colors.is_empty(), "{colors:?}");
    }

    #[test]
    fn presentations_offer_all_27_inks_closest_first_as_decimal() {
        let uri = Url::parse("file:///t.bas").unwrap();
        let doc = Document::new(uri, "10 BORDER 26\n".to_string(), 1);
        let analyzer = BasicAnalyzer::new();
        let swatch = analyzer.document_colors(&doc).into_iter().next().unwrap();
        let presentations = analyzer.color_presentations(
            Color {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                alpha: 1.0
            },
            swatch.range
        );
        assert_eq!(presentations.len(), 27);
        assert_eq!(presentations[0].label, "Ink 0");
        assert_eq!(presentations[0].text_edit.as_ref().unwrap().new_text, "0");
    }
}
