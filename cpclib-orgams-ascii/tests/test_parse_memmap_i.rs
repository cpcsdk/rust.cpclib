use cpclib_orgams_ascii::decoder2::parse_orgams_file;
use cpclib_common::winnow::stream::LocatingSlice;
use cpclib_common::winnow::Parser;
use std::fs;
use std::path::Path;

#[test]
fn test_parse_memmap_i() {
    let input_path = "tests/orgams-main/MEMMAP.I";
    let expected_path = "tests/orgams-main/MEMMAP.Z80";

    let data = fs::read(input_path).unwrap_or_else(|_| panic!("Failed to read {}", input_path));
    let input = LocatingSlice::new(data.as_slice());

    let program = parse_orgams_file.parse(input).unwrap();
    let reconstruction = program.to_string();

    // Optional: write reconstruction to see diff
    // fs::write("tests/orgams-main/MEMMAP.reconstructed.Z80", &reconstruction).unwrap();

    let expected = fs::read_to_string(expected_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", expected_path));

    // Simple normalization to avoid line ending issues
    let reconstruction_lines: Vec<&str> = reconstruction.lines().collect();
    let expected_lines: Vec<&str> = expected.lines().collect();
    
    // Using a diff check or just assert equality
    // assert_eq!(reconstruction, expected);
    
    if reconstruction != expected {
        println!("Reconstruction differs!");
        // Print first few differences
        for (i, (rec, exp)) in reconstruction_lines.iter().zip(expected_lines.iter()).enumerate() {
            if rec != exp {
                println!("Line {}: Expected '{}', Got '{}'", i + 1, exp, rec);
            }
        }
        if reconstruction_lines.len() != expected_lines.len() {
             println!("Length mismatch: Expected {}, Got {}", expected_lines.len(), reconstruction_lines.len());
        }
        panic!("Reconstruction failed");
    }
}
