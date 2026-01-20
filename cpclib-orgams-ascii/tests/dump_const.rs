use cpclib_orgams_ascii::decoder2;
use cpclib_common::winnow::LocatingSlice;
use std::fs;

#[test]
fn dump_const_i() {
    // Read the .I file
    let i_data = fs::read("tests/orgams-main/CONST.I").expect("Failed to read CONST.I");
    // let orgams_file = OrgamsFile::read(&i_data[..]).expect("Failed to parse CONST.I");
    
    // Decode
    let mut input = LocatingSlice::new(i_data.as_slice());
    let program = decoder2::parse_orgams_file(&mut input).expect("Failed to decode CONST.I");
    
    // Convert to text
    let reconstructed = program.to_string();
    
    // Write to file
    fs::write("/tmp/const_reconstructed.txt", &reconstructed).expect("Failed to write output");
    
    println!("\nReconstructed output written to /tmp/const_reconstructed.txt");
    println!("Total {} lines\n", reconstructed.lines().count());
    
    // Show first 50 lines
    println!("First 50 lines:");
    println!("================");
    for (i, line) in reconstructed.lines().take(50).enumerate() {
        println!("{:3}: {}", i+1, line);
    }
}
