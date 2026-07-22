//! Overflow-warning display enrichment: `cpclib-asm` itself now detects and
//! warns about a value that doesn't fit the 8/16-bit slot it's being
//! written into (`Env::checked_byte`/`checked_word` in
//! `cpclib-asm/src/assembler/mod.rs`) - shared with the `basm` CLI, not
//! LSP-only, and already surfaced as a plain `WARNING` diagnostic by
//! `diagnostics.rs`'s `collect_assembler_warnings`.
//!
//! This file only adds display polish on top of that already-detected
//! warning: showing the offending value in the same base the source wrote
//! it in, and what it actually assembles to once truncated. Both need to
//! re-locate the *specific operand expression* the warning was about - the
//! assembler's own warning is located to the whole instruction line (that's
//! all `visit_located_token`'s auto-locate mechanism tracks), not the
//! operand - which is why this still walks the listing structurally,
//! re-resolving each candidate operand until one matches the warning's own
//! parsed value.

use cpclib_asm::assembler::Env;
use cpclib_asm::implementation::expression::ExprEvaluationExt;
use cpclib_asm::parser::obtained::{LocatedDataAccess, LocatedExpr, LocatedListing, LocatedToken};
use cpclib_asm::preamble::{MayHaveSpan, SourceString};
use cpclib_common::parse::{EncodingKind, scan_numeric_literals};
use cpclib_tokens::{ListingElement, Mnemonic};
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use super::diagnostics::asm_diag;

#[derive(Clone, Copy)]
enum SlotWidth {
    Eight,
    Sixteen
}

impl SlotWidth {
    fn bits(self) -> u8 {
        match self {
            SlotWidth::Eight => 8,
            SlotWidth::Sixteen => 16
        }
    }
}

impl AssemblyAnalyzer {
    /// For every overflow-warning diagnostic `collect_assembler_warnings`
    /// already produced, re-locate the specific operand expression it was
    /// about and enrich the message (value shown in the source's own
    /// number base, plus what it actually assembles to once truncated) -
    /// tightening the diagnostic's range to just that operand along the
    /// way. Diagnostics that don't match the expected message shape (e.g.
    /// a real error, or some future warning kind) are left untouched.
    pub(super) fn enrich_overflow_diagnostics(
        listing: &LocatedListing,
        env: &mut Env,
        diagnostics: &mut [Diagnostic]
    ) {
        for diag in diagnostics.iter_mut() {
            if diag.severity != Some(DiagnosticSeverity::WARNING) {
                continue;
            }
            let Some((value, bits)) = parse_overflow_message(&diag.message)
            else {
                continue;
            };
            let Some((token, expr)) =
                find_matching_overflow_operand(listing, env, diag.range.start.line, value, bits)
            else {
                continue;
            };

            let shown_value = format_value_like_source(expr.span().as_str(), value);
            let mut message = format!("value {shown_value} does not fit in {bits} bits");
            if let Some(snippet) = synthesize_replacement(token, expr, value)
                && let Some(disassembled) = super::disassemble::disassemble_snippet(&snippet)
            {
                message.push_str(&format!(" (assembles as: {disassembled})"));
            }
            *diag = asm_diag(Some(expr.span()), message, DiagnosticSeverity::WARNING);
        }
    }
}

/// `cpclib-asm`'s fixed overflow-warning message format
/// (`Env::checked_byte`/`checked_word`, `cpclib-asm/src/assembler/mod.rs`)
/// starts life as the plain `"value {v} does not fit in {bits} bits"` - but
/// by the time it reaches `env.warnings()`, `Env::render_warnings()` has
/// already flattened it into a fully rendered, `"warning: "`-prefixed,
/// codespan-annotated block (the same rendering the `basm` CLI prints to
/// stderr) - so this searches for the fixed phrase within that larger text
/// rather than expecting it to start the string. Keep in sync with the
/// message format in `cpclib-asm` if that ever changes.
fn parse_overflow_message(msg: &str) -> Option<(i32, u8)> {
    let idx = msg.find("value ")?;
    let rest = &msg[idx + "value ".len()..];
    let (value_str, rest) = rest.split_once(" does not fit in ")?;
    let bits_str = rest.split_whitespace().next()?;
    Some((
        value_str.trim().parse().ok()?,
        bits_str.trim().parse().ok()?
    ))
}

/// Find the token/operand-expression pair on `target_line` whose resolved
/// value and slot width match the assembler's own warning - i.e. the exact
/// operand `checked_byte`/`checked_word` warned about.
fn find_matching_overflow_operand<'a>(
    listing: &'a LocatedListing,
    env: &mut Env,
    target_line: u32,
    value: i32,
    bits: u8
) -> Option<(&'a LocatedToken, &'a LocatedExpr)> {
    for token in super::token::flatten_listing(listing.iter()) {
        if super::token::span_line(token) != target_line {
            continue;
        }
        for (expr, width) in overflow_candidates(token) {
            if width.bits() != bits {
                continue;
            }
            if expr.resolve(env).ok().and_then(|v| v.int().ok()) == Some(value) {
                return Some((token, expr));
            }
        }
    }
    None
}

/// Every `(value expression, slot width)` pair worth considering in a
/// single token, if any - mirrors exactly which forms
/// `Env::checked_byte`/`checked_word` are wired into in `cpclib-asm`.
fn overflow_candidates(token: &LocatedToken) -> Vec<(&LocatedExpr, SlotWidth)> {
    let mut candidates = Vec::new();

    // A "fake instruction" token (e.g. `ld hl, de`) is wrapped for the
    // warning pipeline (`LocatedTokenInner::WarningWrapper`); most
    // `ListingElement` accessors - including `mnemonic()`/`mnemonic_arg1()`/
    // `mnemonic_arg2()`/`data_exprs()` - unconditionally unwrap the *un*-
    // wrapped inner token and panic on one of these, rather than returning
    // `None`. `is_warning()`/`is_db()`/`is_dw()` are the safe ones (they
    // fall back to `false`), so check this first.
    if token.is_warning() {
        return candidates;
    }

    if let Some(mnemonic) = token.mnemonic() {
        match mnemonic {
            Mnemonic::Ld => {
                if let (Some(dst), Some(LocatedDataAccess::Expression(expr))) =
                    (token.mnemonic_arg1(), token.mnemonic_arg2())
                    && let Some(width) = ld_destination_width(dst)
                {
                    candidates.push((expr, width));
                }
            },
            // Accumulator-implicit 8-bit-immediate forms: `ADD A,n`,
            // `ADC A,n`, `SBC A,n` (immediate is arg2), and the
            // single-operand `SUB n`/`AND n`/`OR n`/`XOR n`/`CP n`
            // (immediate is arg1). Whichever operand is a plain
            // `Expression` (as opposed to a register) is the one to check;
            // register-register forms like `ADD HL,DE` never have one, so
            // they're naturally left alone.
            Mnemonic::Add
            | Mnemonic::Adc
            | Mnemonic::Sub
            | Mnemonic::Sbc
            | Mnemonic::And
            | Mnemonic::Or
            | Mnemonic::Xor
            | Mnemonic::Cp => {
                for arg in [token.mnemonic_arg1(), token.mnemonic_arg2()]
                    .into_iter()
                    .flatten()
                {
                    if let LocatedDataAccess::Expression(expr) = arg {
                        candidates.push((expr, SlotWidth::Eight));
                    }
                }
            },
            _ => {}
        }
    }

    if token.is_db() {
        candidates.extend(token.data_exprs().iter().map(|e| (e, SlotWidth::Eight)));
    }
    if token.is_dw() {
        candidates.extend(token.data_exprs().iter().map(|e| (e, SlotWidth::Sixteen)));
    }

    candidates
}

/// Build a small, self-contained instruction snippet equivalent to `token`
/// but with `target` (the operand that overflowed) replaced by its
/// resolved `value` written out as a plain literal - every other operand
/// (a register, `(HL)`, `(IX+2)`, ...) is kept verbatim, using its own
/// original source text, so the result never depends on any symbol table:
/// it can always be assembled in isolation, even when `target` itself came
/// from a variable (`ld b, val`) that only exists in the surrounding file.
fn synthesize_replacement(
    token: &LocatedToken,
    target: &LocatedExpr,
    value: i32
) -> Option<String> {
    if token.is_db() {
        return Some(format!("db {value}"));
    }
    if token.is_dw() {
        return Some(format!("dw {value}"));
    }

    let mnemonic = token.mnemonic()?;
    let mut operands = Vec::new();
    for arg in [token.mnemonic_arg1(), token.mnemonic_arg2()]
        .into_iter()
        .flatten()
    {
        if let LocatedDataAccess::Expression(e) = arg
            && std::ptr::eq(e, target)
        {
            operands.push(value.to_string());
        }
        else {
            operands.push(arg.span().as_str().to_string());
        }
    }
    if operands.is_empty() {
        None
    }
    else {
        Some(format!("{mnemonic} {}", operands.join(", ")))
    }
}

/// The slot width a `LD` destination operand represents, or `None` for
/// anything that isn't a plain register/memory-through-register slot (e.g.
/// `(nn)`, flags, `I`/`R` are all left alone - `PortN`/etc. don't arise as
/// `LD` destinations at all).
fn ld_destination_width(dst: &LocatedDataAccess) -> Option<SlotWidth> {
    match dst {
        LocatedDataAccess::Register8(..)
        | LocatedDataAccess::IndexRegister8(..)
        | LocatedDataAccess::MemoryRegister16(..)
        | LocatedDataAccess::MemoryIndexRegister16(..)
        | LocatedDataAccess::IndexRegister16WithIndex(..) => Some(SlotWidth::Eight),
        LocatedDataAccess::Register16(..) | LocatedDataAccess::IndexRegister16(..) => {
            Some(SlotWidth::Sixteen)
        },
        _ => None
    }
}

/// Format `value` the way the original source expression `source_text`
/// wrote it - matching its base (decimal/hex/octal/binary) - so a warning
/// about e.g. `ld b, 0x12C` shows the offending value in hex too, rather
/// than a decimal number the user has to convert back themselves. Falls
/// back to hexadecimal (matching the disassembler's own convention, e.g.
/// `0x2c`) whenever `source_text` isn't a single bare literal: a symbol
/// reference or a computed expression has no one "original base" of its
/// own to preserve.
pub(super) fn format_value_like_source(source_text: &str, value: i32) -> String {
    let trimmed = source_text.trim();
    let literals = scan_numeric_literals(trimmed);
    let kind = match literals.as_slice() {
        [(start, end, _, kind)] if *start == 0 && *end == trimmed.len() => *kind,
        _ => EncodingKind::Hex
    };
    match kind {
        EncodingKind::Dec => value.to_string(),
        EncodingKind::Oct => format_radix(value, 8, "0o"),
        EncodingKind::Bin => format_radix(value, 2, "0b"),
        // `Hex`, plus the internal `AmbiguousBinHex`/`Unk` states that
        // `scan_numeric_literals` never actually returns - hex is the
        // reasonable default either way.
        _ => format_radix(value, 16, "0x")
    }
}

/// Render `value`'s magnitude in the given `radix` with `prefix`, keeping
/// a plain leading `-` for negative values rather than a two's-complement
/// bit pattern (which would be confusing at an arbitrary, non-8/16-bit
/// width like `i32`).
fn format_radix(value: i32, radix: u32, prefix: &str) -> String {
    let magnitude = value.unsigned_abs();
    let digits = match radix {
        16 => format!("{magnitude:x}"),
        8 => format!("{magnitude:o}"),
        2 => format!("{magnitude:b}"),
        _ => magnitude.to_string()
    };
    if value < 0 {
        format!("-{prefix}{digits}")
    }
    else {
        format!("{prefix}{digits}")
    }
}
