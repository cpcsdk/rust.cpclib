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
                            op
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

#[cfg(test)]
mod tests {
    use cpclib_asm::flatten::flatten_for_analysis;
    use cpclib_asm::parser::{LocatedToken, parse_z80_str};

    use super::*;

    fn stream_of(source: &str) -> (cpclib_asm::parser::LocatedListing, Vec<String>) {
        let listing = parse_z80_str(source).expect("source must parse");
        let tokens: Vec<&LocatedToken> = flatten_for_analysis(listing.iter()).collect();
        let stream = build_without_addresses(&tokens);
        let rendered = stream
            .ops()
            .iter()
            .map(|op| {
                match op {
                    AnalysisOp::Real(_) => format!("real {}", op.mnemonic().unwrap()),
                    AnalysisOp::Expanded { step, total, .. } => {
                        format!("exp {}/{} {}", step, total, op.mnemonic().unwrap())
                    },
                    AnalysisOp::Other(_) => "other".to_string()
                }
            })
            .collect();
        (listing, rendered)
    }

    #[test]
    fn plain_instructions_become_one_real_op_each() {
        let (_l, rendered) = stream_of("    ld a, 1\n    nop\n    ret\n");
        assert_eq!(rendered, vec!["real LD", "real NOP", "real RET"]);
    }

    /// A fake instruction becomes its real expansion, every step pointing back
    /// at the one token the user wrote.
    #[test]
    fn a_fake_instruction_expands_into_its_real_steps() {
        let listing = parse_z80_str("    ld hl, de\n").unwrap();
        let tokens: Vec<&LocatedToken> = flatten_for_analysis(listing.iter()).collect();
        let stream = build_without_addresses(&tokens);

        assert!(
            stream.ops().len() > 1,
            "expected a real expansion, got {:?}",
            stream.ops().len()
        );
        let total = stream.ops().len();
        for (i, op) in stream.ops().iter().enumerate() {
            match op {
                AnalysisOp::Expanded {
                    step, total: t, ..
                } => {
                    assert_eq!(*step, i);
                    assert_eq!(*t, total);
                },
                other => panic!("step {i} is not expanded: {other:?}")
            }
            // Every step points back at the single token actually written.
            assert!(std::ptr::eq(op.origin(), tokens[0]));
        }
        // ...and each step really is a different instruction than the fake one.
        assert!(stream.ops().iter().all(|o| o.mnemonic().is_some()));
    }

    #[test]
    fn labels_and_data_become_other_ops() {
        let (_l, rendered) = stream_of("start:\n    defb 1, 2\n    ret\n");
        assert!(rendered.contains(&"other".to_string()), "{rendered:?}");
        assert!(rendered.contains(&"real RET".to_string()), "{rendered:?}");
    }

    /// The index maps have to survive an expansion, or a constraint anchored
    /// at "the instruction after line N" would start its walk in the middle of
    /// a fake instruction's expansion.
    #[test]
    fn index_maps_skip_a_whole_expansion() {
        let listing = parse_z80_str("    nop\n    ld hl, de\n    ret\n").unwrap();
        let tokens: Vec<&LocatedToken> = flatten_for_analysis(listing.iter()).collect();
        let stream = build_without_addresses(&tokens);

        assert_eq!(stream.first_op_of_token(0), Some(0));
        assert_eq!(stream.after_token(0), Some(1));

        // The fake instruction occupies several ops...
        let fake_start = stream.first_op_of_token(1).unwrap();
        let after_fake = stream.after_token(1).unwrap();
        assert!(
            after_fake > fake_start + 1,
            "expansion should span several ops: {fake_start}..{after_fake}"
        );
        // ...and "after" it is exactly where the next token begins.
        assert_eq!(Some(after_fake), stream.first_op_of_token(2));

        // Every op maps back to the token it came from.
        for op_index in fake_start..after_fake {
            assert_eq!(stream.token_of_op(op_index), Some(1));
        }
        assert_eq!(stream.token_of_op(after_fake), Some(2));
    }

    /// Without addresses there is nothing to resolve `JQ` against, so it stays
    /// opaque and anything walking through it must fail closed.
    #[test]
    fn jq_is_opaque_without_addresses_but_concrete_with_them() {
        let listing = parse_z80_str("target:\n    jq target\n").unwrap();
        let tokens: Vec<&LocatedToken> = flatten_for_analysis(listing.iter()).collect();

        let opaque = build_without_addresses(&tokens);
        let jq_op = opaque
            .ops()
            .iter()
            .find(|o| matches!(o, AnalysisOp::Other(t) if t.mnemonic() == Some(&Mnemonic::Jq)));
        assert!(jq_op.is_some(), "JQ should be opaque without addresses");

        // Given the assembler's real choice, it becomes that instruction.
        let resolved = AnalysisStream::build(&tokens, |t| {
            (t.mnemonic() == Some(&Mnemonic::Jq)).then_some(Mnemonic::Jr)
        });
        let jr = resolved
            .ops()
            .iter()
            .find(|o| o.mnemonic() == Some(Mnemonic::Jr))
            .expect("JQ should resolve to JR");
        assert!(jr.is_expanded());
        assert_eq!(jr.expansion_position(), Some((0, 1)));
        // ...still anchored to the `jq` the user wrote.
        assert_eq!(jr.origin().mnemonic(), Some(&Mnemonic::Jq));
    }

    /// A file with no fake instructions and no `JQ` must produce exactly one
    /// op per token - the property that makes this change a no-op for
    /// ordinary source.
    #[test]
    fn ordinary_source_is_one_op_per_token() {
        let listing =
            parse_z80_str("start:\n    ld a, 0\n    cp 0\n    ld b, b\n    ret\n").unwrap();
        let tokens: Vec<&LocatedToken> = flatten_for_analysis(listing.iter()).collect();
        let stream = build_without_addresses(&tokens);
        assert_eq!(stream.ops().len(), tokens.len());
        for i in 0..tokens.len() {
            assert_eq!(stream.first_op_of_token(i), Some(i));
            assert_eq!(stream.token_of_op(i), Some(i));
        }
    }
}
