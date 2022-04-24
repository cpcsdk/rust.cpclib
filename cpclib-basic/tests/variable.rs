use cpclib_basic::parser::parse_variable;
use cpclib_basic::BasicLine;

pub fn test_parse(code: &str) -> BasicLine {
    cpclib_basic::parser::test_parse(parse_variable, code)
}

#[test]
fn variable_string_standard() {
    test_parse("hello$");
}

#[test]
fn variable_string_one_char() {
    test_parse("a$");
}
