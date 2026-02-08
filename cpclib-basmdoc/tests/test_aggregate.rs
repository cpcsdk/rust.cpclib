use cpclib_asm::parse_z80_str;
use cpclib_basmdoc::{aggregate_documentation_on_tokens, build_documentation_page_from_aggregates, UndocumentedConfig};

const FILENAME: &str = "tests/simple_code.asm";

#[test]
fn test_simple_aggregate() {
    let code = fs_err::read_to_string(FILENAME).unwrap();
    let tokens = dbg!(parse_z80_str(&code).unwrap());
    let doc = dbg!(aggregate_documentation_on_tokens(&tokens, UndocumentedConfig::none()));

    assert_eq!(doc.len(), 6);

    assert_eq!(
        doc[0].0,
        "The aim of this file is to do stuffs.\nAnd this comment is a top file comment.\nThis is documentation item 1"
    );
    assert!(doc[0].1.is_none());

    assert_eq!(
        doc[1].0,
        "A raw label is considered to be a function.\nEven if there is no return close to it.\n\n- IN: A, HL\n- OUT: C\n- MOD: None\nThis is documentation item 2"
    );
    assert!(doc[1].1.is_some());

    let doc = build_documentation_page_from_aggregates(FILENAME, doc);

    assert!(!doc.to_markdown().is_empty());
}
