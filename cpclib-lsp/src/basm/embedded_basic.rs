//! Detection and handling of Locomotive BASIC blocks embedded in assembly
//! sources (`LOCOMOTIVE` ... `ENDLOCOMOTIVE` directives). The actual BASIC
//! analysis is delegated to the `locomotive` module.

use cpclib_basic::located::{LocatedBasicProgram, LocatedTokenKind};

use super::token::*;

// ─── LOCOMOTIVE block detection ───────────────────────────────────────────────

pub(super) struct LocomotiveBlock {
    pub(super) directive_line: usize,
    pub(super) hide_lines_line: Option<usize>,
    pub(super) basic_range: std::ops::Range<usize>,
    pub(super) end_line: usize
}

pub(super) fn extract_locomotive_blocks(text: &str) -> Vec<LocomotiveBlock> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let upper = lines[i].trim().to_uppercase();
        if upper == "LOCOMOTIVE"
            || (upper.starts_with("LOCOMOTIVE")
                && upper
                    .as_bytes()
                    .get(10)
                    .map(|b| b.is_ascii_whitespace())
                    .unwrap_or(false))
        {
            let directive_line = i;
            i += 1;

            // Optional HIDE_LINES directive on the very next line.
            let hide_lines_line = if i < lines.len() {
                let u = lines[i].trim().to_uppercase();
                if u.starts_with("HIDE_LINES") {
                    let hl = i;
                    i += 1;
                    Some(hl)
                }
                else {
                    None
                }
            }
            else {
                None
            };

            let basic_start = i;

            // Scan until ENDLOCOMOTIVE.
            while i < lines.len() {
                let u = lines[i].trim().to_uppercase();
                if u == "ENDLOCOMOTIVE" || u.starts_with("ENDLOCOMOTIVE") {
                    blocks.push(LocomotiveBlock {
                        directive_line,
                        hide_lines_line,
                        basic_range: basic_start..i,
                        end_line: i
                    });
                    break;
                }
                i += 1;
            }
        }
        i += 1;
    }

    blocks
}

/// Emit semantic tokens for a LOCOMOTIVE block's BASIC content.
/// Appends raw `(line, col, len, token_type, modifiers)` tuples into `raw`.
pub(super) fn push_locomotive_basic_tokens(
    block: &LocomotiveBlock,
    lines: &[&str],
    raw: &mut Vec<(u32, u32, u32, u32, u32)>
) {
    // Highlight the LOCOMOTIVE directive line itself (keyword + label).
    {
        let src_line = block.directive_line as u32;
        let line = lines[block.directive_line];
        let bytes = line.as_bytes();
        // Find "LOCOMOTIVE" in the line (case-insensitive).
        if let Some(pos) = line.to_uppercase().find("LOCOMOTIVE") {
            raw.push((src_line, pos as u32, 10, TT_MACRO, 0));
            // Everything after the keyword (trimmed) is the label.
            let after = line[pos + 10..].trim_start();
            if !after.is_empty() {
                let label_col = bytes.len() - after.len();
                let label_len = after.split_whitespace().next().unwrap_or("").len();
                if label_len > 0 {
                    raw.push((src_line, label_col as u32, label_len as u32, TT_FUNCTION, 0));
                }
            }
        }
    }

    // Highlight optional HIDE_LINES line.
    if let Some(hl_line_idx) = block.hide_lines_line {
        let src_line = hl_line_idx as u32;
        let line = lines[hl_line_idx];
        if let Some(pos) = line.to_uppercase().find("HIDE_LINES") {
            raw.push((src_line, pos as u32, 10, TT_MACRO, 0));
            let after = line[pos + 10..].trim_start();
            if !after.is_empty() {
                let num_col = line.len() - after.len();
                let num_len = after.split_whitespace().next().unwrap_or("").len();
                if num_len > 0 {
                    raw.push((src_line, num_col as u32, num_len as u32, TT_NUMBER, 0));
                }
            }
        }
    }

    // Parse the BASIC content lines and emit BASIC tokens.
    let basic_source: String = block
        .basic_range
        .clone()
        .map(|i| lines[i])
        .collect::<Vec<_>>()
        .join("\n");

    if let Ok(prog) = LocatedBasicProgram::parse(&basic_source) {
        for bline in &prog.lines {
            let src_line = block.basic_range.start as u32 + bline.source_line;
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
                    _ => continue
                };
                if tok.span.len > 0 {
                    raw.push((src_line, tok.span.col, tok.span.len, tt, 0));
                }
            }
        }
    }

    // Highlight the ENDLOCOMOTIVE line.
    {
        let src_line = block.end_line as u32;
        let line = lines[block.end_line];
        if let Some(pos) = line.to_uppercase().find("ENDLOCOMOTIVE") {
            raw.push((src_line, pos as u32, 13, TT_MACRO, 0));
        }
    }
}
