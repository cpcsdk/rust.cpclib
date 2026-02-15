use cpclib_basic::BasicLine;
use cpclib_basic::string_parser::parse_print;

pub fn test_parse(code: &str) -> BasicLine {
    cpclib_basic::string_parser::test_parse(parse_print, code)
}

#[test]
fn print_empty() {
    test_parse("PRINT");
}

#[test]
fn print_string() {
    test_parse("PRINT \"HELLO\"");
}

#[test]
fn print_tab() {
    test_parse("PRINT TAB(5) \"HELLO\"");
}

#[test]
fn print_spc() {
    test_parse("PRINT SPC(5) \"HELLO\"");
}

#[test]
fn print_using_string() {
    test_parse("PRINT using \"####.###\";a$");
}

#[test]
fn print_using_alpha() {
    test_parse("PRINT using a$;12345.6789");
}

#[test]

fn print_string_fail() {
    test_parse("PRINT \"HELLO");
}
