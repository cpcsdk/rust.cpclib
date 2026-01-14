//! Detailed byte-by-byte analysis to reverse engineer the format

use std::fs;
use std::path::PathBuf;

fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/orgams-main")
}

#[test]
fn test_analyze_byte_markers() {
    let path = test_data_dir().join("EXCEPT.O");
    let data = fs::read(&path).unwrap();
    
    println!("\n=== Analyzing byte markers before strings ===\n");
    
    let mut i = 0;
    while i < data.len() {
        let byte = data[i];
        
        // Look for special marker bytes followed by strings
        if byte == 0x43 || byte == 0x49 || byte == 0x4A || byte == 0x64 || byte == 0x41 {
            // Check if followed by ASCII
            if i + 1 < data.len() {
                let next = data[i + 1];
                if next >= 32 && next <= 126 {
                    // Find the string
                    let mut string = String::new();
                    let mut j = i + 1;
                    while j < data.len() && data[j] >= 32 && data[j] <= 126 {
                        string.push(data[j] as char);
                        j += 1;
                    }
                    
                    if string.len() > 2 {
                        println!("0x{:04x}: Marker 0x{:02x} ('{}') -> {:?}", 
                            i, byte, byte as char, &string[..string.len().min(40)]);
                    }
                }
            }
        }
        
        i += 1;
    }
}

#[test]
fn test_header_table_structure() {
    let path = test_data_dir().join("EXCEPT.O");
    let data = fs::read(&path).unwrap();
    
    println!("\n=== Analyzing Header Table ===\n");
    println!("Bytes 5-96 (before SRC marker):");
    
    // The data between ORGA header and SRC seems to be a table
    let table_start = 5;
    let table_end = data.windows(3)
        .position(|w| w == b"SRC")
        .unwrap_or(100);
    
    println!("Table range: 0x{:04x} to 0x{:04x} ({} bytes)", 
        table_start, table_end, table_end - table_start);
    
    let table = &data[table_start..table_end];
    
    // Try to interpret as 16-bit values
    println!("\nAs 16-bit little-endian values:");
    for (i, chunk) in table.chunks(2).enumerate() {
        if chunk.len() == 2 {
            let value = u16::from_le_bytes([chunk[0], chunk[1]]);
            print!("  [{:02}] 0x{:04x} ({:5})", i, value, value);
            if i % 4 == 3 {
                println!();
            }
        }
    }
    println!();
}

#[test]
fn test_compare_multiple_files() {
    let files = [
        ("EXCEPT.O", "EXCEPT.Z80"),
        ("bricbrac/AAP.O", "bricbrac/AAP.Z80"),
    ];
    
    for (o_file, z_file) in &files {
        let o_path = test_data_dir().join(o_file);
        let z_path = test_data_dir().join(z_file);
        
        let o_data = fs::read(&o_path).unwrap();
        let z_data = fs::read_to_string(&z_path).unwrap();
        
        println!("\n=== {} ===", o_file);
        println!("Binary size: {} bytes", o_data.len());
        println!("Source size: {} bytes", z_data.len());
        println!("Version byte: 0x{:02x}", o_data[4]);
        
        // Find SRC marker
        if let Some(src_pos) = o_data.windows(3).position(|w| w == b"SRC") {
            println!("SRC marker at: 0x{:04x}", src_pos);
            println!("Header size: {} bytes", src_pos);
        }
        
        // Count line markers in source
        let source_lines = z_data.lines().count();
        println!("Source lines: {}", source_lines);
    }
}

#[test]
fn test_byte_frequency() {
    let path = test_data_dir().join("EXCEPT.O");
    let data = fs::read(&path).unwrap();
    
    let mut freq = vec![0u32; 256];
    for &byte in &data[100..] {  // Skip header
        freq[byte as usize] += 1;
    }
    
    println!("\n=== Byte Frequency (top 20) ===\n");
    let mut pairs: Vec<_> = freq.iter().enumerate().collect();
    pairs.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    
    for (byte, count) in pairs.iter().take(20) {
        if *count > &0 {
            let ch = if *byte >= 32 && *byte <= 126 {
                format!("'{}'", *byte as u8 as char)
            } else {
                "   ".to_string()
            };
            println!("  0x{:02x} {} : {} times", byte, ch, count);
        }
    }
}
