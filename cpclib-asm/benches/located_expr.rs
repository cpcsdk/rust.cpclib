use cpclib_asm::parser::ctx_and_span;
use cpclib_asm::parser::expression::located_expr;
use cpclib_common::winnow::Parser;
use criterion::{Criterion, BenchmarkId, black_box, criterion_group, criterion_main};

const EXPR: &str = "A+B*C-(D<<2)/E%F|G^H";

fn bench_located_expr(c: &mut Criterion) {
        let (_ctx, span) = ctx_and_span(EXPR);
    c.bench_with_input(BenchmarkId::new("located_expr", "complex"), &EXPR, |b, &expr| {
        b.iter(|| {
            let span: cpclib_asm::Z80Span = span.clone();
            let mut span: cpclib_asm::InnerZ80Span = span.into();
            let _ = located_expr.parse( span).unwrap();
        });
    });
}

criterion_group!(benches, bench_located_expr);
criterion_main!(benches);
