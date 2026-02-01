use std::env;
use std::fs;
use cpclib_orgams_ascii::binary_decoder::parse_orgams_file;
use encoding_rs::WINDOWS_1252;
use cpclib_common::winnow::binary::le_u16;
use cpclib_common::winnow::{
    Parser, 
    stream::{Offset, Stream}, 
    combinator::{cut_err, peek},
    token::{literal, take, any},
    error::{StrContext, StrContextValue}
};
use cpclib_orgams_ascii::binary_decoder::DisplayState;
use cpclib_orgams_ascii::binary_decoder::{
    Input, OrgamsParseResult, StringTable, 
    parse_labels_table, parse_line,
};

const CHUNK_MAX_SIZE: u8 = 222;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file.orgams>", args[0]);
        return Ok(());
    }
    
    let raw_path = std::path::PathBuf::from(&args[1]);
    // Resolve path: try raw path, then crate manifest dir, then workspace package subdir
    let path_candidates = vec![
        raw_path.clone(),
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&raw_path),
        std::path::PathBuf::from("cpclib-orgams-ascii").join(&raw_path),
    ];

    let path_buf = path_candidates.into_iter().find(|p| p.exists()).unwrap_or(raw_path);
    println!("Reading file: {}", path_buf.display());

    // Check for groundtruth .Z80 file
    let z80_path = path_buf.with_extension("Z80");
    let groundtruth = if z80_path.exists() {
        println!("Found groundtruth file: {:?}", z80_path);
        let content = fs::read(&z80_path)?;
        let (decoded, _, _) = WINDOWS_1252.decode(&content);
        let lines: Vec<String> = decoded.lines().map(|s| s.to_string()).collect();
        Some(lines)
    } else {
        println!("No groundtruth file found (checked {:?})", z80_path);
        None
    };
    
    let bytes = fs::read(&path_buf)?;
    let mut input = Input::new(&bytes);
    
    let mut groundtruth_iter = groundtruth.as_ref().map(|lines| lines.iter());
    
    if let Err(e) = debug_orgams_file(&mut input, &mut groundtruth_iter) {
        eprintln!("Parsing failed.\nError: {:?}", e);
        std::process::exit(1);
    }
    
    Ok(())
}

fn debug_orgams_file(input: &mut Input, groundtruth_iter: &mut Option<std::slice::Iter<String>>) -> OrgamsParseResult<()> {
    
    parse_orgams_file(true, groundtruth_iter).parse_next(input)?;
    Ok(())
}


