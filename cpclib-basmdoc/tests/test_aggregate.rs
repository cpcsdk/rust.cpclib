use cpclib_asm::{Token, parse_z80_str};
use cpclib_basmdoc::aggregate_documentation_on_tokens;

#[test]
fn test_simple_aggregate() {
    let code = std::fs::read_to_string("tests/simple_code.asm").unwrap();
    let tokens = dbg!(parse_z80_str(&code).unwrap());
    let doc = dbg!(aggregate_documentation_on_tokens(&tokens));

    assert_eq!(doc.len(), 2);

    assert_eq!(
        doc[0].0,
        "The aim of this file is to do stuffs.\nAnd this comment is a top file comment."
    );
    assert!(doc[0].1.is_none());

    assert_eq!(
        doc[1].0,
        "A raw label is considered to be a function.\nEven if there is no return close to it"
    );
    assert!(doc[1].1.is_some());
}
