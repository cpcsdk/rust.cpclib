//! Completion for assembly files.
//!
//! The first half of this file is the semantic completion model: operand-type
//! bitflags, the per-instruction operand table, and cursor-context analysis.
//! The second half is the analyzer entry point that renders the candidates
//! into LSP `CompletionItem`s.

use cpclib_tokens::ListingElement;
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use super::token::{DIRECTIVE_FILE_ARGS, is_ident_byte};
use crate::common::document::Document;

/// Semantic completion context for Z80 assembly.
///
/// Determines whether the cursor is in "mnemonic/directive" position
/// (first token on the statement) or in an "operand" position (after
/// the mnemonic, for instruction argument N).

// ─── Operand type flags ───────────────────────────────────────────────────────

pub const T_R8: u32 = 1 << 0; // A B C D E H L
pub const T_R16: u32 = 1 << 1; // BC DE HL SP
pub const T_R16_QQ: u32 = 1 << 2; // BC DE HL AF (PUSH/POP)
pub const T_IX: u32 = 1 << 3; // IX
pub const T_IY: u32 = 1 << 4; // IY
pub const T_COND4: u32 = 1 << 5; // NZ Z NC C  (for JR)
pub const T_COND8: u32 = 1 << 6; // NZ Z NC C PE PO P M  (for JP/CALL/RET)
pub const T_EXPR: u32 = 1 << 7; // any expression / label / numeric literal
pub const T_MEM_HL: u32 = 1 << 8; // (HL)
pub const T_MEM_IX: u32 = 1 << 9; // (IX+n) or (IY+n)
pub const T_MEM_BC: u32 = 1 << 10; // (BC)
pub const T_MEM_DE: u32 = 1 << 11; // (DE)
pub const T_MEM_NN: u32 = 1 << 12; // (nn) — direct address
pub const T_A: u32 = 1 << 13; // A specifically
pub const T_HL: u32 = 1 << 14; // HL specifically
pub const T_I: u32 = 1 << 15; // I register
pub const T_R_REG: u32 = 1 << 16; // R register (memory refresh)
pub const T_AF_ALT: u32 = 1 << 17; // AF'
pub const T_SP: u32 = 1 << 18; // SP specifically
pub const T_DE: u32 = 1 << 19; // DE specifically
pub const T_BIT: u32 = 1 << 20; // literal bit number 0-7 (BIT/SET/RES)
pub const T_PORT_N: u32 = 1 << 21; // (n) — port address for IN/OUT
pub const T_PORT_C: u32 = 1 << 22; // (C) — port via BC

// Shorthand composites used in the table
const R8_MEM: u32 = T_R8 | T_MEM_HL | T_MEM_IX; // r or (HL) or (IX+d)
const R8_MEM_IMM: u32 = T_R8 | T_MEM_HL | T_MEM_IX | T_EXPR;

// ─── Instruction operand table ────────────────────────────────────────────────
//
// Each entry is (MNEMONIC_UPPER, &[slot0_mask, slot1_mask, …]).
// An empty slice means the instruction takes no operands.
// A mask of 0 means "not expected here" — completions fall back to defaults.

static INSTR_OPERANDS: &[(&str, &[u32])] = &[
    // No-operand instructions
    ("CCF", &[]),
    ("CPD", &[]),
    ("CPDR", &[]),
    ("CPI", &[]),
    ("CPIR", &[]),
    ("CPL", &[]),
    ("DAA", &[]),
    ("DI", &[]),
    ("EI", &[]),
    ("EXA", &[]), // EX AF,AF' alias
    ("EXX", &[]),
    ("HALT", &[]),
    ("IND", &[]),
    ("INDR", &[]),
    ("INI", &[]),
    ("INIR", &[]),
    ("LDD", &[]),
    ("LDDR", &[]),
    ("LDI", &[]),
    ("LDIR", &[]),
    ("NEG", &[]),
    ("NOP", &[]),
    ("OTDR", &[]),
    ("OTIR", &[]),
    ("OUTD", &[]),
    ("OUTI", &[]),
    ("RETI", &[]),
    ("RETN", &[]),
    ("RLA", &[]),
    ("RLCA", &[]),
    ("RLD", &[]),
    ("RRA", &[]),
    ("RRCA", &[]),
    ("RRD", &[]),
    ("SCF", &[]),
    // Single-operand 8-bit ALU (operate on A implicitly)
    ("SUB", &[R8_MEM_IMM]),
    ("AND", &[R8_MEM_IMM]),
    ("OR", &[R8_MEM_IMM]),
    ("XOR", &[R8_MEM_IMM]),
    ("CP", &[R8_MEM_IMM]),
    // ADC / SBC — two forms: ADC A,r or ADC HL,rr
    ("ADC", &[T_R8 | T_HL, R8_MEM_IMM | T_R16]),
    ("SBC", &[T_R8 | T_HL, R8_MEM_IMM | T_R16]),
    // ADD — ADD A,r  or  ADD HL,rr  or  ADD IX,rr
    (
        "ADD",
        &[T_A | T_HL | T_IX | T_IY, R8_MEM_IMM | T_R16 | T_IX | T_IY]
    ),
    // INC / DEC — 8-bit or 16-bit or (HL) or (IX+n)
    ("INC", &[T_R8 | T_MEM_HL | T_MEM_IX | T_R16 | T_IX | T_IY]),
    ("DEC", &[T_R8 | T_MEM_HL | T_MEM_IX | T_R16 | T_IX | T_IY]),
    // RL / RLC / RR / RRC / SLA / SRA / SRL / SLL etc.
    ("RL", &[R8_MEM]),
    ("RLC", &[R8_MEM]),
    ("RR", &[R8_MEM]),
    ("RRC", &[R8_MEM]),
    ("SLA", &[R8_MEM]),
    ("SLL", &[R8_MEM]),
    ("SRA", &[R8_MEM]),
    ("SRL", &[R8_MEM]),
    // BIT / SET / RES — b, r
    ("BIT", &[T_BIT, R8_MEM]),
    ("SET", &[T_BIT, R8_MEM]),
    ("RES", &[T_BIT, R8_MEM]),
    // PUSH / POP
    ("PUSH", &[T_R16_QQ | T_IX | T_IY]),
    ("POP", &[T_R16_QQ | T_IX | T_IY]),
    // LD — most complex; first slot accepts almost anything
    (
        "LD",
        &[
            T_R8 | T_R16
                | T_IX
                | T_IY
                | T_MEM_HL
                | T_MEM_IX
                | T_MEM_BC
                | T_MEM_DE
                | T_MEM_NN
                | T_I
                | T_R_REG
                | T_SP,
            T_R8 | T_R16 | T_IX | T_IY | T_MEM_HL | T_MEM_IX | T_EXPR | T_I | T_R_REG
        ]
    ),
    // EX — EX DE,HL  /  EX AF,AF'  /  EX (SP),HL or IX or IY
    (
        "EX",
        &[T_DE | T_AF_ALT | T_MEM_HL, T_HL | T_AF_ALT | T_IX | T_IY]
    ),
    // JP — JP nn  /  JP cc,nn  /  JP HL  /  JP IX
    ("JP", &[T_COND8 | T_EXPR | T_HL | T_IX | T_IY, T_EXPR]),
    // JR — JR n  /  JR cc,n
    ("JR", &[T_COND4 | T_EXPR, T_EXPR]),
    // CALL
    ("CALL", &[T_COND8 | T_EXPR, T_EXPR]),
    // RET — RET  /  RET cc
    ("RET", &[T_COND8]),
    // DJNZ
    ("DJNZ", &[T_EXPR]),
    // RST — target address (fixed values, treat as expression)
    ("RST", &[T_EXPR]),
    // IM — 0, 1, or 2
    ("IM", &[T_EXPR]),
    // IN — IN A,(n)  /  IN r,(C)
    ("IN", &[T_R8, T_PORT_N | T_PORT_C]),
    // OUT — OUT (n),A  /  OUT (C),r
    ("OUT", &[T_PORT_N | T_PORT_C, T_R8 | T_EXPR])
];

// ─── Context analysis ─────────────────────────────────────────────────────────

/// Describes what the cursor is completing.
#[derive(Debug, Clone)]
pub enum CompletionContext {
    /// Cursor is still on the mnemonic/directive (or start of statement).
    MnemonicPosition,
    /// Cursor is in an operand of a Z80 instruction.
    InstructionOperand {
        mnemonic: String,
        /// 0 = first operand, 1 = second operand, etc.
        arg_index: usize
    },
    /// Cursor is in an argument of an assembler directive.
    DirectiveArgument
}

/// Analyse the current line up to `col` and return the completion context.
pub fn analyze_context(line: &str, col: usize) -> CompletionContext {
    let col = col.min(line.len());
    let before = &line[..col];

    let trimmed = before.trim_start();
    // Check for a label definition at start of line (word + ':' + optional whitespace)
    let stmt = skip_label_definition(trimmed);

    // Extract the first word (mnemonic or directive)
    let (mnemonic, rest) = split_first_word(stmt);
    if mnemonic.is_empty() {
        return CompletionContext::MnemonicPosition;
    }

    // If there is no whitespace immediately after the mnemonic, still completing it
    let has_space_after =
        stmt.len() > mnemonic.len() && stmt.as_bytes()[mnemonic.len()].is_ascii_whitespace();
    if !has_space_after {
        return CompletionContext::MnemonicPosition;
    }

    let mnemonic_upper = mnemonic.to_uppercase();

    // Check whether this is an instruction or a directive
    let is_instruction = INSTR_OPERANDS
        .iter()
        .any(|(m, _)| *m == mnemonic_upper.as_str())
        || cpclib_asm::lsp::Z80_INSTRUCTIONS
            .iter()
            .any(|m| m.to_uppercase() == mnemonic_upper);

    if !is_instruction {
        return CompletionContext::DirectiveArgument;
    }

    // Count commas outside parentheses/brackets to find arg_index
    let arg_index = count_arg_index(rest);

    CompletionContext::InstructionOperand {
        mnemonic: mnemonic_upper,
        arg_index
    }
}

/// Return the operand type mask for `(mnemonic, arg_index)`.
///
/// * `None` — instruction not in the table (caller should fall back to a generic default).
/// * `Some(0)` — instruction is known but expects no operand at this position.
/// * `Some(mask)` — valid slot; `mask` describes what can appear here.
pub fn operand_mask(mnemonic: &str, arg_index: usize) -> Option<u32> {
    let (_, slots) = INSTR_OPERANDS.iter().find(|(m, _)| *m == mnemonic)?;
    // Known instruction: use declared mask, or 0 if beyond the declared operand count.
    Some(slots.get(arg_index).copied().unwrap_or(0))
}

// ─── Concrete completions from a mask ────────────────────────────────────────

/// 8-bit register names
pub const REGS_8: &[&str] = &["A", "B", "C", "D", "E", "H", "L"];
/// 16-bit register pair names
pub const REGS_16: &[&str] = &["BC", "DE", "HL", "SP"];
/// PUSH/POP register set
pub const REGS_QQ: &[&str] = &["BC", "DE", "HL", "AF"];
/// Conditions for JR
pub const COND4: &[&str] = &["NZ", "Z", "NC", "C"];
/// Full condition set for JP/CALL/RET
pub const COND8: &[&str] = &["NZ", "Z", "NC", "C", "PE", "PO", "P", "M"];
/// Bit numbers
pub const BIT_NUMS: &[&str] = &["0", "1", "2", "3", "4", "5", "6", "7"];

/// Returns the tokens (register names, conditions, synthetic literals) that
/// a given operand type mask implies.  Expressions/labels are not included here
/// because they come from the document — use `mask_accepts_expression` for that.
pub fn tokens_for_mask(mask: u32) -> Vec<(&'static str, &'static str)> {
    // (token, detail)
    let mut out: Vec<(&'static str, &'static str)> = Vec::new();
    if mask & T_R8 != 0 {
        for &r in REGS_8 {
            out.push((r, "8-bit register"));
        }
    }
    if mask & T_R16 != 0 {
        for &r in REGS_16 {
            if !out.iter().any(|(t, _)| *t == r) {
                out.push((r, "16-bit register"));
            }
        }
    }
    if mask & T_R16_QQ != 0 {
        for &r in REGS_QQ {
            if !out.iter().any(|(t, _)| *t == r) {
                out.push((r, "register (push/pop)"));
            }
        }
    }
    if mask & T_IX != 0 {
        out.push(("IX", "index register"));
    }
    if mask & T_IY != 0 {
        out.push(("IY", "index register"));
    }
    if mask & T_I != 0 {
        out.push(("I", "interrupt vector register"));
    }
    if mask & T_R_REG != 0 {
        out.push(("R", "memory refresh register"));
    }
    if mask & T_AF_ALT != 0 {
        out.push(("AF'", "alternate AF register"));
    }
    if mask & T_A != 0 && mask & T_R8 == 0 {
        // Only add A if not already included via T_R8
        out.push(("A", "accumulator"));
    }
    if mask & T_HL != 0 && mask & T_R16 == 0 {
        out.push(("HL", "16-bit register"));
    }
    if mask & T_SP != 0 && mask & T_R16 == 0 {
        out.push(("SP", "stack pointer"));
    }
    if mask & T_DE != 0 && mask & T_R16 == 0 {
        out.push(("DE", "16-bit register"));
    }
    if mask & T_COND4 != 0 {
        for &c in COND4 {
            if !out.iter().any(|(t, _)| *t == c) {
                out.push((c, "condition code"));
            }
        }
    }
    if mask & T_COND8 != 0 {
        for &c in COND8 {
            if !out.iter().any(|(t, _)| *t == c) {
                out.push((c, "condition code"));
            }
        }
    }
    if mask & T_MEM_HL != 0 {
        out.push(("(HL)", "memory via HL"));
    }
    if mask & T_MEM_IX != 0 {
        out.push(("(IX+n)", "indexed memory (IX)"));
        out.push(("(IY+n)", "indexed memory (IY)"));
    }
    if mask & T_MEM_BC != 0 {
        out.push(("(BC)", "memory via BC"));
    }
    if mask & T_MEM_DE != 0 {
        out.push(("(DE)", "memory via DE"));
    }
    if mask & T_MEM_NN != 0 {
        out.push(("(nn)", "direct address"));
    }
    if mask & T_PORT_N != 0 {
        out.push(("(n)", "port address"));
    }
    if mask & T_PORT_C != 0 {
        out.push(("(C)", "port via BC"));
    }
    if mask & T_BIT != 0 {
        for &b in BIT_NUMS {
            out.push((b, "bit number"));
        }
    }
    out
}

/// Returns `true` when this mask allows any expression (label, number, formula).
pub fn mask_accepts_expression(mask: u32) -> bool {
    mask & T_EXPR != 0
}

// ─── Private helpers ──────────────────────────────────────────────────────────

/// Skip a label definition that may appear at the start of a statement.
/// A label definition is a word immediately followed by `:` (optional space after).
fn skip_label_definition(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len()
        && (bytes[i].is_ascii_alphanumeric()
            || bytes[i] == b'_'
            || bytes[i] == b'@'
            || bytes[i] == b'.')
    {
        i += 1;
    }
    if i == 0 || i >= bytes.len() {
        return s;
    }
    if bytes[i] == b':' {
        // Skip the colon and following whitespace
        let rest = &s[i + 1..];
        rest.trim_start()
    }
    else {
        s
    }
}

/// Split `s` at the first run of whitespace.
/// Returns `(first_word, rest_after_whitespace)`.
fn split_first_word(s: &str) -> (&str, &str) {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let word = &s[..i];
    // skip whitespace
    let rest = s[i..].trim_start();
    (word, rest)
}

/// Count how many top-level commas appear in `s` (outside parentheses).
/// The result is the 0-based argument index of the token at the end of `s`.
fn count_arg_index(s: &str) -> usize {
    let mut depth: i32 = 0;
    let mut commas = 0usize;
    for b in s.bytes() {
        match b {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b',' if depth <= 0 => commas += 1,
            _ => {}
        }
    }
    commas
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(line: &str, col: usize) -> CompletionContext {
        analyze_context(line, col)
    }

    #[test]
    fn empty_line_is_mnemonic_position() {
        assert!(matches!(ctx("", 0), CompletionContext::MnemonicPosition));
        assert!(matches!(ctx("   ", 2), CompletionContext::MnemonicPosition));
    }

    #[test]
    fn mid_mnemonic_is_mnemonic_position() {
        assert!(matches!(
            ctx("  LD", 4),
            CompletionContext::MnemonicPosition
        ));
        assert!(matches!(ctx("JP", 2), CompletionContext::MnemonicPosition));
    }

    #[test]
    fn after_mnemonic_space_is_operand0() {
        let c = ctx("  LD ", 5);
        assert!(matches!(
            c,
            CompletionContext::InstructionOperand { arg_index: 0, .. }
        ));
    }

    #[test]
    fn after_first_comma_is_operand1() {
        let c = ctx("  LD A,", 7);
        assert!(matches!(
            c,
            CompletionContext::InstructionOperand { arg_index: 1, .. }
        ));
    }

    #[test]
    fn comma_inside_parens_not_counted() {
        // BIT 3, — the "3" is not preceded by a nested comma
        let c = ctx("BIT 3,", 6);
        assert!(matches!(
            c,
            CompletionContext::InstructionOperand { arg_index: 1, .. }
        ));
    }

    #[test]
    fn label_before_mnemonic_is_skipped() {
        let c = ctx("loop: LD A,", 11);
        assert!(matches!(
            c,
            CompletionContext::InstructionOperand { arg_index: 1, .. }
        ));
    }

    #[test]
    fn directive_gives_directive_argument_context() {
        assert!(matches!(
            ctx("ORG ", 4),
            CompletionContext::DirectiveArgument
        ));
        assert!(matches!(
            ctx("DEFB ", 5),
            CompletionContext::DirectiveArgument
        ));
    }

    #[test]
    fn ld_operand0_mask_has_r8_and_r16() {
        let m = operand_mask("LD", 0).unwrap();
        assert!(m & T_R8 != 0);
        assert!(m & T_R16 != 0);
    }

    #[test]
    fn push_operand0_has_qq_not_sp() {
        let m = operand_mask("PUSH", 0).unwrap();
        assert!(m & T_R16_QQ != 0);
        // SP is not in the qq set for PUSH
        assert!(m & T_SP == 0);
    }

    #[test]
    fn jp_operand0_has_full_conditions() {
        let m = operand_mask("JP", 0).unwrap();
        assert!(m & T_COND8 != 0);
        // JP also accepts a direct address (expression)
        assert!(m & T_EXPR != 0);
    }

    #[test]
    fn bit_operand0_has_bit_numbers() {
        let m = operand_mask("BIT", 0).unwrap();
        assert!(m & T_BIT != 0);
        assert!(m & T_R8 == 0);
    }

    #[test]
    fn bit_operand1_has_r8_and_mem() {
        let m = operand_mask("BIT", 1).unwrap();
        assert!(m & T_R8 != 0);
        assert!(m & T_MEM_HL != 0);
    }

    #[test]
    fn no_operand_instructions_have_no_mask() {
        assert_eq!(operand_mask("NOP", 0), Some(0));
        assert_eq!(operand_mask("EI", 0), Some(0));
    }
}

// ─── Generated tables ─────────────────────────────────────────────────────────

// `INSTR_FORMS`: valid instruction forms from data/timings.txt.
include!(concat!(env!("OUT_DIR"), "/instr_forms_generated.rs"));

// `SNIPPETS`: completion snippets shared with the Zed extension.
include!(concat!(env!("OUT_DIR"), "/snippets_generated.rs"));

// ─── Case handling ────────────────────────────────────────────────────────────

/// Case preference derived from what the user already typed.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum CasePref {
    /// Prefix starts with a lowercase letter → complete in lowercase.
    Lower,
    /// Prefix starts with an uppercase letter, or nothing typed yet.
    Upper
}

/// Look at the identifier characters just before `col` and decide the case.
pub(super) fn case_pref_at(line: &str, col: usize) -> CasePref {
    let bytes = line.as_bytes();
    let col = col.min(bytes.len());
    let mut start = col;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }
    match bytes[start..col].iter().find(|b| b.is_ascii_alphabetic()) {
        Some(b) if b.is_ascii_lowercase() => CasePref::Lower,
        _ => CasePref::Upper
    }
}

fn apply_case(s: &str, pref: CasePref) -> String {
    match pref {
        CasePref::Lower => s.to_lowercase(),
        CasePref::Upper => s.to_uppercase()
    }
}

// ─── Instruction-form matching (data-driven operand filtering) ───────────────

/// Does the already-typed operand `typed` (trimmed) match the form pattern
/// class `pattern` (both lowercase)?
fn operand_matches(pattern: &str, typed: &str) -> bool {
    let typed = typed.trim();
    if typed.is_empty() {
        return false;
    }
    let squeezed: String = typed.chars().filter(|c| !c.is_whitespace()).collect();
    match pattern {
        "r" | "r'" => matches!(squeezed.as_str(), "a" | "b" | "c" | "d" | "e" | "h" | "l"),
        "rr" => matches!(squeezed.as_str(), "bc" | "de" | "hl" | "sp"),
        "qq" => matches!(squeezed.as_str(), "bc" | "de" | "hl" | "af"),
        "cc" => matches!(squeezed.as_str(), "nz" | "z" | "nc" | "c"),
        "ccc" => {
            matches!(
                squeezed.as_str(),
                "nz" | "z" | "nc" | "c" | "po" | "pe" | "p" | "m"
            )
        },
        "(ix+n)" => squeezed.starts_with("(ix") || squeezed.starts_with("(iy"),
        "(nn)" => {
            squeezed.starts_with('(')
                && !matches!(squeezed.as_str(), "(hl)" | "(bc)" | "(de)" | "(c)" | "(sp)")
                && !squeezed.starts_with("(ix")
                && !squeezed.starts_with("(iy")
        },
        "(n)" => squeezed.starts_with('(') && squeezed.ends_with(')'),
        "ix" => squeezed == "ix" || squeezed == "iy",
        "n" | "nn" | "ttt" => {
            // An expression: anything that is not a register/condition/indirection.
            !squeezed.starts_with('(')
                && !matches!(
                    squeezed.as_str(),
                    "a" | "b"
                        | "c"
                        | "d"
                        | "e"
                        | "h"
                        | "l"
                        | "i"
                        | "r"
                        | "bc"
                        | "de"
                        | "hl"
                        | "sp"
                        | "af"
                        | "af'"
                        | "ix"
                        | "iy"
                        | "nz"
                        | "z"
                        | "nc"
                        | "po"
                        | "pe"
                        | "p"
                        | "m"
                )
        },
        lit => squeezed == lit
    }
}

/// Suggestions implied by one form-pattern class for the slot being completed.
/// Returns `(concrete tokens, accepts_expression)`.
fn pattern_candidates(pattern: &str) -> (Vec<(&'static str, &'static str)>, bool) {
    match pattern {
        "r" | "r'" => {
            (
                REGS_8.iter().map(|r| (*r, "8-bit register")).collect(),
                false
            )
        },
        "rr" => {
            (
                REGS_16.iter().map(|r| (*r, "16-bit register")).collect(),
                false
            )
        },
        "qq" => {
            (
                REGS_QQ
                    .iter()
                    .map(|r| (*r, "register (push/pop)"))
                    .collect(),
                false
            )
        },
        "cc" => {
            (
                COND4.iter().map(|c| (*c, "condition code")).collect(),
                false
            )
        },
        "ccc" => {
            (
                COND8.iter().map(|c| (*c, "condition code")).collect(),
                false
            )
        },
        "ttt" => {
            (
                vec![
                    ("&00", "RST vector"),
                    ("&08", "RST vector"),
                    ("&10", "RST vector"),
                    ("&18", "RST vector"),
                    ("&20", "RST vector"),
                    ("&28", "RST vector"),
                    ("&30", "RST vector"),
                    ("&38", "RST vector"),
                ],
                false
            )
        },
        "n" | "nn" => (Vec::new(), true),
        "(hl)" => (vec![("(HL)", "memory via HL")], false),
        "(bc)" => (vec![("(BC)", "memory via BC")], false),
        "(de)" => (vec![("(DE)", "memory via DE")], false),
        "(sp)" => (vec![("(SP)", "memory via SP")], false),
        "(c)" => (vec![("(C)", "port via BC")], false),
        "(n)" => (vec![("(n)", "port address")], true),
        "(nn)" => (vec![("(nn)", "direct address")], true),
        "(ix+n)" => {
            (
                vec![
                    ("(IX+n)", "indexed memory (IX)"),
                    ("(IY+n)", "indexed memory (IY)"),
                ],
                false
            )
        },
        "ix" => {
            (
                vec![("IX", "index register"), ("IY", "index register")],
                false
            )
        },
        "a" => (vec![("A", "accumulator")], false),
        "hl" => (vec![("HL", "16-bit register")], false),
        "de" => (vec![("DE", "16-bit register")], false),
        "sp" => (vec![("SP", "stack pointer")], false),
        "af" => (vec![("AF", "16-bit register")], false),
        "af'" => (vec![("AF'", "alternate AF register")], false),
        "i" => (vec![("I", "interrupt vector register")], false),
        "b" => (BIT_NUMS.iter().map(|b| (*b, "bit number")).collect(), false),
        "0" => (vec![("0", "interrupt mode")], false),
        "1" => (vec![("1", "interrupt mode")], false),
        "2" => (vec![("2", "interrupt mode")], false),
        _ => (Vec::new(), false)
    }
}

/// Special case: `r` in `LD A,R` / `LD R,A` is the memory-refresh register.
fn form_is_refresh_ld(operands: &[&str]) -> bool {
    operands == ["a", "r"] || operands == ["r", "a"]
}

/// Candidates for `slot` of `mnemonic`, restricted to the instruction forms
/// whose earlier operands match what has already been typed.
///
/// Returns `None` when the mnemonic has no entry in the generated form table
/// (caller falls back to the coarse `INSTR_OPERANDS` masks).
pub(super) fn form_slot_candidates(
    mnemonic_upper: &str,
    typed_args: &[String],
    slot: usize
) -> Option<(Vec<(&'static str, &'static str)>, bool)> {
    let forms: Vec<&(&str, &[&str])> = INSTR_FORMS
        .iter()
        .filter(|(m, _)| *m == mnemonic_upper)
        .collect();
    if forms.is_empty() {
        return None;
    }

    let mut tokens: Vec<(&'static str, &'static str)> = Vec::new();
    let mut accepts_expr = false;

    for (_, operands) in forms {
        if operands.len() <= slot {
            continue;
        }
        // All already-typed operands must match this form.
        let matches_typed = typed_args.iter().take(slot).enumerate().all(|(i, typed)| {
            operands
                .get(i)
                .is_some_and(|pattern| operand_matches(pattern, typed))
        });
        if !matches_typed {
            continue;
        }

        let pattern = operands[slot];
        let (mut cands, expr) = pattern_candidates(pattern);
        if pattern == "r" && form_is_refresh_ld(operands) {
            cands.push(("R", "memory refresh register"));
        }
        accepts_expr |= expr;
        for c in cands {
            if !tokens.iter().any(|(t, _)| *t == c.0) {
                tokens.push(c);
            }
        }
    }

    Some((tokens, accepts_expr))
}

/// Split the operand region of the statement (after the mnemonic, up to the
/// cursor) at top-level commas. The last element is the in-progress operand.
pub(super) fn typed_operands_before(line: &str, col: usize) -> Vec<String> {
    let col = col.min(line.len());
    let before = &line[..col];
    let trimmed = before.trim_start();

    // Skip an optional label definition.
    let stmt = {
        let bytes = trimmed.as_bytes();
        let mut i = 0;
        while i < bytes.len()
            && (bytes[i].is_ascii_alphanumeric()
                || bytes[i] == b'_'
                || bytes[i] == b'@'
                || bytes[i] == b'.')
        {
            i += 1;
        }
        if i > 0 && i < bytes.len() && bytes[i] == b':' {
            trimmed[i + 1..].trim_start()
        }
        else {
            trimmed
        }
    };

    // Skip the mnemonic itself.
    let rest = match stmt.split_once(char::is_whitespace) {
        Some((_, r)) => r,
        None => return Vec::new()
    };

    let mut parts: Vec<String> = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for ch in rest.chars() {
        match ch {
            '(' | '[' => {
                depth += 1;
                cur.push(ch);
            },
            ')' | ']' => {
                depth -= 1;
                cur.push(ch);
            },
            ',' if depth <= 0 => {
                parts.push(cur.trim().to_lowercase());
                cur.clear();
            },
            _ => cur.push(ch)
        }
    }
    parts.push(cur.trim().to_lowercase());
    parts
}

impl AssemblyAnalyzer {
    /// Backwards-compatible entry point: completion using only the current document.
    pub fn completion(&self, document: &Document, position: Position) -> Vec<CompletionItem> {
        self.completion_with_documents(document, position, &[])
    }

    /// Provide context-aware completion suggestions.
    ///
    /// * **Mnemonic position** (start of statement): instructions + directives
    ///   + labels + snippets. Registers are excluded here.
    /// * **Instruction operand**: only the operands valid for that argument
    ///   slot *given the operands already typed* (driven by the instruction
    ///   form table generated from `data/timings.txt`), plus labels when the
    ///   slot accepts an expression.
    /// * **Directive argument**: filenames (quoted) for file-based directives,
    ///   labels / document symbols otherwise.
    ///
    /// Keyword-like completions follow the case the user started typing.
    /// `other_documents` supplies labels from the other open assembly files.
    pub fn completion_with_documents(
        &self,
        document: &Document,
        position: Position,
        other_documents: &[Document]
    ) -> Vec<CompletionItem> {
        let line = document.line(position.line as usize).unwrap_or_default();
        let col = position.character as usize;
        let ctx = analyze_context(&line, col);
        let case = case_pref_at(&line, col);

        // Collect symbols (labels, EQU constants, macros) from this document
        // and every other open assembly document.
        let mut doc_symbols: Vec<(String, String)> = self.collect_symbols(document);
        for other in other_documents {
            let fname = other
                .uri
                .path_segments()
                .and_then(|mut s| s.next_back())
                .unwrap_or("")
                .to_string();
            for (sym, detail) in self.collect_symbols(other) {
                if !doc_symbols.iter().any(|(s, _)| *s == sym) {
                    doc_symbols.push((sym, format!("{detail} ({fname})")));
                }
            }
        }

        let mut completions = Vec::new();

        match ctx {
            CompletionContext::MnemonicPosition => {
                for mnemonic in cpclib_asm::lsp::Z80_INSTRUCTIONS {
                    completions.push(keyword_item(mnemonic, "Z80 instruction", case));
                }
                for (directives, detail) in [
                    (
                        cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_STANDALONE,
                        "assembler directive"
                    ),
                    (
                        cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_START,
                        "block-start directive"
                    ),
                    (
                        cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_END,
                        "block-end directive"
                    )
                ] {
                    for d in directives {
                        completions.push(keyword_item(d, detail, case));
                    }
                }
                // Labels / macros — macros can be invoked like mnemonics.
                for (sym, detail) in &doc_symbols {
                    completions.push(symbol_item(sym, detail));
                }
                // Snippets (shared with the Zed extension).
                for (prefix, description, body) in SNIPPETS {
                    completions.push(CompletionItem {
                        label: prefix.to_string(),
                        kind: Some(CompletionItemKind::SNIPPET),
                        detail: Some(description.to_string()),
                        insert_text: Some(body.to_string()),
                        insert_text_format: Some(InsertTextFormat::SNIPPET),
                        ..Default::default()
                    });
                }
            },

            CompletionContext::InstructionOperand {
                ref mnemonic,
                arg_index
            } => {
                let typed = typed_operands_before(&line, col);

                match form_slot_candidates(mnemonic, &typed, arg_index) {
                    Some((tokens, accepts_expr)) => {
                        for (token, detail) in tokens {
                            completions.push(operand_item(token, detail, case));
                        }
                        if accepts_expr {
                            for (sym, detail) in &doc_symbols {
                                completions.push(symbol_item(sym, detail));
                            }
                        }
                    },
                    None => {
                        // Instruction not in the form table → coarse masks.
                        let mask = match operand_mask(mnemonic, arg_index) {
                            Some(m) => m,
                            None => T_R8 | T_R16 | T_IX | T_IY | T_COND8 | T_EXPR
                        };
                        for (token, detail) in tokens_for_mask(mask) {
                            completions.push(operand_item(token, detail, case));
                        }
                        if mask_accepts_expression(mask) {
                            for (sym, detail) in &doc_symbols {
                                completions.push(symbol_item(sym, detail));
                            }
                        }
                    }
                }
            },

            CompletionContext::DirectiveArgument => {
                let directive = first_statement_word(&line).unwrap_or_default();
                if DIRECTIVE_FILE_ARGS.contains(&directive.as_str()) {
                    completions.extend(directive_filename_completions(document, &line, col));
                }
                else {
                    // Directives accept any expression — offer symbols.
                    for (sym, detail) in &doc_symbols {
                        completions.push(symbol_item(sym, detail));
                    }
                }
            }
        }

        completions
    }

    /// Collect `(name, detail)` for labels / EQU / assigns / macros of a document.
    ///
    /// Falls back to a text scan when the document does not parse — which is
    /// the common case while the user is mid-typing the very line being
    /// completed.
    fn collect_symbols(&self, document: &Document) -> Vec<(String, String)> {
        let Ok(listing) = self.parse_document(document)
        else {
            return collect_symbols_by_text(document);
        };
        let mut syms = Vec::new();
        for token in listing.iter() {
            if token.is_label() {
                syms.push((token.label_symbol().to_string(), "label".to_string()));
            }
            else if token.is_equ() {
                syms.push((
                    token.equ_symbol().to_string(),
                    format!("= {}", token.equ_value())
                ));
            }
            else if token.is_assign() {
                syms.push((
                    token.assign_symbol().to_string(),
                    format!("= {}", token.assign_value())
                ));
            }
            else if token.is_macro_definition() {
                syms.push((
                    token.macro_definition_name().to_string(),
                    "macro".to_string()
                ));
            }
        }
        syms
    }
}

/// Text-based symbol scan used when the document does not parse: labels at
/// line start (with or without `:`), and `name EQU ...` / `name = ...`.
fn collect_symbols_by_text(document: &Document) -> Vec<(String, String)> {
    let text = document.text();
    let mut syms: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let bytes = trimmed.as_bytes();
        let mut i = 0;
        while i < bytes.len()
            && (bytes[i].is_ascii_alphanumeric()
                || bytes[i] == b'_'
                || bytes[i] == b'@'
                || bytes[i] == b'.')
        {
            i += 1;
        }
        if i == 0 || bytes[0].is_ascii_digit() {
            continue;
        }
        let name = &trimmed[..i];
        let rest = trimmed[i..].trim_start();

        let is_label = trimmed[i..].starts_with(':') && !trimmed[i..].starts_with("::")
            || (indent == 0 && (rest.is_empty() || rest.starts_with(';')));
        let is_equ = rest.len() >= 3
            && rest[..3].eq_ignore_ascii_case("equ")
            && !rest.as_bytes().get(3).is_some_and(|&b| is_ident_byte(b));
        let is_assign = rest.starts_with('=') && !rest.starts_with("==");

        let detail = if is_label {
            "label"
        }
        else if is_equ || is_assign {
            "constant"
        }
        else {
            continue;
        };
        if !syms.iter().any(|(s, _)| s == name) {
            syms.push((name.to_string(), detail.to_string()));
        }
    }
    syms
}

/// First word of the statement on `line` (label definitions skipped), uppercased.
fn first_statement_word(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    while i < bytes.len()
        && (bytes[i].is_ascii_alphanumeric()
            || bytes[i] == b'_'
            || bytes[i] == b'@'
            || bytes[i] == b'.')
    {
        i += 1;
    }
    let stmt = if i > 0 && i < bytes.len() && bytes[i] == b':' {
        trimmed[i + 1..].trim_start()
    }
    else {
        trimmed
    };
    let word: String = stmt
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
        .collect();
    if word.is_empty() {
        None
    }
    else {
        Some(word.to_uppercase())
    }
}

/// A keyword-ish completion (instruction, directive, register…) honouring the
/// case the user started typing.
fn keyword_item(word: &str, detail: &str, case: CasePref) -> CompletionItem {
    let text = apply_case(word, case);
    CompletionItem {
        label: text.clone(),
        kind: Some(CompletionItemKind::KEYWORD),
        detail: Some(detail.to_string()),
        insert_text: Some(text),
        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
        ..Default::default()
    }
}

/// An operand completion (register / condition / addressing mode).
/// Only the alphabetic part follows the user's case; parentheses and the
/// `+n` displacement placeholder are kept as-is.
fn operand_item(token: &str, detail: &str, case: CasePref) -> CompletionItem {
    let text = match case {
        CasePref::Lower => token.to_lowercase(),
        CasePref::Upper => token.to_string()
    };
    CompletionItem {
        label: text.clone(),
        kind: Some(CompletionItemKind::CONSTANT),
        detail: Some(detail.to_string()),
        insert_text: Some(text),
        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
        ..Default::default()
    }
}

/// A user-symbol completion — case is preserved verbatim (identifiers are
/// case-sensitive from the user's point of view).
fn symbol_item(sym: &str, detail: &str) -> CompletionItem {
    CompletionItem {
        label: sym.to_string(),
        kind: Some(CompletionItemKind::REFERENCE),
        detail: Some(detail.to_string()),
        ..Default::default()
    }
}

/// Filename completions for a file-based directive (INCLUDE, INCBIN, SAVE…).
/// When the cursor is inside a quoted string, complete the path being typed;
/// otherwise offer quoted filenames.
fn directive_filename_completions(
    document: &Document,
    line: &str,
    col: usize
) -> Vec<CompletionItem> {
    // Detect an open quote before the cursor.
    let before = &line[..col.min(line.len())];
    let in_string = before.matches('"').count() % 2 == 1;
    let prefix = if in_string {
        before.rsplit('"').next().unwrap_or("")
    }
    else {
        ""
    };

    let Ok(doc_path) = document.uri.to_file_path()
    else {
        return Vec::new();
    };
    let Some(base_dir) = doc_path.parent()
    else {
        return Vec::new();
    };

    let (dir_part, file_prefix) = match prefix.rfind('/') {
        Some(idx) => (&prefix[..idx], &prefix[idx + 1..]),
        None => ("", prefix)
    };
    let search_dir = if dir_part.is_empty() {
        base_dir.to_path_buf()
    }
    else {
        base_dir.join(dir_part)
    };

    let Ok(entries) = std::fs::read_dir(&search_dir)
    else {
        return Vec::new();
    };

    let mut items: Vec<CompletionItem> = entries
        .filter_map(|e| e.ok())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with(file_prefix) || name.starts_with('.') {
                return None;
            }
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let insert_text = if in_string {
                if is_dir {
                    format!("{name}/")
                }
                else {
                    name.clone()
                }
            }
            else if is_dir {
                format!("\"{name}/")
            }
            else {
                format!("\"{name}\"")
            };
            Some(CompletionItem {
                label: name,
                kind: Some(if is_dir {
                    CompletionItemKind::FOLDER
                }
                else {
                    CompletionItemKind::FILE
                }),
                insert_text: Some(insert_text),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            })
        })
        .take(200)
        .collect();
    items.sort_by(|a, b| a.label.cmp(&b.label));
    items
}

#[cfg(test)]
mod form_completion_tests {
    use super::*;

    fn labels(items: &[CompletionItem]) -> Vec<String> {
        items.iter().map(|i| i.label.clone()).collect()
    }

    fn complete(line: &str) -> Vec<CompletionItem> {
        let uri = Url::parse("file:///t.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, format!("{line}\n"), 1);
        AssemblyAnalyzer::new().completion(
            &doc,
            Position {
                line: 0,
                character: line.chars().count() as u32
            }
        )
    }

    #[test]
    fn ld_a_offers_bc_indirect_but_ld_b_does_not() {
        let with_a = labels(&complete("    ld a,"));
        assert!(
            with_a.iter().any(|l| l.eq_ignore_ascii_case("(bc)")),
            "{with_a:?}"
        );
        assert!(with_a.iter().any(|l| l.eq_ignore_ascii_case("i")));

        let with_b = labels(&complete("    ld b,"));
        assert!(
            !with_b.iter().any(|l| l.eq_ignore_ascii_case("(bc)")),
            "{with_b:?}"
        );
        assert!(!with_b.iter().any(|l| l.eq_ignore_ascii_case("i")));
        assert!(with_b.iter().any(|l| l.eq_ignore_ascii_case("a")));
    }

    #[test]
    fn ld_sp_offers_hl_ix_but_no_8bit() {
        let items = labels(&complete("    ld sp,"));
        assert!(
            items.iter().any(|l| l.eq_ignore_ascii_case("hl")),
            "{items:?}"
        );
        assert!(items.iter().any(|l| l.eq_ignore_ascii_case("ix")));
        assert!(!items.iter().any(|l| l.eq_ignore_ascii_case("b")));
    }

    #[test]
    fn case_follows_typed_prefix() {
        // lowercase prefix -> lowercase completions
        let lower = complete("    l");
        let ld_lower = lower
            .iter()
            .find(|i| i.label.eq_ignore_ascii_case("ld"))
            .unwrap();
        assert_eq!(ld_lower.label, "ld");
        // uppercase prefix -> uppercase completions
        let upper = complete("    L");
        let ld_upper = upper
            .iter()
            .find(|i| i.label.eq_ignore_ascii_case("ld"))
            .unwrap();
        assert_eq!(ld_upper.label, "LD");
    }

    #[test]
    fn snippets_are_offered_in_mnemonic_position() {
        let items = complete("    ");
        let snippet = items
            .iter()
            .find(|i| i.kind == Some(CompletionItemKind::SNIPPET) && i.label == "rep");
        assert!(snippet.is_some(), "rep snippet should be offered");
        assert_eq!(
            snippet.unwrap().insert_text_format,
            Some(InsertTextFormat::SNIPPET)
        );
    }

    #[test]
    fn operand_case_follows_typed_prefix() {
        let items = complete("    ld a,b");
        // typed lowercase 'b' -> candidates lowercase
        assert!(items.iter().any(|i| i.label == "c"), "{:?}", labels(&items));
    }

    #[test]
    fn expression_slots_offer_labels() {
        let uri = Url::parse("file:///t2.asm").unwrap();
        let text = "my_routine:\n    ret\n    call ";
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let items = AssemblyAnalyzer::new().completion(
            &doc,
            Position {
                line: 2,
                character: 9
            }
        );
        assert!(
            items.iter().any(|i| i.label == "my_routine"),
            "labels should be offered for call target: {:?}",
            labels(&items)
        );
    }
}
