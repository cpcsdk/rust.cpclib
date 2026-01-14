//! Tests for analyzing Orgams file format structure

use std::fs;
use std::path::PathBuf;

/// Get test data directory
fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/orgams-main")
}

/// Find all pairs of .O and .Z80 files
fn find_file_pairs() -> Vec<(PathBuf, PathBuf)> {
    let mut pairs = Vec::new();
    
    fn scan_dir(dir: &std::path::Path, pairs: &mut Vec<(PathBuf, PathBuf)>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    scan_dir(&path, pairs);
                } else if let Some(ext) = path.extension() {
                    if ext == "O" {
                        let z80_path = path.with_extension("Z80");
                        if z80_path.exists() {
                            pairs.push((path.clone(), z80_path));
                        }
                    }
                }
            }
        }
    }
    
    scan_dir(&test_data_dir(), &mut pairs);
    pairs.sort();
    pairs
}

#[test]
fn test_find_pairs() {
    let pairs = find_file_pairs();
    println!("Found {} file pairs", pairs.len());
    assert!(pairs.len() > 0, "Should find at least one file pair");
    
    for (orgams, z80) in pairs.iter().take(5) {
        println!("  {} <-> {}", 
            orgams.file_name().unwrap().to_string_lossy(),
            z80.file_name().unwrap().to_string_lossy());
    }
}

#[test]
fn test_magic_bytes() {
    let pairs = find_file_pairs();
    assert!(pairs.len() > 0);
    
    for (orgams_path, _) in pairs.iter().take(10) {
        let data = fs::read(orgams_path).unwrap();
        assert!(data.len() >= 4, "File too small: {:?}", orgams_path);
        
        let magic = &data[0..4];
        assert_eq!(magic, b"ORGA", 
            "Wrong magic in {:?}: got {:?}", 
            orgams_path.file_name().unwrap(), 
            String::from_utf8_lossy(magic));
    }
}

#[test]
fn test_analyze_header_structure() {
    let pairs = find_file_pairs();
    let (orgams_path, _) = &pairs[0];
    
    let data = fs::read(orgams_path).unwrap();
    
    println!("\n=== Analyzing: {:?} ===", orgams_path.file_name().unwrap());
    println!("File size: {} bytes", data.len());
    println!("\nFirst 100 bytes (hex):");
    for (i, chunk) in data.iter().take(100).enumerate() {
        if i % 16 == 0 {
            print!("\n{:04x}  ", i);
        }
        print!("{:02x} ", chunk);
    }
    println!("\n");
    
    // Analyze header structure
    println!("Magic: {:?}", String::from_utf8_lossy(&data[0..4]));
    println!("Byte 4 (version?): 0x{:02x} ({})", data[4], data[4]);
    println!("Bytes 5-10: {:02x?}", &data[5..10]);
}

#[test]
fn test_compare_sizes() {
    let pairs = find_file_pairs();
    
    println!("\n=== File Size Comparison ===");
    for (orgams, z80) in pairs.iter().take(10) {
        let o_size = fs::metadata(orgams).unwrap().len();
        let z_size = fs::metadata(z80).unwrap().len();
        let ratio = o_size as f64 / z_size as f64;
        
        println!("{:20} .O: {:6} bytes  .Z80: {:6} bytes  ratio: {:.2}", 
            orgams.file_name().unwrap().to_string_lossy(),
            o_size, z_size, ratio);
    }
}

#[test]
fn test_find_patterns() {
    let pairs = find_file_pairs();
    let (orgams_path, z80_path) = &pairs[0];
    
    let o_data = fs::read(orgams_path).unwrap();
    let z_data = fs::read_to_string(z80_path).unwrap();
    
    println!("\n=== Pattern Analysis ===");
    println!("Looking for ASCII strings in binary file...\n");
    
    // Find ASCII strings in binary
    let mut in_string = false;
    let mut current_string = String::new();
    let mut string_start = 0;
    
    for (i, &byte) in o_data.iter().enumerate() {
        if byte >= 32 && byte <= 126 {
            if !in_string {
                in_string = true;
                string_start = i;
            }
            current_string.push(byte as char);
        } else {
            if in_string && current_string.len() > 3 {
                println!("  @0x{:04x}: {:?}", string_start, current_string);
                
                // Check if this string appears in Z80 file
                if z_data.contains(&current_string) {
                    println!("    ✓ Found in Z80 source");
                }
            }
            in_string = false;
            current_string.clear();
        }
    }
}

#[test]
fn test_analyze_section_markers() {
    let pairs = find_file_pairs();
    
    for (orgams_path, _) in pairs.iter().take(5) {
        let data = fs::read(orgams_path).unwrap();
        
        println!("\n=== Analyzing: {:?} ===", orgams_path.file_name().unwrap());
        
        // Look for "SRC" marker mentioned in hex dump
        for (i, window) in data.windows(3).enumerate() {
            if window == b"SRC" {
                println!("Found 'SRC' at offset 0x{:04x}", i);
                println!("  Context: {:02x?}", &data[i.saturating_sub(5)..i.saturating_add(20).min(data.len())]);
            }
        }
        
        // Look for common markers
        for marker in &[b"ORG", b"END", b"EQU", b"IMP"] {
            let count = data.windows(marker.len())
                .filter(|w| w == *marker)
                .count();
            if count > 0 {
                println!("Found '{}' {} times", String::from_utf8_lossy(*marker), count);
            }
        }
    }
}
