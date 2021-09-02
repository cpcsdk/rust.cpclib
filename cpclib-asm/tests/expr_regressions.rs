use cpclib_asm::preamble::*;

#[test]
fn test_negative_expression() {
    let exp = Expr::Value(-18);
    let val = exp.eval().unwrap();

    assert_eq!(val.int(), -18);
}
