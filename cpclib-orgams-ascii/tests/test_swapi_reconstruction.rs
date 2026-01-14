/// Detailed reconstruction test for SWAPI.I
/// Goal: Parse the binary and reconstruct the exact Z80 source
use cpclib_orgams_ascii::{OrgamsFile, OrgamsDecoder};
use std::fs::File;

#[test]
fn test_swapi_full_reconstruction() {
    // Read binary
    let file = File::open("tests/orgams-main/SWAPI.I").unwrap();
    let orgams = OrgamsFile::read(file).unwrap();
    
    // Read source
    let source = std::fs::read_to_string("tests/orgams-main/SWAPI.Z80").unwrap();
    let source_lines: Vec<&str> = source.lines().collect();
    
    println!("\n{}", "=".repeat(70));
    println!("SWAPI.I Full Reconstruction");
    println!("{}", "=".repeat(70));
    println!("Binary: {} bytes content, {} bytes table", 
             orgams.content.len(), orgams.string_table.len());
    println!("Source: {} lines", source_lines.len());
    
    // Decode
    let mut decoder = OrgamsDecoder::new(orgams.content.clone());
    let elements = decoder.decode().unwrap();
    
    println!("\nDecoded {} elements", elements.len());
    
    // Show string table
    println!("\nString table ({} entries):", orgams.string_table.len());
    for (i, s) in orgams.string_table.iter().enumerate() {
        if !s.is_empty() {
            println!("  [{}]: '{}'", i, s);
        }
    }
    
    // Manually parse byte-by-byte for detailed analysis
    println!("\n{}", "=".repeat(70));
    println!("Byte-by-byte parsing:");
    println!("{}", "=".repeat(70));
    
    let content = &orgams.content;
    let mut pos = 0;
    let mut line_num = 1;
    let mut current_line = String::new();
    
    while pos < content.len() && line_num <= 30 {
        let byte = content[pos];
        
        match byte {
            // Newline
            0x4A => {
                if !current_line.is_empty() || line_num > 1 {
                    println!("{:2}: {}", line_num, current_line);
                    if line_num <= source_lines.len() {
                        let expected = source_lines[line_num - 1];
                        if current_line.trim() != expected.trim() {
                            println!("    EXPECTED: {}", expected);
                        }
                    }
                    line_num += 1;
                    current_line.clear();
                }
                pos += 1;
            }
            
            // Comment
            0x43 => {
                if pos + 1 < content.len() {
                    let len = content[pos + 1] as usize;
                    if pos + 2 + len <= content.len() {
                        let text = String::from_utf8_lossy(&content[pos + 2..pos + 2 + len]);
                        current_line.push_str(&format!("; {}", text));
                        pos += 2 + len;
                    } else {
                        pos += 1;
                    }
                } else {
                    pos += 1;
                }
            }
            
            // Indented/Tab
            0x49 => {
                if pos + 1 < content.len() {
                    let len = content[pos + 1] as usize;
                    if pos + 2 + len <= content.len() {
                        let text = String::from_utf8_lossy(&content[pos + 2..pos + 2 + len]);
                        // Indented content
                        if !current_line.is_empty() {
                            current_line.push(' ');
                        } else {
                            current_line.push_str("      "); // typical indent
                        }
                        current_line.push_str(&text);
                        pos += 2 + len;
                    } else {
                        pos += 1;
                    }
                } else {
                    pos += 1;
                }
            }
            
            // Space count
            0x6D => {
                if pos + 1 < content.len() {
                    let count = content[pos + 1] as usize;
                    current_line.push_str(&" ".repeat(count));
                    pos += 2;
                } else {
                    pos += 1;
                }
            }
            
            // Command
            0x7F => {
                if pos + 1 < content.len() {
                    let cmd = content[pos + 1];
                    match cmd {
                        0x17 => { // IMPORT
                            current_line.push_str("IMPORT ");
                            pos += 2;
                            // Skip complex import parsing for now
                            // Just find the string
                            while pos < content.len() && content[pos] != 0x4A {
                                if content[pos] >= 0x20 && content[pos] < 0x7F {
                                    current_line.push(content[pos] as char);
                                }
                                pos += 1;
                            }
                        }
                        0x01 => { // ORG
                            current_line.push_str("ORG ");
                            pos += 2;
                        }
                        0x09 => { // IF
                            current_line.push_str("IF ");
                            pos += 2;
                        }
                        0x0A => { // ELSE
                            current_line.push_str("ELSE");
                            pos += 2;
                        }
                        0x0C => { // END
                            current_line.push_str("END");
                            pos += 2;
                        }
                        _ => {
                            pos += 2;
                        }
                    }
                } else {
                    pos += 1;
                }
            }
            
            // String table reference
            b if b >= 0xC0 => {
                // Look up in table (but we don't have easy access here)
                current_line.push_str(&format!("[${:02X}]", b));
                pos += 1;
            }
            
            // Regular ASCII
            b if b >= 0x20 && b < 0x7F => {
                current_line.push(b as char);
                pos += 1;
            }
            
            // Other
            _ => {
                pos += 1;
            }
        }
    }
    
    // Print final line if any
    if !current_line.is_empty() {
        println!("{:2}: {}", line_num, current_line);
    }
    
    println!("\n{}", "=".repeat(70));
    println!("Comparison with source (first 30 lines):");
    println!("{}", "=".repeat(70));
    for (i, line) in source_lines.iter().take(30).enumerate() {
        println!("{:2}: {}", i + 1, line);
    }
}
