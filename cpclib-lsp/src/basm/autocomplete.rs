//! Completion for assembly files.
//!
//! The first half of this file is the semantic completion model: operand-type
//! bitflags, the per-instruction operand table, and cursor-context analysis.
//! The second half is the analyzer entry point that renders the candidates
//! into LSP `CompletionItem`s.

use cpclib_tokens::ListingElement;
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
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

impl AssemblyAnalyzer {
    /// Provide context-aware completion suggestions.
    ///
    /// * **Mnemonic position** (start of statement): instructions + directives + labels.
    ///   Registers are excluded here — they cannot appear as a mnemonic.
    /// * **Instruction operand**: only registers / conditions valid for that argument slot,
    ///   plus labels from the document when the slot accepts an expression (`n`/`nn`).
    ///   Instructions and directives are never offered inside an operand.
    /// * **Directive argument**: only labels / document symbols (any expression is valid).
    pub fn completion(&self, document: &Document, position: Position) -> Vec<CompletionItem> {
        let line = document.line(position.line as usize).unwrap_or_default();
        let col = position.character as usize;
        let ctx = analyze_context(&line, col);

        // Collect document-level symbols (labels, EQU constants, macros) for use as expressions.
        let doc_symbols: Vec<(String, String)> = if let Ok(listing) = self.parse_document(document)
        {
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
        else {
            Vec::new()
        };

        let mut completions = Vec::new();

        match ctx {
            CompletionContext::MnemonicPosition => {
                // Instructions
                for mnemonic in cpclib_asm::lsp::Z80_INSTRUCTIONS {
                    completions.push(CompletionItem {
                        label: mnemonic.to_string(),
                        kind: Some(CompletionItemKind::KEYWORD),
                        detail: Some("Z80 instruction".to_string()),
                        ..Default::default()
                    });
                }
                // Directives
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
                        completions.push(CompletionItem {
                            label: d.to_string(),
                            kind: Some(CompletionItemKind::KEYWORD),
                            detail: Some(detail.to_string()),
                            ..Default::default()
                        });
                    }
                }
                // Labels — macros can be invoked like mnemonics
                for (sym, detail) in &doc_symbols {
                    completions.push(CompletionItem {
                        label: sym.clone(),
                        kind: Some(CompletionItemKind::REFERENCE),
                        detail: Some(detail.clone()),
                        ..Default::default()
                    });
                }
            },

            CompletionContext::InstructionOperand {
                ref mnemonic,
                arg_index
            } => {
                // None  = instruction not in table → generous fallback
                // Some(0) = known instruction, no operand here → empty list
                // Some(m) = use m
                let mask = match operand_mask(mnemonic, arg_index) {
                    Some(m) => m,
                    None => T_R8 | T_R16 | T_IX | T_IY | T_COND8 | T_EXPR
                };

                // Register / condition / synthetic tokens from the mask
                for (token, detail) in tokens_for_mask(mask) {
                    completions.push(CompletionItem {
                        label: token.to_string(),
                        kind: Some(CompletionItemKind::CONSTANT),
                        detail: Some(detail.to_string()),
                        ..Default::default()
                    });
                }

                // Labels / expressions when the slot accepts one
                if mask_accepts_expression(mask) {
                    for (sym, detail) in &doc_symbols {
                        completions.push(CompletionItem {
                            label: sym.clone(),
                            kind: Some(CompletionItemKind::REFERENCE),
                            detail: Some(detail.clone()),
                            ..Default::default()
                        });
                    }
                }
            },

            CompletionContext::DirectiveArgument => {
                // Directives accept any expression — offer labels / document symbols only.
                for (sym, detail) in &doc_symbols {
                    completions.push(CompletionItem {
                        label: sym.clone(),
                        kind: Some(CompletionItemKind::REFERENCE),
                        detail: Some(detail.clone()),
                        ..Default::default()
                    });
                }
            }
        }

        completions
    }
}
