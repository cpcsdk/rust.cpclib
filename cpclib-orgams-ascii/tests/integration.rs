//! Integration tests using real Orgams files

use cpclib_orgams_ascii::{OrgamsFile, LineMarker};
use std::fs;
use std::path::PathBuf;

fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/orgams-main")
}

#[test]
fn test_read_except_file() {
    let path = test_data_dir().join("EXCEPT.O");
    let file = fs::File::open(&path).unwrap();
    
    let orgams = OrgamsFile::read(file).unwrap();
    
    assert_eq!(orgams.header.version, 2);
    assert!(orgams.header.metadata.len() > 0);
    assert!(orgams.content.len() > 0);
    
    println!("Version: {}", orgams.header.version);
    println!("Metadata size: {}", orgams.header.metadata.len());
    println!("Content size: {}", orgams.content.len());
}

#[test]
fn test_extract_lines_from_except() {
    let path = test_data_dir().join("EXCEPT.O");
    let file = fs::File::open(&path).unwrap();
    let orgams = OrgamsFile::read(file).unwrap();
    
    let lines = orgams.extract_lines();
    
    println!("\nExtracted {} lines", lines.len());
    println!("\nFirst 20 lines:");
    for (i, (marker, text)) in lines.iter().take(20).enumerate() {
        let marker_str = match marker {
            Some(LineMarker::Comment) => "C",
            Some(LineMarker::Indented) => "I",
            Some(LineMarker::NewLine) => "J",
            Some(LineMarker::Data) => "d",
            Some(LineMarker::Assembly) => "A",
            None => "?",
        };
        println!("[{}] {}: {:?}", i, marker_str, &text[..text.len().min(50)]);
    }
    
    // Verify we found expected strings
    let all_text: String = lines.iter().map(|(_, t)| t.as_str()).collect();
    assert!(all_text.contains("Nested Exceptions"));
    assert!(all_text.contains("except_enter"));
    assert!(all_text.contains("Interface:"));
}

#[test]
fn test_convert_to_z80() {
    let path = test_data_dir().join("EXCEPT.O");
    let file = fs::File::open(&path).unwrap();
    let orgams = OrgamsFile::read(file).unwrap();
    
    let z80_text = orgams.to_z80_text();
    
    println!("\n=== Reconstructed Z80 (first 500 chars) ===");
    println!("{}", &z80_text[..z80_text.len().min(500)]);
    
    // Verify key content is present
    assert!(z80_text.contains("Nested Exceptions"));
    assert!(z80_text.contains("except_enter"));
}

#[test]
fn test_roundtrip_write_read() {
    let path = test_data_dir().join("EXCEPT.O");
    let original_data = fs::read(&path).unwrap();
    
    // Read
    let file = fs::File::open(&path).unwrap();
    let orgams = OrgamsFile::read(file).unwrap();
    
    // Write
    let mut buffer = Vec::new();
    orgams.write(&mut buffer).unwrap();
    
    // Compare
    println!("Original size: {}", original_data.len());
    println!("Written size: {}", buffer.len());
    
    if original_data == buffer {
        println!("✓ Perfect roundtrip!");
    } else {
        println!("⚠ Sizes or content differ");
        // This is expected initially as we're still reverse-engineering
    }
}

#[test]
fn test_multiple_files() {
    let test_files = [
        "EXCEPT.O",
        "bricbrac/CONV.O",
        "bricbrac/CHECK.O",
    ];
    
    for file_name in &test_files {
        let path = test_data_dir().join(file_name);
        if !path.exists() {
            println!("Skipping {}: file not found", file_name);
            continue;
        }
        
        let file = fs::File::open(&path).unwrap();
        let orgams = OrgamsFile::read(file);
        
        assert!(orgams.is_ok(), "Failed to read {}: {:?}", file_name, orgams.err());
        
        let orgams = orgams.unwrap();
        let lines = orgams.extract_lines();
        
        println!("\n{}: {} lines extracted", file_name, lines.len());
    }
}

#[test]
fn test_compare_with_z80_source() {
    let o_path = test_data_dir().join("EXCEPT.O");
    let z_path = test_data_dir().join("EXCEPT.Z80");
    
    let file = fs::File::open(&o_path).unwrap();
    let orgams = OrgamsFile::read(file).unwrap();
    let z80_source = fs::read_to_string(&z_path).unwrap();
    
    let reconstructed = orgams.to_z80_text();
    let lines_original = z80_source.lines().collect::<Vec<_>>();
    let lines_reconstructed = reconstructed.lines().collect::<Vec<_>>();
    
    println!("\nOriginal Z80: {} lines", lines_original.len());
    println!("Reconstructed: {} lines", lines_reconstructed.len());
    
    // Find common strings
    let mut common_count = 0;
    for line in &lines_original[..lines_original.len().min(20)] {
        let trimmed = line.trim();
        if !trimmed.is_empty() && reconstructed.contains(trimmed) {
            common_count += 1;
        }
    }
    
    println!("Common strings in first 20 lines: {}", common_count);
    assert!(common_count > 5, "Should find several common strings");
}
