use cpclib_orgams_ascii::*;
use cpclib_orgams_ascii::decoder2;
use cpclib_common::winnow::LocatingSlice;

#[test]
fn test_except_reconstruction() {
    let data = std::fs::read("tests/orgams-main/EXCEPT.O").unwrap();
    let source = std::fs::read_to_string("tests/orgams-main/EXCEPT.Z80").unwrap();
    let source_lines: Vec<&str> = source.lines().collect();
    
    println!("\n{}", "=".repeat(70));
    println!("EXCEPT.O Reconstruction Using decoder2");
    println!("{}", "=".repeat(70));
    
    // Use decoder2
    let mut input = LocatingSlice::new(data.as_slice());
    let program = decoder2::parse_orgams_file(&mut input).expect("Failed to parse EXCEPT.O");

    println!("Source: {} lines", source_lines.len());
    // println!("Decoded: {} elements", elements.len());
    
    let reconstructed = program.to_string();
    let reconstructed_lines: Vec<&str> = reconstructed.lines().collect();
    
    println!("Reconstructed: {} lines", reconstructed_lines.len());
    
    println!("\n{}", "=".repeat(70));
    println!("Side-by-side comparison (first 30 lines):");
    println!("{}", "=".repeat(70));
    
    for i in 0..30.min(source_lines.len()).min(reconstructed_lines.len()) {
        let source_line = source_lines.get(i).unwrap_or(&"");
        let recon_line = reconstructed_lines.get(i).unwrap_or(&"");
        
        let match_marker = if source_line == recon_line { "✓" } else { "✗" };
        
        println!("{} Line {:2}:", match_marker, i + 1);
        println!("  SOURCE: {:?}", source_line);
        println!("  RECON:  {:?}", recon_line);
        if source_line != recon_line {
            println!("  DIFF: Expected vs Got");
        }
        println!();
    }
    
    // Statistics
    let mut matching = 0;
    let total = source_lines.len().max(reconstructed_lines.len());
    
    for i in 0..total {
        let src = source_lines.get(i).unwrap_or(&"");
        let rec = reconstructed_lines.get(i).unwrap_or(&"");
        if src == rec {
            matching += 1;
        }
    }
    
    println!("{}", "=".repeat(70));
    println!("RESULTS: {}/{} lines match ({:.1}%)", 
             matching, total, (matching as f64 / total as f64) * 100.0);
    println!("{}", "=".repeat(70));
    
    if matching < total * 8 / 10 {
        println!("\nFirst 30 lines of reconstructed output:");
        println!("{}", "=".repeat(70));
        for (i, line) in reconstructed_lines.iter().take(30).enumerate() {
            println!("{:2}: {}", i + 1, line);
        }
    }
}
