//! What an instruction costs — the one question both timing analyses ask, and
//! the one place that knows an instruction is not always what it looks like.
//!
//! Two shapes of instruction do not cost what a table lookup on their own text
//! says, and both are ordinary in basm source:
//!
//! * **Fake instructions.** `ld hl, de` is not a Z80 opcode; basm assembles it
//!   to `ld h, d` / `ld l, e`. A cost source keyed on the text of what the user
//!   wrote finds nothing, so before this module the whole instruction
//!   contributed **zero** to a cycle count and merely bumped
//!   `unrecognized_count`. The real corpus has 29 of them across 15 files.
//! * **`JQ`**, basm's "assembler picks `JR` or `JP`" form. Also absent from any
//!   opcode table, for the same reason.
//!
//! Both are handled by asking the caller's cost source about the *real* opcodes
//! involved instead of the written form.

use cpclib_tokens::{DataAccess, ListingElement, Mnemonic, Token};

/// A cost source the algorithm queries once per token - kept fully
/// decoupled from any specific timing-data representation (see
/// [`crate::branch_balance`]'s module doc comment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionCost {
    /// A plain instruction's single cost.
    Fixed(u32),
    /// A conditional `JR`/`JP`'s own two costs. Mirrors `timing::
    /// format_hover`'s "taken/not taken" convention in `cpclib-lsp`.
    Conditional { taken: u32, not_taken: u32 },
    /// The cost source doesn't recognize this instruction.
    Unknown
}

/// Prices instructions.
///
/// Blanket-implemented for any `Fn(&T) -> InstructionCost`, so the ordinary
/// caller still passes a closure and nothing changes. Implementing the trait
/// directly adds one thing: the ability to price a *real opcode* that the
/// caller never wrote, which is what makes a fake instruction cost what it
/// actually assembles to.
pub trait CostModel<T> {
    /// The cost of a token the caller supplied.
    fn cost(&self, token: &T) -> InstructionCost;

    /// The cost of one real opcode that a fake instruction expanded into, or
    /// that a `JQ` resolves to.
    ///
    /// Defaults to [`InstructionCost::Unknown`], which reproduces exactly what
    /// a caller without this used to get: the fake instruction as a whole was
    /// unrecognized. So adding the trait changed no existing behaviour on its
    /// own - only implementing this method does.
    fn expanded_cost(&self, _op: &Token) -> InstructionCost {
        InstructionCost::Unknown
    }
}

impl<T, F> CostModel<T> for F
where F: Fn(&T) -> InstructionCost
{
    fn cost(&self, token: &T) -> InstructionCost {
        self(token)
    }
}

/// Sum a fake instruction's expansion, or `Unknown` if any part is unknown.
///
/// All-or-nothing on purpose: a partial sum is a number that looks right and
/// is short, which for a cycle-exact routine is worse than admitting the gap.
fn sum_expansion<T, C: CostModel<T>>(
    ops: &[(Mnemonic, Option<DataAccess>, Option<DataAccess>)],
    cost: &C
) -> InstructionCost {
    let mut total = 0;
    for (mnemonic, arg1, arg2) in ops {
        let op = Token::OpCode(*mnemonic, arg1.clone(), arg2.clone(), None);
        match cost.expanded_cost(&op) {
            InstructionCost::Fixed(n) => total += n,
            // An expansion is a run of plain opcodes; nothing in one branches,
            // so a conditional cost here means the cost source disagrees with
            // that assumption and the honest answer is that we do not know.
            InstructionCost::Conditional { .. } | InstructionCost::Unknown => {
                return InstructionCost::Unknown;
            }
        }
    }
    InstructionCost::Fixed(total)
}

/// `token`'s real cost, seeing through the two forms that are not opcodes.
///
/// Falls back to `cost.cost(token)` for everything else, so an ordinary
/// instruction takes exactly the path it always did.
pub(crate) fn instruction_cost<T, C>(token: &T, cost: &C) -> InstructionCost
where
    T: ListingElement,
    T::DataAccess: cpclib_tokens::DataAccessElem,
    C: CostModel<T>
{
    let Some(mnemonic) = token.mnemonic().copied()
    else {
        return cost.cost(token);
    };
    let (arg1, arg2, arg3) = (
        token.mnemonic_arg1(),
        token.mnemonic_arg2(),
        token.mnemonic_arg3()
    );

    // A fake instruction costs what it assembles to. `cpclib-tokens` owns both
    // the "is it fake" test and the expansion, so this stays in step with the
    // assembler by construction rather than by a list kept here.
    if T::is_fake_instruction_from_access(mnemonic, arg1, arg2, arg3)
        && let Some(expansion) = T::fake_to_listing_from_access(mnemonic, arg1, arg2, arg3)
    {
        return sum_expansion(&expansion, cost);
    }

    if mnemonic == Mnemonic::Jq {
        return jq_cost(token, cost);
    }

    cost.cost(token)
}

/// What a `JQ` costs, without knowing which instruction basm will pick.
///
/// The trick is that it usually does not matter: on the CPC an unconditional
/// `JR` and `JP` are both 3 NOPs, so the answer is the same either way. Rather
/// than assume that from the timing table, this *asks* for both and only
/// answers when they agree - so if the data ever said otherwise, the result
/// would become `Unknown` instead of quietly wrong.
///
/// A conditional `JQ` genuinely differs (`jr cc` is "3 or 2", `jp cc` is always
/// 3), so it stays unknown: which one it is depends on a distance only a real
/// assemble knows.
fn jq_cost<T, C>(token: &T, cost: &C) -> InstructionCost
where
    T: ListingElement,
    T::DataAccess: cpclib_tokens::DataAccessElem,
    C: CostModel<T>
{
    use cpclib_tokens::DataAccessElem;

    let arg1 = token.mnemonic_arg1().map(|a| a.to_data_access().into_owned());
    let arg2 = token.mnemonic_arg2().map(|a| a.to_data_access().into_owned());

    let conditional = arg1.as_ref().is_some_and(DataAccess::is_flag_test);
    if conditional {
        return InstructionCost::Unknown;
    }

    let as_op = |mnemonic| Token::OpCode(mnemonic, arg1.clone(), arg2.clone(), None);
    let as_jr = cost.expanded_cost(&as_op(Mnemonic::Jr));
    let as_jp = cost.expanded_cost(&as_op(Mnemonic::Jp));
    if as_jr == as_jp {
        as_jr
    }
    else {
        InstructionCost::Unknown
    }
}
