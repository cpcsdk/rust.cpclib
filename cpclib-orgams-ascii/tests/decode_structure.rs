/// Deep analysis of Orgams encoding to understand the command structure
use std::fs::File;
use std::io::Read;
use std::collections::HashMap;

#[test]
fn analyze_command_codes() {
    let file = File::open("tests/orgams-main/EXCEPT.O").unwrap();
    let mut data = Vec::new();
    file.take(100_000).read_to_end(&mut data).unwrap();
    
    // Find SRC section
    let src_pos = data.windows(3).position(|w| w == b"SRC").unwrap();
    let content = &data[src_pos + 3..];
    
    println!("\n=== Analyzing 0x7f command codes ===");
    let mut commands = HashMap::new();
    let mut pos = 0;
    
    while pos < content.len() {
        if content[pos] == 0x7f {
            if pos + 1 < content.len() {
                let cmd = content[pos + 1];
                *commands.entry(cmd).or_insert(0) += 1;
                
                // Show first occurrence of each command
                if commands[&cmd] == 1 {
                    print!("\n0x7f 0x{:02x}: ", cmd);
                    for i in 2..8 {
                        if pos + i < content.len() {
                            let b = content[pos + i];
                            if (0x20..0x7f).contains(&b) {
                                print!("{:02x}('{}') ", b, b as char);
                            } else {
                                print!("{:02x} ", b);
                            }
                        }
                    }
                }
            }
        }
        pos += 1;
    }
    
    println!("\n\n=== Command frequency ===");
    let mut sorted: Vec<_> = commands.iter().collect();
    sorted.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for (cmd, count) in sorted {
        println!("0x{:02x}: {} occurrences", cmd, count);
    }
}

#[test]
fn compare_with_source_structure() {
    // Read Z80 source
    let z80_source = std::fs::read_to_string("tests/orgams-main/EXCEPT.Z80").unwrap();
    
    println!("\n=== Z80 Source Structure ===");
    for (i, line) in z80_source.lines().take(30).enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("IF ") || trimmed.starts_with("ELSE") || trimmed.starts_with("END") {
            println!("Line {}: {}", i + 1, trimmed);
        } else if trimmed.starts_with("IMPORT ") || trimmed.starts_with("INCLUDE ") {
            println!("Line {}: {}", i + 1, trimmed);
        } else if trimmed.starts_with("ORG ") {
            println!("Line {}: {}", i + 1, trimmed);
        }
    }
}

#[test]
fn analyze_data_sections() {
    let file = File::open("tests/orgams-main/EXCEPT.O").unwrap();
    let mut data = Vec::new();
    file.take(100_000).read_to_end(&mut data).unwrap();
    
    let src_pos = data.windows(3).position(|w| w == b"SRC").unwrap();
    let content = &data[src_pos + 3..];
    
    println!("\n=== First 5 Data sections ===");
    let mut pos = 0;
    let mut data_count = 0;
    
    while pos < content.len() && data_count < 5 {
        if content[pos] == 0x64 {  // Data marker
            pos += 1;
            if pos >= content.len() {
                break;
            }
            
            let length = content[pos] as usize;
            pos += 1;
            
            data_count += 1;
            println!("\n[Data #{}] Length: {} bytes", data_count, length);
            
            // Parse content
            let end_pos = (pos + length).min(content.len());
            let mut inner_pos = pos;
            
            while inner_pos < end_pos {
                let b = content[inner_pos];
                
                if b == 0x7f && inner_pos + 1 < end_pos {
                    let cmd = content[inner_pos + 1];
                    print!("\n  [0x7f cmd=0x{:02x}]", cmd);
                    inner_pos += 2;
                    continue;
                }
                
                // Check for nested markers
                if matches!(b, 0x43 | 0x49 | 0x4a | 0x41) && inner_pos + 1 < end_pos {
                    let marker_name = match b {
                        0x43 => "Comment",
                        0x49 => "Indented",
                        0x4a => "NewLine",
                        0x41 => "Assembly",
                        _ => "Unknown"
                    };
                    let str_len = content[inner_pos + 1] as usize;
                    print!("\n  [{}({}):", marker_name, str_len);
                    
                    let str_end = (inner_pos + 2 + str_len).min(end_pos);
                    for i in (inner_pos + 2)..str_end {
                        if let Some(c) = content.get(i) {
                            if (0x20..0x7f).contains(c) {
                                print!("{}", *c as char);
                            }
                        }
                    }
                    print!("]");
                    inner_pos = str_end;
                    continue;
                }
                
                // Regular byte
                if (0x20..0x7f).contains(&b) {
                    print!("{}", b as char);
                } else {
                    print!("[{:02x}]", b);
                }
                inner_pos += 1;
            }
            
            pos = end_pos;
        } else {
            pos += 1;
        }
    }
    println!();
}
