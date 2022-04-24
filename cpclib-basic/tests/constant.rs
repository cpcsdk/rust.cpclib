use cpclib_basic::parser::parse_floating_point;

#[test]
pub fn parse_float() {
    cpclib_basic::parser::test_parse1(parse_floating_point, "123.456");
}
