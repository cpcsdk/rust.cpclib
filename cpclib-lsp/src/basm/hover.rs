//! Hover for assembly files: instruction timings, register/directive
//! documentation, numeric-literal bases, label/symbol info, embedded BASIC.
//!
//! Logic (what is under the cursor) is in `hover()`; the markdown-building
//! rendering lives in the free helper functions below it.

use cpclib_tokens::ListingElement;
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use super::diagnostics::collect_asm_diagnostics;
use super::embedded_basic::extract_locomotive_blocks;
use super::token::*;
use crate::common::document::Document;
use crate::common::render::make_hover;

impl AssemblyAnalyzer {
    /// Provide hover information at the given position
    pub fn hover(&self, document: &Document, position: Position) -> Option<Hover> {
        let line_idx = position.line as usize;
        let line = document.line(line_idx)?;
        let col = position.character as usize;

        // Delegate to BASIC hover when the cursor is inside a LOCOMOTIVE block.
        {
            let text = document.text();
            let loco_blocks = extract_locomotive_blocks(&text);
            if let Some(block) = loco_blocks
                .iter()
                .find(|b| b.basic_range.contains(&line_idx))
            {
                let all_lines: Vec<&str> = text.lines().collect();
                let basic_text: String = block
                    .basic_range
                    .clone()
                    .map(|i| all_lines[i])
                    .collect::<Vec<_>>()
                    .join("\n");
                let basic_line = position.line - block.basic_range.start as u32;
                let line_trimmed = line.trim_end_matches(|c: char| c == '\n' || c == '\r');
                return crate::locomotive::hover::locomotive_basic_hover(
                    line_trimmed,
                    &basic_text,
                    basic_line,
                    position.character
                );
            }
        }

        // Hovering an included filename (INCLUDE/INCBIN/BINCLUDE): preview
        // its content — a real file on disk or an embedded `inner://...`
        // resource are equally valid directive arguments, and equally worth
        // previewing without leaving the current file.
        if let Some(filename) = super::definition::include_filename_at(&line, col)
            && let Some(content) = super::includes::read_included_file(&filename, &document.uri)
        {
            return Some(make_hover(format_include_preview(&filename, &content)));
        }

        // Numeric literal — show all bases
        if let Some((num_str, value)) = extract_number_at_position(&line, col) {
            return Some(make_hover(crate::common::render::format_number_hover(
                &num_str, value
            )));
        }

        let word = self.extract_word_at_position(&line, col)?;
        let word_upper = word.to_uppercase();

        // Instruction — show timing data from the full instruction line
        if INSTRUCTION_SET.contains(word_upper.as_str()) {
            let full = super::timing::extract_instruction_at_col(&line, col)
                .unwrap_or_else(|| word.clone());
            let entries = super::timing::find_timings(&full);
            let md = if entries.is_empty() {
                format!("**{}** — Z80 instruction", word_upper)
            }
            else {
                super::timing::format_hover(&full, &entries)
            };
            return Some(make_hover(md));
        }

        // Register / condition code
        if let Some(md) = register_description(&word_upper) {
            return Some(make_hover(md));
        }

        // Assembler directive — look up in documentation generated from directives.md
        if let Some(md) = directive_hover(&word_upper) {
            return Some(make_hover(md));
        }

        // SNASET flag (e.g. `Z80_AF`, `GA_PAL:5`) — its specific purpose,
        // scraped from the same `### SNASET` section's flag list.
        if let Some(md) = snaset_flag_hover(&word_upper) {
            return Some(make_hover(md));
        }

        // Symbol — look up EQU / assign / label in the listing (only when parse succeeds)
        if let Ok(listing) = self.parse_document(document) {
            for token in super::token::flatten_listing(listing.iter()) {
                if token.is_equ() {
                    let sym = token.equ_symbol();
                    if sym.to_uppercase() == word_upper {
                        return Some(make_hover(format!(
                            "**{}** = `{}`\n\n*EQU constant*",
                            sym,
                            token.equ_value()
                        )));
                    }
                }
                else if token.is_assign() {
                    let sym = token.assign_symbol();
                    if sym.to_uppercase() == word_upper {
                        return Some(make_hover(format!(
                            "**{}** = `{}`\n\n*Assign*",
                            sym,
                            token.assign_value()
                        )));
                    }
                }
                else if token.is_label() {
                    let sym = token.label_symbol();
                    if sym.to_uppercase() == word_upper {
                        return Some(make_hover(format!("**{}** — label", sym)));
                    }
                }
            }
        }

        // Fallback: if the document has an assembly error on this line, show it
        // using an `ansi` code block so ANSI escape codes (colors from codespan)
        // are rendered — red for `error:` / `^` lines, blue/cyan for line numbers.
        // The raw (non-stripped) error string is used so colors are preserved.
        if let Err(listing_with_errors) = self.parse_document(document) {
            let error = listing_with_errors.cpclib_error_unchecked();
            let mut diags = Vec::new();
            collect_asm_diagnostics(error, None, &mut diags);
            if diags.iter().any(|d| d.range.start.line == position.line) {
                return Some(make_hover(format!("```ansi\n{error}\n```")));
            }
        }

        None
    }
}

/// Detect a numeric literal under `col` and return its text + i64 value.
/// Handles `$`, `&`, `#` (hex), `%` (binary), `0x`/`0b`/`0o`, and plain decimal.
fn extract_number_at_position(line: &str, col: usize) -> Option<(String, i64)> {
    let bytes = line.as_bytes();
    if col >= bytes.len() {
        return None;
    }
    let ch = bytes[col];
    let is_hex_digit = |b: u8| b.is_ascii_digit() || matches!(b, b'a'..=b'f' | b'A'..=b'F');
    let is_prefix = |b: u8| matches!(b, b'$' | b'%' | b'&' | b'#');

    // Cursor must be on a digit, hex letter, or a numeric prefix
    if !ch.is_ascii_digit() && !is_prefix(ch) && !is_hex_digit(ch) {
        return None;
    }

    // Scan backward over alphanumeric chars to find the token start
    let mut start = col;
    while start > 0 && bytes[start - 1].is_ascii_alphanumeric() {
        start -= 1;
    }
    // Include a single-char prefix ($, %, &, #) immediately before the digits
    if start > 0 && is_prefix(bytes[start - 1]) {
        start -= 1;
    }

    // Scan forward to end of token
    let body_start = if is_prefix(bytes[start]) {
        start + 1
    }
    else {
        start
    };
    let mut end = body_start;
    // Consume a 0x / 0b / 0o prefix if present
    if end + 1 < bytes.len()
        && bytes[end] == b'0'
        && matches!(bytes[end + 1], b'x' | b'X' | b'b' | b'B' | b'o' | b'O')
    {
        end += 2;
    }
    while end < bytes.len() && bytes[end].is_ascii_alphanumeric() {
        end += 1;
    }

    if start >= end {
        return None;
    }
    let num_str = &line[start..end];

    let value: i64 = if let Some(h) = num_str
        .strip_prefix('$')
        .or_else(|| num_str.strip_prefix('&'))
        .or_else(|| num_str.strip_prefix('#'))
    {
        i64::from_str_radix(h, 16).ok()?
    }
    else if let Some(b) = num_str.strip_prefix('%') {
        i64::from_str_radix(b, 2).ok()?
    }
    else if let Some(h) = num_str
        .strip_prefix("0x")
        .or_else(|| num_str.strip_prefix("0X"))
    {
        i64::from_str_radix(h, 16).ok()?
    }
    else if let Some(b) = num_str
        .strip_prefix("0b")
        .or_else(|| num_str.strip_prefix("0B"))
    {
        i64::from_str_radix(b, 2).ok()?
    }
    else if let Some(o) = num_str
        .strip_prefix("0o")
        .or_else(|| num_str.strip_prefix("0O"))
    {
        i64::from_str_radix(o, 8).ok()?
    }
    else if num_str.bytes().all(|b| b.is_ascii_digit()) {
        num_str.parse().ok()?
    }
    else {
        return None;
    };

    Some((num_str.to_string(), value))
}

fn register_description(upper: &str) -> Option<String> {
    let desc = match upper {
        "A" => "**A** — Accumulator (8-bit). Primary register for arithmetic/logic.",
        "B" => "**B** — 8-bit general purpose register.",
        "C" => "**C** — 8-bit general purpose register. Also the carry condition code.",
        "D" => "**D** — 8-bit general purpose register.",
        "E" => "**E** — 8-bit general purpose register.",
        "H" => "**H** — High byte of HL.",
        "L" => "**L** — Low byte of HL.",
        "F" => "**F** — Flags register (8-bit). Bits: S Z 5 H 3 P/V N C.",
        "BC" => "**BC** — 16-bit register pair (B:C). Often used as counter or source address.",
        "DE" => "**DE** — 16-bit register pair (D:E). Often used as destination pointer.",
        "HL" => "**HL** — 16-bit register pair (H:L). Primary 16-bit address register.",
        "AF" => "**AF** — Accumulator + Flags register pair.",
        "AF'" => "**AF'** — Alternate Accumulator + Flags register pair (shadow).",
        "IX" => "**IX** — 16-bit index register X. Used with `(IX+d)` displacement addressing.",
        "IY" => "**IY** — 16-bit index register Y. Used with `(IY+d)` displacement addressing.",
        "SP" => "**SP** — Stack Pointer (16-bit). Points to the top of the hardware stack.",
        "PC" => "**PC** — Program Counter (16-bit). Points to the next instruction to execute.",
        "I" => {
            "**I** — Interrupt vector register (8-bit). High byte of the IM 2 vector table address."
        },
        "R" => "**R** — Memory Refresh register (8-bit). Auto-incremented each M1 machine cycle.",
        "IXH" => "**IXH** — High byte of IX (undocumented).",
        "IXL" => "**IXL** — Low byte of IX (undocumented).",
        "IYH" => "**IYH** — High byte of IY (undocumented).",
        "IYL" => "**IYL** — Low byte of IY (undocumented).",
        "NZ" => "**NZ** — Condition code: not zero (Z=0).",
        "Z" => "**Z** — Condition code: zero (Z=1).",
        "NC" => "**NC** — Condition code: no carry (C=0).",
        "PE" => "**PE** — Condition code: parity even / overflow set (P/V=1).",
        "PO" => "**PO** — Condition code: parity odd / overflow clear (P/V=0).",
        "P" => "**P** — Condition code: positive / sign clear (S=0).",
        "M" => "**M** — Condition code: minus / sign set (S=1).",
        _ => return None
    };
    Some(desc.to_string())
}

// ─── Included-file content preview ───────────────────────────────────────────

/// Cap on how much of an included file's content is shown in the hover, so a
/// large source file doesn't produce an unwieldy tooltip.
const INCLUDE_PREVIEW_MAX_LINES: usize = 60;

/// Render a markdown preview of an included file's content, truncated to
/// `INCLUDE_PREVIEW_MAX_LINES`.
fn format_include_preview(filename: &str, content: &str) -> String {
    let total_lines = content.lines().count();
    let preview: String = content
        .lines()
        .take(INCLUDE_PREVIEW_MAX_LINES)
        .collect::<Vec<_>>()
        .join("\n");

    let mut md = format!("**{filename}**\n\n```z80\n{preview}\n");
    if total_lines > INCLUDE_PREVIEW_MAX_LINES {
        md.push_str(&format!(
            "... ({} more line{})\n",
            total_lines - INCLUDE_PREVIEW_MAX_LINES,
            if total_lines - INCLUDE_PREVIEW_MAX_LINES == 1 {
                ""
            }
            else {
                "s"
            }
        ));
    }
    md.push_str("```");
    md
}

// ─── Directive documentation (generated from docs/basm/directives.md) ────────

use super::token::{DIRECTIVE_DOCS, SNASET_FLAGS};

/// Look up an assembler directive by name (case-insensitive) and return a
/// markdown hover string, or `None` if not found.
fn directive_hover(word_upper: &str) -> Option<String> {
    DIRECTIVE_DOCS
        .iter()
        .find(|(names, _)| names.iter().any(|n| n.to_uppercase() == word_upper))
        .map(|(_, doc)| doc.to_string())
}

/// Look up a SNASET flag (e.g. `Z80_AF`) by name and return its specific
/// purpose. An indexed family like `GA_PAL:n` is matched by its base name
/// alone (`GA_PAL`) — word extraction stops at `:`, so hovering the actual
/// usage `GA_PAL:5` only ever offers `GA_PAL` as the word under the cursor.
fn snaset_flag_hover(word_upper: &str) -> Option<String> {
    SNASET_FLAGS
        .iter()
        .find(|(name, _)| name.strip_suffix(":n").unwrap_or(name).to_uppercase() == word_upper)
        .map(|(name, desc)| format!("**{name}** — SNASET flag\n\n{desc}"))
}

#[cfg(test)]
mod include_preview_tests {
    use super::*;

    fn hover_at(text: &str, uri: Url, line: u32, character: u32) -> Option<Hover> {
        let doc = Document::new(uri, text.to_string(), 1);
        AssemblyAnalyzer::new().hover(&doc, Position { line, character })
    }

    fn markdown(hover: &Hover) -> &str {
        match &hover.contents {
            HoverContents::Markup(m) => m.value.as_str(),
            _ => panic!("expected markdown hover contents")
        }
    }

    #[test]
    fn hovering_an_on_disk_included_filename_previews_its_content() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("helper.asm"), "HELPER_LABEL:\n    ret\n").unwrap();
        let uri = Url::from_file_path(tmp.path().join("main.asm")).unwrap();
        let text = "    include \"helper.asm\"\n";

        // Cursor inside "helper.asm".
        let hover = hover_at(text, uri, 0, 16).expect("hover on helper.asm");
        let md = markdown(&hover);
        assert!(md.contains("helper.asm"), "{md}");
        assert!(md.contains("HELPER_LABEL"), "{md}");
    }

    #[test]
    fn hovering_an_inner_included_filename_previews_its_content() {
        let uri = Url::parse("file:///main.asm").unwrap();
        let text = "    include \"inner://crtc.asm\"\n";

        // Cursor inside "inner://crtc.asm".
        let hover = hover_at(text, uri, 0, 20).expect("hover on inner://crtc.asm");
        let md = markdown(&hover);
        assert!(md.contains("inner://crtc.asm"), "{md}");
        assert!(md.contains("CRTC_REG_COUNTER"), "{md}");
    }

    #[test]
    fn hovering_an_unresolvable_include_yields_no_preview() {
        let uri = Url::parse("file:///main.asm").unwrap();
        let text = "    include \"does_not_exist.asm\"\n";
        assert!(hover_at(text, uri, 0, 20).is_none());
    }
}

#[cfg(test)]
mod symbol_hover_tests {
    use super::*;

    /// Regression test: a label wrapped in an `ifndef ... endif` header
    /// guard must still show hover info — `listing.iter()` alone only sees
    /// the top-level `IF` token, not what's inside it.
    #[test]
    fn hovering_a_label_wrapped_in_an_ifndef_guard_shows_its_info() {
        let uri = Url::parse("file:///guarded.asm").unwrap();
        let text = "    ifndef GUARD\nGUARDED_LABEL:\n    ret\n    endif\n";
        let doc = Document::new(uri, text.to_string(), 1);
        // Cursor inside "GUARDED_LABEL" on line 1.
        let hover = AssemblyAnalyzer::new()
            .hover(
                &doc,
                Position {
                    line: 1,
                    character: 5
                }
            )
            .expect("hover on GUARDED_LABEL");
        let md = match &hover.contents {
            HoverContents::Markup(m) => m.value.as_str(),
            _ => panic!("expected markdown hover contents")
        };
        assert!(md.contains("GUARDED_LABEL"), "{md}");
    }
}

#[cfg(test)]
mod directive_doc_hover_tests {
    use super::*;

    fn hover_at(text: &str, line: u32, character: u32) -> Option<Hover> {
        let uri = Url::parse("file:///t.asm").unwrap();
        let doc = Document::new(uri, text.to_string(), 1);
        AssemblyAnalyzer::new().hover(&doc, Position { line, character })
    }

    fn markdown(hover: &Hover) -> &str {
        match &hover.contents {
            HoverContents::Markup(m) => m.value.as_str(),
            _ => panic!("expected markdown hover contents")
        }
    }

    /// Regression test: `directives.md` documents some directives under a
    /// single heading joining their aliases with `/` (e.g.
    /// `### RANGE, DEFSECTION`, previously `### RANGE/DEFSECTION`) instead
    /// of `,` like every other heading. The doc-generation build script only
    /// split headings on `,`, so a `/`-joined heading became one opaque name
    /// ("RANGE/DEFSECTION") that could never match the bare word "RANGE" a
    /// user actually hovers - hover silently showed nothing.
    #[test]
    fn hovering_range_shows_its_documentation() {
        let text = "RANGE 0x4000, 0x8000, MY_SECTION\n";
        let hover = hover_at(text, 0, 2).expect("hover on RANGE");
        let md = markdown(&hover);
        assert!(md.contains("RANGE"), "{md}");
        assert!(md.contains("DEFSECTION"), "{md}");
    }

    #[test]
    fn hovering_defsection_shows_the_same_documentation() {
        let text = "DEFSECTION 0x4000, 0x8000, MY_SECTION\n";
        let hover = hover_at(text, 0, 2).expect("hover on DEFSECTION");
        let md = markdown(&hover);
        assert!(md.contains("RANGE"), "{md}");
        assert!(md.contains("DEFSECTION"), "{md}");
    }

    #[test]
    fn hovering_ifused_shows_its_documentation() {
        let text = "    ifused SOME_LABEL\n    endif\n";
        let hover = hover_at(text, 0, 6).expect("hover on ifused");
        let md = markdown(&hover);
        assert!(md.to_uppercase().contains("IFUSED"), "{md}");
        assert!(md.to_uppercase().contains("IFEXIST"), "{md}");
    }

    #[test]
    fn hovering_a_plain_snaset_flag_shows_its_own_purpose() {
        let text = "SNASET Z80_AF, 1\n";
        // Cursor inside "Z80_AF".
        let hover = hover_at(text, 0, 9).expect("hover on Z80_AF");
        let md = markdown(&hover);
        assert!(md.contains("Z80_AF"), "{md}");
        assert!(md.contains("16-bit register pairs"), "{md}");
    }

    #[test]
    fn hovering_an_indexed_snaset_flag_shows_its_family_purpose() {
        let text = "SNASET GA_PAL:5, 1\n";
        // Word extraction stops at ':', so the cursor lands on "GA_PAL".
        let hover = hover_at(text, 0, 9).expect("hover on GA_PAL");
        let md = markdown(&hover);
        assert!(md.contains("GA_PAL"), "{md}");
        assert!(md.contains("requires index"), "{md}");
    }
}
