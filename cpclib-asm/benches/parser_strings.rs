#![feature(str_as_str)]

use cpclib_common::winnow::{BStr, LocatingSlice, Stateful, Parser};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use cpclib_asm::{InnerZ80Span, ParserContext, parser::parse_string};
use    cpclib_asm::parser::ctx_and_span;

fn bench_parse_strings(c: &mut Criterion) {

    let (ctx_simple, span_simple) = ctx_and_span(r#""HELLOHELLHELLOO""#);
    let (ctx_escaped, span_escaped) = ctx_and_span(r#""HE\"LL\nO\tHE\"LL\nO\tHE\"LL\nO\t""#);

    let mut group = c.benchmark_group("parser_strings");

    group.bench_function(BenchmarkId::new("simple", 0), |b| {
        b.iter(|| {
            let mut simple_copy: cpclib_asm::Z80Span = span_simple.clone();
            let mut simple_inner: InnerZ80Span = simple_copy.into();
            parse_string.parse(black_box(simple_inner)).unwrap();
        })
    });

    group.bench_function(BenchmarkId::new("escaped", 0), |b| {
        b.iter(|| {
            let mut escaped_copy: cpclib_asm::Z80Span = span_escaped.clone();
            let mut escaped_inner: InnerZ80Span = escaped_copy.into();
            parse_string.parse(black_box(escaped_inner)).unwrap();
        })
    });

    group.finish();
}

criterion_group!(parser_strings, bench_parse_strings);
criterion_main!(parser_strings);
