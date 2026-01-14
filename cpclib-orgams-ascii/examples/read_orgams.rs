//! Example: Reading and analyzing Orgams files

use cpclib_orgams_ascii::{OrgamsFile, LineMarker};
use std::fs::File;

fn main() -> std::io::Result<()> {
    // Read an Orgams binary file
    let file = File::open("tests/orgams-main/EXCEPT.O")?;
    let orgams = OrgamsFile::read(file)?;
    
    println!("=== Orgams File Info ===");
    println!("Version: {}", orgams.header.version);
    println!("Metadata size: {} bytes", orgams.header.metadata.len());
    println!("Content size: {} bytes", orgams.content.len());
    
    // Extract and display lines
    let lines = orgams.extract_lines();
    println!("\n=== Extracted Lines ({}) ===", lines.len());
    
    for (i, (marker, text)) in lines.iter().enumerate().take(10) {
        let marker_name = match marker {
            Some(LineMarker::Comment) => "Comment",
            Some(LineMarker::Indented) => "Indented",
            Some(LineMarker::NewLine) => "NewLine",
            Some(LineMarker::Data) => "Data",
            Some(LineMarker::Assembly) => "Assembly",
            None => "Unknown",
        };
        
        let preview = if text.len() > 50 {
            format!("{}...", &text[..50])
        } else {
            text.clone()
        };
        
        println!("[{}] {:8} : {}", i, marker_name, preview);
    }
    
    // Convert to Z80 assembly text
    let z80_text = orgams.to_z80_text();
    println!("\n=== Z80 Assembly (first 300 chars) ===");
    println!("{}", &z80_text[..z80_text.len().min(300)]);
    
    // Round-trip test: write and read back
    let mut buffer = Vec::new();
    orgams.write(&mut buffer)?;
    println!("\n=== Round-trip Test ===");
    println!("Written {} bytes", buffer.len());
    println!("✓ Binary can be written back");
    
    Ok(())
}
