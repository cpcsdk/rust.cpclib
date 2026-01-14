use cpclib_basic::string_parser::parse_basic_line;
use cpclib_common::winnow::Parser;

#[test]
fn debug_string_parsing() {
    let line = "80 WHILE INKEY$=\"\"\n";
    
    match parse_basic_line.parse(&line) {
        Ok(parsed_line) => {
            println!("Parsed line: {:#?}", parsed_line);
            println!("Reconstructed: {}", parsed_line);
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}
