use std::env;
use std::fs;
use cpclib_common::winnow::{
    Parser, 
    stream::{Offset, Stream}, 
    combinator::{cut_err, peek},
    token::{literal, take, any},
    error::{StrContext, StrContextValue}
};
use cpclib_orgams_ascii::binary_decoder::DisplayState;
use cpclib_orgams_ascii::binary_decoder::{
    Input, BasmParseResult, StringTable, 
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
        let content = fs::read_to_string(&z80_path)?;
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
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

fn debug_orgams_file(input: &mut Input, groundtruth_iter: &mut Option<std::slice::Iter<String>>) -> BasmParseResult<()> {
    println!("DEBUG: Starting debug_orgams_file");
    
    // 1. Parse Header
    const ORGA: &[u8] = b"ORGA";
    const SRCC: &[u8] = b"SRCc";
    const LBLS: &[u8] = b"LBLs";

    cut_err(literal(ORGA).context(StrContext::Expected(StrContextValue::StringLiteral("ORGA"))))
        .parse_next(input)?;

    // Skip rest of header (0x67 bytes total, already read 4)
    let _header = take(0x67 - ORGA.len()).parse_next(input)?;

    cut_err(literal(SRCC).context(StrContext::Expected(StrContextValue::StringLiteral("SRCc"))))
        .parse_next(input)?;

    let _version = any.verify(|&b| b==2).context(StrContext::Expected(StrContextValue::Description("version 2 expected"))).parse_next(input)?;

    // 2. Mark code start
    let code_start_checkpoint = input.checkpoint();
    
    // 3. Skip chunks to find LBLs
    println!("DEBUG: Skipping chunks to find LBLs...");
    let mut chunk_count = 0;
    loop {
        let b = peek(any).parse_next(input)?;
        if b == 0 {
            // Found terminator
             break;
        }
        let chunk_size = any.verify(|&s| s<=CHUNK_MAX_SIZE).parse_next(input)?;
        let _ = take(chunk_size as usize).parse_next(input)?;
        chunk_count += 1;
    }
    println!("DEBUG: Found terminator 0 after {} chunks", chunk_count);

    // 4. Parse Labels
    let _null_separator = literal([0x00]).parse_next(input)?;
    cut_err(literal(LBLS).context(StrContext::Expected(StrContextValue::StringLiteral("LBLs"))))
        .parse_next(input)?;
    let labels = parse_labels_table.parse_next(input)?;
    println!("DEBUG: Labels parsed. Count: {}", labels.len());
    
    // Print labels
    for (idx, label) in labels.iter().enumerate() {
         println!("  Label #{:03}: \"{}\"", idx, label.as_str());
    }

    // 5. Rewind to code start
    input.reset(&code_start_checkpoint);
    
    // 6. Parse Code with debug
    println!("DEBUG: Parsing code with trace...");
    parse_all_code_debug(input, &labels, groundtruth_iter)?;
    
    Ok(())
}

fn parse_all_code_debug(input: &mut Input, labels: &StringTable, groundtruth_iter: &mut Option<std::slice::Iter<String>>) -> BasmParseResult<()> {
    let mut chunk_idx = 0;
    let mut start_line = 0;
    loop {
        // Peek to check for the terminator (chunk size 0)
        let b = peek(any).parse_next(input)?;
        if b == 0 {
             println!("DEBUG: End of code segments (byte 0 found).");
             break;
        }
        
        println!("DEBUG: Parsing Chunk #{}", chunk_idx);
        start_line += parse_chunk_debug(input, labels, groundtruth_iter, chunk_idx, start_line)?;
        chunk_idx += 1;
    }
    Ok(())
}

fn parse_chunk_debug(input: &mut Input, labels: &StringTable, groundtruth_iter: &mut Option<std::slice::Iter<String>>, chunk_idx: usize, start_line: usize) -> BasmParseResult<usize> {
    let chunk_size = any.verify(|&s| s<=CHUNK_MAX_SIZE).parse_next(input)? as usize;
    
    // Chunk content peek
    let chunk_content = peek(take(chunk_size)).parse_next(input)?;
    println!("  Chunk Size: {}  Content: {:02X?}", chunk_size, chunk_content);
    
    let chunk_start = input.checkpoint();

	let mut render = DisplayState::new(None);
    let mut line_number = 0;    
	while input.offset_from(&chunk_start) < chunk_size {
        let line_offset = input.offset_from(&chunk_start);
        
        let line_start_check = input.checkpoint();
        

       // println!("Remaining bytes: [{:02X?}]", &input[.. ]);

        match parse_line.parse_next(input) {
            Ok(line) => {
				line_number += 1;
                 let consumed_len = input.offset_from(&line_start_check);
                 
                 // Reconstruct line
				 render.render_items(line.iter(), labels).unwrap();
				 let line_text = render.last_line().unwrap().to_string(); // XXX we assume only one line has been generated
                 
                 // Get bytes
                 input.reset(&line_start_check);
                 let raw_bytes = take(consumed_len).parse_next(input)?;
                 
                 // Printable bytes
                 let printable: String = raw_bytes.iter().map(|&b| if b >= 32 && b <= 126 { b as char } else { '.' }).collect();
                let reconstructed_bytes = line.bytes(&labels);
				println!("\n Chunk: {}       line: {} (total: {})", chunk_idx, line_number, start_line + line_number);
                 println!("          Debug:  {:?}", line);
                 println!("    [{:3}] Bytes:  {:02X?}", line_offset, raw_bytes);
                 println!("  Reconstructed:  {:02X?} ", reconstructed_bytes);
                 println!("      Printable:  {}", printable);
                 println!("           Text:  `{}`", line_text);
                 let groundtruth = if let Some(iter) = groundtruth_iter {
                     if let Some(ground) = iter.next() {
                         println!("         Ground:  `{}`", ground);
					 	Some(ground.to_owned())
                     } else {
                         println!("         Ground:  <End of Stream>");
						 Some("".to_owned())
                     }
                 } else {
					None
				 };

				 assert_eq!(raw_bytes, reconstructed_bytes, "Reconstructed bytes do not match original bytes at chunk offset {}", line_offset);
				 if let Some(groundtruth) = groundtruth {
					 assert_eq!(groundtruth, line_text, "Groundtruth does not match reconstructed text at chunk offset {}", line_offset);
				 }
				 assert_eq!(render.line_number()-1, line_number, "Line number mismatch at chunk offset {}", line_offset);
            },
            Err(e) => {
                println!("    ERROR at chunk offset {}: {:?}", line_offset, e);
                // Print context
                let remainder = &input[..];
                let context_len = std::cmp::min(100, remainder.len());
                let context_bytes = &remainder[..context_len];
                let context_printable: String = context_bytes.iter().map(|&b| if b >= 32 && b <= 126 { b as char } else { '.' }).collect();

                println!("    Context bytes (next {}): {:02X?}", context_len, context_bytes);
                println!("    Context chars (next {}): {}", context_len, context_printable);
                return Err(e);
            }
        }
    }
    
    if input.offset_from(&chunk_start) != chunk_size {
        println!("    WARN: Chunk mismatch. Expected {}, got {}", chunk_size, input.offset_from(&chunk_start));
    }
    
    Ok(render.line_number()-1)
}
