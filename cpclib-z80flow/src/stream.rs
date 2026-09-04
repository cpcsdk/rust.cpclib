//! Turning a parsed listing into the normalized [`AnalysisOp`] stream.
//!
//! Two things stand between a token slice and something analyzable:
//!
//! * **Fake instructions.** basm accepts convenience forms (`ld hl, de`,
//!   `srl8 hl`, ...) that assemble to several real opcodes. Reasoning about
//!   registers means reasoning about what actually executes, so they are
//!   expanded here - the same thing upstream does at parse time.
//! * **`JQ`.** basm picks `JR` or `JP` for it at assembly time depending on
//!   whether the target is in relative range. Given the real addresses, that
//!   decision can be replayed, so the stream carries the concrete instruction
//!   rather than something the analysis would have to treat as opaque.
//!
//! Both cases produce [`AnalysisOp::Expanded`], which keeps a handle on the
//! token the user actually wrote plus `step`/`total` - enough for a quickfix
//! to tell "this match covers the whole original instruction" from "this match
//! cuts one in half".

use cpclib_tokens::{DataAccessElem, ListingElement, Mnemonic, Token};

use crate::analysis_op::AnalysisOp;

/// The normalized instruction stream for one token slice, plus the index
/// mappings needed to move between the two coordinate systems.
pub struct AnalysisStream<'t, T> {
    ops: Vec<AnalysisOp<'t, T>>,
    /// `token index -> index of that token's first op`.
    token_to_op: Vec<usize>,
    /// `op index -> the token it came from`.
    op_to_token: Vec<usize>
}

impl<'t, T> AnalysisStream<'t, T>
where
    T: ListingElement,
    T::DataAccess: DataAccessElem
{
    /// Build the stream, resolving `JQ` through `resolve_jq` when possible.
    ///
    /// `resolve_jq` is handed the token and returns the mnemonic it really
    /// assembles to (`Jp` or `Jr`); returning `None` - which is what a caller
    /// with no assembled addresses does - leaves the `JQ` opaque, so any
    /// analysis crossing it fails closed rather than guessing.
    pub fn build(tokens: &'t [&'t T], mut resolve_jq: impl FnMut(&T) -> Option<Mnemonic>) -> Self {
        let mut ops = Vec::with_capacity(tokens.len());
        let mut token_to_op = Vec::with_capacity(tokens.len());
        let mut op_to_token = Vec::with_capacity(tokens.len());

        for (token_index, token) in tokens.iter().enumerate() {
            token_to_op.push(ops.len());
            let before = ops.len();

            let token = *token;
            match classify(token, &mut resolve_jq) {
                Classified::Real => ops.push(AnalysisOp::Real(token)),
                Classified::NotAnInstruction => ops.push(AnalysisOp::Other(token)),
                Classified::Expansion(expansion) => {
                    let total = expansion.len();
                    for (step, op) in expansion.into_iter().enumerate() {
                        ops.push(AnalysisOp::Expanded {
                            origin: token,
                            step,
                            total,
                            op: Box::new(op)
                        });
                    }
                }
            }

            for _ in before..ops.len() {
                op_to_token.push(token_index);
            }
        }

        Self {
            ops,
            token_to_op,
            op_to_token
        }
    }

    pub fn ops(&self) -> &[AnalysisOp<'t, T>] {
        &self.ops
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Index of the *first* op produced by `token_index`.
    pub fn first_op_of_token(&self, token_index: usize) -> Option<usize> {
        self.token_to_op.get(token_index).copied()
    }

    /// Index just past the *last* op produced by `token_index` - i.e. where a
    /// walk that must start "after this instruction" begins. Skipping the
    /// whole expansion matters: starting one op later would land in the middle
    /// of a fake instruction.
    pub fn after_token(&self, token_index: usize) -> Option<usize> {
        let start = self.first_op_of_token(token_index)?;
        let end = self
            .token_to_op
            .get(token_index + 1)
            .copied()
            .unwrap_or(self.ops.len());
        // A token that produced no ops at all would give `end == start`;
        // never hand back an index before where it began.
        Some(end.max(start))
    }

    /// The token an op came from.
    pub fn token_of_op(&self, op_index: usize) -> Option<usize> {
        self.op_to_token.get(op_index).copied()
    }

    /// The ops covering a *token* range - what a constraint asking "what does
    /// this matched region do?" needs.
    ///
    /// Returns an empty slice for an empty range, which is a real case rather
    /// than an error: a `*` line that matched no instructions covers nothing,
    /// and every block-local constraint is then trivially satisfied. `None`
    /// means the range could not be resolved at all.
    ///
    /// Handling the expansion boundaries (`first_op_of_token` at the start,
    /// `after_token` at the end, so a fake instruction is never entered
    /// halfway) in one place - it was previously repeated at each caller,
    /// empty-range special case included.
    pub fn ops_for_token_range(
        &self,
        range: std::ops::Range<usize>
    ) -> Option<&[AnalysisOp<'t, T>]> {
        if range.is_empty() {
            return Some(&[]);
        }
        let first = self.first_op_of_token(range.start)?;
        let last = self.after_token(range.end - 1)?;
        self.ops.get(first..last)
    }
}

enum Classified {
    Real,
    NotAnInstruction,
    Expansion(Vec<Token>)
}

fn classify<T>(token: &T, resolve_jq: &mut impl FnMut(&T) -> Option<Mnemonic>) -> Classified
where
    T: ListingElement,
    T::DataAccess: DataAccessElem
{
    // Checked before the mnemonic, not after: `push bc, hl` is one statement
    // standing for two real pushes - the same relationship a fake instruction
    // has - but it carries no mnemonic of its own, so it would otherwise be
    // dismissed as "not an instruction" here and never reach the
    // mnemonic-keyed expansion below.
    if let Some(expansion) = token.multi_push_pop_to_listing() {
        return Classified::Expansion(
            expansion
                .into_iter()
                .map(|(m, a1, a2)| Token::OpCode(m, a1, a2, None))
                .collect()
        );
    }

    let Some(mnemonic) = token.mnemonic().copied()
    else {
        return Classified::NotAnInstruction;
    };

    if mnemonic == Mnemonic::Jq {
        // basm chooses `JR` or `JP` by how far the target is; replay that
        // choice so the analysis sees a real instruction. Without addresses
        // there is nothing to replay, so leave it opaque.
        return match resolve_jq(token) {
            Some(resolved) => {
                Classified::Expansion(vec![Token::OpCode(
                    resolved,
                    token.mnemonic_arg1().map(|a| a.to_data_access().into_owned()),
                    token.mnemonic_arg2().map(|a| a.to_data_access().into_owned()),
                    None
                )])
            },
            None => Classified::NotAnInstruction
        };
    }

    let arg1 = token.mnemonic_arg1();
    let arg2 = token.mnemonic_arg2();
    let arg3 = token.mnemonic_arg3();

    if T::is_fake_instruction_from_access(mnemonic, arg1, arg2, arg3)
        && let Some(expansion) = T::fake_to_listing_from_access(mnemonic, arg1, arg2, arg3)
    {
        return Classified::Expansion(
            expansion
                .into_iter()
                .map(|(m, a1, a2)| Token::OpCode(m, a1, a2, None))
                .collect()
        );
    }

    Classified::Real
}

/// Convenience: build a stream with no `JQ` resolution at all.
pub fn build_without_addresses<'t, T>(tokens: &'t [&'t T]) -> AnalysisStream<'t, T>
where
    T: ListingElement,
    T::DataAccess: DataAccessElem
{
    AnalysisStream::build(tokens, |_| None)
}
