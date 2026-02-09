#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications,
    nonstandard_style,
    rust_2018_idioms,
    unused,
    warnings
)]
#![deny(clippy::pedantic)]

use std::fs::File;
use std::io::{Read, Write};

/// Catalog tool manipulator.
use clap::{Parser, Subcommand};
use cpclib_catalog::{catalog_to_basic_listing, display_catalog_using_catart};
use cpclib_catart::char_command::CharCommandList;
use cpclib_catart::entry::{Catalog, CatalogType, PrintableEntryFileName, ScreenMode, SerialCatalogBuilder, UnifiedCatalog};
use cpclib_catart::interpret::{Interpreter, Mode, screens_are_equal, display_screen_diff};
use cpclib_basic::BasicProgram;
use cpclib_common::clap::value_parser;
use cpclib_common::num::Num;
use cpclib_disc::amsdos::{AmsdosEntries, AmsdosManagerNonMut, BlocIdx};
use cpclib_disc::disc::Disc;
use cpclib_disc::edsk::Head;
use cpclib_disc::{open_disc, AnyDisc};
use log::{error, info};
use simple_logger::SimpleLogger;

#[derive(Parser, Debug)]
#[command(name = "catalog")]
#[command(about = "Amsdos catalog manipulation tool.", author = "Krusty/Benediction")]
struct Args {
    /// Input file that contains the entries of the catalog (a binary file or a dsk). For 'build' command, this is the BASIC file if not specified in the command.
    input_file: Option<String>,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Display the catalog using CatArt rendering (sorted alphabetically)
    Cat,
    
    /// Display the catalog using CatArt rendering (directory order, unsorted)
    Dir,
    
    /// List the content of the catalog ONLY for files having no control chars
    List,
    
    /// List the content of the catalog EVEN for files having control chars
    Listall,
    
    /// Build a catart from a BASIC program. Output will be a DSK/HFE file if the output filename ends with .dsk or .hfe, otherwise a raw 2048-byte catalog binary.
    Build {
        /// BASIC file to convert to catart (optional if input_file is provided at top level)
        basic_file: Option<String>,
        
        /// Output file (defaults to catart.dsk). Use .dsk or .hfe extension for disc images, otherwise creates raw binary
        #[arg(short = 'o', long = "output")]
        output_file: Option<String>,
    },
    
    /// Modify an entry in the catalog
    Modify {
        /// Selects the entry to modify
        #[arg(long, value_parser = value_parser!(u8).range(..=63))]
        entry: u8,
        
        /// Set the selected entry readonly
        #[arg(long = "readonly")]
        setreadonly: bool,
        
        /// Set the selected entry hidden
        #[arg(long = "system")]
        setsystem: bool,
        
        /// Set the selected entry read and write
        #[arg(long = "noreadonly")]
        unsetreadonly: bool,
        
        /// Set the selected entry visible
        #[arg(long = "nosystem")]
        unsetsystem: bool,
        
        /// Set the user value
        #[arg(long)]
        user: Option<u8>,
        
        /// Set the filename of the entry
        #[arg(long)]
        filename: Option<String>,
        
        /// Set the blocs to load (and update the number of blocs accordingly to that)
        #[arg(long, num_args = ..=16)]
        blocs: Option<Vec<u8>>,
        
        /// Set the page number
        #[arg(long)]
        numpage: Option<String>,
        
        /// Force the size of the entry
        #[arg(long)]
        size: Option<String>,
    },
}

#[must_use]
/// # Panics
///
/// Panics if the string cannot be parsed as a number in the expected format.
pub fn to_number<T>(repr: &str) -> T
where
    T: Num,
    T::FromStrRadixErr: std::fmt::Debug
{
    dbg!(repr);
    let repr = repr.trim();
    if let Some(stripped) = repr.strip_prefix("0x") {
        T::from_str_radix(stripped, 16)
    }
    else if let Some(stripped) = repr.strip_prefix("\\$") {
        T::from_str_radix(stripped, 16)
    }
    else if let Some(stripped) = repr.strip_prefix('&') {
        T::from_str_radix(stripped, 16)
    }
    else if repr.starts_with('0') {
        T::from_str_radix(repr, 8)
    }
    else {
        T::from_str_radix(repr, 10)
    }
    .expect("Unable to parse number")
}

fn list_catalog_entries(catalog_content: &AmsdosEntries, listall: bool) {
    for (idx, entry) in catalog_content.all_entries().enumerate() {

        let contains_id = !entry.is_erased();
        let is_hidden = entry.is_system();
        let is_read_only = entry.is_read_only();

        let fname = entry.format();
        let contains_control_chars = !fname.as_str().chars().all(|c| c.is_ascii_graphic());

        if contains_id && !contains_control_chars {
            print!("{idx}. {fname}");
            if is_hidden {
                print!(" [hidden]");
            }
            if is_read_only {
                print!(" [read only]");
            }

            print!(" {:>4}Kb {:?}", entry.used_space(), entry.used_blocs());
            println!();
        }
        else if contains_id && contains_control_chars && listall {
            println!("{idx}. => CAT ART <=");
        }
        else if !contains_id && listall {
            println!("{idx}. => EMPTY SLOT <=");
        } 
    }
}

/// Parse a BASIC program from a file
fn parse_basic_file(basic_filename: &str) -> std::io::Result<BasicProgram> {
    let mut file = File::open(basic_filename)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    
    BasicProgram::parse(&content)
        .map_err(|e| std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to parse BASIC program: {:?}", e)
        ))
}

/// Convert BASIC program to CharCommandList
fn basic_to_char_commands(basic: &BasicProgram) -> std::io::Result<CharCommandList> {
    use cpclib_catart::basic_command::BasicCommandList;
    
    let basic_commands = BasicCommandList::try_from(basic)
        .map_err(|e| std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to convert BASIC to commands: {:?}", e)
        ))?;
    
    basic_commands.to_char_commands()
        .map_err(|e| std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to convert commands to char commands: {:?}", e)
        ))
}

/// Build UnifiedCatalog using SerialCatalogBuilder
fn build_unified_catalog(commands: &CharCommandList) -> UnifiedCatalog {
    let builder = SerialCatalogBuilder::new();
    builder.build(commands, ScreenMode::Mode1)
}



/// Save catalog as binary file or disc image
fn save_catalog_output(catalog_bytes: &[u8], output_path: &str) -> std::io::Result<()> {
    let output_lower = output_path.to_lowercase();
    
    if output_lower.ends_with(".dsk") || output_lower.ends_with(".hfe") {
        // Create a disc image with the catalog
        let mut disc = AnyDisc::default();
        
        // Write catalog to track 0, sectors 0-3 (first 2048 bytes)
        // Standard CPC sector IDs start at 0xC1
        let head = Head::A;
        let track_id = 0;
        
        // Split catalog into 512-byte sectors
        for (sector_idx, chunk) in catalog_bytes.chunks(512).enumerate() {
            if sector_idx >= 4 {
                break; // Catalog is only 4 sectors
            }
            
            let sector_id = 0xC1 + sector_idx as u8;
            disc.sector_write_bytes(head, track_id, sector_id, chunk)
                .map_err(|e| std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to write sector: {:?}", e)
                ))?;
        }
        
        disc.save(output_path)
            .map_err(|e| std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to save disc: {:?}", e)
            ))?;
        
        info!("Created disc image: {}", output_path);
    } else {
        // Save as raw binary file
        let mut file = File::create(output_path)?;
        file.write_all(catalog_bytes)?;
        info!("Created binary catalog: {}", output_path);
    }
    
    Ok(())
}

/// Main build process: BASIC file -> catalog output
fn build_catart_from_basic(basic_filename: &str, output_filename: &str) -> std::io::Result<()> {
    info!("Building catart from BASIC file: {}", basic_filename);
    
    // Step 1: Parse the BASIC file
    let basic_program = parse_basic_file(basic_filename)?;
    info!("Successfully parsed BASIC program");
    
    // Step 1.5: Display original BASIC program execution
    info!("Original BASIC program execution:");
    println!("\n=== Original BASIC Program ===");
    println!("{}\n", basic_program);
    
    // Step 2: Convert to CharCommandList
    let char_commands = basic_to_char_commands(&basic_program)?;
    info!("Converted to {} char commands", char_commands.len());
    
    // Step 2.5: Interpret original catart to get Screen
    let mut original_interpreter = Interpreter::new(Mode::Mode1);
    original_interpreter.interpret(&char_commands, false)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Failed to interpret original commands: {}", e)))?;
    let original_screen = original_interpreter.screen().clone();
    let original_palette = original_interpreter.palette().clone();
    
    // Step 2.6: Display original catart output
    info!("Original catart output:");
    println!("=== Original CatArt Output ===");
    println!("{}\n", char_commands);
    
    // Step 3: Build UnifiedCatalog using SerialCatalogBuilder
    let unified_catalog = build_unified_catalog(&char_commands);
    info!("Built unified catalog with {} entries", unified_catalog.entries.len());
    
    // Step 4: Convert to Catalog using existing infrastructure
    // Extract PrintableEntryFileName from each UnifiedPrintableEntry
    let fnames: Vec<PrintableEntryFileName> = unified_catalog.entries
        .iter()
        .map(|e| e.fname)
        .collect();
    
    let catalog = Catalog::try_from(fnames.as_slice())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    info!("Converted to catalog");
    
    // Step 5: Convert to bytes (32*64 = 2048 bytes)
    let catalog_bytes = catalog.as_bytes();
    assert_eq!(catalog_bytes.len(), 2048, "Catalog must be exactly 2048 bytes");
    info!("Generated {} bytes of catalog data", catalog_bytes.len());
    
    // Step 6: Save to file
    save_catalog_output(catalog_bytes, output_filename)?;
    
    // Step 6.5: List all catalog entries and their commands
    info!("Catalog entries:");
    for (idx, entry) in catalog.entries.iter().enumerate() {
        if !entry.is_empty() {
            let display_name = entry.fname.display_name();
            let commands = entry.fname.commands();
            info!("  Entry {}: {} ({} commands)", idx, display_name, commands.len());
            
            // Show ALL commands for each entry
            for (cmd_idx, cmd) in commands.iter().enumerate() {
                info!("    [{}] {:?}", cmd_idx, cmd);
            }
        }
    }
    
    // Step 7: Interpret reconstructed catart and compare
    info!("Interpreting reconstructed catart from catalog:");
    // Re-extract fnames for reconstruction
    let fnames_for_reconstruction: Vec<PrintableEntryFileName> = unified_catalog.entries
        .iter()
        .map(|e| e.fname)
        .collect();
    let catalog_for_reconstruction = Catalog::try_from(fnames_for_reconstruction.as_slice())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    
    // Extract commands from catalog entries directly (not using commands_by_mode_and_order which generates DIR listing)
    let mut reconstructed_commands = CharCommandList::new();
    for entry in catalog_for_reconstruction.entries.iter() {
        if !entry.is_empty() {
            let entry_commands = entry.fname.commands();
            reconstructed_commands.extend(entry_commands);
        }
    }
    
    // Step 7.3: Byte-level comparison (using CharCommandList::bytes() which merges consecutive strings/chars)
    info!("\n=== BYTE-LEVEL ROUND-TRIP COMPARISON ===");
    
    // Convert to bytes using the CharCommandList method (merges consecutive strings/chars)
    let original_bytes = char_commands.bytes();
    info!("Original: {} commands -> {} bytes", char_commands.len(), original_bytes.len());
    
    let reconstructed_bytes = reconstructed_commands.bytes();
    info!("Reconstructed: {} commands -> {} bytes", reconstructed_commands.len(), reconstructed_bytes.len());
    
    // Parse both byte streams back to commands
    let original_parsed = CharCommandList::from_bytes(&original_bytes);
    info!("Original bytes parsed back to {} commands", original_parsed.len());
    
    let reconstructed_parsed = CharCommandList::from_bytes(&reconstructed_bytes);
    info!("Reconstructed bytes parsed back to {} commands", reconstructed_parsed.len());
    
    // Compare the parsed command lists
    info!("\nComparing byte-parsed commands:");
    let min_len = original_parsed.len().min(reconstructed_parsed.len());
    let mut byte_diff_count = 0;
    
    for idx in 0..min_len {
        if original_parsed[idx] != reconstructed_parsed[idx] {
            info!("  [{}] BYTE-DIFF: {:?} != {:?}", idx, original_parsed[idx], reconstructed_parsed[idx]);
            byte_diff_count += 1;
            if byte_diff_count >= 30 {
                info!("  ... (showing first 30 byte-level differences)");
                break;
            }
        }
    }
    
    if byte_diff_count == 0 && original_parsed.len() == reconstructed_parsed.len() {
        info!("  ✓ Byte-parsed commands match perfectly!");
    }
    
    if original_parsed.len() != reconstructed_parsed.len() {
        info!("  Byte-parsed length mismatch: original={}, reconstructed={}", 
            original_parsed.len(), reconstructed_parsed.len());
        if reconstructed_parsed.len() > original_parsed.len() {
            info!("  Extra commands in reconstructed (first 10):");
            for (idx, cmd) in reconstructed_parsed.iter().skip(original_parsed.len()).take(10).enumerate() {
                info!("    [{}]: {:?}", original_parsed.len() + idx, cmd);
            }
        } else {
            info!("  Missing commands from reconstructed (first 10):");
            for (idx, cmd) in original_parsed.iter().skip(reconstructed_parsed.len()).take(10).enumerate() {
                info!("    [{}]: {:?}", reconstructed_parsed.len() + idx, cmd);
            }
        }
    }
    
    // Step 7.4: Direct command comparison
    info!("\n=== DIRECT COMMAND COMPARISON ===");
    info!("  Original: {} commands", char_commands.len());
    info!("  Reconstructed: {} commands", reconstructed_commands.len());
    
    // Find differences in commands
    let mut diff_count = 0;
    let cmd_min_len = char_commands.len().min(reconstructed_commands.len());
    
    for idx in 0..cmd_min_len {
        if char_commands[idx] != reconstructed_commands[idx] {
            info!("  [{}] CMD-DIFF: {:?} != {:?}", idx, char_commands[idx], reconstructed_commands[idx]);
            diff_count += 1;
            if diff_count >= 30 {
                info!("  ... (showing first 30 command differences)");
                break;
            }
        }
    }
    
    if diff_count == 0 && char_commands.len() == reconstructed_commands.len() {
        info!("  ✓ All commands match perfectly!");
    }
    
    if char_commands.len() != reconstructed_commands.len() {
        info!("  Command length mismatch! Original has {} extra, Reconstructed has {} extra",
            char_commands.len().saturating_sub(reconstructed_commands.len()),
            reconstructed_commands.len().saturating_sub(char_commands.len()));
        
        // Show what's missing or extra
        if reconstructed_commands.len() > char_commands.len() {
            info!("  Extra commands in reconstructed (showing first 10):");
            for (idx, cmd) in reconstructed_commands.iter().skip(char_commands.len()).take(10).enumerate() {
                info!("    [{}]: {:?}", char_commands.len() + idx, cmd);
            }
        } else {
            info!("  Missing commands from reconstructed (showing first 10):");
            for (idx, cmd) in char_commands.iter().skip(reconstructed_commands.len()).take(10).enumerate() {
                info!("    [{}]: {:?}", reconstructed_commands.len() + idx, cmd);
            }
        }
    }
    
    let mut reconstructed_interpreter = Interpreter::new(Mode::Mode1);
    reconstructed_interpreter.interpret(&reconstructed_commands, false)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Failed to interpret reconstructed commands: {}", e)))?;
    let reconstructed_screen = reconstructed_interpreter.screen().clone();
    let reconstructed_palette = reconstructed_interpreter.palette().clone();
    
    // Step 7.5: Compare screens and display diff if different
    if screens_are_equal(&original_screen, &reconstructed_screen) {
        info!("✓ Screens are identical!");
        println!("\n=== Reconstructed CatArt Output ===");
        match display_catalog_using_catart(&catalog_bytes, CatalogType::Cat) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("Warning: Failed to display catart: {}", e);
            }
        }
    } else {
        info!("⚠ Screens differ! Displaying side-by-side comparison:");
        println!("\n=== CatArt Comparison (Screens Differ!) ===");
        let diff_display = display_screen_diff(&original_screen, &original_palette, &reconstructed_screen, &reconstructed_palette);
        println!("{}", diff_display);
    }
    
    // Step 8: Display the generated BASIC program and compare its bytes
    info!("Displaying generated BASIC program from catalog:");
    println!("\n=== Reconstructed BASIC Program ===");
    match catalog_to_basic_listing(&catalog_bytes, CatalogType::Cat) {
        Ok(reconstructed_basic) => {
            println!("\n{}", reconstructed_basic);
            
            // Convert to CharCommandList for byte comparison
            info!("\n=== BASIC PROGRAM BYTE COMPARISON ===");
            match basic_to_char_commands(&reconstructed_basic) {
                Ok(reconstructed_basic_commands) => {
                    // Get bytes from both original and reconstructed BASIC programs
                    let original_basic_bytes = char_commands.bytes();
                    let reconstructed_basic_bytes = reconstructed_basic_commands.bytes();
                    
                    info!("Original BASIC program: {} commands -> {} bytes", 
                        char_commands.len(), original_basic_bytes.len());
                    info!("Reconstructed BASIC program: {} commands -> {} bytes", 
                        reconstructed_basic_commands.len(), reconstructed_basic_bytes.len());
                    
                    // Compare byte arrays
                    if original_basic_bytes == reconstructed_basic_bytes {
                        info!("✓ BASIC program bytes match perfectly!");
                    } else {
                        info!("⚠ BASIC program bytes differ!");
                        info!("  Byte length: original={}, reconstructed={}", 
                            original_basic_bytes.len(), reconstructed_basic_bytes.len());
                        
                        // Find first difference
                        let min_len = original_basic_bytes.len().min(reconstructed_basic_bytes.len());
                        let mut first_diff = None;
                        for i in 0..min_len {
                            if original_basic_bytes[i] != reconstructed_basic_bytes[i] {
                                first_diff = Some(i);
                                break;
                            }
                        }
                        
                        if let Some(idx) = first_diff {
                            info!("  First difference at byte index {}", idx);
                            let start = idx.saturating_sub(5);
                            let end = (idx + 10).min(min_len);
                            info!("  Original bytes [{}..{}]: {:02X?}", start, end, &original_basic_bytes[start..end]);
                            info!("  Reconstructed bytes [{}..{}]: {:02X?}", start, end, &reconstructed_basic_bytes[start..end]);
                        }
                        
                        // Show extra bytes if lengths differ
                        if reconstructed_basic_bytes.len() > original_basic_bytes.len() {
                            let extra_start = original_basic_bytes.len();
                            let extra_end = (extra_start + 20).min(reconstructed_basic_bytes.len());
                            info!("  Extra bytes in reconstructed [{}..{}]: {:02X?}", 
                                extra_start, extra_end, &reconstructed_basic_bytes[extra_start..extra_end]);
                        } else if original_basic_bytes.len() > reconstructed_basic_bytes.len() {
                            let missing_start = reconstructed_basic_bytes.len();
                            let missing_end = (missing_start + 20).min(original_basic_bytes.len());
                            info!("  Missing bytes from reconstructed [{}..{}]: {:02X?}", 
                                missing_start, missing_end, &original_basic_bytes[missing_start..missing_end]);
                        }
                    }
                },
                Err(e) => {
                    eprintln!("Warning: Failed to convert reconstructed BASIC to commands: {}", e);
                }
            }
        },
        Err(e) => {
            eprintln!("Warning: Failed to generate BASIC listing: {}", e);
        }
    }
    
    Ok(())
}


#[allow(clippy::too_many_lines)]
fn main() -> std::io::Result<()> {
    // XXX this has been disabled for compatbility reasons with gpu
    // XXX as this software has been used since ages, I have no idea if this is an issue or not
    // TermLogger::init(
    // LevelFilter::Debug,
    // Config::default(),
    // TerminalMode::Mixed,
    // ColorChoice::Auto
    // )
    // .expect("Unable to build logger");
    let logger = SimpleLogger::new();
    log::set_max_level(log::LevelFilter::Debug);
    log::set_boxed_logger(Box::new(logger)).unwrap();

    let args = Args::parse();

    match args.command {
        Commands::Build { basic_file, output_file } => {
            // Use Build's basic_file if provided, otherwise fall back to top-level input_file
            let input = basic_file.or(args.input_file).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "BASIC file must be provided either as top-level argument or with the build command")
            })?;
            let output = output_file.as_deref().unwrap_or("catart.dsk");
            build_catart_from_basic(&input, output)
        }
        
        Commands::Cat => {
            let input_file = args.input_file.ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "input_file is required for 'cat' command")
            })?;
            display_catalog_command(&input_file, CatalogType::Cat)
        }
        
        Commands::Dir => {
            let input_file = args.input_file.ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "input_file is required for 'dir' command")
            })?;
            display_catalog_command(&input_file, CatalogType::Dir)
        }
        
        Commands::List => {
            let input_file = args.input_file.ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "input_file is required for 'list' command")
            })?;
            list_catalog_command(&input_file, false)
        }
        
        Commands::Listall => {
            let input_file = args.input_file.ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "input_file is required for 'listall' command")
            })?;
            list_catalog_command(&input_file, true)
        }
        
        Commands::Modify {
            entry: idx,
            setreadonly,
            setsystem,
            unsetreadonly,
            unsetsystem,
            user,
            filename,
            blocs,
            numpage,
            size,
        } => {
            let input_file = args.input_file.ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "input_file is required for 'modify' command")
            })?;
            modify_entry_command(
                &input_file,
                idx,
                setreadonly,
                setsystem,
                unsetreadonly,
                unsetsystem,
                user,
                filename,
                blocs,
                numpage,
                size,
            )
        }
    }
}

fn display_catalog_command(catalog_fname: &str, catalog_type: CatalogType) -> std::io::Result<()> {
    // For CatArt display, we need the raw catalog bytes
    let catalog_bytes: Vec<u8> = if catalog_fname.to_lowercase().contains("dsk") || catalog_fname.to_lowercase().contains("hfe") {
        // Get raw bytes directly from disc
        let disc = open_disc(catalog_fname, true).expect("unable to read the disc file");
        let manager = AmsdosManagerNonMut::new_from_disc(&disc, Head::A);
        manager.catalog_slice()
    } else {
        // For catalog files, re-read the raw content
        let mut file = File::open(catalog_fname)?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;
        content
    };

    if let Err(e) = display_catalog_using_catart(&catalog_bytes, catalog_type) {
        error!("Failed to display catalog using CatArt: {}", e);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
    }
    
    Ok(())
}

fn list_catalog_command(catalog_fname: &str, listall: bool) -> std::io::Result<()> {
    let catalog_content: AmsdosEntries = if catalog_fname.to_lowercase().contains("dsk") || catalog_fname.to_lowercase().contains("hfe") {
        // Read a dsk or hfe file
        error!(
            "Current implementation is buggy when using dsks. Please extract first the catalog with another tool for real results."
        );
        let disc = open_disc(catalog_fname, true).expect("unable to read the disc file");
        let manager = AmsdosManagerNonMut::new_from_disc(&disc, Head::A);
        manager.catalog()
    } else {
        // Read a catalog file
        let mut file = File::open(catalog_fname)?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;
        AmsdosEntries::from_slice(&content)
    };
    
    list_catalog_entries(&catalog_content, listall);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn modify_entry_command(
    catalog_fname: &str,
    idx: u8,
    setreadonly: bool,
    setsystem: bool,
    unsetreadonly: bool,
    unsetsystem: bool,
    user: Option<u8>,
    filename: Option<String>,
    blocs: Option<Vec<u8>>,
    numpage: Option<String>,
    size: Option<String>,
) -> std::io::Result<()> {
    let mut catalog_content: AmsdosEntries = if catalog_fname.to_lowercase().contains("dsk") || catalog_fname.to_lowercase().contains("hfe") {
        // Read a dsk or hfe file
        error!(
            "Current implementation is buggy when using dsks. Please extract first the catalog with another tool for real results."
        );
        let disc = open_disc(catalog_fname, true).expect("unable to read the disc file");
        let manager = AmsdosManagerNonMut::new_from_disc(&disc, Head::A);
        manager.catalog()
    } else {
        // Read a catalog file
        let mut file = File::open(catalog_fname)?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;
        AmsdosEntries::from_slice(&content)
    };
    
    info!("Manipulate entry {idx}");

    let entry = catalog_content.get_entry_mut(idx as _);

    if setreadonly {
        entry.set_read_only();
    }
    if setsystem {
        entry.set_system();
    }
    if unsetreadonly {
        entry.unset_read_only();
    }
    if unsetsystem {
        entry.unset_system();
    }

    if let Some(user_val) = user {
        entry.set_user(user_val);
    }

    if let Some(ref fname) = filename {
        entry.set_filename(fname);
    }

    if let Some(ref blocs_vec) = blocs {
        let blocs_idx = blocs_vec
            .iter()
            .map(|bloc| BlocIdx::from(*bloc))
            .collect::<Vec<BlocIdx>>();
        entry.set_blocs(&blocs_idx);
    }

    if let Some(ref numpage_str) = numpage {
        entry.set_num_page(to_number::<u8>(numpage_str));
    }

    // XXX It is important ot keep it AFTER the blocs as it override their value
    if let Some(ref size_str) = size {
        let size_val = to_number::<u8>(size_str);
        entry.set_page_size(size_val);
    }

    // Write the result
    if catalog_fname.contains("dsk") {
        unimplemented!("Need to implement that");
    }
    else {
        let mut file = File::create(catalog_fname)?;
        file.write_all(&catalog_content.as_bytes())?;
    }
    
    Ok(())
}
