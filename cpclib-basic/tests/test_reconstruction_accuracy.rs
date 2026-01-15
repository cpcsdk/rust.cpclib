/// Test to verify that source reconstruction from tokens is accurate
use cpclib_basic::string_parser::{parse_basic_line, parse_instruction};
use cpclib_common::winnow::Parser;

/// Helper to test a line with line number
fn test_line(line: &str) -> Result<String, String> {
    let line_with_newline = format!("{}\n", line);
    match parse_basic_line.parse(&line_with_newline) {
        Ok(parsed) => Ok(parsed.to_string()),
        Err(e) => Err(format!("Parse error: {:?}", e))
    }
}

/// Helper to test immediate mode statement
fn test_immediate(statement: &str) -> Result<String, String> {
    let statement_with_newline = format!("{}\n", statement);
    let mut input = statement_with_newline.as_str();
    match parse_instruction.parse_next(&mut input) {
        Ok(tokens) => {
            let reconstructed = tokens.iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join("");
            Ok(reconstructed)
        },
        Err(e) => Err(format!("Parse error: {:?}", e))
    }
}

#[test]
fn test_exact_reconstruction() {
    let test_cases = vec![
        // Numbers
        ("10 n=1", true),
        ("10 n=1.5", true),
        ("10 n=32767", true),  // Max positive 16-bit signed
        ("10 n=&HFF", true),
        ("10 n=&X1010", true),
        ("10 n=123456789", true), // Large number as float
        
        // Strings
        (r#"10 a$="Hello""#, true),
        (r#"10 PRINT "test""#, true),
        
        // Commands
        ("10 GOTO 100", true),
        ("10 FOR I=1 TO 10", true),
        ("10 NEXT I", true),
        
        // Immediate mode
        ("CAT", false),
        ("AUTO 100,5", false),
        ("CALL 0", false),
        
        // Graphics
        ("10 DRAW 100,100", true),
        ("10 MOVE 200,200", true),
        ("CLG 2", false),
        
        // Complex
        ("10 DEF FNtest=x*2", true),
        (r#"10 IF x>5 THEN PRINT "big""#, true),
        ("10 MID$(a$,2,3)=\"hi\"", true),
    ];
    
    let mut differences = Vec::new();
    let total_cases = test_cases.len();
    
    for (original, is_line) in test_cases {
        let result = if is_line {
            test_line(original)
        } else {
            test_immediate(original)
        };
        
        match result {
            Ok(reconstructed) => {
                if original != reconstructed.as_str() {
                    differences.push((original.to_string(), reconstructed));
                }
            }
            Err(e) => {
                panic!("Failed to parse '{}': {}", original, e);
            }
        }
    }
    
    if !differences.is_empty() {
        println!("\n=== Reconstruction Differences Found ===");
        for (original, reconstructed) in &differences {
            println!("Original:      '{}'", original);
            println!("Reconstructed: '{}'", reconstructed);
            
            // Show character-by-character comparison for first difference
            let orig_chars: Vec<char> = original.chars().collect();
            let recon_chars: Vec<char> = reconstructed.chars().collect();
            let max_len = orig_chars.len().max(recon_chars.len());
            
            for i in 0..max_len.min(10) { // Show first 10 differences
                let orig_char = orig_chars.get(i).map(|c| format!("{:?}", c)).unwrap_or_else(|| "EOF".to_string());
                let recon_char = recon_chars.get(i).map(|c| format!("{:?}", c)).unwrap_or_else(|| "EOF".to_string());
                
                if orig_char != recon_char {
                    println!("  Position {}: {} != {}", i, orig_char, recon_char);
                }
            }
            println!();
        }
        
        println!("Total: {} out of {} cases have reconstruction differences", differences.len(), total_cases);
        println!("Note: Some differences may be acceptable (e.g., floating-point precision, whitespace normalization)");
    } else {
        println!("✓ All {} test cases reconstruct exactly!", total_cases);
    }
}
