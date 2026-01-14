use std::fs;
use std::path::PathBuf;

fn main() {
    let orgams_dir = PathBuf::from("tests/orgams-main");
    let i_file = orgams_dir.join("CONST.I");
    let z80_file = orgams_dir.join("CONST.Z80");
    
    // Read expected content
    let expected = fs::read_to_string(&z80_file).expect("Failed to read Z80 file");
    
    // Decode the .I file using the module
    let content = fs::read(&i_file).expect("Failed to read I file");
    let mut decoder = cpclib_orgams_ascii::OrgamsDecoder::new(content);
    let elements = decoder.decode().expect("Failed to decode");
    let reconstructed = cpclib_orgams_ascii::as_text(&elements);
    
    // Compare line by line
    let expected_lines: Vec<&str> = expected.lines().collect();
    let reconstructed_lines: Vec<&str> = reconstructed.lines().collect();
    
    let max_lines = expected_lines.len().max(reconstructed_lines.len());
    let mut mismatches = Vec::new();
    
    for i in 0..max_lines {
        let expected_line = expected_lines.get(i).copied().unwrap_or("");
        let reconstructed_line = reconstructed_lines.get(i).copied().unwrap_or("");
        
        if expected_line != reconstructed_line {
            mismatches.push((i + 1, expected_line, reconstructed_line));
        }
    }
    
    println!("CONST.I - {:.1}% match ({}/{} lines)\n", 
        (expected_lines.len() - mismatches.len()) as f64 / expected_lines.len() as f64 * 100.0,
        expected_lines.len() - mismatches.len(),
        expected_lines.len()
    );
    
    println!("Remaining errors (showing first 20):\n");
    for (i, (line_num, expected, got)) in mismatches.iter().enumerate() {
        if i >= 20 {
            println!("\n... and {} more errors", mismatches.len() - 20);
            break;
        }
        println!("{}. Line {}: Content differs", i + 1, line_num);
        println!("   Expected: {:?}", expected);
        println!("   Got:      {:?}\n", got);
    }
}
