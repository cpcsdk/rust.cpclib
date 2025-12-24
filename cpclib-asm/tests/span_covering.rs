#![feature(box_patterns)]

use cpclib_asm::parser::expression::located_expr;
use cpclib_asm::parser::parser::ctx_and_span;
use cpclib_asm::{InnerZ80Span, MayHaveSpan};
use cpclib_common::winnow::stream::AsBStr;

#[test]
fn test_span_covering_basic() {
    // Parse the expression "A+B*C"
    let (_ctx, span) = ctx_and_span("A+B*C");
    let mut inner: InnerZ80Span = span.clone().into();
    let expr = located_expr(&mut inner).expect("parse");

    // Print the top-level span and sub-spans for debugging
    let top_span = expr.span();
    println!(
        "[span_covering] top_span.as_bstr() = {:?}",
        top_span.as_bstr()
    );
    println!(
        "[span_covering] top_span.offset_from_start() = {:?}",
        top_span.offset_from_start()
    );
    println!(
        "[span_covering] top_span.len = {:?}",
        top_span.as_bstr().len()
    );

    // The top-level should be a BinaryOperation(Add, A, B*C)
    if let cpclib_asm::parser::obtained::LocatedExpr::BinaryOperation(
        cpclib_tokens::BinaryOperation::Add,
        box left,
        box right,
        _
    ) = &expr
    {
        println!(
            "[span_covering] left.span().as_bstr() = {:?}",
            left.span().as_bstr()
        );
        println!(
            "[span_covering] right.span().as_bstr() = {:?}",
            right.span().as_bstr()
        );
        // Right should be BinaryOperation(Mul, B, C) covering "B*C"
        if let cpclib_asm::parser::obtained::LocatedExpr::BinaryOperation(
            cpclib_tokens::BinaryOperation::Mul,
            box b,
            box c,
            mul_span
        ) = right
        {
            println!(
                "[span_covering] mul_span.as_bstr() = {:?}",
                mul_span.as_bstr()
            );
            println!(
                "[span_covering] b.span().as_bstr() = {:?}",
                b.span().as_bstr()
            );
            println!(
                "[span_covering] c.span().as_bstr() = {:?}",
                c.span().as_bstr()
            );
        }
        else {
            println!("[span_covering] right is not a BinaryOperation(Mul, ...) as expected");
        }
    }
    else {
        println!("[span_covering] top-level is not a BinaryOperation(Add, ...) as expected");
    }
    // Keep the original assertions (may fail, but now we get debug output)
    assert_eq!(top_span.as_bstr(), b"A+B*C");
    assert_eq!(top_span.offset_from_start(), 0);
    assert_eq!(top_span.as_bstr().len(), "A+B*C".len());
}

#[test]
fn test_span_covering_single_var() {
    let (_ctx, mut span) = ctx_and_span("A");
    let expr = located_expr(&mut span).expect("parse");
    let top_span = expr.span();
    println!(
        "[test_span_covering_single_var] top_span.as_bstr() = {:?}",
        top_span.as_bstr()
    );
    assert_eq!(top_span.as_bstr(), b"A");
}

#[test]
fn test_span_covering_simple_add() {
    let (_ctx, mut span) = ctx_and_span("A+B");
    let expr = located_expr(&mut span).expect("parse");
    let top_span = expr.span();
    println!(
        "[test_span_covering_simple_add] top_span.as_bstr() = {:?}",
        top_span.as_bstr()
    );
    assert_eq!(top_span.as_bstr(), b"A+B");
}

// #[test]
// fn test_span_covering_nested() {
// let (_ctx, left) = ctx_and_span("A+B");
// let (_ctx, right) = ctx_and_span("C-D");
// let left_span: Z80Span = left.clone();
// let right_span: Z80Span = right.clone();
// let covering = build_span_covering(&left_span, &right_span);
// assert!(covering.offset_from_start() == left_span.offset_from_start());
// assert!(covering.offset_from_start() + covering.as_bstr().len() >= right_span.offset_from_start() + right_span.as_bstr().len());
// }

#[test]
fn test_span_covering_complex_expression() {
    let (_ctx, mut span) = ctx_and_span("A+B*C-D/E");
    let expr = located_expr(&mut span).expect("parse");
    let top_span = expr.span();
    assert_eq!(top_span.as_bstr(), b"A+B*C-D/E");
    // The top-level should be a BinaryOperation(Sub, left, right)
    if let cpclib_asm::parser::obtained::LocatedExpr::BinaryOperation(
        cpclib_tokens::BinaryOperation::Sub,
        box left,
        box right,
        sub_span
    ) = &expr
    {
        // left: should be BinaryOperation(Add, A, B*C)
        if let cpclib_asm::parser::obtained::LocatedExpr::BinaryOperation(
            cpclib_tokens::BinaryOperation::Add,
            box a,
            box b_mul_c,
            add_span
        ) = left
        {
            // a: should be variable A
            if let cpclib_asm::parser::obtained::LocatedExpr::Label(a_span) = a {
                assert_eq!(a_span.as_bstr(), b"A");
            }
            else {
                panic!("Expected leftmost to be label A");
            }
            // b_mul_c: should be BinaryOperation(Mul, B, C)
            if let cpclib_asm::parser::obtained::LocatedExpr::BinaryOperation(
                cpclib_tokens::BinaryOperation::Mul,
                box b,
                box c,
                mul_span
            ) = b_mul_c
            {
                if let cpclib_asm::parser::obtained::LocatedExpr::Label(b_span) = b {
                    assert_eq!(b_span.as_bstr(), b"B");
                }
                else {
                    panic!("Expected B in B*C");
                }
                if let cpclib_asm::parser::obtained::LocatedExpr::Label(c_span) = c {
                    assert_eq!(c_span.as_bstr(), b"C");
                }
                else {
                    panic!("Expected C in B*C");
                }
                assert_eq!(mul_span.as_bstr(), b"B*C");
            }
            else {
                panic!("Expected B*C as right of +");
            }
            assert_eq!(add_span.as_bstr(), b"A+B*C");
        }
        else {
            panic!("Expected left of - to be A+B*C");
        }
        // right: should be BinaryOperation(Div, D, E)
        if let cpclib_asm::parser::obtained::LocatedExpr::BinaryOperation(
            cpclib_tokens::BinaryOperation::Div,
            box d,
            box e,
            div_span
        ) = right
        {
            if let cpclib_asm::parser::obtained::LocatedExpr::Label(d_span) = d {
                assert_eq!(d_span.as_bstr(), b"D");
            }
            else {
                panic!("Expected D in D/E");
            }
            if let cpclib_asm::parser::obtained::LocatedExpr::Label(e_span) = e {
                assert_eq!(e_span.as_bstr(), b"E");
            }
            else {
                panic!("Expected E in D/E");
            }
            assert_eq!(div_span.as_bstr(), b"D/E");
        }
        else {
            panic!("Expected right of - to be D/E");
        }
        assert_eq!(sub_span.as_bstr(), b"A+B*C-D/E");
    }
    else {
        panic!("Top-level is not a BinaryOperation(Sub, ...) as expected");
    }
}

#[test]
fn test_span_covering_overlap() {
    // Expression with overlapping/adjacent tokens: "AB+BC"
    let (_ctx, mut span) = ctx_and_span("AB+BA");
    let expr = dbg!(located_expr(&mut span).expect("parse"));
    let top_span = expr.span();
    // The top-level should be a BinaryOperation(Add, AB, BC)
    if let cpclib_asm::parser::obtained::LocatedExpr::BinaryOperation(
        cpclib_tokens::BinaryOperation::Add,
        box left,
        box right,
        _
    ) = &expr
    {
        // Check left and right operand spans
        assert_eq!(left.span().as_bstr(), b"AB");
        assert_eq!(right.span().as_bstr(), b"BA");
        // The top span should cover the whole expression
        assert_eq!(top_span.as_bstr(), b"AB+BA");
    }
    else {
        panic!("Top-level is not a BinaryOperation(Add, ...) as expected");
    }
}
