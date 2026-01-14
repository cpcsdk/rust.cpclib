use cpclib_orgams_ascii::*;

#[test]
fn debug_orgamsfile_content() {
    let data = std::fs::read("tests/orgams-main/EXCEPT.O").unwrap();
    let orgams_file = OrgamsFile::read(&data[..]).unwrap();
    
    println!("Content size: {}", orgams_file.content.len());
    println!("First 50 bytes of content:");
    for (i, chunk) in orgams_file.content.chunks(16).take(4).enumerate() {
        print!("{:04x}: ", i * 16);
        for b in chunk {
            print!("{:02x} ", b);
        }
        print!("  ");
        for b in chunk {
            if *b >= 0x20 && *b < 0x7f {
                print!("{}", *b as char);
            } else {
                print!(".");
            }
        }
        println!();
    }
}
