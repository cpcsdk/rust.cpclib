use cpclib_basic::string_parser::parse_floating_point;

#[test]
pub fn parse_float() {
    cpclib_basic::string_parser::test_parse1(parse_floating_point, "123.456");
}
