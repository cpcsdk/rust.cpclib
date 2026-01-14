use cpclib_orgams_ascii::*;
use std::fs;

#[test]
fn test_except_reconstruction() {
    let data = std::fs::read("tests/orgams-main/EXCEPT.O").unwrap();
    let source = std::fs::read_to_string("tests/orgams-main/EXCEPT.Z80").unwrap();
    let source_lines: Vec<&str> = source.lines().collect();
    
    println!("\n{}", "=".repeat(70));
    println!("EXCEPT.O Reconstruction Using OrgamsDecoder");
    println!("{}", "=".repeat(70));
    
    // First parse with OrgamsFile to skip the header
    let orgams_file = OrgamsFile::read(&data[..]).unwrap();
    
    // Now use the updated OrgamsDecoder on the content only
    let mut decoder = OrgamsDecoder::new(orgams_file.content.clone());
    let elements = decoder.decode().unwrap();
    
    println!("Source: {} lines", source_lines.len());
    println!("Decoded: {} elements", elements.len());
    
    // Reconstruct using elements_to_z80_source
    let reconstructed = elements_to_z80_source(&elements);
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
    let mut total = source_lines.len().max(reconstructed_lines.len());
    
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
