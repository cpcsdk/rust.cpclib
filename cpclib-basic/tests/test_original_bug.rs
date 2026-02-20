use cpclib_basic::BasicProgram;

#[test]
fn verify_original_bug_fixed() {
    // This is the exact case reported by the user
    let code = r#"10 PRINT " NO";"#;

    println!("Testing the originally reported bug case:");
    println!("Input code: {}", code);

    // Parse
    let prog = BasicProgram::parse(code).unwrap();
    println!("\nParsed: {}", prog);

    // Encode to bytes
    let bytes = prog.as_bytes();
    println!("\nEncoded to {} bytes", bytes.len());
    println!(
        "Hex: {}",
        bytes
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ")
    );

    // The bytes should include the closing quote (0x22) before the semicolon (0x3B)
    // Looking for pattern: ... 22 20 4E 4F 22 3B ... (opening quote, " NO", closing quote, semicolon)
    let has_closing_quote = bytes.windows(2).any(|w| w == [0x22, 0x3B]);
    println!(
        "\nClosing quote present before semicolon: {}",
        has_closing_quote
    );
    assert!(has_closing_quote, "FAIL: Closing quote is missing!");

    // Decode back
    let decoded = BasicProgram::decode(&bytes).unwrap();
    println!("\nDecoded back: {}", decoded);

    // The decoded version should match the original
    assert_eq!(prog.to_string().trim(), decoded.to_string().trim());

    // And it should still have the closing quote
    assert!(
        decoded.to_string().contains(r#"" NO""#),
        "Decoded version missing closing quote!"
    );

    println!(
        "\n✓ Original bug is FIXED! The closing quote is preserved through encode/decode cycle."
    );
}
