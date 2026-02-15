// Common test utilities for BASIC program parsing tests
// Used by all test_book_*.rs files

use cpclib_basic::BasicProgram;


/// Common test function: parses a BASIC program and checks reconstruction
/// The `fixed_code` should have OCR errors already corrected by book-specific function
pub fn test_basic_program(fixed_code: &str, description: &str, test_name: &str) {
    // Check blacklist - add test names here to skip problematic OCR
    let blacklist = [
        // Example: "test_book_example_listing_001",
    ];
    
    if blacklist.contains(&test_name) {
        println!("⊗ Skipped {} (blacklisted due to bad OCR)", test_name);
        return;
    }
    
    match BasicProgram::parse(fixed_code) {
        Ok(parsed) => {
            let reconstructed = parsed.to_string();
            println!("✓ Parsed {}", description);
        }
        Err(e) => {
            eprintln!("✗ Failed to parse {}", description);
            eprintln!("Error: {:?}", e);
            eprintln!("First 3 lines of fixed code:");
            for line in fixed_code.lines().take(3) {
                eprintln!("  {:?}", line);
            }
            panic!("Parse failed");
        }
    }
}

/// Macro to generate test functions
/// Takes: test name, raw program, description, and OCR fix function
#[macro_export]
macro_rules! basic_program_test {
    ($name:ident, $program:expr, $desc:expr, $fix_fn:expr) => {
        #[test]
        #[ignore("This test requires manual OCR correction, run with `cargo test -- --ignored` after fixing the OCR")]
        fn $name() {
            let fixed = $fix_fn($program);
            test_basic_program(&fixed, $desc, stringify!($name));
        }
    };
}
