use cpclib_orgams_ascii::decoder2;
use cpclib_common::winnow::LocatingSlice;

#[test]
fn debug_decode_elements() {
    let data = std::fs::read("tests/orgams-main/EXCEPT.O").unwrap();
    // let orgams_file = OrgamsFile::read(&data[..]).unwrap();
    
    let mut input = LocatingSlice::new(data.as_slice());
    let program = decoder2::parse_orgams_file(&mut input).unwrap();
    
    println!("First 10 decoded chunks:");
    for (i, chunk) in program.chunks.iter().take(10).enumerate() {
        println!("{:2}: {:?}", i + 1, chunk);
    }
}
