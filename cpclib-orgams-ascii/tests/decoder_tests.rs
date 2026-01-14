/// Test the decoder module
use cpclib_orgams_ascii::{OrgamsFile, OrgamsDecoder, elements_to_z80_source};
use std::fs::File;

#[test]
fn test_decode_except() {
    let file = File::open("tests/orgams-main/EXCEPT.O").unwrap();
    let orgams = OrgamsFile::read(file).unwrap();
    
    // Create decoder and decode
    let mut decoder = OrgamsDecoder::new(orgams.content.clone());
    let elements = decoder.decode().unwrap();
    
    println!("\n=== Decoded {} elements ===", elements.len());
    for (i, elem) in elements.iter().take(20).enumerate() {
        println!("[{}] {:?}", i, elem);
    }
    
    // Convert to Z80 source
    let source = elements_to_z80_source(&elements);
    println!("\n=== First 800 chars of reconstructed source ===");
    println!("{}", &source[..source.len().min(800)]);
    
    // Check for expected content
    assert!(source.contains("Nested Exceptions"));
    assert!(source.contains("IF ") || source.contains("IMPORT"));
}

#[test]
fn test_decode_and_compare() {
    let file = File::open("tests/orgams-main/EXCEPT.O").unwrap();
    let orgams = OrgamsFile::read(file).unwrap();
    
    let mut decoder = OrgamsDecoder::new(orgams.content.clone());
    let elements = decoder.decode().unwrap();
    let reconstructed = elements_to_z80_source(&elements);
    
    // Read original Z80 source
    let original = std::fs::read_to_string("tests/orgams-main/EXCEPT.Z80").unwrap();
    
    println!("\n=== Comparison ===");
    println!("Original lines: {}", original.lines().count());
    println!("Reconstructed lines: {}", reconstructed.lines().count());
    
    // Check that we have reasonable content
    assert!(reconstructed.len() > 100, "Reconstructed source too short");
    assert!(elements.len() > 20, "Too few decoded elements");
}
