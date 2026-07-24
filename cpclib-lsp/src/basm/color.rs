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
//! A bare identifier (e.g. `GA_WHITE` in `db GA_WHITE, ...`) also gets a
//! swatch when its `EQU`/`=` definition — found in this document or any
//! file it includes, e.g. `inner://ga.asm` — resolves to a GA byte,
//! following a chain of aliases if needed (`GA_WHITE` → `GA_COL_13` →
//! `0x40`). These swatches are read-only: replacing a meaningful constant
//! name with a raw literal isn't something `colorPresentation` offers.
//!
//! LOCOMOTIVE block content is delegated to `locomotive::color` (its own
//! numerals are BASIC, not Z80) with positions translated in and out.

use std::collections::HashMap;

use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use super::embedded_basic::extract_locomotive_blocks;
use super::token::{NumeralLiteral, NumeralStyle, scan_numeral_literals};
use crate::common::colors::{
    INK_GA_VALUE, from_lsp_color, ink_index_from_ga_value, ink_rgb,
    inks_by_distance_including_duplicates, to_lsp_color
};
use crate::common::document::Document;

/// One colorizable byte within a numeral literal: `prefix`/`suffix` are the
/// base notation text (`0x`/`$`/`%`/... or a trailing `h`/`b`) to
/// re-include when reformatting a replacement, captured verbatim from the
/// source. For a split 16-bit literal, the high byte's span claims the
/// prefix (it comes first) and the low byte's span claims the suffix (it
/// comes last) — neither owns both.
struct ColorSpan {
    line: u32,
    start: u32,
    end: u32,
    ink_idx: usize,
    prefix: String,
    suffix: String,
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
        let text = document.text();
        // Computed once and threaded through - `asm_spans`/`symbol_spans`
        // (and the LOCOMOTIVE loop below) used to each independently
        // re-extract the same blocks from the same text, three times total
        // per request.
        let blocks = extract_locomotive_blocks(&text);
        let skip_lines = skip_lines_for(&blocks);

        let mut colors: Vec<ColorInformation> = asm_spans(document, &skip_lines)
            .iter()
            .chain(symbol_spans(self, document, &skip_lines).iter())
            .filter_map(|s| {
                Some(ColorInformation {
                    range: s.range(),
                    color: to_lsp_color(ink_rgb(s.ink_idx)?)
                })
            })
            .collect();

        for block in &blocks {
            let basic_doc = embedded_basic_document(&text, block);
            for mut c in crate::locomotive::BasicAnalyzer::new().document_colors(&basic_doc) {
                c.range.start.line += block.basic_range.start as u32;
                c.range.end.line += block.basic_range.start as u32;
                colors.push(c);
            }
        }
        colors
    }

    /// Offer all 32 GA byte encodings (duplicates included — each is a
    /// distinct, individually valid `SNASET GA_PAL:n` value) as the
    /// `colorPresentation` list, sorted by proximity to whatever the
    /// client's own (continuous) color picker landed on. VSCode always
    /// shows its native RGB/HSV picker for `documentColor` — the LSP
    /// protocol has no way to suppress that — but this makes the
    /// CPC-correct choices the ones actually offered for one-click
    /// selection. The replacement preserves the original literal's own
    /// style (hex `$`/`0x`, binary `%`/`0b`, or bare decimal) and, for a
    /// byte that owns the prefix, re-includes it.
    pub fn color_presentations(
        &self,
        document: &Document,
        color: Color,
        range: Range
    ) -> Vec<ColorPresentation> {
        let text = document.text();
        let blocks = extract_locomotive_blocks(&text);
        if let Some(block) = blocks
            .iter()
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
        let skip_lines = skip_lines_for(&blocks);
        let spans = asm_spans(document, &skip_lines);
        // A range with no matching numeral span is a symbol-reference
        // swatch (e.g. `GA_WHITE`) — those are read-only, no presentations.
        let Some(matched) = spans.iter().find(|s| s.range() == range)
        else {
            return Vec::new();
        };
        let (prefix, suffix, style) = (
            matched.prefix.clone(),
            matched.suffix.clone(),
            matched.style
        );

        inks_by_distance_including_duplicates(target)
            .into_iter()
            .filter_map(|idx| {
                let byte = *INK_GA_VALUE.get(idx)?;
                let new_text = match style {
                    NumeralStyle::Hex => format!("{prefix}{byte:02X}{suffix}"),
                    NumeralStyle::Decimal => format!("{byte}"),
                    NumeralStyle::Binary => format!("{prefix}{byte:08b}{suffix}")
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
    let basic_text = super::embedded_basic::text_for_block(text, block);
    let uri = Url::parse("file:///__embedded__.bas").unwrap();
    Document::new(uri, basic_text, 0)
}

/// Line indices that belong to LOCOMOTIVE blocks (directive/HIDE_LINES/
/// BASIC content/ENDLOCOMOTIVE) — their content is BASIC, not Z80, and is
/// scanned separately via delegation. Takes already-extracted `blocks`
/// rather than re-extracting them, so callers that already have them (or
/// need them for another purpose too) don't pay for a second scan.
fn skip_lines_for(
    blocks: &[super::embedded_basic::LocomotiveBlock]
) -> std::collections::HashSet<usize> {
    let mut skip_lines = std::collections::HashSet::new();
    for block in blocks {
        skip_lines.insert(block.directive_line);
        if let Some(hl) = block.hide_lines_line {
            skip_lines.insert(hl);
        }
        for i in block.basic_range.clone() {
            skip_lines.insert(i);
        }
        skip_lines.insert(block.end_line);
    }
    skip_lines
}

/// Every colorizable byte on the assembly (non-LOCOMOTIVE) side of the
/// document.
fn asm_spans(document: &Document, skip_lines: &std::collections::HashSet<usize>) -> Vec<ColorSpan> {
    let text = document.text();
    scan_numeral_literals(&text, skip_lines)
        .iter()
        .flat_map(spans_for)
        .collect()
}

/// Every bare identifier in the document whose `EQU`/`=` definition
/// (possibly in an included file, possibly through a chain of aliases)
/// resolves to a GA byte — e.g. `GA_WHITE` in `db GA_WHITE, ...`.
fn symbol_spans(
    analyzer: &AssemblyAnalyzer,
    document: &Document,
    skip_lines: &std::collections::HashSet<usize>
) -> Vec<ColorSpan> {
    let mut table: HashMap<String, String> = HashMap::new();
    for (name, detail) in analyzer.collect_symbols(document) {
        if let Some(rhs) = detail.strip_prefix("= ") {
            table.entry(name).or_insert_with(|| rhs.to_string());
        }
    }
    for (_, name, detail) in analyzer.collect_symbols_from_includes(document) {
        if let Some(rhs) = detail.strip_prefix("= ") {
            table.entry(name).or_insert_with(|| rhs.to_string());
        }
    }
    if table.is_empty() {
        return Vec::new();
    }

    let text = document.text();
    let mut out = Vec::new();

    for (line_idx, line) in text.lines().enumerate() {
        if skip_lines.contains(&line_idx) {
            continue;
        }
        let code = super::format::strip_asm_comment(line);
        let bytes = code.as_bytes();
        let mut col = 0usize;
        while col < bytes.len() {
            let c = bytes[col];
            if c == b'"' {
                col += 1;
                while col < bytes.len() && bytes[col] != b'"' {
                    col += 1;
                }
                if col < bytes.len() {
                    col += 1;
                }
                continue;
            }
            if c.is_ascii_alphabetic() || c == b'_' || c == b'.' || c == b'@' {
                let start = col;
                let mut end = col;
                while end < bytes.len() {
                    let ch = bytes[end];
                    if ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'.' || ch == b'@' {
                        end += 1;
                    }
                    else {
                        break;
                    }
                }
                let name = &code[start..end];
                if let Some(idx) =
                    resolve_symbol_byte(&table, name, 0).and_then(ink_index_from_ga_value)
                {
                    out.push(ColorSpan {
                        line: line_idx as u32,
                        start: start as u32,
                        end: end as u32,
                        ink_idx: idx,
                        prefix: String::new(),
                        suffix: String::new(),
                        style: NumeralStyle::Decimal // unused: symbol swatches are read-only
                    });
                }
                col = end.max(col + 1);
                continue;
            }
            col += 1;
        }
    }

    out
}

/// Resolve `name` to a literal `u8` by following its `EQU`/`=` definition,
/// and — if that definition is itself just a bare identifier (an alias,
/// e.g. `GA_WHITE = GA_COL_13`) — the alias's definition next, depth-capped
/// against a reference cycle.
fn resolve_symbol_byte(table: &HashMap<String, String>, name: &str, depth: usize) -> Option<u8> {
    const MAX_DEPTH: usize = 8;
    if depth >= MAX_DEPTH {
        return None;
    }
    let rhs = table.get(name)?.trim();

    let literals = cpclib_common::parse::scan_numeric_literals(rhs);
    if literals.len() == 1 && literals[0].0 == 0 && literals[0].1 == rhs.len() {
        return u8::try_from(literals[0].2).ok();
    }

    let is_bare_identifier = rhs
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '.' || c == '@')
        && rhs
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '@');
    if is_bare_identifier {
        return resolve_symbol_byte(table, rhs, depth + 1);
    }

    None
}

fn spans_for(lit: &NumeralLiteral) -> Vec<ColorSpan> {
    if lit.value <= 0xFF {
        let Some(idx) = ink_index_from_ga_value(lit.value as u8)
        else {
            return Vec::new();
        };
        return vec![ColorSpan {
            line: lit.line,
            start: lit.token_start,
            end: lit.token_end,
            ink_idx: idx,
            prefix: lit.prefix.clone(),
            suffix: lit.suffix.clone(),
            style: lit.style
        }];
    }

    if lit.value > 0xFFFF {
        return Vec::new();
    }

    // 16-bit: only a byte-aligned digit run (exactly 4 hex digits / 16
    // binary digits — decimal is never split, since decimal digit
    // boundaries don't align to bytes) has a sensible high/low split.
    let per_byte = lit.style.digits_per_byte();
    if per_byte == 0 || (lit.digits_end - lit.digits_start) as usize != per_byte * 2 {
        return Vec::new();
    }

    let hi = (lit.value >> 8) as u8;
    let lo = (lit.value & 0xFF) as u8;
    let hi_idx = ink_index_from_ga_value(hi);
    let lo_idx = ink_index_from_ga_value(lo);
    let eligible =
        matches!((hi_idx, lo_idx), (Some(_), Some(_))) || (hi == 0x7F && lo_idx.is_some());
    if !eligible {
        return Vec::new();
    }

    let mid = lit.digits_start + per_byte as u32;
    let mut out = Vec::new();
    if let Some(idx) = hi_idx {
        // High byte "before the number": spans the whole token's own start
        // (base prefix included) through the end of its digits.
        out.push(ColorSpan {
            line: lit.line,
            start: lit.token_start,
            end: mid,
            ink_idx: idx,
            prefix: lit.prefix.clone(),
            suffix: String::new(),
            style: lit.style
        });
    }
    if let Some(idx) = lo_idx {
        // Low byte "after": its own digits through the end of the token
        // (a trailing suffix, if any, belongs here — it trails the whole
        // number just like the low byte does).
        out.push(ColorSpan {
            line: lit.line,
            start: mid,
            end: lit.token_end,
            ink_idx: idx,
            prefix: String::new(),
            suffix: lit.suffix.clone(),
            style: lit.style
        });
    }
    out
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
    fn presentations_offer_all_32_ga_encodings_closest_first() {
        // All 32, duplicates included — each is a distinct, individually
        // valid GA byte even when a few share an RGB with an earlier index.
        let presentations = presentations_for("LD A, 0x54\n"); // ink 0 = black
        assert_eq!(presentations.len(), 32);
        assert_eq!(presentations[0].label, "Ink 0 (0x54)");
    }

    #[test]
    fn duplicated_indices_offer_their_own_distinct_ga_byte() {
        // Ink 27 shares ink 13's RGB but is a different, independently
        // selectable GA byte (0x41 vs 0x40) — must appear in the list too.
        let presentations = presentations_for("LD A, 0x54\n");
        let dup = presentations
            .iter()
            .find(|p| p.label.starts_with("Ink 27 "))
            .expect("ink 27 should be offered");
        assert_eq!(dup.label, "Ink 27 (0x41)");
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

    #[test]
    fn hash_and_ampersand_hex_prefixes_are_recognized() {
        // These forms only work now that the scanner delegates to
        // cpclib_common::parse::scan_numeric_literals (the real basm
        // numeral grammar) instead of a hand-rolled subset of it.
        let colors = colors_for("LD A, #54\nLD B, &54\n");
        assert_eq!(colors.len(), 2, "{colors:?}");
    }

    #[test]
    fn trailing_h_suffix_hex_form_is_recognized() {
        let colors = colors_for("LD A, 54h\n");
        assert_eq!(colors.len(), 1, "{colors:?}");
        assert_eq!(colors[0].range.start.character, "LD A, ".len() as u32);
        assert_eq!(colors[0].range.end.character, "LD A, 54h".len() as u32);
    }

    #[test]
    fn trailing_h_suffix_on_a_16_bit_low_byte_is_kept_by_the_low_swatch() {
        // "7F40h": only the low byte (0x40) is a color; the trailing "h"
        // suffix trails the whole number, same side as the low byte.
        let colors = colors_for("LD BC, 7F40h\n");
        assert_eq!(colors.len(), 1, "{colors:?}");
        assert_eq!(colors[0].range.end.character, "LD BC, 7F40h".len() as u32);
    }

    #[test]
    fn symbol_references_through_an_include_are_colorized_via_the_alias_chain() {
        // GA_WHITE -> GA_COL_13 -> 0x40 (ink 13, gray) and GA_BLACK ->
        // GA_COL_00 -> 0x54 (ink 0, black) — a two-hop alias chain defined
        // entirely inside the embedded inner://ga.asm resource.
        let text = "include once \"inner://ga.asm\"\n\ndb GA_WHITE, GA_BLACK\n";
        let colors = colors_for(text);
        assert_eq!(colors.len(), 2, "{colors:?}");
        assert!(colors.iter().all(|c| c.range.start.line == 2), "{colors:?}");

        let white_col = "db ".len() as u32;
        let white = colors
            .iter()
            .find(|c| c.range.start.character == white_col)
            .expect("GA_WHITE swatch");
        assert_eq!(
            white.range.end.character,
            white_col + "GA_WHITE".len() as u32
        );
        assert_eq!(
            white.color,
            Color {
                red: 0x80 as f32 / 255.0,
                green: 0x80 as f32 / 255.0,
                blue: 0x80 as f32 / 255.0,
                alpha: 1.0
            }
        );

        let black_col = "db GA_WHITE, ".len() as u32;
        let black = colors
            .iter()
            .find(|c| c.range.start.character == black_col)
            .expect("GA_BLACK swatch");
        assert_eq!(
            black.range.end.character,
            black_col + "GA_BLACK".len() as u32
        );
        assert_eq!(
            black.color,
            Color {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                alpha: 1.0
            }
        );
    }

    #[test]
    fn symbol_reference_swatches_offer_no_color_presentations() {
        let text = "include once \"inner://ga.asm\"\n\ndb GA_WHITE\n";
        let colors = colors_for(text);
        assert_eq!(colors.len(), 1, "{colors:?}");

        let uri = Url::parse("file:///t.asm").unwrap();
        let doc = Document::new(uri, text.to_string(), 1);
        let presentations = AssemblyAnalyzer::new().color_presentations(
            &doc,
            Color {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                alpha: 1.0
            },
            colors[0].range
        );
        assert!(
            presentations.is_empty(),
            "symbol-reference swatches should be read-only, got {presentations:?}"
        );
    }
}
