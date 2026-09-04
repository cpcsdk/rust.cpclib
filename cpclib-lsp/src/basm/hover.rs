//! Hover for assembly files: instruction timings, register/directive
//! documentation, numeric-literal bases, label/symbol info, embedded BASIC.
//!
//! Logic (what is under the cursor) is in `hover()`; the markdown-building
//! rendering lives in the free helper functions below it.

use cpclib_asm::implementation::expression::ExprEvaluationExt;
use cpclib_tokens::{ListingElement, Register16};
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use super::diagnostics::collect_asm_diagnostics;
use super::embedded_basic::block_and_text_at;
use super::token::*;
use crate::common::document::Document;
use crate::common::render::make_hover;

impl AssemblyAnalyzer {
    /// Provide hover information at the given position
    pub fn hover(&self, document: &Document, position: Position) -> Option<Hover> {
        let line_idx = position.line as usize;
        let line = document.line(line_idx)?;
        let col = position.character as usize;
        let text = document.text();

        // Delegate to bndbuild hover when the cursor is inside a
        // `#!bndbuild` embedded block.
        if let Some(hover) = self.embedded_bndbuild_hover(document, position) {
            return Some(hover);
        }

        // Delegate to BASIC hover when the cursor is inside a LOCOMOTIVE block.
        if let Some((block, basic_text)) = block_and_text_at(&text, line_idx) {
            let basic_line = position.line - block.basic_range.start as u32;
            let line_trimmed = line.trim_end_matches(['\n', '\r']);
            return crate::locomotive::hover::locomotive_basic_hover(
                line_trimmed,
                &basic_text,
                basic_line,
                position.character,
                self.config().firmware_docs
            );
        }

        // Hovering an included filename (INCLUDE/INCBIN/BINCLUDE): preview
        // its content — a real file on disk or an embedded `inner://...`
        // resource are equally valid directive arguments, and equally worth
        // previewing without leaving the current file. `INCBIN` is binary
        // data, so it gets a hex/ASCII dump instead of a text preview.
        if let Some((directive, filename)) =
            self.include_directive_and_filename_at(document, position)
        {
            if directive == "INCBIN" {
                if let Some(hover) = self.incbin_hover(document, position, &filename) {
                    return Some(hover);
                }
            }
            else if let Some(content) =
                super::includes::read_included_file(&filename, &document.uri)
            {
                return Some(make_hover(format_include_preview(&filename, &content)));
            }
        }

        // Macro/struct call — show the expanded content for these arguments.
        if let Some(md) = self.macro_or_struct_call_hover(document, position) {
            return Some(make_hover(md));
        }

        // User-defined FUNCTION call — show its evaluated return value.
        if let Some(md) = self.function_call_hover(document, &line, col) {
            return Some(make_hover(md));
        }

        // Numeric literal — show all bases, plus firmware docs if the value
        // resolves to a known firmware routine/constant address.
        if let Some((num_str, value, _start)) = extract_number_at_position(&line, col) {
            let mut md = crate::common::render::format_number_hover(&num_str, value);
            if self.config().firmware_docs
                && let Some(doc) = crate::common::firmware_docs::lookup_by_value(value)
            {
                md.push_str(&format!("\n\n---\n\n**{}**\n\n{}", doc.symbol, doc.doc));
            }
            return Some(make_hover(md));
        }

        let word = self.extract_word_at_position(&line, col)?;
        let word_upper = word.to_uppercase();

        // Instruction — show timing data from the full instruction line
        if INSTRUCTION_SET.contains(word_upper.as_str()) {
            let full = super::timing::extract_instruction_at_col(&line, col)
                .unwrap_or_else(|| word.clone());

            // Parse once and reuse for both of the checks below (a "fake
            // instruction" needs its own breakdown; a real one can have its
            // pseudocode's placeholders substituted with the real hovered
            // operands) instead of parsing separately for each.
            let listing = self.parse_document(document).ok();
            let token = listing.as_ref().and_then(|l| {
                super::token::flatten_listing(l.iter()).find(|t| span_line(*t) == position.line)
            });

            let md = match token {
                // A "fake instruction" (e.g. `ld hl, sp`, assembled as
                // several real opcodes) isn't itself a real Z80
                // instruction, so looking it up directly in the timing
                // table is the wrong move: several unrelated real entries
                // can tie on `find_timings`'s scoring (e.g. `ld i,a` /
                // `ld a,i` / `ld sp,hl` / `ld sp,ix` all score the same
                // non-match against `hl,sp`) and all get shown at once.
                // basm's parser already tags these tokens with a real,
                // specific `is_fake_instruction()` query (not just "did
                // this token produce *any* warning", which `is_warning()`
                // alone can't distinguish from an unrelated warning kind),
                // so show what it actually expands to instead of querying
                // the table with text that was never in it.
                Some(token) if token.is_fake_instruction() => {
                    format_fake_instruction_hover(&full)
                        .unwrap_or_else(|| format!("**{}** — fake instruction", word_upper))
                },
                // Any other token, warning-wrapped or not: resolve its
                // operands (when they're plain expressions, e.g. a literal
                // or an `EQU`-defined symbol) so the pseudocode can show the
                // actual value instead of a generic placeholder.
                // `mnemonic()`/`mnemonic_arg1()`/`mnemonic_arg2()` already
                // look through any non-fake-instruction wrapper (e.g. the
                // redundant explicit `A,` accumulator prefix, or `WRITE
                // DIRECT` in `directives.rs`) transparently, so this covers
                // both real, non-warned tokens and those uniformly.
                Some(token) if !token.is_fake_instruction() => {
                    let (_, ops_text) = super::timing::split_head(&full);
                    let src_ops = super::timing::parse_ops(ops_text);
                    let mut resolved: Vec<Option<i32>> = vec![None; src_ops.len()];
                    if let Some(l) = &listing {
                        let mut env = self.local_symbols_env_cached(document, l);
                        for (i, arg) in [token.mnemonic_arg1(), token.mnemonic_arg2()]
                            .into_iter()
                            .flatten()
                            .enumerate()
                        {
                            if let cpclib_asm::parser::obtained::LocatedDataAccess::Expression(expr) =
                                arg
                                && let Some(slot) = resolved.get_mut(i)
                            {
                                *slot = expr.resolve(&mut env).ok().and_then(|v| v.int_value().ok());
                            }
                        }
                    }
                    let entries = super::timing::find_timings(&full);
                    if entries.is_empty() {
                        format!("**{}** — Z80 instruction", word_upper)
                    }
                    else {
                        let known_bc = listing
                            .as_ref()
                            .and_then(|l| self.known_bc_for_hover(document, l, position, &entries));
                        super::timing::format_hover(&full, &entries, &src_ops, &resolved, known_bc)
                    }
                },
                // No usable token (parse failed, e.g. mid-edit syntax
                // error, or an unrelated warning-wrapped construct) - fall
                // back to the text-only path, no substitution possible.
                _ => {
                    let entries = super::timing::find_timings(&full);
                    if entries.is_empty() {
                        format!("**{}** — Z80 instruction", word_upper)
                    }
                    else {
                        let known_bc = listing
                            .as_ref()
                            .and_then(|l| self.known_bc_for_hover(document, l, position, &entries));
                        super::timing::format_hover(&full, &entries, &[], &[], known_bc)
                    }
                }
            };
            return Some(make_hover(md));
        }

        // Register / condition code - plus, when this is a tracked
        // register, its statically-known value (or an explicit "not known")
        // at this exact point, walked forward from the nearest control-flow
        // boundary via `registers::register_state_at`.
        if let Some(mut md) = register_description(&word_upper) {
            if let Ok(listing) = self.parse_document(document) {
                let mut env = self.local_symbols_env_cached(document, &listing);
                let state =
                    super::registers::register_state_at(&listing, &mut env, position, &word_upper);
                let contract = super::token::label_scope_at_line(listing.iter(), position.line)
                    .and_then(|(_, scope)| {
                        super::registers::parse_function_contract(&text, scope.start)
                    });
                if let Some(extra) =
                    super::registers::format_known_value(&word_upper, &state, contract.as_ref())
                {
                    md.push_str("\n\n---\n");
                    md.push_str(&extra);
                }
            }
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

        // Firmware routine/constant referenced by its symbolic name (e.g.
        // `call TXT_OUTPUT`) — tried only after the local-symbol lookup
        // above, so a locally-defined symbol of the same name always wins.
        if self.config().firmware_docs
            && let Some(doc) = crate::common::firmware_docs::lookup_by_symbol(&word_upper)
        {
            return Some(make_hover(format!(
                "**{}** = `{}`  \n*({})*\n\n{}",
                doc.symbol, doc.value, doc.source_file, doc.doc
            )));
        }

        // Fallback: if the document has an assembly error on this line, show it
        // using an `ansi` code block so ANSI escape codes (colors from codespan)
        // are rendered — red for `error:` / `^` lines, blue/cyan for line numbers.
        // The raw (non-stripped) error string is used so colors are preserved.
        if let Err(listing_with_errors) = self.parse_document(document) {
            let error = listing_with_errors.cpclib_error_unchecked();
            let mut diags = Vec::new();
            collect_asm_diagnostics(error, None, document, &mut diags);
            if diags.iter().any(|d| d.range.start.line == position.line) {
                return Some(make_hover(format!("```ansi\n{error}\n```")));
            }
        }

        None
    }

    /// Delegates hover to `BuildFileAnalyzer::hover` when `position` is
    /// inside a `#!bndbuild` embedded block, against a synthetic `Document`
    /// wrapping just the block's own text (the `run_embedded_rule` pattern).
    /// No out-translation needed: every `Hover` `bndbuild::hover` returns is
    /// built via `common::render::make_hover`, which always sets
    /// `range: None`.
    fn embedded_bndbuild_hover(&self, document: &Document, position: Position) -> Option<Hover> {
        let blocks = self.embedded_bndbuild_blocks(document);
        let block = super::embedded_bndbuild::block_at(&blocks, position.line as usize)?;
        let local_pos = super::embedded_bndbuild::position_into_block(block, position)?;
        let block_doc = Document::new(document.uri.clone(), block.yaml_text.clone(), 0);
        crate::bndbuild::BuildFileAnalyzer::new().hover(&block_doc, local_pos)
    }

    /// Hover for an `INCBIN` directive: a hex/ASCII dump of the actual bytes
    /// that get included, honoring `offset`/`length` arguments when present
    /// — showing raw text (like `INCLUDE`/`BINCLUDE` do) makes no sense for
    /// binary data.
    fn incbin_hover(
        &self,
        document: &Document,
        position: Position,
        filename: &str
    ) -> Option<Hover> {
        let bytes = super::includes::read_included_file_bytes(filename, &document.uri)?;

        // Resolve `offset`/`length` (possibly symbolic expressions) against
        // the real document — same machinery macro/FUNCTION hover uses.
        let listing = self.parse_document(document).ok()?;
        let token = super::token::flatten_listing(listing.iter())
            .find(|t| t.is_incbin() && span_line(*t) == position.line)?;

        let mut env = self.local_symbols_env_cached(document, &listing);
        let offset = token
            .incbin_offset()
            .and_then(|e| e.resolve(&mut env).ok())
            .and_then(|v| v.int_value().ok())
            .filter(|v| *v >= 0)
            .map(|v| v as usize)
            .unwrap_or(0);
        let length = token
            .incbin_length()
            .and_then(|e| e.resolve(&mut env).ok())
            .and_then(|v| v.int_value().ok())
            .filter(|v| *v >= 0)
            .map(|v| v as usize);

        let start = offset.min(bytes.len());
        let end = match length {
            Some(len) => (start + len).min(bytes.len()),
            None => bytes.len()
        };
        let slice = &bytes[start..end];

        let mut md = format!("**{filename}** ({} bytes", slice.len());
        if start > 0 || end < bytes.len() {
            md.push_str(&format!(
                ", showing bytes {start}..{end} of {}",
                bytes.len()
            ));
        }
        md.push_str(")\n\n");
        md.push_str(&format_hex_dump(slice, 16));

        Some(make_hover(md))
    }

    /// BC's statically-known value entering `position`'s instruction, via
    /// `registers::register_state_at` - only worth computing (a full
    /// backward walk) when `entries` actually includes a block-repeat
    /// mnemonic (`timing::is_block_repeat`, e.g. LDIR/OTIR), the only place
    /// `format_hover` uses it; `None` otherwise, including when BC's value
    /// isn't statically known at this point.
    fn known_bc_for_hover(
        &self,
        document: &Document,
        listing: &cpclib_asm::parser::obtained::LocatedListing,
        position: Position,
        entries: &[&super::timing::TimingEntry]
    ) -> Option<i32> {
        if !entries
            .iter()
            .any(|e| super::timing::is_block_repeat(e.mnemonic))
        {
            return None;
        }
        let mut env = self.local_symbols_env_cached(document, listing);
        super::registers::register_state_at(listing, &mut env, position, "BC").get16(Register16::Bc)
    }
}

/// Hover content for a "fake instruction" (e.g. `ld hl, sp`, assembled
/// using several real opcodes): a numbered breakdown, one step per real
/// instruction it actually expands to, each rendered with the *same*
/// timing-table formatting (`find_timings`/`format_hover`) used for every
/// other instruction hover in this file — bytes, NOPs, opcodes and flags
/// all come from that one real reference table, not a separate/ad hoc one.
/// The real instruction(s) themselves are recovered by actually assembling
/// `full` and disassembling the result (`disassemble_snippet_lines`), so
/// this stays correct for any current or future fake instruction without
/// hardcoding its expansion here.
fn format_fake_instruction_hover(full: &str) -> Option<String> {
    let lines = super::disassemble::disassemble_snippet_lines(full)?;

    // Positional S,Z,5,H,3,V,N,C notation, same convention as
    // `TimingEntry.flags`/`describe_flags`. Since the flags register is one
    // shared piece of state, the value it holds *after* the whole sequence
    // runs is whichever step last touched each position - not simply "any
    // step touched it" - so this keeps the *last* non-`.` character seen at
    // each position while walking the steps in order, rather than merging
    // them some other way.
    let mut merged_flags = [b'.'; 8];
    let mut sections = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        let entries = super::timing::find_timings(line);
        let body = if entries.is_empty() {
            format!("**{line}**")
        }
        else {
            if let Some(entry) = entries.first() {
                for (slot, ch) in merged_flags.iter_mut().zip(entry.flags.bytes()) {
                    if ch != b'.' {
                        *slot = ch;
                    }
                }
            }
            super::timing::format_hover(line, &entries, &[], &[], None)
        };
        sections.push(format!("**Step {}**\n\n{body}", i + 1));
    }

    let header = format!(
        "**{full}** — fake instruction, expands to {} real opcode{}\n\n",
        lines.len(),
        if lines.len() == 1 { "" } else { "s" }
    );
    let merged_flags_str = std::str::from_utf8(&merged_flags).unwrap_or("........");
    let footer = format!(
        "\n\n---\n**Flags after this sequence** `{merged_flags_str}`: {}",
        super::timing::describe_flags(merged_flags_str)
    );
    Some(format!("{header}{}{footer}", sections.join("\n---\n")))
}

/// Render `bytes` as a fixed-width hex + ASCII dump, 16 bytes per row,
/// capped at `max_rows` rows (with a "... N more bytes" suffix if there's
/// more) — used for `INCBIN` hover, where binary data would be nonsensical
/// to show as raw text.
///
/// The fence is tagged `text` (not left bare): an unlabeled fence in a
/// hover popup gets syntax-highlighted using the current document's
/// language grammar (Z80 asm), which colors the hex digits as numbers and
/// everything else differently — `text` disables highlighting entirely, so
/// the dump renders in one uniform color.
fn format_hex_dump(bytes: &[u8], max_rows: usize) -> String {
    const ROW_WIDTH: usize = 16;

    let mut out = String::from("```text\n");
    let mut offset = 0usize;
    for row in bytes.chunks(ROW_WIDTH).take(max_rows) {
        let hex: String = row.iter().map(|b| format!("{b:02X} ")).collect();
        let ascii: String = row
            .iter()
            .map(|&b| {
                if (0x20..0x7F).contains(&b) {
                    b as char
                }
                else {
                    '.'
                }
            })
            .collect();
        out.push_str(&format!("{offset:04X}  {hex:<48}{ascii}\n"));
        offset += row.len();
    }
    let shown_bytes = max_rows * ROW_WIDTH;
    if bytes.len() > shown_bytes {
        out.push_str(&format!("... {} more bytes\n", bytes.len() - shown_bytes));
    }
    out.push_str("```");
    out
}

/// Detect a numeric literal under `col` and return its text + i64 value.
/// Handles `$`, `&`, `#` (hex), `%` (binary), `0x`/`0b`/`0o`, and plain decimal.
/// The numeric literal at `col` (a digit, hex letter, or numeric prefix
/// `$`/`%`/`&`/`#`), if any: its own source text, resolved value, and
/// 0-based byte start column - `pub(super)` so `refactor.rs`'s firmware-
/// symbol quickfix can reuse the exact same detection hover already uses,
/// rather than re-implementing the scan.
pub(super) fn extract_number_at_position(line: &str, col: usize) -> Option<(String, i64, usize)> {
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
    let value = crate::common::render::parse_numeric_literal_str(num_str)?;

    Some((num_str.to_string(), value, start))
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
mod locomotive_block_tests {
    use super::*;

    /// Regression test for the LOCOMOTIVE-block dedup (`block_and_text_at`):
    /// hovering inside a `LOCOMOTIVE` block must still delegate to BASIC
    /// hover with the correctly-reconstructed block text.
    #[test]
    fn hovering_inside_a_locomotive_block_delegates_to_basic_hover() {
        let uri = Url::parse("file:///t.asm").unwrap();
        let text = "ORG 0x8000\nLOCOMOTIVE\n10 CLS\nENDLOCOMOTIVE\n";
        let doc = Document::new(uri, text.to_string(), 1);
        // Cursor on "CLS" (line 2, a BASIC keyword).
        let hover = AssemblyAnalyzer::new()
            .hover(
                &doc,
                Position {
                    line: 2,
                    character: 4
                }
            )
            .expect("expected BASIC hover inside the LOCOMOTIVE block");
        match hover.contents {
            HoverContents::Markup(m) => assert!(m.value.contains("CLS"), "{}", m.value),
            _ => panic!("expected markdown hover contents")
        }
    }
}

#[cfg(test)]
mod embedded_bndbuild_hover_tests {
    use super::*;

    #[test]
    fn hovering_outside_an_embedded_bndbuild_block_still_gets_normal_asm_hover() {
        // Proves the block-scoped delegation (tested in the next test)
        // doesn't swallow the whole document - a real asm construct on a
        // *different* line still gets real asm hover.
        let uri = Url::parse("file:///t.asm").unwrap();
        let text = "; #!bndbuild\n; - tgt: test\n;   cmd: echo hi\nORG 0x8000\n";
        let doc = Document::new(uri, text.to_string(), 1);

        let on_a_different_line = AssemblyAnalyzer::new()
            .hover(
                &doc,
                Position {
                    line: 3,
                    character: 1
                }
            )
            .expect("expected normal asm hover outside the block");
        match on_a_different_line.contents {
            HoverContents::Markup(m) => assert!(m.value.contains("ORG"), "{}", m.value),
            _ => panic!("expected markdown hover contents")
        }
    }

    #[test]
    fn hovering_a_bndbuild_keyword_inside_an_embedded_block_shows_bndbuild_style_hover() {
        // "tgt" is a real bndbuild keyword (`BuildFileAnalyzer::hover`'s
        // `get_keyword_help` path) - if suppression is scoped correctly, a
        // cursor on it inside an embedded block should get *that*
        // documentation, not `None` (asm hover doesn't recognize "tgt" as
        // anything) and not a crash.
        let uri = Url::parse("file:///t.asm").unwrap();
        let text = "; #!bndbuild\n; - tgt: test\n;   cmd: echo hi\n";
        let doc = Document::new(uri, text.to_string(), 1);

        // Cursor on "tgt" (block-local line 0, "- tgt: test" content starts
        // at outer-doc column 2, "tgt" itself two further in).
        let hover = AssemblyAnalyzer::new()
            .hover(
                &doc,
                Position {
                    line: 1,
                    character: 6
                }
            )
            .expect("expected bndbuild keyword hover inside the embedded block");
        match hover.contents {
            HoverContents::Markup(m) => {
                assert!(m.value.to_lowercase().contains("target"), "{}", m.value)
            },
            _ => panic!("expected markdown hover contents")
        }
    }
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

    #[test]
    fn hovering_an_incbin_filename_shows_a_hex_dump_not_raw_text() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("data.bin"), [0u8, 1, 2, 3, b'h', b'i']).unwrap();
        let uri = Url::from_file_path(tmp.path().join("main.asm")).unwrap();
        let text = "    incbin \"data.bin\"\n";

        // Cursor inside "data.bin".
        let hover = hover_at(text, uri, 0, 15).expect("hover on data.bin");
        let md = markdown(&hover);
        assert!(md.contains("data.bin"), "{md}");
        assert!(md.contains("6 bytes"), "{md}");
        assert!(md.contains("00 01 02 03"), "{md}");
        assert!(md.contains("hi"), "{md}");
        // Fenced as `text`, not left bare — an unlabeled fence gets
        // syntax-highlighted using the document's language grammar, which
        // colors hex digits differently from the rest ("several colors").
        assert!(md.contains("```text"), "{md}");
    }

    #[test]
    fn incbin_hover_respects_offset_and_length_arguments() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("data.bin"),
            [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9]
        )
        .unwrap();
        let uri = Url::from_file_path(tmp.path().join("main.asm")).unwrap();
        let text = "    incbin \"data.bin\", 2, 3\n";

        // Cursor inside "data.bin".
        let hover = hover_at(text, uri, 0, 15).expect("hover on data.bin");
        let md = markdown(&hover);
        assert!(md.contains("3 bytes"), "{md}");
        assert!(md.contains("showing bytes 2..5 of 10"), "{md}");
        assert!(md.contains("02 03 04"), "{md}");
        assert!(!md.contains("06 07"), "{md}");
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
mod firmware_hover_tests {
    use super::*;
    use crate::common::config::AsmConfig;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///t.asm").unwrap(), text.to_string(), 1)
    }

    fn md_of(hover: Hover) -> String {
        match hover.contents {
            HoverContents::Markup(m) => m.value,
            _ => panic!("expected markdown hover contents")
        }
    }

    #[test]
    fn hovering_a_firmware_address_shows_its_doc() {
        let text = "    call &BB5A\n";
        // Cursor on the "B" of "BB5A".
        let hover = AssemblyAnalyzer::new()
            .hover(
                &doc(text),
                Position {
                    line: 0,
                    character: 11
                }
            )
            .expect("hover on &BB5A");
        let md = md_of(hover);
        assert!(md.contains("TXT_OUTPUT"), "{md}");
    }

    #[test]
    fn hovering_a_firmware_symbol_name_shows_its_doc() {
        let text = "    call TXT_OUTPUT\n";
        let hover = AssemblyAnalyzer::new()
            .hover(
                &doc(text),
                Position {
                    line: 0,
                    character: 11
                }
            )
            .expect("hover on TXT_OUTPUT");
        let md = md_of(hover);
        assert!(md.contains("TXT_OUTPUT"), "{md}");
    }

    #[test]
    fn firmware_docs_disabled_via_config_suppresses_both() {
        let analyzer = AssemblyAnalyzer::new();
        analyzer.set_config(AsmConfig {
            firmware_docs: false,
            ..AsmConfig::default()
        });

        let by_value = analyzer.hover(
            &doc("    call &BB5A\n"),
            Position {
                line: 0,
                character: 11
            }
        );
        // The numeric-literal branch itself still fires (base conversions),
        // just without the firmware doc appended.
        let md = md_of(by_value.expect("still shows base conversions"));
        assert!(!md.contains("TXT_OUTPUT"), "{md}");

        let by_symbol = analyzer.hover(
            &doc("    call TXT_OUTPUT\n"),
            Position {
                line: 0,
                character: 11
            }
        );
        assert!(by_symbol.is_none(), "{by_symbol:?}");
    }

    #[test]
    fn a_locally_defined_symbol_of_the_same_name_wins_over_firmware_docs() {
        let text = "TXT_OUTPUT equ 42\n    call TXT_OUTPUT\n";
        let hover = AssemblyAnalyzer::new()
            .hover(
                &doc(text),
                Position {
                    line: 1,
                    character: 11
                }
            )
            .expect("hover on TXT_OUTPUT");
        let md = md_of(hover);
        assert!(md.contains("EQU constant"), "{md}");
        assert!(!md.contains("Action:"), "{md}");
    }
}

#[cfg(test)]
mod timing_hover_tests {
    use super::*;

    fn hover_at_line(text: &str, line: u32, character: u32) -> String {
        let uri = Url::parse("file:///main.asm").unwrap();
        let doc = Document::new(uri, text.to_string(), 1);
        let hover = AssemblyAnalyzer::new()
            .hover(&doc, Position { line, character })
            .expect("expected a hover result");
        match &hover.contents {
            HoverContents::Markup(m) => m.value.clone(),
            _ => panic!("expected markdown hover contents")
        }
    }

    #[test]
    fn pseudocode_substitutes_a_literal_immediate() {
        let md = hover_at_line("    org 0x4000\n    ld bc, 5\n", 1, 5);
        assert!(md.contains("`BC <- 5`"), "{md}");
    }

    #[test]
    fn pseudocode_substitutes_a_hex_literal_immediate_in_hex() {
        // The substituted value must be shown in the same base the source
        // wrote it in - not silently converted to decimal.
        let md = hover_at_line("    org 0x4000\n    ld bc, 0x2c\n", 1, 5);
        assert!(md.contains("`BC <- 0x2c`"), "{md}");
    }

    #[test]
    fn pseudocode_substitutes_a_binary_literal_immediate_in_binary() {
        let md = hover_at_line("    org 0x4000\n    ld bc, 0b101\n", 1, 5);
        assert!(md.contains("`BC <- 0b101`"), "{md}");
    }

    #[test]
    fn pseudocode_substitutes_an_equ_resolved_symbol() {
        // Per the original request: pseudocode substitution must work "for
        // indirection with variables" too, not just literal immediates -
        // resolving `val` against the fully-assembled `Env`'s symbol table
        // (the same `dry_run_env` machinery used elsewhere in this crate)
        // covers this the same way it covers a literal. A symbol reference
        // has no "original base" of its own to preserve (the base only
        // lives in `val`'s own definition, not at this use site), so it
        // falls back to hexadecimal - the same convention `overflow.rs`
        // already established for overflow-warning values.
        let md = hover_at_line("    org 0x4000\n    val equ 9\n    ld b, val\n", 2, 5);
        assert!(md.contains("`B <- 0x9`"), "{md}");
    }

    #[test]
    fn pseudocode_substitutes_register_operands_with_explicit_bit_semantics() {
        let md = hover_at_line("    org 0x4000\n    rlc c\n", 1, 5);
        assert!(
            md.contains("`Carry <- C.7, C <- (C shl 1) | C.7(old)`"),
            "{md}"
        );
    }

    #[test]
    fn hover_still_works_without_substitution_when_the_document_has_a_syntax_error() {
        // A syntax error elsewhere means the document doesn't currently
        // parse, so no token/resolution data is available for the hovered
        // instruction - the text-only fallback path must still produce a
        // hover (just without value substitution), not panic or return
        // nothing.
        let md = hover_at_line("    org 0x4000\n@#$ garbage @#$\n    nop\n", 2, 5);
        assert!(md.contains("NOP"), "{md}");
    }

    /// End-to-end regression test for `known_bc_for_hover`: when BC's value
    /// is statically known before an LDIR, hover must show the exact total,
    /// not the generic "unbounded" text.
    #[test]
    fn hovering_ldir_with_a_known_bc_shows_the_exact_total() {
        let md = hover_at_line("    org 0x4000\n    ld bc, 3\n    ldir\n", 2, 5);
        assert!(md.contains("17"), "{md}"); // 2*6 + 5
        assert!(!md.contains("unbounded"), "{md}");
    }

    /// Same instruction, but BC's value is never set - hover must say the
    /// total is unbounded, not a wrong flat number.
    #[test]
    fn hovering_ldir_with_an_unknown_bc_shows_unbounded() {
        let md = hover_at_line("    org 0x4000\n    ldir\n", 1, 5);
        assert!(md.contains("unbounded"), "{md}");
    }

    /// The user only cares about NOPs, not T-states — trim the latter from
    /// instruction hover text and keep the former.
    #[test]
    fn instruction_hover_shows_nops_but_not_t_states() {
        let uri = Url::parse("file:///main.asm").unwrap();
        let text = "    ld a, 1\n";
        let doc = Document::new(uri, text.to_string(), 1);
        // Cursor on "ld".
        let hover = AssemblyAnalyzer::new()
            .hover(
                &doc,
                Position {
                    line: 0,
                    character: 5
                }
            )
            .expect("hover on ld a, 1");
        let md = match &hover.contents {
            HoverContents::Markup(m) => m.value.as_str(),
            _ => panic!("expected markdown hover contents")
        };
        assert!(md.contains("NOP"), "{md}");
        assert!(!md.to_lowercase().contains("t-state"), "{md}");
    }

    #[test]
    fn fake_instruction_hover_shows_a_numbered_breakdown() {
        // `ld hl, sp` has no direct Z80 equivalent - basm expands it as
        // `ld hl, 0` followed by `add hl, sp`. Hovering it must show each
        // real step individually (numbered), each rendered with the exact
        // same timing-table format as a normal instruction hover (bytes,
        // NOPs, opcodes, flags) - not a flat "assembles as" line, and not
        // the unrelated multi-entry mess `find_timings("ld hl, sp")` would
        // produce if queried directly (several real `LD`-family patterns
        // tie on that unmatched text - see the regression test below).
        let uri = Url::parse("file:///main.asm").unwrap();
        let text = "    ld hl, sp\n";
        let doc = Document::new(uri, text.to_string(), 1);
        let hover = AssemblyAnalyzer::new()
            .hover(
                &doc,
                Position {
                    line: 0,
                    character: 5
                }
            )
            .expect("hover on ld hl, sp");
        let md = match &hover.contents {
            HoverContents::Markup(m) => m.value.as_str(),
            _ => panic!("expected markdown hover contents")
        };
        assert!(md.contains("fake instruction"), "{md}");
        assert!(md.contains("Step 1") && md.contains("LD HL, 0x0"), "{md}");
        assert!(md.contains("Step 2") && md.contains("ADD HL, SP"), "{md}");
        // Each step's own NOPs/opcodes/flags, from the real timing table.
        assert!(md.contains("NOP"), "{md}");
        assert!(md.contains("Opcodes:"), "{md}");
        assert!(md.contains("Flags"), "{md}");
        // The merged, final flag state after the whole sequence runs: `ld
        // hl, 0` never touches flags, so the result is entirely determined
        // by `add hl, sp` (H, N, C affected; S, Z, P/V untouched, the one
        // asymmetric case for 16-bit ADD on real Z80 hardware).
        assert!(
            md.contains("Flags after this sequence") && md.contains("`..!!!.0C`"),
            "{md}"
        );
        // Not the unrelated real instructions a naive direct lookup of
        // "ld hl, sp" would have tied on and shown all of.
        assert!(!md.contains("ld i,a") && !md.contains("ld sp,ix"), "{md}");
    }
}

#[cfg(test)]
mod register_value_hover_tests {
    use super::*;

    fn hover_at_line(text: &str, line: u32, character: u32) -> String {
        let uri = Url::parse("file:///main.asm").unwrap();
        let doc = Document::new(uri, text.to_string(), 1);
        let hover = AssemblyAnalyzer::new()
            .hover(&doc, Position { line, character })
            .expect("expected a hover result");
        match &hover.contents {
            HoverContents::Markup(m) => m.value.clone(),
            _ => panic!("expected markdown hover contents")
        }
    }

    #[test]
    fn hovering_a_tracked_register_shows_its_known_value() {
        let md = hover_at_line("    ld a,5\n    ld b,a\n", 1, 9);
        assert!(md.contains("Known value at this point"), "{md}");
        assert!(md.contains("0x05"), "{md}");
    }

    #[test]
    fn hovering_a_register_with_no_known_value_says_so_explicitly() {
        // A fresh register at the very start of the file, never assigned -
        // must explicitly say "not known", not silently omit the line.
        let md = hover_at_line("    ld b,a\n", 0, 7);
        assert!(
            md.contains("Value not statically known at this point"),
            "{md}"
        );
    }

    #[test]
    fn hovering_after_a_label_resets_to_unknown_in_the_hover_too() {
        let md = hover_at_line("    ld a,5\nfoo:\n    ld b,a\n", 2, 9);
        assert!(
            md.contains("Value not statically known at this point"),
            "{md}"
        );
    }

    #[test]
    fn hovering_an_ld_destination_shows_the_value_this_line_gives_it() {
        let text = "    ld bc, 6*256 + 7\n";
        let col_b = text.lines().next().unwrap().find('b').unwrap() as u32;
        let md = hover_at_line(text, 0, col_b);
        assert!(md.contains("Known value at this point"), "{md}");
        assert!(md.contains("0x06"), "{md}");
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
