//! `textDocument/documentColor` support for basm: a color swatch next to
//! any numeral literal that plausibly encodes a Gate Array ink-select byte
//! — not just `SNASET GA_PAL:n, value`'s own argument, since the same
//! bytes commonly appear written directly to the GA port (`LD BC,0x7F40 :
//! OUT (C),C`) or listed in a `DB` table. Scope is still just "does this
//! byte match one of the 32 GA values" — arbitrary bytes that don't match
//! anything in the table never get a swatch, so this stays silent on
//! ordinary code.
//!
//! For a 16-bit literal, only the two "meaningful" cases get a swatch:
//! both bytes independently match a GA value (e.g. a two-tone table
//! entry), or the high byte is `0x7F` — the real GA "select pen ink"
//! function code — and the low byte is a color. A high byte that's neither
//! never gets colorized, even if the low byte alone matches a GA value,
//! since that's very likely a coincidence rather than an actual ink write.
//!
//! LOCOMOTIVE block content is delegated to `locomotive::color` (its own
//! numerals are BASIC, not Z80) with positions translated in and out.

use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use super::embedded_basic::extract_locomotive_blocks;
use super::token::{NumeralLiteral, NumeralStyle, scan_numeral_literals};
use crate::common::colors::{
    INK_GA_VALUE, from_lsp_color, ink_index_from_ga_value, ink_rgb, inks_by_distance, to_lsp_color
};
use crate::common::document::Document;

/// One colorizable byte within a numeral literal: `prefix` is the base
/// prefix text (`0x`/`$`/`%`/...) to prepend when reformatting a
/// replacement, or empty when this span doesn't own the prefix (the low
/// byte of a split 16-bit literal never does — the high byte's span claims
/// it, since it comes first).
struct ColorSpan {
    line: u32,
    start: u32,
    end: u32,
    ink_idx: usize,
    prefix: String,
    style: NumeralStyle
}

impl ColorSpan {
    fn range(&self) -> Range {
        Range {
            start: Position {
                line: self.line,
                character: self.start
            },
            end: Position {
                line: self.line,
                character: self.end
            }
        }
    }
}

impl AssemblyAnalyzer {
    pub fn document_colors(&self, document: &Document) -> Vec<ColorInformation> {
        let mut colors: Vec<ColorInformation> = asm_spans(document)
            .iter()
            .filter_map(|s| {
                Some(ColorInformation {
                    range: s.range(),
                    color: to_lsp_color(ink_rgb(s.ink_idx)?)
                })
            })
            .collect();

        let text = document.text();
        for block in &extract_locomotive_blocks(&text) {
            let basic_doc = embedded_basic_document(&text, block);
            for mut c in crate::locomotive::BasicAnalyzer::new().document_colors(&basic_doc) {
                c.range.start.line += block.basic_range.start as u32;
                c.range.end.line += block.basic_range.start as u32;
                colors.push(c);
            }
        }
        colors
    }

    /// Snap a client's (typically continuous) color picker to the nearest
    /// of the 27 CPC inks, offering all 27 sorted by proximity so the user
    /// can browse the exact discrete palette rather than an arbitrary RGB
    /// value with no meaning on real hardware. The replacement preserves
    /// the original literal's own style (hex `$`/`0x`, binary `%`/`0b`, or
    /// bare decimal) and, for a byte that owns the prefix, re-includes it.
    pub fn color_presentations(
        &self,
        document: &Document,
        color: Color,
        range: Range
    ) -> Vec<ColorPresentation> {
        let text = document.text();
        if let Some(block) = extract_locomotive_blocks(&text)
            .into_iter()
            .find(|b| b.basic_range.contains(&(range.start.line as usize)))
        {
            let relative_range = Range {
                start: Position {
                    line: range.start.line - block.basic_range.start as u32,
                    character: range.start.character
                },
                end: Position {
                    line: range.end.line - block.basic_range.start as u32,
                    character: range.end.character
                }
            };
            let mut presentations =
                crate::locomotive::BasicAnalyzer::new().color_presentations(color, relative_range);
            for p in &mut presentations {
                if let Some(edit) = &mut p.text_edit {
                    edit.range.start.line += block.basic_range.start as u32;
                    edit.range.end.line += block.basic_range.start as u32;
                }
            }
            return presentations;
        }

        let target = from_lsp_color(color);
        let spans = asm_spans(document);
        let matched = spans.iter().find(|s| s.range() == range);
        let (prefix, style) = matched
            .map(|s| (s.prefix.clone(), s.style))
            .unwrap_or((String::new(), NumeralStyle::Decimal));

        inks_by_distance(target)
            .into_iter()
            .filter_map(|idx| {
                let byte = *INK_GA_VALUE.get(idx)?;
                let new_text = match style {
                    NumeralStyle::Hex => format!("{prefix}{byte:02X}"),
                    NumeralStyle::Decimal => format!("{byte}"),
                    NumeralStyle::Binary => format!("{prefix}{byte:08b}")
                };
                Some(ColorPresentation {
                    label: format!("Ink {idx} (0x{byte:02X})"),
                    text_edit: Some(TextEdit { range, new_text }),
                    additional_text_edits: None
                })
            })
            .collect()
    }
}

fn embedded_basic_document(text: &str, block: &super::embedded_basic::LocomotiveBlock) -> Document {
    let all_lines: Vec<&str> = text.lines().collect();
    let basic_text: String = block
        .basic_range
        .clone()
        .map(|i| all_lines[i])
        .collect::<Vec<_>>()
        .join("\n");
    let uri = Url::parse("file:///__embedded__.bas").unwrap();
    Document::new(uri, basic_text, 0)
}

/// Every colorizable byte on the assembly (non-LOCOMOTIVE) side of the
/// document.
fn asm_spans(document: &Document) -> Vec<ColorSpan> {
    let text = document.text();
    let loco_blocks = extract_locomotive_blocks(&text);
    let mut skip_lines = std::collections::HashSet::new();
    for block in &loco_blocks {
        skip_lines.insert(block.directive_line);
        if let Some(hl) = block.hide_lines_line {
            skip_lines.insert(hl);
        }
        for i in block.basic_range.clone() {
            skip_lines.insert(i);
        }
        skip_lines.insert(block.end_line);
    }

    scan_numeral_literals(&text, &skip_lines)
        .iter()
        .flat_map(spans_for)
        .collect()
}

fn spans_for(lit: &NumeralLiteral) -> Vec<ColorSpan> {
    let per_byte = lit.style.digits_per_byte();

    // Single byte: decimal (only <= 0xFF is ever produced by the scanner),
    // or a hex/binary run short enough to be one byte on its own.
    if lit.style == NumeralStyle::Decimal || (per_byte > 0 && lit.digits.len() < per_byte * 2) {
        let Ok(v) = u32::from_str_radix(&lit.digits, lit.style.radix())
        else {
            return Vec::new();
        };
        if v > 0xFF {
            return Vec::new();
        }
        let Some(idx) = ink_index_from_ga_value(v as u8)
        else {
            return Vec::new();
        };
        let end = lit.digits_start + lit.digits.len() as u32;
        return vec![ColorSpan {
            line: lit.line,
            start: lit.token_start,
            end,
            ink_idx: idx,
            prefix: lit.prefix.clone(),
            style: lit.style
        }];
    }

    // Two bytes: exactly a 16-bit-wide digit run (4 hex digits / 16 binary
    // digits) — anything else (3 hex digits, a stray 5th, etc.) isn't
    // byte-aligned and is skipped.
    if per_byte > 0 && lit.digits.len() == per_byte * 2 {
        let (hi_txt, lo_txt) = lit.digits.split_at(per_byte);
        let (Ok(hi), Ok(lo)) = (
            u32::from_str_radix(hi_txt, lit.style.radix()),
            u32::from_str_radix(lo_txt, lit.style.radix())
        )
        else {
            return Vec::new();
        };
        if hi > 0xFF || lo > 0xFF {
            return Vec::new();
        }
        let hi_idx = ink_index_from_ga_value(hi as u8);
        let lo_idx = ink_index_from_ga_value(lo as u8);
        let eligible =
            matches!((hi_idx, lo_idx), (Some(_), Some(_))) || (hi == 0x7F && lo_idx.is_some());
        if !eligible {
            return Vec::new();
        }

        let mut out = Vec::new();
        let mid = lit.digits_start + per_byte as u32;
        let end = lit.digits_start + lit.digits.len() as u32;
        if let Some(idx) = hi_idx {
            // High byte "before the number": spans the whole token's own
            // start (base prefix included) through the end of its digits.
            out.push(ColorSpan {
                line: lit.line,
                start: lit.token_start,
                end: mid,
                ink_idx: idx,
                prefix: lit.prefix.clone(),
                style: lit.style
            });
        }
        if let Some(idx) = lo_idx {
            // Low byte "after": just its own digits, no prefix to claim.
            out.push(ColorSpan {
                line: lit.line,
                start: mid,
                end,
                ink_idx: idx,
                prefix: String::new(),
                style: lit.style
            });
        }
        return out;
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::document::Document;

    fn colors_for(text: &str) -> Vec<ColorInformation> {
        let uri = Url::parse("file:///t.asm").unwrap();
        let doc = Document::new(uri, text.to_string(), 1);
        AssemblyAnalyzer::new().document_colors(&doc)
    }

    #[test]
    fn plain_ld_a_with_a_known_ga_byte_yields_a_swatch_over_the_whole_literal() {
        // 0x54 is ink 0 (black); the swatch spans "0x54" including the prefix.
        let colors = colors_for("LD A, 0x54 ; TODO remove\n");
        assert_eq!(colors.len(), 1, "{colors:?}");
        assert_eq!(
            colors[0].color,
            Color {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                alpha: 1.0
            }
        );
        let start = "LD A, ".len() as u32;
        assert_eq!(colors[0].range.start.character, start);
        assert_eq!(colors[0].range.end.character, start + 4); // "0x54"
    }

    #[test]
    fn db_directive_with_two_ga_bytes_yields_two_swatches() {
        let colors = colors_for("DB 0x40, 0x54 ; TODO remove\n");
        assert_eq!(colors.len(), 2, "{colors:?}");
    }

    #[test]
    fn a_16_bit_literal_with_a_0x7f_high_byte_only_colorizes_the_low_byte() {
        // 0x7f10: low byte 0x10 is not a GA value -> no swatch at all.
        // 0x7f40: low byte 0x40 IS a GA value (ink 13), high byte 0x7f is
        // the real GA "select ink" prefix -> only the low byte is colorized.
        let colors = colors_for("LD BC, 0x7f10 : OUT (C), C : LD BC, 0x7f40 : OUT (C), C\n");
        assert_eq!(colors.len(), 1, "{colors:?}");
        let low_byte_col = "LD BC, 0x7f10 : OUT (C), C : LD BC, 0x7f".len() as u32;
        assert_eq!(colors[0].range.start.character, low_byte_col);
        assert_eq!(colors[0].range.end.character, low_byte_col + 2);
    }

    #[test]
    fn both_bytes_of_a_hex_pair_get_a_swatch_when_both_match() {
        // 0x5440: high byte 0x54 (ink 0) and low byte 0x40 (ink 13) both
        // match a GA value -> two swatches, high one prefix-inclusive.
        let colors = colors_for("LD BC, 0x5440\n");
        assert_eq!(colors.len(), 2, "{colors:?}");
        let token_start = "LD BC, ".len() as u32;
        assert_eq!(colors[0].range.start.character, token_start); // high: "0x54"
        assert_eq!(colors[0].range.end.character, token_start + 4);
        assert_eq!(colors[1].range.start.character, token_start + 4); // low: "40"
        assert_eq!(colors[1].range.end.character, token_start + 6);
    }

    #[test]
    fn a_16_bit_literal_whose_high_byte_is_neither_a_color_nor_0x7f_yields_nothing() {
        // High byte 0x10 is not a GA value and isn't 0x7F -> even though
        // the low byte 0x54 matches, no swatch at all.
        let colors = colors_for("LD BC, 0x1054\n");
        assert!(colors.is_empty(), "{colors:?}");
    }

    #[test]
    fn unrecognized_byte_yields_no_swatch() {
        let colors = colors_for("LD A, 0x00\n");
        assert!(colors.is_empty(), "{colors:?}");
    }

    #[test]
    fn byte_inside_a_comment_is_not_colorized() {
        let colors = colors_for("LD A, 1 ; 0x54 is not code\n");
        assert!(colors.is_empty(), "{colors:?}");
    }

    #[test]
    fn byte_inside_a_string_is_not_colorized() {
        let colors = colors_for("DB \"0x54\"\n");
        assert!(colors.is_empty(), "{colors:?}");
    }

    #[test]
    fn multi_statement_line_still_finds_every_byte_after_a_colon() {
        let colors = colors_for("LD A,1 : LD B, 0x54\n");
        assert_eq!(colors.len(), 1, "{colors:?}");
    }

    #[test]
    fn locomotive_block_ink_statement_is_colorized_via_delegation() {
        let text = "LOCOMOTIVE\n10 BORDER 26\nENDLOCOMOTIVE\nLD A, 0x54\n";
        let colors = colors_for(text);
        assert_eq!(colors.len(), 2, "{colors:?}");
        let loco_swatch = colors.iter().find(|c| c.range.start.line == 1).unwrap();
        assert_eq!(
            loco_swatch.color,
            Color {
                red: 1.0,
                green: 1.0,
                blue: 1.0,
                alpha: 1.0
            }
        );
        assert!(colors.iter().any(|c| c.range.start.line == 3));
    }

    #[test]
    fn dollar_and_percent_forms_are_also_recognized() {
        let colors = colors_for("LD A, $54\nLD B, %01010100\n");
        assert_eq!(colors.len(), 2, "{colors:?}");
    }

    #[test]
    fn a_digit_run_trailing_an_identifier_is_not_a_standalone_numeral() {
        // STATE84's trailing "84" (== 0x54, a GA value) must not be
        // mistaken for a bare numeric literal.
        let colors = colors_for("LD A, STATE84\n");
        assert!(colors.is_empty(), "{colors:?}");
    }

    fn presentations_for(text: &str) -> Vec<ColorPresentation> {
        let uri = Url::parse("file:///t.asm").unwrap();
        let doc = Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        let swatch = analyzer
            .document_colors(&doc)
            .into_iter()
            .next()
            .expect("expected at least one swatch");
        analyzer.color_presentations(
            &doc,
            Color {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                alpha: 1.0
            },
            swatch.range
        )
    }

    #[test]
    fn presentations_offer_all_27_inks_closest_first() {
        let presentations = presentations_for("LD A, 0x54\n"); // ink 0 = black
        assert_eq!(presentations.len(), 27);
        assert_eq!(presentations[0].label, "Ink 0 (0x54)");
    }

    #[test]
    fn whole_token_hex_literal_gets_a_prefixed_hex_replacement() {
        let presentations = presentations_for("LD A, 0x54\n");
        let edit = presentations[0].text_edit.as_ref().unwrap();
        assert_eq!(edit.new_text, "0x54");
    }

    #[test]
    fn dollar_style_literal_gets_a_dollar_prefixed_replacement() {
        let presentations = presentations_for("LD A, $54\n");
        let edit = presentations[0].text_edit.as_ref().unwrap();
        assert_eq!(edit.new_text, "$54");
    }

    #[test]
    fn decimal_style_literal_gets_a_decimal_replacement() {
        // 84 decimal == 0x54, a GA value (ink 0, whose GA byte 0x54 == 84).
        let presentations = presentations_for("LD A, 84\n");
        let edit = presentations[0].text_edit.as_ref().unwrap();
        assert_eq!(edit.new_text, "84");
    }

    #[test]
    fn high_byte_presentation_replaces_the_prefix_and_high_digits_only() {
        let presentations = presentations_for("LD BC, 0x5440\n"); // matches high byte first
        let edit = presentations[0].text_edit.as_ref().unwrap();
        assert_eq!(edit.new_text, "0x54");
    }

    #[test]
    fn low_byte_presentation_replaces_only_its_own_digits_no_prefix() {
        let presentations = presentations_for("LD BC, 0x7f40\n"); // only the low byte gets a swatch
        let edit = presentations[0].text_edit.as_ref().unwrap();
        assert_eq!(edit.new_text, "54");
    }
}
