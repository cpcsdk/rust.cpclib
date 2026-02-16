use cpclib_basic::string_parser::parse_basic_line;
use cpclib_basic::tokens::{BasicFloat, BasicToken, BasicValue};

#[test]
fn test_str_float_parsing() {
    // Test direct TryFrom<&str>
    let f1 = BasicFloat::try_from("0.5").unwrap();
    println!("0.5 -> bytes: {:?}, value: {}", f1.as_bytes(), f1.to_f64());

    let f2 = BasicFloat::try_from("0.1").unwrap();
    println!("0.1 -> bytes: {:?}, value: {}", f2.as_bytes(), f2.to_f64());

    let f3 = BasicFloat::try_from("0.25").unwrap();
    println!("0.25 -> bytes: {:?}, value: {}", f3.as_bytes(), f3.to_f64());

    // Test parsing from BASIC line
    let input = "120 r=r+0.5\n";
    let parsed = parse_basic_line(&mut input.as_ref()).unwrap();
    println!("\nParsed line: {}", parsed);
}
