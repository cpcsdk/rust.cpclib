use cpclib_orgams_ascii::*;
use cpclib_orgams_ascii::decoder2;
use std::fs;
use cpclib_common::winnow::LocatingSlice;

#[test]
fn test_const_i_parsing() {
    println!("\n=== Testing CONST.I (assignments and comments only) ===\n");
    
    // Read the .I file
    let i_data = fs::read("tests/orgams-main/CONST.I").expect("Failed to read CONST.I");
    
    // Decode with decoder2
    let mut input = LocatingSlice::new(i_data.as_slice());
    let program = decoder2::parse_orgams_file(&mut input).expect("Failed to parse CONST.I");
    
    // Convert to text
    let reconstructed = program.to_string();
    
    // Read the expected .Z80 file
    let expected = fs::read_to_string("tests/orgams-main/CONST.Z80")
        .expect("Failed to read CONST.Z80");
    
    // Split into lines
    let reconstructed_lines: Vec<_> = reconstructed.lines().collect();
    let expected_lines: Vec<_> = expected.lines().collect();
    
    println!("Reconstructed {} lines from .I file", reconstructed_lines.len());
    println!("Expected {} lines from .Z80 file\n", expected_lines.len());
    
    // Count matches
    let mut exact_matches = 0;
    let total_lines = expected_lines.len().max(reconstructed_lines.len());
    
    // Display first 20 lines comparison
    println!("First 20 lines comparison:\n");
    for i in 0..20.min(total_lines) {
        let expected_line = expected_lines.get(i).unwrap_or(&"");
        let reconstructed_line = reconstructed_lines.get(i).unwrap_or(&"");
        
        let match_symbol = if expected_line == reconstructed_line {
            exact_matches += 1;
            "✓"
        } else {
            "✗"
        };
        
        println!("{} Line {}:", match_symbol, i + 1);
        println!("  EXPECTED: {:?}", expected_line);
        println!("  GOT:      {:?}", reconstructed_line);
        
        if expected_line != reconstructed_line {
            // Show character-by-character diff for mismatches
            println!("  DIFF:");
            let max_len = expected_line.len().max(reconstructed_line.len());
            for j in 0..max_len {
                let e = expected_line.chars().nth(j);
                let r = reconstructed_line.chars().nth(j);
                if e != r {
                    println!("    pos {}: expected {:?}, got {:?}", j, e, r);
                }
            }
        }
        println!();
    }
    
    // Show lines 21-30 if they exist
    if total_lines > 20 {
        println!("\nLines 21-30:\n");
        for i in 20..30.min(total_lines) {
            let expected_line = expected_lines.get(i).unwrap_or(&"");
            let reconstructed_line = reconstructed_lines.get(i).unwrap_or(&"");
            
            let match_symbol = if expected_line == reconstructed_line {
                exact_matches += 1;
                "✓"
            } else {
                "✗"
            };
            
            println!("{} Line {}: {:?} vs {:?}", match_symbol, i + 1, expected_line, reconstructed_line);
        }
    }
    
    // Count remaining matches
    for i in 30.min(total_lines)..total_lines {
        let expected_line = expected_lines.get(i).unwrap_or(&"");
        let reconstructed_line = reconstructed_lines.get(i).unwrap_or(&"");
        if expected_line == reconstructed_line {
            exact_matches += 1;
        }
    }
    
    let match_percentage = (exact_matches as f64 / total_lines as f64) * 100.0;
    
    println!("\n======================================================================");
    println!("RESULTS: {}/{} lines match ({:.1}%)", exact_matches, total_lines, match_percentage);
    println!("======================================================================");
    
    // Check for key assignments that should definitely be present
    println!("\nChecking key assignments:");
    
    let key_assignments = vec![
        ("max_sources", "64"),
        ("max_filename_len", "&50"),
        ("max_symbols", "&0600"),
        ("max_ram_pages", "10"),
        ("chunks_per_bk", "&40"),
        ("heap_chunk_start", "5"),
    ];
    
    for (var_name, expected_value) in key_assignments {
        let pattern = format!("{} = {}", var_name, expected_value);
        if reconstructed.contains(&pattern) {
            println!("  ✓ Found: {}", pattern);
        } else {
            println!("  ✗ Missing: {}", pattern);
            // Show what we have for this variable
            for line in reconstructed_lines.iter() {
                if line.contains(var_name) {
                    println!("    Got instead: {}", line);
                }
            }
        }
    }
    
    println!("\n=== Test complete ===\n");
    
    // For a file with only assignments and comments, we should get a high match rate
    assert!(match_percentage > 50.0, 
        "Match percentage too low: {:.1}%. Expected >50% for a simple file with only assignments and comments", 
        match_percentage);
}
