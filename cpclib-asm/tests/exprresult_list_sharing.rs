//! `ExprResult::List`/`Matrix` now wrap their payload in `Arc<Vec<ExprResult>>`
//! instead of storing it inline (see `cpclib-tokens/src/tokens/expression.rs`) -
//! cloning a list value (which happens every time a symbol holding one is
//! referenced, since `resolve()` must return an owned `ExprResult`) is now an
//! atomic refcount bump instead of a deep copy. Mutation goes through
//! `Arc::make_mut`/`Arc::unwrap_or_clone`, which clones only when the value
//! is genuinely shared.
//!
//! These tests lock down the property that makes that safe: mutating a list
//! obtained from a symbol reference must never be visible through a
//! *different* reference to the same underlying value - the copy-on-write
//! must actually trigger when the `Arc` is shared, not just when it happens
//! to be convenient.

use cpclib_asm::assemble;

/// A list symbol referenced twice, with `list_push` applied to the first
/// reference - the second reference (and the symbol's own stored value)
/// must still see the original, unmodified list. If `Arc::make_mut`/
/// `unwrap_or_clone` were wired wrong (e.g. mutating in place because the
/// refcount check was skipped), this would silently corrupt the shared list
/// for every other reader instead of erroring - the most dangerous failure
/// mode for this kind of change.
#[test]
fn pushing_to_one_reference_does_not_mutate_the_shared_original() {
    let code = r#"
        org 0x8000
        BASE = [1, 2, 3]
        EXTENDED = list_push(BASE, 4)
        db list_len(BASE)
        db list_len(EXTENDED)
        db list_get(BASE, 0)
        db list_get(EXTENDED, 3)
    "#;
    let bytes = assemble(code).expect("list_push must not corrupt the original list");
    assert_eq!(bytes, vec![3, 4, 1, 4], "{bytes:?}");
}

/// Same property for `list_set` (in-place element mutation, not just
/// append): setting an element through one reference must not change what a
/// second reference to the same original list sees.
#[test]
fn setting_an_element_through_one_reference_does_not_mutate_the_shared_original() {
    let code = r#"
        org 0x8000
        BASE = [10, 20, 30]
        MODIFIED = list_set(BASE, 1, 99)
        db list_get(BASE, 1)
        db list_get(MODIFIED, 1)
    "#;
    let bytes = assemble(code).expect("list_set must not corrupt the original list");
    assert_eq!(bytes, vec![20, 99], "{bytes:?}");
}

/// `list_sort`/`list_reverse` (whole-list in-place algorithms) must be
/// equally safe: sorting a copy must not reorder the original.
#[test]
fn sorting_a_copy_does_not_reorder_the_shared_original() {
    let code = r#"
        org 0x8000
        BASE = [3, 1, 2]
        SORTED = list_sort(BASE)
        db list_get(BASE, 0)
        db list_get(SORTED, 0)
    "#;
    let bytes = assemble(code).expect("list_sort must not corrupt the original list");
    assert_eq!(bytes, vec![3, 1], "{bytes:?}");
}

/// A list referenced repeatedly (e.g. inside a `REPEAT`-unrolled read, the
/// exact pattern the `Arc` change targets) must still read the same,
/// correct values every time - the whole point of sharing the allocation is
/// that repeated reads see identical data without each becoming its own
/// deep copy that could theoretically drift.
#[test]
fn a_list_read_repeatedly_is_consistent_every_time() {
    let code = r#"
        org 0x8000
        TABLE = [7, 8, 9]
        repeat 3, i, 0
            db list_get(TABLE, {i})
        endrepeat
    "#;
    let bytes = assemble(code).expect("repeated reads of the same list must assemble");
    assert_eq!(bytes, vec![7, 8, 9], "{bytes:?}");
}

/// Nested lists (a list of lists, matching `Matrix`'s own row-of-lists
/// shape) - the recursive-clone problem specifically motivating this change -
/// must still read back correctly through shared references.
#[test]
fn nested_lists_read_correctly_through_shared_references() {
    let code = r#"
        org 0x8000
        NESTED = [[1, 2], [3, 4]]
        ALSO_NESTED = NESTED
        db list_get(list_get(NESTED, 0), 1)
        db list_get(list_get(ALSO_NESTED, 1), 0)
    "#;
    let bytes = assemble(code).expect("nested lists must assemble and read back correctly");
    assert_eq!(bytes, vec![2, 3], "{bytes:?}");
}
