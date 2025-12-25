use cpclib_asm::located_expr;
use cpclib_asm::parser::InnerZ80Span;
use cpclib_asm::parser::expression::{parse_factor, prefixed_label_expr};
use cpclib_asm::parser::obtained::LocatedExpr;
use cpclib_asm::parser::parser::ctx_and_span;
use cpclib_asm::parser::source::SourceString;

#[test]
fn test_located_expr_parses_prefixed_label() {
    // Example: {bank}mylabel
    let (_ctx, span) = ctx_and_span("{bank}mylabel");
    let mut input: InnerZ80Span = span.into();
    let parsed = prefixed_label_expr(&mut input).expect("Should parse prefixed label");
    match parsed {
        LocatedExpr::PrefixedLabel(prefix, label_span, _span) => {
            assert_eq!(prefix.to_string().to_lowercase(), "{bank}");
            assert_eq!(SourceString::as_str(&label_span), "mylabel");
        },
        _ => panic!("Did not parse as PrefixedLabel: {:?}", parsed)
    }
}

#[test]
fn test_located_expr_parses_prefixed_label_with_parse_factor() {
    // parse_factor should also handle prefixed labels
    let (_ctx, span) = ctx_and_span("{page}foo");
    let mut input: InnerZ80Span = span.into();
    let parsed = parse_factor(&mut input).expect("Should parse prefixed label via parse_factor");
    match parsed {
        LocatedExpr::PrefixedLabel(prefix, label_span, _span) => {
            assert_eq!(prefix.to_string().to_lowercase(), "{page}");
            assert_eq!(SourceString::as_str(&label_span), "foo");
        },
        _ => panic!("Did not parse as PrefixedLabel: {:?}", parsed)
    }
}
#[test]
fn test_prefixed_label_expr_direct() {
    // Systematically test all three prefixes
    let cases = [
        ("{bank}barbaz", "{bank}", "barbaz"),
        ("{page}barbaz", "{page}", "barbaz"),
        ("{pageset}barbaz", "{pageset}", "barbaz")
    ];
    for (input_str, expected_prefix, expected_label) in cases.iter() {
        let (_ctx, span) = ctx_and_span(input_str);
        let mut input: InnerZ80Span = span.into();
        let parsed = located_expr(&mut input).expect("Should parse prefixed label");
        match parsed {
            LocatedExpr::PrefixedLabel(prefix, label_span, _span) => {
                assert_eq!(prefix.to_string().to_lowercase(), *expected_prefix);
                assert_eq!(SourceString::as_str(&label_span), *expected_label);
            },
            _ => panic!("Did not parse as PrefixedLabel: {:?}", parsed)
        }
    }
}
