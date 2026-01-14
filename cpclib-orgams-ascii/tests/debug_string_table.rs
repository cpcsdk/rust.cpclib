use cpclib_orgams_ascii::*;

#[test]
fn debug_string_table() {
    let data = std::fs::read("tests/orgams-main/EXCEPT.O").unwrap();
    let orgams_file = OrgamsFile::read(&data[..]).unwrap();
    
    let decoder = OrgamsDecoder::new(orgams_file.content.clone());
    
    println!("String table size: {}", decoder.string_table().len());
    
    println!("\nAll entries:");
    let mut entries: Vec<_> = decoder.string_table().iter().collect();
    entries.sort_by_key(|(k, _)| *k);
    
    for (key, value) in entries.iter() {
        println!("  0x{:02x} ({}): {:?}", key, key, value);
    }
}
