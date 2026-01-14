use cpclib_orgams_ascii::*;
use std::fs;

#[test]
fn test_multiple_o_files() {
    let test_dir = "tests/orgams-main";
    
    // Find all .O files
    let o_files = fs::read_dir(test_dir)
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()? == "O" {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    
    println!("\nFound {} .O files\n", o_files.len());
    
    for o_file in o_files.iter() {
        let filename = o_file.file_name().unwrap().to_str().unwrap();
        print!("Testing {}... ", filename);
        
        // Try to parse
        let data = fs::read(o_file).unwrap();
        let result = OrgamsFile::read(&data[..]);
        
        match result {
            Ok(orgams_file) => {
                // Try to decode
                let mut decoder = OrgamsDecoder::new(orgams_file.content.clone());
                match decoder.decode() {
                    Ok(decoded) => {
                        let text = decoded.iter()
                            .map(|elem| match elem {
                                DecodedElement::Text(s) => s.as_str(),
                                _ => "",
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        
                        let lines: Vec<_> = text.lines().collect();
                        println!("✓ Decoded {} lines", lines.len());
                        
                        // Show first 5 lines
                        if lines.len() > 0 {
                            for (i, line) in lines.iter().take(5).enumerate() {
                                println!("  {}: {}", i+1, line);
                            }
                            if lines.len() > 5 {
                                println!("  ...");
                            }
                            println!();
                        }
                    }
                    Err(e) => {
                        println!("✗ Decode failed: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("✗ Failed to parse: {:?}", e);
            }
        }
    }
}
