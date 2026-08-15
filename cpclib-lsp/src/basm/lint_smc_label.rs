//! A label written where `equ $-1` was meant.
//!
//! Self-modifying code names the *operand byte* of an instruction so a patch
//! elsewhere does not hard-code an offset:
//!
//! ```text
//!     ld a, 0 : .counter equ $-1     ; .counter is the `0`
//!     ...
//!     ld a, 5 : ld (.counter), a     ; patch it
//! ```
//!
//! Drop the `equ $-1` and the line still assembles - but `.counter` now marks
//! the address *after* the `ld`, so the patch overwrites the next
//! instruction's opcode instead of the operand. Nothing complains: the file
//! builds, and the demo misbehaves at run time.
//!
//! The same goes for an offset that does not match the operand. `ld hl, 0`
//! carries a word - low byte first - so `$-2` is the operand and `$-1` is only
//! its high half:
//!
//! ```text
//!     ld hl, 0 : .address equ $-2    ; the whole operand
//!     ld hl, 0 : .address equ $-1    ; just the high byte
//! ```
//!
//! The second is occasionally what someone means (patching a page number, say)
//! but far more often it is `$-1` typed out of habit from the 8-bit case, so
//! it is reported and left to the reader to dismiss.
//!
//! This is the mirror image of what `cpclib_asmoptim::smc` already knows. That
//! module reads `equ $-1` to decide an instruction must not be rewritten; this
//! one notices the same idiom with the `equ` missing.
//!
//! # What is deliberately *not* flagged
//!
//! A trailing label on the same line is legitimate when it marks where the
//! following code starts. Two things have to hold before this fires:
//!
//! 1. the instruction just before the label carries a *literal constant* -
//!    that is the byte the idiom exists to name, so `call setup : .after`
//!    is left alone;
//! 2. nothing jumps to the label. A label something branches to is a code
//!    address by definition, whatever precedes it. This is what a real source
//!    in the wild looks like:
//!
//!    ```text
//!    ld (hl),%1111 : jr c,Non : ld (hl),%11110000 : Non
//!    ```
//!
//!    `Non` follows an instruction with a literal, and is still perfectly
//!    correct - the `jr` two statements earlier says so.

use std::collections::HashSet;

use cpclib_asm::implementation::expression::ExprEvaluationExt;
use cpclib_asm::parser::obtained::{LocatedListing, LocatedToken, MayHaveSpan};
use cpclib_tokens::{
    BinaryOperation, DataAccessElem, ExprElement, ListingElement, Mnemonic, OperandKind
};

/// What is wrong with the label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SmcLabelProblem {
    /// No `equ` at all - the label marks the address after the instruction.
    Missing,
    /// An `equ $-N` whose `N` does not reach the start of the operand. Carries
    /// the offset that was actually written.
    WrongOffset(u8)
}

/// One suspicious label, with everything a diagnostic or quickfix needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SuspiciousSmcLabel {
    /// 0-based line the label sits on.
    pub line: u32,
    /// The label as written, `.counter` included.
    pub name: String,
    /// The offset that reaches the first byte of the operand.
    pub offset: u8,
    pub problem: SmcLabelProblem
}

impl SuspiciousSmcLabel {
    /// The `equ` that would be right here.
    pub fn suggestion(&self) -> String {
        format!("equ $-{}", self.offset)
    }

    /// What to tell the user, without the label name (the caller has it).
    pub fn explanation(&self) -> String {
        match self.problem {
            SmcLabelProblem::Missing => {
                "marks the address after this instruction, not its operand".to_string()
            },
            SmcLabelProblem::WrongOffset(written) => {
                format!(
                    "is $-{written}, which lands {} of this {}-byte operand, not its first byte",
                    if written == 1 { "on the last byte" } else { "past the start" },
                    self.offset
                )
            }
        }
    }
}

/// How many bytes back from `$` the immediate of this instruction starts, or
/// `None` when it carries no literal constant at all.
///
/// Only the *width* of the constant matters, and on the Z80 that follows the
/// company it keeps: a 16-bit register or an absolute address means a word,
/// everything else a byte. `ld a, 0` -> 1, `ld hl, 0` -> 2, `ld (0x1234), a`
/// -> 2.
fn literal_operand_width(token: &LocatedToken) -> Option<u8> {
    if token.mnemonic().is_none() {
        return None;
    }

    let operands = [token.mnemonic_arg1(), token.mnemonic_arg2()];
    let carries_literal = operands.iter().flatten().any(|op| {
        matches!(op.kind(), OperandKind::Expression | OperandKind::Memory)
            && op.get_expression().is_some_and(|e| e.is_value())
    });
    if !carries_literal {
        return None;
    }

    let wide = operands.iter().flatten().any(|op| {
        matches!(
            op.kind(),
            OperandKind::Reg16(_) | OperandKind::IndexReg16(_) | OperandKind::Memory
        )
    });
    Some(if wide { 2 } else { 1 })
}

/// The `N` of an `equ $-N` (or `= $-N`), when that is exactly what the
/// definition says.
///
/// Deliberately narrow: only `$ - <literal>`. Anything else - `$-1-1`, an
/// offset built from a symbol - is not the idiom being checked, and guessing at
/// its value would mean evaluating expressions this lint has no business
/// evaluating.
fn current_address_offset(token: &LocatedToken) -> Option<u8> {
    let value = if token.is_equ() {
        token.equ_value()
    }
    else if token.is_assign() {
        token.assign_value()
    }
    else {
        return None;
    };

    if !value.is_binary_operation() || value.binary_operation() != BinaryOperation::Sub {
        return None;
    }
    if !value.arg1().is_label() || value.arg1().label() != "$" {
        return None;
    }
    let offset = value.arg2();
    if !offset.is_value() {
        return None;
    }
    u8::try_from(offset.value()).ok()
}

/// The name an `EQU`/`=` defines.
fn defined_symbol(token: &LocatedToken) -> Option<&str> {
    if token.is_equ() {
        Some(token.equ_symbol())
    }
    else if token.is_assign() {
        Some(token.assign_symbol())
    }
    else {
        None
    }
}

/// 0-based line a token starts on.
fn line_of(token: &LocatedToken) -> u32 {
    token.span().relative_line_and_column().0.saturating_sub(1) as u32
}

/// Every label some instruction branches to.
///
/// A label reached by `jr`/`jp`/`call`/`djnz` names code, and no amount of
/// what surrounds its declaration changes that.
fn branch_targets(tokens: &[&LocatedToken]) -> HashSet<String> {
    let mut targets = HashSet::new();
    for token in tokens {
        if !matches!(
            token.mnemonic(),
            Some(Mnemonic::Jr | Mnemonic::Jp | Mnemonic::Call | Mnemonic::Djnz)
        ) {
            continue;
        }
        for operand in [token.mnemonic_arg1(), token.mnemonic_arg2()]
            .into_iter()
            .flatten()
        {
            if let Some(expr) = operand.get_expression() {
                targets.extend(expr.symbols_used().into_iter().map(|s| s.into_owned()));
            }
        }
    }
    targets
}

/// Every label that looks like a forgotten `equ $-1`.
pub(super) fn find_suspicious_smc_labels(listing: &LocatedListing) -> Vec<SuspiciousSmcLabel> {
    let tokens: Vec<&LocatedToken> = super::token::flatten_listing(listing.iter()).collect();
    let targets = branch_targets(&tokens);
    let mut out = Vec::new();

    for (index, token) in tokens.iter().enumerate() {
        // Either a bare label, or a definition that tries to name the operand
        // and misses. Both are the same idiom, only one half-written.
        let (name, problem) = if token.is_label() {
            (token.label_symbol(), None)
        }
        else if let Some(name) = defined_symbol(token) {
            match current_address_offset(token) {
                Some(written) => (name, Some(written)),
                // A definition that is not `$-N` is not this idiom at all.
                None => continue
            }
        }
        else {
            continue;
        };

        let Some(previous) = index.checked_sub(1).map(|i| tokens[i]) else {
            continue;
        };
        // Same source line: a label on its own line is naming the code that
        // follows, which is exactly what a label is for.
        if line_of(token) != line_of(previous) {
            continue;
        }
        let Some(offset) = literal_operand_width(previous) else {
            continue;
        };
        if targets.contains(name) {
            continue;
        }

        let problem = match problem {
            None => SmcLabelProblem::Missing,
            // The offset that reaches the operand is right - nothing to say.
            Some(written) if written == offset => continue,
            Some(written) => SmcLabelProblem::WrongOffset(written)
        };

        out.push(SuspiciousSmcLabel {
            line: line_of(token),
            name: name.to_string(),
            offset,
            problem
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basm::AssemblyAnalyzer;
    use crate::common::document::Document;

    fn found(text: &str) -> Vec<SuspiciousSmcLabel> {
        let d = Document::new(
            tower_lsp::lsp_types::Url::parse("file:///t.asm").unwrap(),
            text.to_string(),
            1
        );
        let analyzer = AssemblyAnalyzer::new();
        let listing = analyzer.parse_document(&d).expect("must parse");
        find_suspicious_smc_labels(&listing)
    }

    /// The whole shape of the problem, in one file: which of these five
    /// lines is wrong and which is not.
    #[test]
    fn the_reported_sample() {
        let text = "\tld hl, 0 : .counter1\n\
                    \tld a, 0: .counter2\n\
                    \tld hl, 0 : .counter3 equ $-2\n\
                    \tld hl, 0: .counter4 equ $-1\n\
                    \tld hl, 0\n\
                    .counter5\n";
        let names: Vec<String> = found(text).into_iter().map(|f| f.name).collect();
        assert_eq!(
            names,
            vec![".counter1", ".counter2", ".counter4"],
            "counter3 is correct, counter5 is a plain label on its own line"
        );
    }

    /// `$-1` after a 16-bit load names the high half of the operand, not the
    /// operand.
    #[test]
    fn an_offset_that_misses_the_operand_is_flagged() {
        let found = found("\tld hl, 0 : .address equ $-1\n");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].problem, SmcLabelProblem::WrongOffset(1));
        assert_eq!(found[0].suggestion(), "equ $-2");
    }

    /// The right offset says nothing, for either width.
    #[test]
    fn a_correct_offset_is_silent() {
        assert!(found("\tld hl, 0 : .address equ $-2\n").is_empty());
        assert!(found("\tld a, 0 : .counter equ $-1\n").is_empty());
        assert!(found("\tld a, 0 : .counter = $-1\n").is_empty());
    }

    /// An offset built from anything other than a plain literal is not this
    /// idiom, and this lint does not evaluate expressions.
    #[test]
    fn a_computed_offset_is_left_alone() {
        assert!(found("OFF equ 2\n\tld hl, 0 : .address equ $-OFF\n").is_empty());
    }

    /// The report that started this.
    #[test]
    fn a_bare_label_after_an_eight_bit_immediate_is_flagged() {
        let found = found("\tld a, 0 : .counter\n");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].name, ".counter");
        assert_eq!(found[0].suggestion(), "equ $-1");
    }

    /// A 16-bit load hides its constant two bytes back, not one.
    #[test]
    fn a_sixteen_bit_immediate_suggests_two() {
        let found = found("\tld hl, 0 : .address\n");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].suggestion(), "equ $-2");
    }

    /// The correct spelling must stay silent - it is the whole point.
    #[test]
    fn the_idiom_written_properly_is_not_flagged() {
        assert!(found("\tld a, 0 : .counter equ $-1\n").is_empty());
    }

    /// A label naming the code that follows is ordinary.
    #[test]
    fn a_label_on_its_own_line_is_not_flagged() {
        assert!(found("\tld a, 0\n.counter\n\tnop\n").is_empty());
    }

    /// A label something jumps to is a code address, whatever precedes it.
    /// Straight from a real source: the label follows `ld (hl),%11110000`,
    /// which does carry a literal, and is still entirely correct.
    #[test]
    fn a_branch_target_is_never_flagged() {
        assert!(
            found("\tld (hl),%1111 : jr c,Non : ld (hl),%11110000 : Non\n\tnop\n").is_empty()
        );
    }

    /// The discriminator: an instruction whose operand is a symbol is not the
    /// idiom, so a trailing label after it means what it says.
    #[test]
    fn a_trailing_label_after_a_jump_is_not_flagged() {
        assert!(found("retry\n\tjr nz, retry : .done\n\tnop\n").is_empty());
        assert!(found("\tcall setup : .after\n\tnop\nsetup\n\tret\n").is_empty());
    }

    /// An instruction with no operand at all cannot be the idiom.
    #[test]
    fn a_trailing_label_after_a_bare_instruction_is_not_flagged() {
        assert!(found("\tnop : .here\n").is_empty());
    }

    /// The address form: `ld (0x1234), a` patches through an absolute address,
    /// which is a word.
    #[test]
    fn an_absolute_address_operand_suggests_two() {
        let found = found("\tld (0x1234), a : .target\n");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].suggestion(), "equ $-2");
    }
}

#[cfg(test)]
mod quickfix_tests {
    use tower_lsp::lsp_types::*;

    use crate::basm::AssemblyAnalyzer;
    use crate::common::document::Document;

    fn action_on(text: &str, line: u32) -> Option<CodeAction> {
        let d = Document::new(
            Url::parse("file:///t.asm").unwrap(),
            text.to_string(),
            1
        );
        let cursor = Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 0 }
        };
        AssemblyAnalyzer::new()
            .code_actions(&d, cursor)
            .into_iter()
            .find(|a| a.title.contains("operand byte"))
    }

    /// The fix appends the missing half, leaving the rest of the line alone.
    #[test]
    fn the_quickfix_appends_the_equ_after_the_label() {
        let action = action_on("\tld a, 0 : .counter\n", 0).expect("a quickfix must be offered");
        let changes = action.edit.unwrap().changes.unwrap();
        let edits = changes.values().next().unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, " equ $-1");
        // An insertion, not a replacement.
        assert_eq!(edits[0].range.start, edits[0].range.end);
        // Right after `.counter`, which ends at column 19 of "\tld a, 0 : .counter".
        assert_eq!(edits[0].range.start.character, 19);
    }

    /// A 16-bit load needs the other offset.
    #[test]
    fn a_sixteen_bit_load_gets_two() {
        let action = action_on("\tld hl, 0 : .address\n", 0).unwrap();
        let changes = action.edit.unwrap().changes.unwrap();
        assert_eq!(changes.values().next().unwrap()[0].new_text, " equ $-2");
    }

    /// Apply the quickfix and hand back the line the user would see.
    fn fixed(text: &str, line: u32) -> Option<String> {
        let d = Document::new(Url::parse("file:///t.asm").unwrap(), text.to_string(), 1);
        let cursor = Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 0 }
        };
        let action = AssemblyAnalyzer::new()
            .code_actions(&d, cursor)
            .into_iter()
            .find(|a| a.title.contains("operand byte"))?;
        let changes = action.edit.unwrap().changes.unwrap();
        let edits = changes.values().next().unwrap().clone();
        assert_eq!(edits.len(), 1);
        let original = d.line(line as usize).unwrap();
        let original = original.trim_end_matches(['\r', '\n']);
        let start = edits[0].range.start.character as usize;
        let end = edits[0].range.end.character as usize;
        let mut out: String = original.chars().take(start).collect();
        out.push_str(&edits[0].new_text);
        out.extend(original.chars().skip(end));
        Some(out)
    }

    /// A wrong offset is corrected in place - appending a second `equ` would
    /// not even assemble.
    #[test]
    fn a_wrong_offset_is_rewritten_not_appended() {
        assert_eq!(
            fixed("\tld hl, 0: .counter4 equ $-1\n", 0).as_deref(),
            Some("\tld hl, 0: .counter4 equ $-2")
        );
    }

    /// Spacing and a trailing comment survive, because only the number moves.
    #[test]
    fn the_rest_of_the_line_is_left_alone() {
        assert_eq!(
            fixed("\tld hl, 0 : .p equ $ - 1 ; page\n", 0).as_deref(),
            Some("\tld hl, 0 : .p equ $ - 2 ; page")
        );
    }

    /// And the missing-`equ` case still appends.
    #[test]
    fn a_missing_equ_is_still_appended() {
        assert_eq!(
            fixed("\tld a, 0 : .counter\n", 0).as_deref(),
            Some("\tld a, 0 : .counter equ $-1")
        );
    }

    /// Nothing to offer where there is nothing wrong.
    #[test]
    fn no_quickfix_on_a_correct_line() {
        assert!(action_on("\tld a, 0 : .counter equ $-1\n", 0).is_none());
        assert!(action_on("\tld a, 0\n", 0).is_none());
    }
}

