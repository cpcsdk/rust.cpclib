use cpclib_orgams_ascii::decoder2::parse_orgams_file;
use cpclib_common::winnow::stream::LocatingSlice;
use cpclib_common::winnow::Parser;

#[test]
fn debug_string_table() {
    let data = std::fs::read("tests/orgams-main/EXCEPT.O").unwrap();
    let input = LocatingSlice::new(data.as_slice());
    
    // We parse the whole file now, which includes the string table
    let program = parse_orgams_file.parse(input).unwrap();
    
    // In decoder2, the string table is called "labels" and is part of the Program struct
    println!("String table size: {}", program.labels.len());
    
    println!("\nAll entries:");
    for (idx, string) in program.labels.iter().enumerate() {
        println!("  0x{:02x} ({}): {:?}", idx, idx, string.as_str());
    }
}
