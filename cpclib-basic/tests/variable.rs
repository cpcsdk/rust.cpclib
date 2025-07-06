use cpclib_basic::BasicLine;
use cpclib_basic::string_parser::parse_variable;

pub fn test_parse(code: &str) -> BasicLine {
    cpclib_basic::string_parser::test_parse(parse_variable, code)
}

#[test]
fn variable_string_standard() {
    test_parse("hello$");
}

#[test]
fn variable_string_one_char() {
    test_parse("a$");
}
