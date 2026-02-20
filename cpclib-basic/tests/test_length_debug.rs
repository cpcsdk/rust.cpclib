use cpclib_basic::BasicProgram;

#[test]
fn test_length_calculation() {
    let code = r#"10 PRINT "HELLO";"#;

    // Parse
    let prog = BasicProgram::parse(code).unwrap();
    println!("Original code: '{}'", code);
    println!("Parsed program: {}", prog);

    // Get the bytes
    let bytes = prog.as_bytes();

    println!("\nGenerated {} bytes total:", bytes.len());
    println!("Bytes (decimal): {:?}", bytes);
    println!(
        "Bytes (hex): {}",
        bytes
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ")
    );

    // Decode the length fields
    let declared_length = bytes[0] as u16 + ((bytes[1] as u16) << 8);
    let line_number = bytes[2] as u16 + ((bytes[3] as u16) << 8);

    println!("\nDeclared length (bytes[0..2]): {}", declared_length);
    println!("Line number (bytes[2..4]): {}", line_number);
    println!("Actual data after line number: {} bytes", bytes.len() - 4);

    // Explanation of what should be in the line
    println!("\nExpected line content:");
    println!("  2 bytes: length");
    println!("  2 bytes: line number");
    println!("  1 byte: PRINT token (0xBF)");
    println!("  1 byte: space (0x20)");
    println!("  1 byte: opening quote (0x22)");
    println!("  5 bytes: 'HELLO' (48 45 4C 4C 4F)");
    println!("  1 byte: closing quote (0x22) <-- THE FIX");
    println!("  1 byte: semicolon (0x3B)");
    println!("  1 byte: end of line (0x00)");
    println!("  Total content: {} bytes", 1 + 1 + 1 + 5 + 1 + 1 + 1);
    println!(
        "  Plus length/line fields: {} bytes",
        2 + 2 + 1 + 1 + 1 + 5 + 1 + 1 + 1
    );

    // Try to decode
    match BasicProgram::decode(&bytes) {
        Ok(decoded) => {
            println!("\n✓ Successfully decoded!");
            println!("Decoded: {}", decoded);
            let bytes2 = decoded.as_bytes();
            println!("Re-encoded: {} bytes", bytes2.len());
            println!("Bytes match: {}", bytes == bytes2);
        },
        Err(e) => {
            println!("\n✗ Failed to decode: {:?}", e);
        }
    }
}

#[test]
fn test_no_bug_length() {
    let code = r#"10 PRINT " NO";"#;

    // Parse
    let prog = BasicProgram::parse(code).unwrap();
    println!("Original code: '{}'", code);

    // Get the bytes
    let bytes = prog.as_bytes();

    println!("\nGenerated {} bytes total:", bytes.len());
    println!(
        "Bytes (hex): {}",
        bytes
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ")
    );

    let declared_length = bytes[0] as u16 + ((bytes[1] as u16) << 8);
    println!("Declared length: {}", declared_length);
    println!("Actual total: {} bytes", bytes.len());

    // Try to decode
    match BasicProgram::decode(&bytes) {
        Ok(decoded) => {
            println!("✓ Decoded: {}", decoded);
        },
        Err(e) => {
            println!("✗ Failed: {:?}", e);
        }
    }
}
