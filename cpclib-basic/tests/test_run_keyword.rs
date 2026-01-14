use cpclib_basic::string_parser::parse_basic_line;
use cpclib_common::winnow::Parser;

#[test]
fn debug_run_keyword() {
    let line = "1380 IF UPPER$(a$)=\"Y\" THEN RUN ELSE GOTO 3480\n";
    
    match parse_basic_line.parse(&line) {
        Ok(parsed_line) => {
            println!("Parsed successfully");
            println!("Reconstructed: {}", parsed_line);
            println!("\nTokens around RUN:");
            for (i, token) in parsed_line.tokens().iter().enumerate() {
                println!("  {}: {:?}", i, token);
            }
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}
