//! Overflow detection: flag a value that doesn't fit the 8-bit or 16-bit
//! slot it's being written into - e.g. `ld b, 300` (300 doesn't fit in a
//! byte). Per the original feature request, this must work for immediate
//! literals *and* for values reached through a variable (`val equ 300` /
//! `ld b, val`) - both are handled uniformly here by resolving the
//! expression against a fully-assembled `Env` (e.g. via `dry_run_env`),
//! reusing the exact `expr.resolve(env)` pattern `hover.rs` already uses for
//! `INCBIN`/`FUNCTION`-call value display.

use cpclib_asm::assembler::Env;
use cpclib_asm::implementation::expression::ExprEvaluationExt;
use cpclib_asm::parser::obtained::{LocatedDataAccess, LocatedExpr, LocatedListing, LocatedToken};
use cpclib_asm::preamble::{MayHaveSpan, SourceString};
use cpclib_tokens::{ListingElement, Mnemonic};
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use super::diagnostics::asm_diag;

/// Accepted range for a value written into an 8-bit slot: permissive enough
/// to accept both the signed (`-128..=127`) and unsigned (`0..=255`)
/// conventional ways of writing a byte.
const EIGHT_BIT_RANGE: std::ops::RangeInclusive<i32> = -128..=255;
/// Same idea, for a 16-bit slot.
const SIXTEEN_BIT_RANGE: std::ops::RangeInclusive<i32> = -32768..=65535;

#[derive(Clone, Copy)]
enum SlotWidth {
    Eight,
    Sixteen
}

impl SlotWidth {
    fn range(self) -> std::ops::RangeInclusive<i32> {
        match self {
            SlotWidth::Eight => EIGHT_BIT_RANGE,
            SlotWidth::Sixteen => SIXTEEN_BIT_RANGE
        }
    }

    fn bits(self) -> u8 {
        match self {
            SlotWidth::Eight => 8,
            SlotWidth::Sixteen => 16
        }
    }
}

impl AssemblyAnalyzer {
    /// Walk `listing` looking for a value written into a slot it doesn't fit
    /// in. `env` must already be fully assembled (e.g. via `dry_run_env`) so
    /// `EQU`/`=`-assigned variables resolve to their real value, not just
    /// literal immediates - the same evaluation covers both cases uniformly.
    pub(super) fn check_overflow_diagnostics(
        listing: &LocatedListing,
        env: &mut Env,
        out: &mut Vec<Diagnostic>
    ) {
        for token in super::token::flatten_listing(listing.iter()) {
            for (expr, width) in overflow_candidates(token) {
                let Some(value) = expr.resolve(env).ok().and_then(|v| v.int().ok())
                else {
                    // Forward reference, macro parameter not yet bound, or
                    // anything else that doesn't resolve to a plain integer
                    // right now - not something to diagnose here.
                    continue;
                };
                if !width.range().contains(&value) {
                    let mut message =
                        format!("value {value} does not fit in {} bits", width.bits());
                    // Show what the value actually becomes once truncated,
                    // by re-assembling a version of this instruction with
                    // the resolved value substituted in, then disassembling
                    // the result - rather than hardcoding per-mnemonic
                    // truncation logic, this stays correct for any
                    // instruction shape (including ones added later).
                    if let Some(snippet) = synthesize_replacement(token, expr, value)
                        && let Some(disassembled) =
                            super::disassemble::disassemble_snippet(&snippet)
                    {
                        message.push_str(&format!(" (assembles as: {disassembled})"));
                    }
                    out.push(asm_diag(
                        Some(expr.span()),
                        message,
                        DiagnosticSeverity::WARNING
                    ));
                }
            }
        }
    }
}

/// Every `(value expression, slot width)` pair worth range-checking in a
/// single token, if any.
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
/// `(nn)`, flags, `I`/`R` are all left alone - existing/`PortN`/etc. don't
/// arise as `LD` destinations at all).
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
