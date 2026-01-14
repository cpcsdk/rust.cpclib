/// Test decoder with smallest files first to build up understanding
use cpclib_orgams_ascii::{OrgamsFile, OrgamsDecoder, elements_to_z80_source};
use std::fs::File;

#[test]
fn test_macro_i() {
    // Smallest file: 164 bytes
    let file = File::open("tests/orgams-main/MACRO.I").unwrap();
    let orgams = OrgamsFile::read(file).unwrap();
    
    println!("\n=== MACRO.I Analysis ===");
    println!("Version: {}", orgams.header.version);
    println!("Content size: {} bytes", orgams.content.len());
    
    // Show raw content
    println!("\nRaw content (hex):");
    for (i, chunk) in orgams.content.chunks(16).enumerate() {
        print!("{:04x}: ", i * 16);
        for b in chunk {
            print!("{:02x} ", b);
        }
        print!("  ");
        for b in chunk {
            if (0x20..0x7f).contains(b) {
                print!("{}", *b as char);
            } else {
                print!(".");
            }
        }
        println!();
    }
    
    // Decode
    let mut decoder = OrgamsDecoder::new(orgams.content.clone());
    let elements = decoder.decode().unwrap();
    
    println!("\n=== Decoded {} elements ===", elements.len());
    for (i, elem) in elements.iter().enumerate() {
        println!("[{}] {:?}", i, elem);
    }
    
    // Convert to Z80
    let reconstructed = elements_to_z80_source(&elements);
    println!("\n=== Reconstructed source ===");
    println!("{}", reconstructed);
    
    // Compare with original
    let original = std::fs::read_to_string("tests/orgams-main/MACRO.Z80").unwrap();
    println!("\n=== Original source ===");
    println!("{}", original);
    
    println!("\n=== Comparison ===");
    println!("Original lines: {}", original.lines().count());
    println!("Reconstructed lines: {}", reconstructed.lines().count());
}

#[test]
fn test_ch_i() {
    // Second smallest: 599 bytes
    let file = File::open("tests/orgams-main/orgams/CH.I").unwrap();
    let orgams = OrgamsFile::read(file).unwrap();
    
    println!("\n=== CH.I Analysis ===");
    println!("Version: {}", orgams.header.version);
    println!("Content size: {} bytes", orgams.content.len());
    
    let mut decoder = OrgamsDecoder::new(orgams.content.clone());
    let elements = decoder.decode().unwrap();
    
    println!("Decoded {} elements", elements.len());
    
    let reconstructed = elements_to_z80_source(&elements);
    println!("Reconstructed {} lines", reconstructed.lines().count());
    
    // Show first 20 lines
    println!("\n=== First 20 lines ===");
    for (i, line) in reconstructed.lines().take(20).enumerate() {
        println!("{:3}: {}", i+1, line);
    }
}
