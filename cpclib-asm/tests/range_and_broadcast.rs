//! Range expressions (`a..b`/`a..=b`) and scalar/list broadcasting - a
//! backport from the Baron 6502 assembler, adapted to this codebase's own
//! conventions (Rust's own range semantics, no dedicated stepped-range
//! syntax, `Range` a genuine runtime type rather than eagerly expanding
//! into a `List`).

#[test]
fn range_exclusive_emits_the_right_bytes() {
    let bin = cpclib_asm::assemble("org 0x4000\n db 0..5\n").unwrap();
    assert_eq!(bin, vec![0, 1, 2, 3, 4]);
}

#[test]
fn range_inclusive_emits_the_right_bytes() {
    let bin = cpclib_asm::assemble("org 0x4000\n db 0..=5\n").unwrap();
    assert_eq!(bin, vec![0, 1, 2, 3, 4, 5]);
}

#[test]
fn descending_range_emits_zero_bytes_both_forms() {
    // Matches Rust exactly - no auto-descending.
    let bin = cpclib_asm::assemble("org 0x4000\n db 5..1\n ret\n").unwrap();
    assert_eq!(bin, vec![0xc9]);
    let bin = cpclib_asm::assemble("org 0x4000\n db 5..=1\n ret\n").unwrap();
    assert_eq!(bin, vec![0xc9]);
}

#[test]
fn float_range_bound_is_an_error_not_a_silent_truncation() {
    assert!(cpclib_asm::assemble("org 0x4000\n db 1.5..5\n").is_err());
}

#[test]
fn range_bounds_can_reference_forward_labels() {
    let bin = cpclib_asm::assemble("org 0x4000\n db 0..count\ncount equ 3\n").unwrap();
    assert_eq!(bin, vec![0, 1, 2]);
}

#[test]
fn iterate_over_a_range_works_with_no_directive_changes() {
    let bin = cpclib_asm::assemble("org 0x4000\n iterate i in 0..4\n db {i}\n endi\n").unwrap();
    assert_eq!(bin, vec![0, 1, 2, 3]);
}

#[test]
fn range_has_the_same_low_precedence_as_rusts_own_range_operator() {
    // `1 + 0..5` parses as `(1 + 0)..5`, exactly like Rust's own `1 +
    // 0..5` - the range check only ever runs once, after the full
    // arithmetic chain has consumed as much as it can.
    let bin = cpclib_asm::assemble("org 0x4000\n db 1 + 0..5\n").unwrap();
    assert_eq!(bin, vec![1, 2, 3, 4]);
}

#[test]
fn parenthesized_range_broadcasts_with_arithmetic() {
    let bin = cpclib_asm::assemble("org 0x4000\n db (0..3) * 2\n").unwrap();
    assert_eq!(bin, vec![0, 2, 4]);
}

#[test]
fn broadcast_add_list_scalar_both_orders() {
    let bin = cpclib_asm::assemble("org 0x4000\n db [1,2,3] + 10\n").unwrap();
    assert_eq!(bin, vec![11, 12, 13]);
    let bin = cpclib_asm::assemble("org 0x4000\n db 10 + [1,2,3]\n").unwrap();
    assert_eq!(bin, vec![11, 12, 13]);
}

#[test]
fn broadcast_bitwise_and_list_scalar() {
    let bin = cpclib_asm::assemble("org 0x4000\n db [6,5] & 3\n").unwrap();
    assert_eq!(bin, vec![2, 1]);
}

#[test]
fn broadcast_comparison_produces_a_list_of_bool() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n assert list_get([1,2,3] < 2, 0) = true\n assert list_get([1,2,3] < \
         2, 1) = false\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn mismatched_length_list_broadcast_still_errors() {
    assert!(cpclib_asm::assemble("org 0x4000\n db [1,2] + [1,2,3]\n").is_err());
}

#[test]
fn a_range_used_as_an_if_condition_errors_exactly_like_a_plain_list() {
    // No new list/range-to-bool coercion introduced by this feature - both
    // must fail the same way.
    let range_err = cpclib_asm::assemble("org 0x4000\n if 0..5\n db 1\n endif\n")
        .unwrap_err()
        .to_string();
    let list_err = cpclib_asm::assemble("org 0x4000\n if [1,2,3]\n db 1\n endif\n")
        .unwrap_err()
        .to_string();
    // Both fail via the same `.bool()` conversion error shape - not
    // asserting on exact text (which mentions the operand itself and so
    // differs), just that both are genuine errors, not silently accepted.
    assert!(!range_err.is_empty());
    assert!(!list_err.is_empty());
}

#[test]
fn range_step_by_steps_through_iterate() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n iterate i in range_step_by(0..10, 2)\n db {i}\n endi\n"
    )
    .unwrap();
    assert_eq!(bin, vec![0, 2, 4, 6, 8]);
}

#[test]
fn list_sublist_with_a_range_selector_gathers_the_right_elements() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n db list_sublist([10,20,30,40,50], 1..3)\n"
    )
    .unwrap();
    assert_eq!(bin, vec![20, 30]);
}

#[test]
fn list_sublist_three_argument_form_still_works_unchanged() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n db list_sublist([10,20,30,40,50], 1, 3)\n"
    )
    .unwrap();
    assert_eq!(bin, vec![20, 30]);
}

#[test]
fn list_len_and_list_get_on_a_range_are_o1_no_materialization_needed_to_be_correct() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n assert list_len(0..1000000) = 1000000\n assert list_get(0..1000000, \
         500000) = 500000\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}
