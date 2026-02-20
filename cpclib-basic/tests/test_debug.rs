use cpclib_basic::BasicProgram;

#[test]
fn debug_print_encoding() {
    let code = "10 PRINT \"HELLO\"";
    let prog = BasicProgram::parse(code).expect("Failed to parse");

    let bytes = prog.as_bytes();

    // Print the bytes for debugging
    eprintln!("\n=== Debug output ===");
    eprintln!("Input: '{}'", code);
    eprintln!("Bytes ({} total): {:?}", bytes.len(), bytes);
    eprintln!(
        "Hex  : {}",
        bytes
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ")
    );
    eprintln!("==================\n");

    // Try to decode
    match BasicProgram::decode(&bytes) {
        Ok(decoded) => {
            eprintln!("✓ Decoded successfully!");
            eprintln!("Output: '{}'", decoded);
        },
        Err(e) => {
            eprintln!("✗ Decode failed: {:?}", e);
        }
    }
}

#[test]
fn debug_print_no_bug() {
    let code = "10 PRINT \" NO\";";
    let prog = BasicProgram::parse(code).expect("Failed to parse");

    let bytes = prog.as_bytes();

    eprintln!("\n=== Debug output ===");
    eprintln!("Input: '{}'", code);
    eprintln!("Bytes ({} total): {:?}", bytes.len(), bytes);
    eprintln!(
        "Hex  : {}",
        bytes
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ")
    );
    eprintln!("==================\n");

    match BasicProgram::decode(&bytes) {
        Ok(decoded) => {
            eprintln!("✓ Decoded successfully!");
            eprintln!("Output: '{}'", decoded);
        },
        Err(e) => {
            eprintln!("✗ Decode failed: {:?}", e);
        }
    }
}
