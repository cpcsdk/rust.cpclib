//! `cost_range` over real parsed Z80 source.
//!
//! Lives here rather than inside `cpclib-z80flow` because it needs a real
//! parser, and that crate must not depend on the assembler - see this crate's
//! `Cargo.toml`.

use cpclib_z80flow::cost_range::{CostRange, InstructionCost, cost_range};
use cpclib_tokens::{DataAccessElem, ListingElement, Mnemonic};

use cpclib_asm::parser::obtained::LocatedToken;
use cpclib_asm::parser::parse_z80_str;

/// A tiny, test-only cost source - deliberately distinct values from
/// `branch_balance`'s own test cost function, to keep every hand
/// derivation below self-contained and easy to re-check independently.
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
        Some(Mnemonic::Djnz) => {
            InstructionCost::Conditional {
                taken: 3,
                not_taken: 2
            }
        },
        Some(Mnemonic::Ld) | Some(Mnemonic::Nop) => InstructionCost::Fixed(1),
        Some(Mnemonic::Call) => {
            if token.mnemonic_arg1().is_some_and(|a| a.is_flag_test()) {
                InstructionCost::Conditional {
                    taken: 5,
                    not_taken: 2
                }
            }
            else {
                InstructionCost::Fixed(5)
            }
        },
        None => InstructionCost::Fixed(0),
        _ => InstructionCost::Unknown
    }
}

fn range(code: &str) -> Result<CostRange, String> {
    let listing = parse_z80_str(code).unwrap();
    let tokens: Vec<&LocatedToken> = listing.iter().collect();
    cost_range(&tokens, test_cost)
}

/// A diamond with a real prefix *before* the branch, proving the
/// prefix is actually included in both totals (not just the delta, the
/// way `branch_balance::balance` gets to skip it). Hand-verified:
/// taken = prefix(1) + jr_taken(3) + ld d,e(1) + ld h,l(1) + ret(3) =
/// 9; not-taken = prefix(1) + jr_not_taken(2) + ld c,d(1) + jr
/// .over(3) + ret(3) = 10.
#[test]
fn a_diamond_with_a_real_prefix_reports_absolute_totals() {
    let code = "\
ld a,b
jr nz,.b
ld c,d
jr .over
.b
ld d,e
ld h,l
.over
ret
";
    let summary = range(code).unwrap();
    assert_eq!(
        summary,
        CostRange {
            min: 9,
            max: 10,
            unbounded: false,
            instruction_count: 7,
            unrecognized_count: 0,
            incomplete: false
        }
    );
}

/// A `DJNZ` loop: unbounded (an unknown iteration count), but the
/// minimum is still real and well-defined - one pass through the loop
/// body. Hand-verified: min = ld a,b(1) + nop(1) + djnz not-taken(2) +
/// ret(3) = 7 (the loop's *taken*/looping side never competes as a
/// forward candidate at all - see the module doc comment).
#[test]
fn a_djnz_loop_is_unbounded_with_a_real_minimum() {
    let code = "\
ld a,b
.loop
nop
djnz .loop
ret
";
    let summary = range(code).unwrap();
    assert_eq!(summary.min, 7, "{summary:?}");
    assert!(summary.unbounded, "{summary:?}");
}

/// `CALL` folds into straight-line cost using only its own known
/// instruction cost - it doesn't split the CFG, and doesn't stop a
/// real branch elsewhere in the same selection from being modeled
/// correctly. Hand-verified: taken = jr_taken(3) + nop(1) = 4;
/// not-taken = jr_not_taken(2) + call(5) + jr .over(3) = 10.
#[test]
fn call_contributes_its_own_cost_without_splitting_the_cfg() {
    let code = "\
jr nz,.b
call foo
jr .over
.b
nop
.over
";
    let summary = range(code).unwrap();
    assert_eq!(summary.min, 4, "{summary:?}");
    assert_eq!(summary.max, 10, "{summary:?}");
    assert!(!summary.unbounded, "{summary:?}");
}

/// An escaping jump target (not defined anywhere in the selection) is
/// a well-defined partial cost, not an error - that path simply stops
/// being trackable at the point of escape. Hand-verified: taken
/// (escapes) = jr_taken(3) alone; not-taken = jr_not_taken(2) +
/// nop(1) = 3. Both sides happen to total 3 here.
#[test]
fn an_escaping_target_is_a_well_defined_partial_cost_not_an_error() {
    let code = "    jr nz,.elsewhere\n    nop\n";
    let summary = range(code).unwrap();
    assert_eq!(summary.min, 3, "{summary:?}");
    assert_eq!(summary.max, 3, "{summary:?}");
    assert!(!summary.unbounded, "{summary:?}");
}

/// An unrecognized instruction (`halt`, deliberately mapped to
/// `Unknown` by this test's own cost function) increments
/// `unrecognized_count` and contributes 0, rather than aborting.
#[test]
fn an_unrecognized_instruction_increments_the_count_instead_of_aborting() {
    let code = "    nop\n    halt\n    nop\n";
    let summary = range(code).unwrap();
    assert_eq!(summary.min, 2, "{summary:?}");
    assert_eq!(summary.max, 2, "{summary:?}");
    assert_eq!(summary.unrecognized_count, 1, "{summary:?}");
    assert_eq!(summary.instruction_count, 2, "{summary:?}");
}

#[test]
fn a_selection_with_no_branch_at_all_is_a_single_fixed_total() {
    let code = "    ld a,b\n    ld c,d\n    nop\n";
    let summary = range(code).unwrap();
    assert_eq!(summary.min, 3, "{summary:?}");
    assert_eq!(summary.max, 3, "{summary:?}");
    assert!(!summary.unbounded, "{summary:?}");
}

#[test]
fn an_empty_selection_is_a_zero_total() {
    let listing = parse_z80_str("").unwrap();
    let tokens: Vec<&LocatedToken> = listing.iter().collect();
    let summary = cost_range(&tokens, test_cost).unwrap();
    assert_eq!(summary, CostRange::default());
}

// ─── Following a `CALL` into its callee ──────────────────────────────
//
// "What does this cost" is not answerable without the routine it calls,
// and for a demo coder the routine is usually right there in the file.

/// The headline case: a callee inside the selection is priced.
///
/// `call helper` (5) + `ret` (3) reaches the exit; `helper` is `ld` (1) +
/// `ret` (3) = 4. So 5 + 4 + 3 = 12, where the old behaviour said 8 - it
/// charged the `call` and ignored what it called.
#[test]
fn a_call_is_priced_with_the_body_of_the_routine_it_calls() {
    let code = "\
call helper
ret
helper
ld a,b
ret
";
    let summary = range(code).unwrap();
    assert_eq!(summary.min, 12, "{summary:?}");
    assert_eq!(summary.max, 12, "{summary:?}");
    assert!(!summary.unbounded, "{summary:?}");
    assert!(!summary.incomplete, "{summary:?}");
    // Four instructions are in the slice and four are counted. The number
    // to watch is 6: the callee is priced by a *sub-query* over the same
    // tokens, and folding that query's own counts into this one would
    // count `helper`'s body twice over.
    assert_eq!(summary.instruction_count, 4, "{summary:?}");
}

/// A routine defined elsewhere keeps the old answer - the call
/// instruction's own cost - and says the total is a lower bound rather
/// than pretending it is complete.
#[test]
fn a_call_leaving_the_selection_is_flagged_incomplete() {
    let code = "    call somewhere_else\n    ret\n";
    let summary = range(code).unwrap();
    assert_eq!(summary.min, 8, "{summary:?}");
    assert_eq!(summary.max, 8, "{summary:?}");
    assert!(
        summary.incomplete,
        "an unpriceable callee must be visible to the caller: {summary:?}"
    );
}

/// A conditional call either happens or it does not, so it bounds the two
/// ends differently - exactly like a conditional branch.
///
/// Not taken: 2. Taken: 5 + the body's 4 = 9. Then `ret` (3) either way.
#[test]
fn a_conditional_call_bounds_min_without_the_body_and_max_with_it() {
    let code = "\
call nz, helper
ret
helper
ld a,b
ret
";
    let summary = range(code).unwrap();
    assert_eq!(summary.min, 2 + 3, "{summary:?}");
    assert_eq!(summary.max, 5 + 4 + 3, "{summary:?}");
    assert!(!summary.unbounded, "{summary:?}");
}

/// A routine that calls itself costs something that depends on how many
/// times it does - a runtime property. Saying "unbounded" is the honest
/// answer; the important part is that it terminates at all.
#[test]
fn a_recursive_call_is_unbounded_rather_than_a_hang() {
    let code = "\
call recurse
ret
recurse
call recurse
ret
";
    let summary = range(code).unwrap();
    assert!(
        summary.unbounded,
        "recursion has no static total: {summary:?}"
    );
}

/// Mutual recursion is the same property one step removed, and is what an
/// in-progress set catches that a simple "am I my own callee" check would
/// not.
#[test]
fn mutual_recursion_is_unbounded_rather_than_a_hang() {
    let code = "\
call ping
ret
ping
call pong
ret
pong
call ping
ret
";
    let summary = range(code).unwrap();
    assert!(summary.unbounded, "{summary:?}");
}

/// A callee that loops makes its caller's worst case unknowable too - the
/// property has to propagate outward, or a caller would report a
/// confident number for something that has none.
#[test]
fn a_looping_callee_makes_its_caller_unbounded() {
    let code = "\
call spin
ret
spin
ld b,4
.again
djnz .again
ret
";
    let summary = range(code).unwrap();
    assert!(
        summary.unbounded,
        "a loop inside the callee is still a loop: {summary:?}"
    );
}

/// Two calls to the same routine must not price it twice over - and, more
/// importantly, must not walk it twice. The memo is what keeps a selection
/// full of calls from becoming quadratic.
#[test]
fn the_same_routine_called_twice_is_priced_consistently() {
    let once = range("    call helper\n    ret\nhelper\n    ld a,b\n    ret\n").unwrap();
    let twice =
        range("    call helper\n    call helper\n    ret\nhelper\n    ld a,b\n    ret\n")
            .unwrap();
    assert_eq!(
        twice.min - once.min,
        9,
        "the second call costs the same as the first: {once:?} vs {twice:?}"
    );
}
