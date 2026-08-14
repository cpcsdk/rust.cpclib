//! `AnalysisStream` construction over real parsed Z80 source. See
//! `cost_range.rs` in this directory for why these tests live outside the
//! crate they exercise.

use cpclib_tokens::{ListingElement, Mnemonic};
use cpclib_z80flow::analysis_op::AnalysisOp;
use cpclib_z80flow::stream::AnalysisStream;
use cpclib_z80flow::stream::build_without_addresses;
use cpclib_asm::flatten::flatten_for_analysis;
use cpclib_asm::parser::{LocatedToken, parse_z80_str};


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
