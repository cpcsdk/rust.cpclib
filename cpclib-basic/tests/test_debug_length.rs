use cpclib_basic::BasicProgram;
use cpclib_basic::tokens::{BasicToken, BasicTokenNoPrefix};

#[test]
fn debug_length_issue() {
    let code = r#"10 PRINT ",";"#;

    // First: parse from text
    let prog1 = BasicProgram::parse(code).unwrap();
    println!("=== First encoding (from text) ===");
    println!("Code: '{}'", code);
    println!("Parsed program: {}", prog1);

    let line1 = prog1.lines()[0].clone();
    println!("Line has {} tokens", line1.tokens().len());
    for (i, token) in line1.tokens().iter().enumerate() {
        println!("  Token {}: {:?}", i, token);
    }

    // Check if last token is EndOfTokenisedLine
    let has_end_marker = line1.tokens().last()
        == Some(&BasicToken::SimpleToken(
            BasicTokenNoPrefix::EndOfTokenisedLine
        ));
    println!("Has EndOfTokenisedLine: {}", has_end_marker);
    println!("tokens_bytes_length: {}", line1.tokens_bytes_length());
    println!("complete_bytes_length: {}", line1.complete_bytes_length());

    let bytes1 = prog1.as_bytes();
    println!("Encoded to {} bytes: {:?}", bytes1.len(), bytes1);
    println!(
        "Declared length: {}",
        bytes1[0] as u16 + ((bytes1[1] as u16) << 8)
    );

    // Second: decode and re-encode
    println!("\n=== Second encoding (after decode) ===");
    let prog2 = BasicProgram::decode(&bytes1).unwrap();
    println!("Decoded program: {}", prog2);

    let line2 = prog2.lines()[0].clone();
    println!("Line has {} tokens", line2.tokens().len());
    for (i, token) in line2.tokens().iter().enumerate() {
        println!("  Token {}: {:?}", i, token);
    }

    let has_end_marker2 = line2.tokens().last()
        == Some(&BasicToken::SimpleToken(
            BasicTokenNoPrefix::EndOfTokenisedLine
        ));
    println!("Has EndOfTokenisedLine: {}", has_end_marker2);
    println!("tokens_bytes_length: {}", line2.tokens_bytes_length());
    println!("complete_bytes_length: {}", line2.complete_bytes_length());

    let bytes2 = prog2.as_bytes();
    println!("Encoded to {} bytes: {:?}", bytes2.len(), bytes2);
    println!(
        "Declared length: {}",
        bytes2[0] as u16 + ((bytes2[1] as u16) << 8)
    );

    println!("\n=== Comparison ===");
    println!("Bytes match: {}", bytes1 == bytes2);
    if bytes1 != bytes2 {
        println!("Length difference: {} vs {}", bytes1[0], bytes2[0]);
    }
}
