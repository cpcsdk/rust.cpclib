use std::fs;
use std::path::PathBuf;

fn main() {
    let i_file = PathBuf::from("tests/orgams-main/CONST.I");
    let content = fs::read(&i_file).expect("Failed to read I file");
    let decoder = cpclib_orgams_ascii::OrgamsDecoder::new(content);
    
    let table = decoder.string_table();
    
    // Check what's at index 0x87
    if let Some(s) = table.get(&0x87) {
        println!("String at 0x87: {:?}", s);
        println!("Length: {}", s.len());
        println!("Bytes: {:?}", s.as_bytes());
    } else {
        println!("No string at index 0x87");
    }
    
    // Show all entries around 0x87
    println!("\nStrings near 0x87:");
    for i in 0x80..=0x90 {
        if let Some(s) = table.get(&i) {
            println!("  0x{:02x}: {:?}", i, s);
        }
    }
}
