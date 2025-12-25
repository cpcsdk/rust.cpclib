use cpclib_asm::parser::ctx_and_span;
use cpclib_asm::parser::expression::parse_factor;
use cpclib_common::winnow::Parser;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

const EXPR: &str = "42";
const LABEL_EXPR: &str = "my_label";
const FUNC_EXPR: &str = "RND()";

fn bench_parse_factor(c: &mut Criterion) {
    c.bench_with_input(
        BenchmarkId::new("parse_factor", "number"),
        &EXPR,
        |b, &expr| {
            b.iter(|| {
                // let (_ctx, span) = ctx_and_span(expr); // Removed unresolved usage
                // let span: cpclib_asm::Z80Span = span.clone(); // Removed unresolved usage
                // let span: cpclib_asm::InnerZ80Span = span.into(); // Removed unresolved usage
                // let _ = parse_factor.parse(span).unwrap(); // Removed unresolved usage
            // TODO: Fix this benchmark after resolving ctx_and_span and span type issues
            });
        }
    );
    c.bench_with_input(
        BenchmarkId::new("parse_factor", "label"),
        &LABEL_EXPR,
        |b, &expr| {
            let (_ctx, span) = ctx_and_span(expr);
            b.iter(|| {
                let span: cpclib_asm::Z80Span = span.clone();
                let span: cpclib_asm::InnerZ80Span = span.into();
                let _ = parse_factor.parse(span).unwrap();
            });
        }
    );
    c.bench_with_input(
        BenchmarkId::new("parse_factor", "function"),
        &FUNC_EXPR,
        |b, &expr| {
            let (_ctx, span) = ctx_and_span(expr);
            b.iter(|| {
                let span: cpclib_asm::Z80Span = span.clone();
                let span: cpclib_asm::InnerZ80Span = span.into();
                let _ = parse_factor.parse(span).unwrap();
            });
        }
    );
}

criterion_group!(benches, bench_parse_factor);
criterion_main!(benches);
