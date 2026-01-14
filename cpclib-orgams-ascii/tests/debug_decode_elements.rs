use cpclib_orgams_ascii::*;

#[test]
fn debug_decode_elements() {
    let data = std::fs::read("tests/orgams-main/EXCEPT.O").unwrap();
    let orgams_file = OrgamsFile::read(&data[..]).unwrap();
    
    let mut decoder = OrgamsDecoder::new(orgams_file.content.clone());
    let elements = decoder.decode().unwrap();
    
    println!("First 10 decoded elements:");
    for (i, elem) in elements.iter().take(10).enumerate() {
        println!("{:2}: {:?}", i + 1, elem);
    }
}
