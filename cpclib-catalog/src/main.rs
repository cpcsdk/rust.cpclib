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
use clap::Parser;
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
    /// List the content of the catalog ONLY for files having no control chars
    #[arg(short = 'l', long, requires = "input_file")]
    list: bool,

    /// List the content of the catalog EVEN for files having no control chars
    #[arg(short = 'a', long, requires = "input_file")]
    listall: bool,

    /// Display the catalog using CatArt rendering (sorted alphabetically)
    #[arg(long, requires = "input_file")]
    cat: bool,

    /// Display the catalog using CatArt rendering (directory order, unsorted)
    #[arg(long, requires = "input_file")]
    dir: bool,

    /// Build a catart from a BASIC program. Output will be a DSK/HFE file if the output filename ends with .dsk or .hfe, otherwise a raw 2048-byte catalog binary.
    #[arg(long, value_name = "BASIC_FILE", conflicts_with = "input_file")]
    build: Option<String>,

    /// Output file for --build operation (defaults to catart.dsk)
    #[arg(short = 'o', long = "output", requires = "build")]
    output_file: Option<String>,

    /// Input/Output file that contains the entries of the catalog (a binary file or a dsk)
    #[arg(short = 'i', long = "input", conflicts_with = "build")]
    input_file: Option<String>,

    /// Selects the entry to modify
    #[arg(long, value_parser = value_parser!(u8).range(..=63), requires = "input_file")]
    entry: Option<u8>,

    /// Set the selected entry readonly
    #[arg(long = "readonly", requires = "entry")]
    setreadonly: bool,

    /// Set the selected entry hidden
    #[arg(long = "system", requires = "entry")]
    setsystem: bool,

    /// Set the selected entry read and write
    #[arg(long = "noreadonly", requires = "entry")]
    unsetreadonly: bool,

    /// Set the selected entry visible
    #[arg(long = "nosystem", requires = "entry")]
    unsetsystem: bool,

    /// Set the user value
    #[arg(long, requires = "entry")]
    user: Option<u8>,

    /// Set the filename of the entry
    #[arg(long, requires = "entry")]
    filename: Option<String>,

    /// Set the blocs to load (and update the number of blocs accordingly to that)
    #[arg(long, requires = "entry", num_args = ..=16)]
    blocs: Option<Vec<u8>>,

    /// Set the page number
    #[arg(long)]
    numpage: Option<String>,

    /// Force the size of the entry
    #[arg(long)]
    size: Option<String>,
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
        else if !contains_id {
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
    let unified_catalog_reconstructed = UnifiedCatalog::from(catalog_for_reconstruction);
    let reconstructed_commands = unified_catalog_reconstructed.commands_by_mode_and_order(ScreenMode::Mode1, CatalogType::Cat);
    
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
    
    // Step 8: Display the generated BASIC program
    info!("Displaying generated BASIC program from catalog:");
    println!("\n=== Reconstructed BASIC Program ===");
    match catalog_to_basic_listing(&catalog_bytes, CatalogType::Cat) {
        Ok(basic_program) => {
            println!("\n{}", basic_program);
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

    // Handle --build option
    if let Some(ref basic_file) = args.build {
        let output_file = args.output_file.as_ref().map(|s| s.as_str()).unwrap_or("catart.dsk");
        
        return build_catart_from_basic(basic_file, output_file);
    }

    // Retrieve the current entries ...
    let catalog_fname = args.input_file.as_ref()
        .ok_or_else(|| std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Either --build or --input must be provided"
        ))?;
    let mut catalog_content: AmsdosEntries = {
        let mut content = Vec::new();

        if catalog_fname.to_lowercase().contains("dsk") || catalog_fname.to_lowercase().contains("hfe") {
            // Read a dsk or hfe file
            error!(
                "Current implementation is buggy when using dsks. Please extract first the catalog with another tool for real results."
            );
            let disc = open_disc(catalog_fname, true).expect("unable to read the disc file");
            let manager = AmsdosManagerNonMut::new_from_disc(&disc, Head::A);
            manager.catalog()
        }
        else {
            // Read a catalog file
            let mut file = File::open(catalog_fname)?;
            file.read_to_end(&mut content)?;
            AmsdosEntries::from_slice(&content)
        }
    };

    // ... and manipulate them
    if args.cat || args.dir {
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

        let catalog_type = if args.cat { CatalogType::Cat } else { CatalogType::Dir };
        if let Err(e) = display_catalog_using_catart(&catalog_bytes, catalog_type) {
            error!("Failed to display catalog using CatArt: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }
    }
    else if args.list || args.listall {
        list_catalog_entries(&catalog_content, args.listall);
    }
    else {
        let catalog_fname_lower = catalog_fname.to_lowercase();
        if catalog_fname_lower.contains("dsk") || catalog_fname_lower.contains("hfe") {
            list_catalog_entries(&catalog_content, args.listall);
        }
    }

    if let Some(idx) = args.entry {
        info!("Manipulate entry {idx}");

        let entry = catalog_content.get_entry_mut(idx as _);

        if args.setreadonly {
            entry.set_read_only();
        }
        if args.setsystem {
            entry.set_system();
        }
        if args.unsetreadonly {
            entry.unset_read_only();
        }
        if args.unsetsystem {
            entry.unset_system();
        }

        if let Some(user) = args.user {
            entry.set_user(user);
        }

        if let Some(ref filename) = args.filename {
            entry.set_filename(filename);
        }

        if let Some(ref blocs) = args.blocs {
            let blocs = blocs
                .iter()
                .map(|bloc| BlocIdx::from(*bloc))
                .collect::<Vec<BlocIdx>>();
            entry.set_blocs(&blocs);
        }

        if let Some(ref numpage) = args.numpage {
            entry.set_num_page(to_number::<u8>(numpage));
        }

        // XXX It is important ot keep it AFTER the blocs as it override their value
        if let Some(ref size) = args.size {
            let size = to_number::<u8>(size);
            entry.set_page_size(size);
        }

        // Write the result
        if catalog_fname.contains("dsk") {
            unimplemented!("Need to implement that");
        }
        else {
            let mut file = File::create(catalog_fname)?;
            file.write_all(&catalog_content.as_bytes())?;
        }
    }

    Ok(())
}
