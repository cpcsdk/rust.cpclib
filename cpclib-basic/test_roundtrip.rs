use std::fs;

fn main() {
    // Read TEST-CAT.BAS
    let bytes = fs::read("tests/TEST-CAT.BAS").expect("Failed to read TEST-CAT.BAS");
    println!("Original file: {} bytes", bytes.len());
    println!("Last 5 bytes: {:?}", &bytes[bytes.len()-5..]);
    
    // Parse it
    match cpclib_basic::BasicProgram::decode(&bytes) {
        Ok(prog) => {
            println!("Parsed successfully: {} lines", prog.lines.len());
            
            // Convert back to bytes
            let new_bytes = prog.as_bytes();
            println!("Re-encoded: {} bytes", new_bytes.len());
            println!("Last 5 bytes: {:?}", &new_bytes[new_bytes.len()-5..]);
            
            // Compare
            if bytes == new_bytes {
                println!("✓ Perfect round-trip!");
            } else {
                println!("✗ Mismatch!");
                println!("  Original length: {}", bytes.len());
                println!("  New length: {}", new_bytes.len());
                println!("  Difference: {}", (new_bytes.len() as i32) - (bytes.len() as i32));
            }
        }
        Err(e) => {
            println!("Failed to parse: {:?}", e);
        }
    }
}
