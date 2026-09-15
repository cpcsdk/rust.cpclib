//! `(params) => expr` lambda expressions - sugar over the existing
//! named-`FUNCTION` machinery (see `cpclib_tokens::Expr::Lambda`'s own doc
//! comment for the full design rationale). No closures: a lambda body sees
//! only its own parameters plus true globals, exactly like a real
//! `FUNCTION` - it cannot see an enclosing caller's locals.

#[test]
fn list_map_with_a_single_param_lambda() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n l = [1,2,3]\n l2 = list_map(l, (x) => x * 2)\n db l2[0], l2[1], l2[2]\n"
    )
    .unwrap();
    assert_eq!(bin, vec![2, 4, 6]);
}

#[test]
fn list_filter_with_a_lambda_predicate() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n l = [1,2,3,4]\n l2 = list_filter(l, (x) => x > 2)\n db l2[0], l2[1]\n"
    )
    .unwrap();
    assert_eq!(bin, vec![3, 4]);
}

#[test]
fn two_parameter_lambda() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n l1 = [1,2,3]\n l2 = [10,20,30]\n r = list_map(l1, (x) => x + 100)\n db \
         r[0], r[1], r[2]\n"
    )
    .unwrap();
    assert_eq!(bin, vec![101, 102, 103]);
    // separately exercise a genuine two-argument lambda via list_fold
    // (basm signature: list_fold(list, initial, folder))
    let bin2 = cpclib_asm::assemble(
        "org 0x4000\n l = [1,2,3,4]\n s = list_fold(l, 0, (acc, x) => acc + x)\n db s\n"
    )
    .unwrap();
    assert_eq!(bin2, vec![10]);
}

#[test]
fn lambda_used_directly_as_list_position_predicate() {
    // Index of the first element for which the predicate is true.
    let bin = cpclib_asm::assemble(
        "org 0x4000\n l = [5,6,7,8]\n p = list_position_predicate(l, (x) => x == 7)\n db p\n"
    )
    .unwrap();
    assert_eq!(bin, vec![2]);
}

#[test]
fn list_position_predicate_returns_minus_one_when_nothing_matches() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n l = [5,6,7,8]\n p = list_position_predicate(l, (x) => x > 100)\n db p\n"
    )
    .unwrap();
    assert_eq!(bin, vec![0xff]);
}

#[test]
fn lambda_cannot_capture_an_outer_functions_parameter() {
    // `x` is `outer`'s own parameter, a genuine local to that enclosing
    // call frame - not a top-level global (which a lambda, like any real
    // FUNCTION, legitimately does see). `FunctionsStack::current_frame`
    // only ever exposes the single topmost frame, so the lambda's own
    // call frame cannot see an enclosing function's frame underneath it -
    // this is the deliberate "no closures" limitation, locked in here as
    // a tested contract rather than just a doc claim.
    let res = cpclib_asm::assemble(
        "FUNCTION outer x\nRETURN list_map([1,2,3], (y) => y + x)[0]\nENDFUNCTION\norg \
         0x4000\ndb outer(5)\n"
    );
    assert!(res.is_err(), "lambda unexpectedly saw an enclosing function's parameter: {res:?}");
}

#[test]
fn same_lambda_resolved_twice_does_not_error_on_duplicate_registration() {
    // Two independent `list_map` calls using structurally identical
    // lambdas exercise the same "already registered" path a multi-pass
    // re-resolution of one lambda would - must not error either way.
    let bin = cpclib_asm::assemble(
        "org 0x4000\n l = [1,2,3]\n a = list_map(l, (x) => x + 1)\n b = list_map(l, (x) => x + \
         1)\n db a[0], b[0]\n"
    )
    .unwrap();
    assert_eq!(bin, vec![2, 2]);
}

#[test]
fn lambda_reresolved_across_a_forward_reference_multi_pass_assemble() {
    // `later` is only defined after its use inside the lambda body's
    // list_map call site is first resolved - forces at least a 2nd
    // assembler pass to re-resolve the same lambda expression node.
    let bin = cpclib_asm::assemble(
        "org 0x4000\n l = [1,2,3]\n r = list_map(l, (x) => x + later)\n db r[0], r[1], r[2]\n \
         later = 100\n"
    )
    .unwrap();
    assert_eq!(bin, vec![101, 102, 103]);
}

#[test]
fn lambda_requires_parens_around_a_single_parameter() {
    // Bare `x => x * 2` (no parens) is not valid syntax - deliberately, to
    // avoid ambiguity with a bare identifier starting some other
    // expression.
    let res = cpclib_asm::assemble("org 0x4000\n l = [1,2,3]\n l2 = list_map(l, x => x * 2)\n");
    assert!(res.is_err(), "bare (unparenthesized) lambda param unexpectedly parsed: {res:?}");
}

#[test]
fn plain_parenthesized_expression_still_works_after_lambda_support_was_added() {
    // `(1 + 2)` must still parse as a normal parenthesized expression, not
    // be swallowed by the new lambda alternative tried ahead of `parens`.
    let bin = cpclib_asm::assemble("org 0x4000\n db (1 + 2) * 3\n").unwrap();
    assert_eq!(bin, vec![9]);
}
