//! `balance_branches` over real parsed Z80 source. See `cost_range.rs` in this
//! directory for why these tests live outside the crate they exercise.

use cpclib_z80flow::branch_balance::{InstructionCost, StabilizeEdit, balance_branches};
use cpclib_tokens::{DataAccessElem, ListingElement, Mnemonic};

use cpclib_asm::parser::obtained::LocatedToken;
use cpclib_asm::parser::parse_z80_str;

/// A tiny, test-only cost source mirroring the real Z80/CPC "NOPs"
/// timing convention (1 NOP = 4 T-states) this whole feature is built
/// around: `jr cc`/`ret cc` = 3 taken / 2 not-taken, unconditional
/// `jr`/`ret` = 3, `ld r,r'`/`nop` = 1 each - the same real values used
/// to hand-verify the shipped LSP version's own tests, kept identical
/// here so the same hand-verified expected numbers still apply.
fn test_cost(token: &LocatedToken) -> InstructionCost {
    match token.mnemonic() {
        Some(Mnemonic::Jr) | Some(Mnemonic::Ret) => {
            if token.mnemonic_arg1().is_some_and(|a| a.is_flag_test()) {
                InstructionCost::Conditional {
                    taken: 3,
                    not_taken: 2
                }
            }
            else {
                InstructionCost::Fixed(3)
            }
        },
        Some(Mnemonic::Ld) | Some(Mnemonic::Nop) => InstructionCost::Fixed(1),
        Some(Mnemonic::Djnz) => InstructionCost::Unknown,
        // `CALL` is no longer rejected out of hand, so a test exercising
        // it needs it priced. 5 is not invented: it is what
        // `cpclib-lsp/data/timings.txt` gives for `call nn`. This crate
        // deliberately owns no timing table (see the module doc comment) -
        // the real one lives in `cpclib-lsp`, which depends on *this*
        // crate, so reaching for it here would invert the dependency. The
        // real table is exercised end to end by
        // `cpclib-lsp::basm::cycles`'s own call-following tests instead.
        Some(Mnemonic::Call) => InstructionCost::Fixed(5),
        _ => InstructionCost::Fixed(0)
    }
}

fn balance(code: &str) -> Result<Vec<StabilizeEdit>, String> {
    // `LocatedToken::clone()` is an `unimplemented!()` stub in this
    // codebase today, so tests borrow straight from the parsed
    // `LocatedListing` rather than collecting an owned `Vec`.
    let listing = parse_z80_str(code).unwrap();
    let tokens: Vec<&LocatedToken> = listing.iter().collect();
    balance_branches(&tokens, test_cost)
}

/// The classic shape: `jr nz,.b` / cheap not-taken arm (ends with its
/// own unconditional jump over) / `.b:` expensive taken arm / `.over:`.
/// Hand-verified: taken path = 3 (jr taken) + 1 (ld a,b) + 1 (ld c,d) =
/// 5; not-taken path = 2 (jr not taken) + 1 (ld a,b) + 3 (jr .over) =
/// 6. The taken arm is cheaper by 1, so 1 NOP must land right before
/// the token starting the `.over:` block (index 6: `jr nz,.b`(0),
/// `ld a,b`(1), `jr .over`(2), `.b`(3), `ld a,b`(4), `ld c,d`(5),
/// `.over`(6)).
#[test]
fn single_branch_pads_the_cheaper_arm() {
    let code = "    jr nz,.b\n    ld a,b\n    jr .over\n.b\n    ld a,b\n    ld c,d\n.over\n";
    let edits = balance(code).unwrap();
    assert_eq!(
        edits,
        vec![StabilizeEdit::InsertPadding {
            insert_before_index: 6,
            nop_count: 1
        }]
    );
}

#[test]
fn already_balanced_branch_needs_no_edits() {
    // not-taken: 2 (jr) + 1+1 (nop*2) + 3 (jr .over) = 7
    // taken:      3 (jr) + 1+1+1+1 (nop*4) = 7
    let code = "    jr nz,.b\n    nop\n    nop\n    jr .over\n.b\n    nop\n    nop\n    nop\n    nop\n.over\n";
    assert!(balance(code).unwrap().is_empty());
}

#[test]
fn nested_branch_is_resolved_innermost_first() {
    let code = "\
jr nz,.outer_b
nop
jr .outer_over
.outer_b
jr z,.inner_b
nop
jr .inner_over
.inner_b
nop
nop
.inner_over
.outer_over
";
    let edits = balance(code).unwrap();
    // Inner branch: taken (3) + nop*2 (2) = 5; not-taken (2) + nop (1)
    // + jr .inner_over (3) = 6 -> inner taken arm padded by 1, landing
    // right before the `.inner_over` token (index 10).
    assert!(
        edits.iter().any(|e| {
            matches!(
                e,
                StabilizeEdit::InsertPadding {
                    insert_before_index: 10,
                    nop_count: 1
                }
            )
        }),
        "{edits:?}"
    );
    // Whatever the outer branch's own imbalance resolves to, it must
    // land at or before `.outer_over` (index 11), never past it.
    assert!(
        edits.iter().all(|e| {
            match e {
                StabilizeEdit::InsertPadding {
                    insert_before_index,
                    ..
                } => *insert_before_index <= 11,
                StabilizeEdit::RewriteConditionalRetAndPad {
                    ret_token_index, ..
                } => *ret_token_index <= 11
            }
        }),
        "{edits:?}"
    );
}

#[test]
fn sibling_branches_are_each_balanced_independently() {
    let code = "\
jr nz,.a_b
nop
jr .a_over
.a_b
nop
nop
.a_over
jr z,.c_b
nop
jr .c_over
.c_b
nop
nop
nop
.c_over
";
    let edits = balance(code).unwrap();
    // First branch: taken (3+2=5) vs not-taken (2+1+3=6) -> pad taken
    // arm by 1 before `.a_over` (index 6).
    assert!(
        edits.contains(&StabilizeEdit::InsertPadding {
            insert_before_index: 6,
            nop_count: 1
        }),
        "{edits:?}"
    );
    // Second branch: taken (3+3=6) vs not-taken (2+1+3=6) -> already
    // balanced, no edit for it.
    assert_eq!(edits.len(), 1, "{edits:?}");
}

#[test]
fn a_backward_jump_is_rejected_as_a_loop() {
    let code = ".loop\n    nop\n    jr nz,.loop\n";
    assert!(balance(code).is_err());
}

#[test]
fn djnz_is_rejected() {
    let code = "    djnz .x\n.x\n    nop\n";
    assert!(balance(code).is_err());
}

/// `CALL` used to be rejected on sight, so a selection containing one
/// could never be balanced at all. It can be now, as long as the routine
/// it calls has a single exact cost - which is the ordinary case for a
/// helper defined in the same file.
///
/// Both arms here call a routine, and the routines differ in cost: taken
/// calls `.big` (5 + 1 + 1 + 3 = 10), not-taken calls `.small` (5 + 1 + 3
/// = 9). Before this change the whole selection was refused; now the
/// 1-unit difference is found and padded.
#[test]
fn a_call_with_an_exact_cost_no_longer_blocks_balancing() {
    let code = "\
jr nz,.taken
call .small
jr .over
.taken
call .big
.over
ret
.small
ld a,b
ret
.big
ld a,b
ld c,d
ret
";
    let edits = balance(code).expect("a call with a knowable cost must not block the balance");
    assert!(
        !edits.is_empty(),
        "the two arms differ by their routines' costs: {edits:?}"
    );
}

/// The narrower rule that replaced the blanket rejection: a routine
/// whose own cost is not a single number cannot be padded against, and
/// the refusal now names that rather than the mnemonic.
#[test]
fn a_call_without_an_exact_cost_is_still_rejected() {
    // Defined outside the selection - nothing to price.
    let outside = "    jr nz,.b\n    call elsewhere\n    jr .over\n.b\n    nop\n.over\n    ret\n";
    let err = balance(outside).expect_err("an unpriceable routine must stop the balance");
    assert!(err.contains("CALL"), "{err}");

    // Defined, but it branches *unevenly* - so it has a range, not a
    // cost. (An evenly balanced routine would have an exact cost and
    // would rightly be accepted: taken 3 + ret 3 = 6 either way. It takes
    // a genuine imbalance - here 6 against 2+1+1+3 = 7 - to make the
    // callee unpriceable.)
    let ranged = "\
jr nz,.b
call .ranged
jr .over
.b
nop
.over
ret
.ranged
jr nz,.late
ld a,b
ld c,d
.late
ret
";
    assert!(balance(ranged).is_err());
}

/// A real-world idiom (from the user's own `bc26_hl` example): `RET cc`
/// is no longer blanket-rejected the way `DJNZ`/`CALL` still are. A
/// lone unconditional `RET` in particular - how nearly every real
/// subroutine selection ends - must not abort the whole pass just
/// because it's present (this was the actual root cause of the Quick
/// Fix never appearing at all for real selections).
#[test]
fn a_lone_unconditional_ret_is_not_rejected() {
    let code = "    ld a,b\n    ret\n";
    assert!(balance(code).unwrap().is_empty());
}

/// `RET cc` alone, nothing following it in the selection: its taken
/// side reaches the virtual exit directly (cost 3, no arm content -
/// there is nothing after this token at all), and so does its
/// not-taken side (cost 2, also no arm content - `next_block` defaults
/// to `exit` when nothing follows). The not-taken side is cheaper by
/// 1, and - unlike the taken side - it has a perfectly well-defined
/// insertion point (right after this one token, index 1) via
/// `arm_padding_index`'s own "end == exit" fallback, so this is a
/// plain `InsertPadding`, not a rewrite.
#[test]
fn ret_cc_alone_pads_the_fallthrough_arm_via_plain_insertion() {
    let code = "    ret nc\n";
    let edits = balance(code).unwrap();
    assert_eq!(
        edits,
        vec![StabilizeEdit::InsertPadding {
            insert_before_index: 1,
            nop_count: 1
        }]
    );
}

/// The user's own idiom, minimally reproduced: `ret nc` (early exit)
/// followed by more code ending in its own unconditional `ret`. Hand
/// verified: taken (early-exit) path = 3 (ret nc taken) + 0 (nothing
/// else - it leaves immediately) = 3; not-taken path = 2 (ret nc not
/// taken) + 1 (nop) + 3 (the trailing ret) = 6. The early-exit arm is
/// cheaper by 3, and it has no in-selection code to pad into - this
/// must come back as a rewrite, not a plain insertion.
#[test]
fn ret_cc_pads_the_early_exit_arm_via_rewrite() {
    let code = "    ret nc\n    nop\n    ret\n";
    let edits = balance(code).unwrap();
    assert_eq!(
        edits,
        vec![StabilizeEdit::RewriteConditionalRetAndPad {
            ret_token_index: 0,
            nop_count: 3
        }]
    );
}

/// Same shape as the rewrite case above, but the fallthrough arm's own
/// content (just one `nop`, no trailing `ret` - it simply runs off the
/// end of the selection, itself reaching the virtual exit) is sized so
/// both paths cost exactly 3: taken = 3 (ret nc taken); not-taken = 2
/// (ret nc not taken) + 1 (nop) = 3.
#[test]
fn ret_cc_already_balanced_needs_no_edits() {
    let code = "    ret nc\n    nop\n";
    assert!(balance(code).unwrap().is_empty());
}

#[test]
fn a_jump_target_outside_the_tokens_is_rejected() {
    let code = "    jr nz,.elsewhere\n    nop\n";
    assert!(balance(code).is_err());
}

#[test]
fn a_selection_with_no_branch_at_all_yields_no_edits() {
    let code = "    ld a,b\n    ld c,d\n    nop\n";
    assert!(balance(code).unwrap().is_empty());
}
