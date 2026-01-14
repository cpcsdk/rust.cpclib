use cpclib_orgams_ascii::*;

#[test]
fn debug_data_section() {
    let data = std::fs::read("tests/orgams-main/EXCEPT.O").unwrap();
    let orgams_file = OrgamsFile::read(&data[..]).unwrap();
    
    let content = &orgams_file.content;
    
    println!("Content size: {}", content.len());
    println!("\nFirst bytes:");
    for i in (0..min(120, content.len())).step_by(16) {
        print!("{:04x}: ", i);
        for j in 0..16 {
            if i + j < content.len() {
                print!("{:02x} ", content[i + j]);
            } else {
                print!("   ");
            }
        }
        print!(" ");
        for j in 0..16 {
            if i + j < content.len() {
                let byte = content[i + j];
                if byte >= 32 && byte < 127 {
                    print!("{}", byte as char);
                } else {
                    print!(".");
                }
            }
        }
        println!();
    }
    
    // If first byte is 0x64 and second is 0x70, then we have 112 bytes of data
    if content.len() > 2 && content[0] == 0x64 && content[1] == 0x70 {
        println!("\n\nData section detected: 0x64 0x70 = 112 bytes");
        println!("After 112 bytes (at offset 114):");
        let offset = 114; // 2 for marker+length + 112 for data
        if offset < content.len() {
            print!("{:04x}: ", offset);
            for j in 0..16 {
                if offset + j < content.len() {
                    print!("{:02x} ", content[offset + j]);
                }
            }
            println!();
        }
    }
}

fn min(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}
