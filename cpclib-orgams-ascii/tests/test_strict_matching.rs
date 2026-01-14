/// Test that .I/.O files decode to match their corresponding .Z80 files exactly
use cpclib_orgams_ascii::*;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn test_all_i_files_strict_match() {
    let orgams_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/orgams-main");
    
    // List of .I files with their corresponding .Z80 files
    let test_files = vec![
        "CONST.I",
        "MACRO.I", 
        "MEMMAP.I",
        "SWAPI.I",
    ];
    
    let mut failed_files = Vec::new();
    let mut passed_files = Vec::new();
    
    for filename in test_files {
        let i_file = orgams_dir.join(filename);
        let z80_file = orgams_dir.join(filename.replace(".I", ".Z80"));
        
        if !i_file.exists() {
            eprintln!("⚠ Warning: {} not found", i_file.display());
            continue;
        }
        
        if !z80_file.exists() {
            eprintln!("⚠ Warning: {} not found", z80_file.display());
            continue;
        }
        
        println!("\n{}", "=".repeat(60));
        println!("Testing: {}", filename);
        println!("{}", "=".repeat(60));
        
        // Decode the .I file
        let reconstructed = match decode_orgams_file(&i_file) {
            Ok(content) => {
                // Debug MACRO.I specifically
                if filename == "MACRO.I" {
                    eprintln!("DEBUG MACRO.I decoded content:");
                    eprintln!("{:?}", content);
                    eprintln!("Lines: {:?}", content.lines().collect::<Vec<_>>());
                }
                content
            }
            Err(e) => {
                eprintln!("✗ Failed to decode {}: {}", filename, e);
                failed_files.push((filename.to_string(), format!("Decode error: {}", e)));
                continue;
            }
        };
        
        // Read expected .Z80 content
        let expected = fs::read_to_string(&z80_file)
            .expect(&format!("Failed to read {}", z80_file.display()));
        
        // Compare line by line
        let result = compare_line_by_line(filename, &reconstructed, &expected);
        
        if result.is_match {
            println!("✓ {} matches perfectly ({} lines)", filename, result.total_lines);
            passed_files.push(filename.to_string());
        } else {
            println!("✗ {} does not match strictly", filename);
            println!("  Matched: {}/{} lines ({:.1}%)", 
                result.matched_lines, result.total_lines,
                result.match_percentage);
            
            if !result.differences.is_empty() {
                println!("\n  First 10 differences:");
                for (i, diff) in result.differences.iter().take(10).enumerate() {
                    println!("    {}. Line {}: {}", i + 1, diff.line_num, diff.message);
                    println!("       Expected: {:?}", diff.expected);
                    println!("       Got:      {:?}", diff.got);
                }
            }
            
            failed_files.push((filename.to_string(), format!(
                "{:.1}% match ({}/{} lines)",
                result.match_percentage, result.matched_lines, result.total_lines
            )));
        }
    }
    
    // Summary
    println!("\n{}", "=".repeat(60));
    println!("SUMMARY");
    println!("{}", "=".repeat(60));
    println!("Passed: {}", passed_files.len());
    for f in &passed_files {
        println!("  ✓ {}", f);
    }
    
    if !failed_files.is_empty() {
        println!("\nFailed: {}", failed_files.len());
        for (f, reason) in &failed_files {
            println!("  ✗ {} - {}", f, reason);
        }
        
        panic!("\n{} file(s) did not match strictly. See details above.", failed_files.len());
    }
}

#[derive(Debug)]
struct ComparisonResult {
    is_match: bool,
    total_lines: usize,
    matched_lines: usize,
    match_percentage: f64,
    differences: Vec<LineDifference>,
}

#[derive(Debug)]
struct LineDifference {
    line_num: usize,
    expected: String,
    got: String,
    message: String,
}

fn compare_line_by_line(_filename: &str, reconstructed: &str, expected: &str) -> ComparisonResult {
    let reconstructed_lines: Vec<&str> = reconstructed.lines().collect();
    let expected_lines: Vec<&str> = expected.lines().collect();
    
    let total_lines = expected_lines.len();
    let mut matched_lines = 0;
    let mut differences = Vec::new();
    
    let max_lines = total_lines.max(reconstructed_lines.len());
    
    for i in 0..max_lines {
        let line_num = i + 1;
        
        let expected_line = expected_lines.get(i).map(|s| *s).unwrap_or("");
        let reconstructed_line = reconstructed_lines.get(i).map(|s| *s).unwrap_or("");
        
        if expected_line == reconstructed_line {
            matched_lines += 1;
        } else {
            differences.push(LineDifference {
                line_num,
                expected: expected_line.to_string(),
                got: reconstructed_line.to_string(),
                message: if expected_lines.get(i).is_none() {
                    "Extra line in reconstruction".to_string()
                } else if reconstructed_lines.get(i).is_none() {
                    "Missing line in reconstruction".to_string()
                } else {
                    "Content differs".to_string()
                },
            });
        }
    }
    
    let match_percentage = if total_lines > 0 {
        (matched_lines as f64 / total_lines as f64) * 100.0
    } else {
        0.0
    };
    
    ComparisonResult {
        is_match: matched_lines == total_lines && reconstructed_lines.len() == expected_lines.len(),
        total_lines,
        matched_lines,
        match_percentage,
        differences,
    }
}

fn decode_orgams_file(path: &Path) -> Result<String, String> {
    let data = fs::read(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let orgams_file = OrgamsFile::read(&data[..])
        .map_err(|e| format!("Failed to parse: {:?}", e))?;
    
    // Decode
    let mut decoder = OrgamsDecoder::new(orgams_file.content.clone());
    let decoded_elements = decoder.decode()
        .map_err(|e| format!("Failed to decode: {:?}", e))?;
    
    // Convert to text - concatenate without adding newlines (0x4a marker handles those)
    let reconstructed = decoded_elements.iter()
        .filter_map(|elem| match elem {
            DecodedElement::Text(s) => {
                // Only include non-empty text
                if s.is_empty() {
                    None
                } else {
                    Some(s.as_str())
                }
            },
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");  // No separator - elements already contain proper formatting
    
    Ok(reconstructed)
}
