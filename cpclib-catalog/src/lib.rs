pub mod cli;

use std::fs::File;
use std::io::{Read, Write};

use cpclib_basic::BasicProgram;
use cpclib_catart::Locale;
use cpclib_catart::basic_command::BasicCommandList;
use cpclib_catart::char_command::CharCommandList;
use cpclib_catart::entry::{
    Catalog, CatalogType, PrintableEntry, PrintableEntryFileName, ScreenMode, SerialCatalogBuilder,
    UnifiedCatalog
};
use cpclib_catart::interpret::{Interpreter, Mode, display_screen_diff, screens_are_equal};
use cpclib_common::num::Num;
use cpclib_disc::amsdos::{AmsdosEntries, AmsdosManagerNonMut, BlocIdx};
use cpclib_disc::disc::Disc;
use cpclib_disc::edsk::Head;
use cpclib_disc::{AnyDisc, open_disc};
use log::{error, info};

use crate::cli::{CatalogApp, CatalogCommand};

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub fn display_catalog_using_catart(
    catalog_bytes: &[u8],
    catalog_type: CatalogType
) -> Result<(), String> {
    let screen_output = catalog_screen_output(catalog_bytes, catalog_type)?;

    println!("{}", screen_output);

    Ok(())
}

pub fn catalog_screen_output(
    catalog_bytes: &[u8],
    catalog_type: CatalogType
) -> Result<String, String> {
    let commands = catalog_to_catart_commands(catalog_bytes, catalog_type)?;
    Ok(commands.to_string())
}

pub fn catalog_extraction(
    catalog_bytes: &[u8],
    _catalog_type: CatalogType
) -> Result<Catalog, String> {
    assert_eq!(catalog_bytes.len(), 64 * 32);

    // Work directly with raw catalog bytes to avoid any filtering
    // Parse raw catalog bytes: each entry is 32 bytes
    // Byte 0 (St): Status/User number (0-15=user, 16-31=user P2DOS, 32=label, 33=timestamp, 0xE5=erased)
    // Bytes 1-8 (F0-F7): Filename (bit 7 = attributes)
    // Bytes 9-11 (E0-E2): Extension (bit 7 = attributes: E0=read-only, E1=system, E2=archived)
    // Byte 12 (Xl): Extent number low
    // Byte 13 (Bc): Byte count
    // Byte 14 (Xh): Extent number high
    // Byte 15 (Rc): Record count
    // Bytes 16-31 (Al): Allocation blocks (16 bytes)

    // For CatArt disks, each entry is a separate display element, not extents of the same file
    // So we process each entry individually without grouping
    let mut printable_entries = Vec::new();

    // Process each 32-byte entry
    for chunk_idx in 0..64 {
        let offset = chunk_idx * 32;
        let entry_bytes = &catalog_bytes[offset..offset + 32];

        if entry_bytes[1] == b'W' {
            dbg!(&entry_bytes);
        }

        let status = entry_bytes[0];

        // empty entries are kept as-is (status 0xE5)

        // Create PrintableEntryFileName directly from raw CP/M bytes
        // Bytes 1-8: filename (f1-f8), Bytes 9-11: extension (e1-e3)
        // IMPORTANT: Keep ALL bits as-is - bit 7 is part of CatArt encoding!
        let fname = PrintableEntryFileName {
            f1: entry_bytes[1],
            f2: entry_bytes[2],
            f3: entry_bytes[3],
            f4: entry_bytes[4],
            f5: entry_bytes[5],
            f6: entry_bytes[6],
            f7: entry_bytes[7],
            f8: entry_bytes[8],
            e1: entry_bytes[9],
            e2: entry_bytes[10],
            e3: entry_bytes[11]
        };

        let new_entry = PrintableEntry {
            user: status,
            fname,
            pieces: [0; 4], // TODO use a real information
            sectors: entry_bytes[16..32].try_into().unwrap()
        };

        // TODO handle files on several entries (we need to track this information)

        printable_entries.push(new_entry);
    }

    // Create catalog from entries
    let catalog = Catalog::try_from(printable_entries.as_slice())
        .map_err(|e| format!("Failed to create catalog: {}", e))?;

    Ok(catalog)
}

pub fn catalog_to_catart_commands(
    catalog_bytes: &[u8],
    catalog_type: CatalogType
) -> Result<CharCommandList, String> {
    let catalog = catalog_extraction(catalog_bytes, catalog_type)?;

    // Convert to UnifiedCatalog
    let unified_catalog = UnifiedCatalog::from(catalog);

    // Get commands for display
    Ok(unified_catalog.commands_by_mode_and_order(ScreenMode::Mode1, catalog_type))
}

pub fn catalog_to_basic_listing(
    catalog_bytes: &[u8],
    catalog_type: CatalogType
) -> Result<BasicProgram, String> {
    catalog_to_basic_listing_with_headers(catalog_bytes, catalog_type, true)
}

pub fn catalog_to_basic_listing_with_headers(
    catalog_bytes: &[u8],
    catalog_type: CatalogType,
    show_headers: bool
) -> Result<BasicProgram, String> {
    let catalog = catalog_extraction(catalog_bytes, catalog_type)?;

    // Get BASIC listing with optional headers
    Ok(catalog.extract_basic_from_sequential_catart(show_headers))
}

pub fn handle_catalog_command(args: CatalogApp) -> Result<(), String> {
    match args.command {
        CatalogCommand::Build {
            basic_file,
            output_file,
            render_options
        } => {
            // Use Build's basic_file if provided, otherwise fall back to top-level input_file
            let input = basic_file.or(args.input_file).ok_or_else(|| {
                "BASIC file must be provided either as top-level argument or with the build command".to_string()
            })?;
            let output = output_file.as_deref().unwrap_or("catart.dsk");

            build_catart_from_basic(
                &input,
                output,
                render_options.png_path(),
                render_options.parse_locale()
            )
        },

        CatalogCommand::Cat { render_options } => {
            let input_file = args
                .input_file
                .ok_or_else(|| "input_file is required for 'cat' command".to_string())?;

            display_catalog_command(
                &input_file,
                CatalogType::Cat,
                render_options.png_path(),
                render_options.parse_locale(),
                render_options.parse_mode()
            )
        },

        CatalogCommand::Dir { render_options } => {
            let input_file = args
                .input_file
                .ok_or_else(|| "input_file is required for 'dir' command".to_string())?;

            display_catalog_command(
                &input_file,
                CatalogType::Dir,
                render_options.png_path(),
                render_options.parse_locale(),
                render_options.parse_mode()
            )
        },

        CatalogCommand::List => {
            let input_file = args
                .input_file
                .ok_or_else(|| "input_file is required for 'list' command".to_string())?;
            list_catalog_command(&input_file, false)
        },

        CatalogCommand::Listall => {
            let input_file = args
                .input_file
                .ok_or_else(|| "input_file is required for 'listall' command".to_string())?;
            list_catalog_command(&input_file, true)
        },

        CatalogCommand::Decode { output_file } => {
            let input_file = args
                .input_file
                .ok_or_else(|| "input_file is required for 'decode' command".to_string())?;
            decode_catalog_command(&input_file, output_file.as_deref())
        },

        CatalogCommand::Modify {
            entry: idx,
            setreadonly,
            setsystem,
            unsetreadonly,
            unsetsystem,
            user,
            filename,
            blocs,
            numpage,
            size
        } => {
            let input_file = args
                .input_file
                .ok_or_else(|| "input_file is required for 'modify' command".to_string())?;
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
                size
            )
        },

        CatalogCommand::Debug { cat, dir } => {
            let input_file = args
                .input_file
                .ok_or_else(|| "input_file is required for 'debug' command".to_string())?;

            // Determine catalog type based on flags
            let catalog_type = if cat && !dir {
                CatalogType::Cat
            }
            else if dir && !cat {
                CatalogType::Dir
            }
            else if !cat && !dir {
                // Default: no sorting (directory order)
                CatalogType::Dir
            }
            else {
                return Err("Cannot specify both --cat and --dir".to_string());
            };

            debug_catalog_command(&input_file, catalog_type)
        }
    }
}

fn display_catalog_command(
    catalog_fname: &str,
    catalog_type: CatalogType,
    png_output: Option<&str>,
    locale: Locale,
    mode: Mode
) -> Result<(), String> {
    // Load the raw catalog bytes
    let catalog_bytes = load_catalog_bytes(catalog_fname)
        .map_err(|e| format!("Error while loading catalog {}", e))?;

    let commands = catalog_to_catart_commands(&catalog_bytes, catalog_type)?;

    // Interpret the commands with the selected locale and mode
    let mut interpreter = Interpreter::builder()
        .as_6128(true)
        .screen_mode(mode)
        .locale(locale)
        .build();
    interpreter
        .interpret(&commands, true)
        .map_err(|e| format!("Failed to interpret commands: {}", e))?;

    println!("{}", interpreter);
    // Generate PNG if requested
    if let Some(png_path) = png_output {
        save_interpreter_png(&interpreter, png_path)?;
    }

    Ok(())
}

fn list_catalog_command(catalog_fname: &str, listall: bool) -> Result<(), String> {
    let catalog_content = load_catalog_entries(catalog_fname)
        .map_err(|e| format!("Error while loading catalog entries: {}", e))?;

    list_catalog_entries(&catalog_content, listall);
    Ok(())
}

fn debug_catalog_command(catalog_fname: &str, catalog_type: CatalogType) -> Result<(), String> {
    // Load the raw catalog bytes
    let catalog_bytes = load_catalog_bytes(catalog_fname)
        .map_err(|e| format!("Error while loading catalog bytes: {}", e))?;

    // Validate catalog size
    if catalog_bytes.len() != 64 * 32 {
        return Err(format!(
            "Invalid catalog size: expected {} bytes, got {}",
            64 * 32,
            catalog_bytes.len()
        ));
    }

    println!("=== CatArt Debug Information ===\n");

    // Extract catalog and convert to UnifiedCatalog
    let catalog = catalog_extraction(&catalog_bytes, catalog_type)
        .map_err(|e| format!("Error while extracting catalog: {}", e))?;
    let unified_catalog = UnifiedCatalog::from(catalog);

    // Get sorted entries using EntriesGrid delegation
    let sorted_entries = unified_catalog.visible_sorted_entries(catalog_type);

    // Create a map from fname to original index for reference
    let mut fname_to_idx: std::collections::HashMap<[u8; 11], usize> =
        std::collections::HashMap::new();
    for chunk_idx in 0..64 {
        let offset = chunk_idx * 32;
        let entry_bytes = &catalog_bytes[offset..offset + 32];
        let fname_bytes = [
            entry_bytes[1],
            entry_bytes[2],
            entry_bytes[3],
            entry_bytes[4],
            entry_bytes[5],
            entry_bytes[6],
            entry_bytes[7],
            entry_bytes[8],
            entry_bytes[9],
            entry_bytes[10],
            entry_bytes[11]
        ];
        fname_to_idx.insert(fname_bytes, chunk_idx);
    }

    // Process each entry in sorted order
    for entry in sorted_entries {
        let fname = entry.fname();

        let fname_bytes = [
                fname.f1, fname.f2, fname.f3, fname.f4, fname.f5, fname.f6, fname.f7, fname.f8,
                fname.e1, fname.e2, fname.e3
        ];

        // Skip empty entries
        if fname.is_empty() {
            if let Some(&original_idx) = fname_to_idx.get(&fname_bytes) {
                println!("Entry {}: (empty)", original_idx);
            } else {
                unreachable!("Empty entry not found in original catalog bytes");
            }
            continue;
        }

        let original_idx = fname_to_idx.get(&fname_bytes).copied().unwrap_or(0);

        // Get the 8+3 bytes (without dot) as hexadecimal
        let bytes_without_dot: Vec<String> = fname_bytes.iter()
            .map(|b| format!("{:02X}", b))
            .collect();

        // Get the 8+1+3 bytes (with dot) for BASIC conversion
        let all_bytes = fname.all_generated_bytes();

        // Convert bytes to printable string (replace non-printable with dots)
        let printable_bytes: String = all_bytes
            .iter()
            .map(|&b| {
                if (32..=126).contains(&b) {
                    b as char
                }
                else {
                    '?'
                }
            })
            .collect();

        // Print entry information
        println!("Entry {:<2}: {}", original_idx, bytes_without_dot.join(" "));
        println!("          {}", std::str::from_utf8(&entry.all_generated_bytes()
                                .iter().map(|&b| if b>=b' ' && b<=127 {b} else {b'?'})
                                .collect::<Vec<u8>>()
                    )        .unwrap_or("Invalid UTF-8"));
    
        println!("          {}", printable_bytes);

        // Check if entry is hidden/system
        if fname.is_hidden() || fname.is_system() {
            let mut flags = Vec::new();
            if fname.is_hidden() {
                flags.push("hidden");
            }
            if fname.is_system() {
                flags.push("system");
            }
            println!("  Status: ({})", flags.join(", "));
        }
        else {
            // Get the commands that represent this entry
            let commands = fname.commands();

            // Show CharCommands (more explicit than BASIC)
            println!("          {}", commands.to_command_string());

            // Convert commands to BASIC string
            let basic_string = commands.to_basic_string();

            println!("          {}", basic_string);
        }
        println!();
    }

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
    size: Option<String>
) -> Result<(), String> {
    let mut catalog_content = load_catalog_entries(catalog_fname)
        .map_err(|e| format!("Error while loading catalog entries: {}", e))?;

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
        let mut file = File::create(catalog_fname)
            .map_err(|e| format!("Failed to create catalog file: {}", e))?;
        file.write_all(&catalog_content.as_bytes())
            .map_err(|e| format!("Failed to write catalog file: {}", e))?;
    }

    Ok(())
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
fn parse_basic_file(basic_filename: &str) -> Result<BasicProgram, String> {
    let mut file = File::open(basic_filename).map_err(|e| e.to_string())?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| e.to_string())?;

    BasicProgram::parse(&content).map_err(|e| format!("Failed to parse BASIC program: {:?}", e))
}

/// Convert BASIC program to CharCommandList
fn basic_to_char_commands(basic: &BasicProgram) -> Result<CharCommandList, String> {
    use cpclib_catart::basic_command::BasicCommandList;

    let basic_commands = BasicCommandList::try_from(basic)
        .map_err(|e| format!("Failed to convert BASIC to commands: {:?}", e))?;

    basic_commands
        .to_char_commands()
        .map_err(|e| format!("Failed to convert commands to char commands: {:?}", e))
}

/// Build UnifiedCatalog using SerialCatalogBuilder
fn build_unified_catalog(commands: &CharCommandList) -> UnifiedCatalog {
    let builder = SerialCatalogBuilder::new();
    builder.build(commands, ScreenMode::Mode1)
}

/// Check if the filename is a disc image (DSK or HFE)
fn is_disc_image(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    lower.ends_with(".dsk") || lower.ends_with(".hfe")
}

/// Load raw catalog bytes from a disc image or binary file
fn load_catalog_bytes(catalog_fname: &str) -> Result<Vec<u8>, String> {
    if is_disc_image(catalog_fname) {
        let disc = open_disc(catalog_fname, true)
            .map_err(|e| format!("Unable to read the disc file: {:?}", e))?;
        let manager = AmsdosManagerNonMut::new_from_disc(&disc, Head::A);
        Ok(manager.catalog_slice())
    }
    else {
        let mut file = File::open(catalog_fname).map_err(|e| e.to_string())?;
        let mut content = Vec::new();
        file.read_to_end(&mut content).map_err(|e| e.to_string())?;
        Ok(content)
    }
}

/// Load catalog entries (AmsdosEntries) from a disc image or binary file
fn load_catalog_entries(catalog_fname: &str) -> Result<AmsdosEntries, String> {
    if is_disc_image(catalog_fname) {
        error!(
            "Current implementation is buggy when using dsks. Please extract first the catalog with another tool for real results."
        );
        let disc = open_disc(catalog_fname, true)
            .map_err(|e| format!("Unable to read the disc file: {:?}", e))?;
        let manager = AmsdosManagerNonMut::new_from_disc(&disc, Head::A);
        Ok(manager.catalog())
    }
    else {
        let mut file = File::open(catalog_fname).map_err(|e| e.to_string())?;
        let mut content = Vec::new();
        file.read_to_end(&mut content).map_err(|e| e.to_string())?;
        Ok(AmsdosEntries::from_slice(&content))
    }
}

/// Generate PNG from interpreter screen
fn save_interpreter_png(interpreter: &Interpreter, png_path: &str) -> Result<(), String> {
    info!("Generating pixel-accurate PNG: {}", png_path);

    let color_matrix = interpreter
        .memory_screen()
        .to_color_matrix()
        .ok_or_else(|| "Failed to convert screen to image".to_string())?;

    let img = color_matrix.as_image();
    img.save(png_path)
        .map_err(|e| format!("Failed to save PNG: {}", e))?;

    info!("Successfully saved PNG to: {}", png_path);
    Ok(())
}

/// Save catalog as binary file or disc image
fn save_catalog_output(catalog_bytes: &[u8], output_path: &str) -> Result<(), String> {
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
                .map_err(|e| format!("Failed to write sector: {:?}", e))?;
        }

        disc.save(output_path)
            .map_err(|e| format!("Failed to save disc: {:?}", e))?;

        info!("Created disc image: {}", output_path);
    }
    else {
        // Save as raw binary file
        let mut file = File::create(output_path).map_err(|e| e.to_string())?;
        file.write_all(catalog_bytes).map_err(|e| e.to_string())?;
        info!("Created binary catalog: {}", output_path);
    }

    Ok(())
}

/// Main build process: BASIC file -> catalog output
fn build_catart_from_basic(
    basic_filename: &str,
    output_filename: &str,
    png_output: Option<&str>,
    locale: Locale
) -> Result<(), String> {
    info!("Building catart from BASIC file: {}", basic_filename);

    // Step 1: Parse the BASIC file
    let basic_program = parse_basic_file(basic_filename)?;
    info!("Successfully parsed BASIC program");

    // Step 1.5: Display original BASIC program execution
    info!("Original BASIC program execution:");
    println!("\n=== Original BASIC Program ===");
    println!("{}\n", basic_program);

    // Step 2: Convert to CharCommandList
    let generated_char_commands = basic_to_char_commands(&basic_program)?;
    info!(
        "Converted to {} char commands",
        generated_char_commands.len()
    );

    // Step 2.5: Interpret original catart to get Screen
    let mut original_interpreter = Interpreter::builder()
        .screen_mode(Mode::Mode1)
        .locale(locale)
        .as_6128(true)
        .build();
    original_interpreter
        .interpret(&generated_char_commands, false)
        .map_err(|e| format!("Failed to interpret original commands: {}", e))?;
    let original_screen = original_interpreter.screen().clone();
    let original_palette = original_interpreter.palette().clone();

    // Step 2.5.1: Save PNG if requested
    if let Some(png_path) = png_output {
        save_interpreter_png(&original_interpreter, png_path)?;
    }

    // Step 2.6: Display original catart output
    info!("Original catart output:");
    println!("=== Original CatArt Output ===");
    println!("{}\n", generated_char_commands);

    // Step 3: Build UnifiedCatalog using SerialCatalogBuilder
    let unified_catalog = build_unified_catalog(&generated_char_commands);
    info!(
        "Built unified catalog with {} entries",
        unified_catalog.entries.len()
    );

    // Step 4: Convert to Catalog using existing infrastructure
    // Extract PrintableEntryFileName from each UnifiedPrintableEntry
    let fnames: Vec<PrintableEntryFileName> =
        unified_catalog.entries.iter().map(|e| e.fname).collect();

    let catalog = Catalog::try_from(fnames.as_slice())
        .map_err(|e| format!("Failed to build catalog: {}", e))?;
    info!("Converted to catalog");

    // Step 5: Convert to bytes (32*64 = 2048 bytes)
    let catalog_bytes = catalog.as_bytes();
    assert_eq!(
        catalog_bytes.len(),
        2048,
        "Catalog must be exactly 2048 bytes"
    );
    info!("Generated {} bytes of catalog data", catalog_bytes.len());

    // Step 6: Save to file
    save_catalog_output(catalog_bytes, output_filename)?;

    // Step 6.5: List all catalog entries and their commands
    info!("Catalog entries:");
    for (idx, entry) in catalog.entries.iter().enumerate() {
        if !entry.is_empty() {
            let display_name = entry.fname.display_name();
            let commands = entry.fname.commands();
            info!(
                "  Entry {}: {} ({} commands)",
                idx,
                display_name,
                commands.len()
            );

            // Show ALL commands for each entry
            for (cmd_idx, cmd) in commands.iter().enumerate() {
                info!("    [{}] {:?}", cmd_idx, cmd);
            }
        }
    }

    // Step 7: Interpret reconstructed catart and compare
    info!("Interpreting reconstructed catart from catalog:");
    // Re-extract fnames for reconstruction
    let fnames_for_reconstruction: Vec<PrintableEntryFileName> =
        unified_catalog.entries.iter().map(|e| e.fname).collect();
    let catalog_for_reconstruction = Catalog::try_from(fnames_for_reconstruction.as_slice())
        .map_err(|e| format!("Failed to create catalog for reconstruction: {}", e))?;

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
    let original_bytes = generated_char_commands.bytes();
    info!(
        "Original: {} commands -> {} bytes",
        generated_char_commands.len(),
        original_bytes.len()
    );

    let reconstructed_bytes = reconstructed_commands.bytes();
    info!(
        "Reconstructed: {} commands -> {} bytes",
        reconstructed_commands.len(),
        reconstructed_bytes.len()
    );

    // Parse both byte streams back to commands
    let original_parsed = CharCommandList::from_bytes(&original_bytes);
    info!(
        "Original bytes parsed back to {} commands",
        original_parsed.len()
    );

    let reconstructed_parsed = CharCommandList::from_bytes(&reconstructed_bytes);
    info!(
        "Reconstructed bytes parsed back to {} commands",
        reconstructed_parsed.len()
    );

    // Compare the parsed command lists
    info!("\nComparing byte-parsed commands:");
    let min_len = original_parsed.len().min(reconstructed_parsed.len());
    let mut byte_diff_count = 0;

    for idx in 0..min_len {
        if original_parsed[idx] != reconstructed_parsed[idx] {
            info!(
                "  [{}] BYTE-DIFF: {:?} != {:?}",
                idx, original_parsed[idx], reconstructed_parsed[idx]
            );
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
        info!(
            "  Byte-parsed length mismatch: original={}, reconstructed={}",
            original_parsed.len(),
            reconstructed_parsed.len()
        );
        if reconstructed_parsed.len() > original_parsed.len() {
            info!("  Extra commands in reconstructed (first 10):");
            for (idx, cmd) in reconstructed_parsed
                .iter()
                .skip(original_parsed.len())
                .take(10)
                .enumerate()
            {
                info!("    [{}]: {:?}", original_parsed.len() + idx, cmd);
            }
        }
        else {
            info!("  Missing commands from reconstructed (first 10):");
            for (idx, cmd) in original_parsed
                .iter()
                .skip(reconstructed_parsed.len())
                .take(10)
                .enumerate()
            {
                info!("    [{}]: {:?}", reconstructed_parsed.len() + idx, cmd);
            }
        }
    }

    // Step 7.4: Direct command comparison
    info!("\n=== DIRECT COMMAND COMPARISON ===");
    info!("  Original: {} commands", generated_char_commands.len());
    info!("  Reconstructed: {} commands", reconstructed_commands.len());

    // Find differences in commands
    let mut diff_count = 0;
    let cmd_min_len = generated_char_commands
        .len()
        .min(reconstructed_commands.len());

    for idx in 0..cmd_min_len {
        if generated_char_commands[idx] != reconstructed_commands[idx] {
            info!(
                "  [{}] CMD-DIFF: {:?} != {:?}",
                idx, generated_char_commands[idx], reconstructed_commands[idx]
            );
            diff_count += 1;
            if diff_count >= 30 {
                info!("  ... (showing first 30 command differences)");
                break;
            }
        }
    }

    if diff_count == 0 && generated_char_commands.len() == reconstructed_commands.len() {
        info!("  ✓ All commands match perfectly!");
    }

    if generated_char_commands.len() != reconstructed_commands.len() {
        info!(
            "  Command length mismatch! Original has {} extra, Reconstructed has {} extra",
            generated_char_commands
                .len()
                .saturating_sub(reconstructed_commands.len()),
            reconstructed_commands
                .len()
                .saturating_sub(generated_char_commands.len())
        );

        // Show what's missing or extra
        if reconstructed_commands.len() > generated_char_commands.len() {
            info!("  Extra commands in reconstructed (showing first 10):");
            for (idx, cmd) in reconstructed_commands
                .iter()
                .skip(generated_char_commands.len())
                .take(10)
                .enumerate()
            {
                info!("    [{}]: {:?}", generated_char_commands.len() + idx, cmd);
            }
        }
        else {
            info!("  Missing commands from reconstructed (showing first 10):");
            for (idx, cmd) in generated_char_commands
                .iter()
                .skip(reconstructed_commands.len())
                .take(10)
                .enumerate()
            {
                info!("    [{}]: {:?}", reconstructed_commands.len() + idx, cmd);
            }
        }
    }

    let mut reconstructed_interpreter = Interpreter::builder()
        .as_6128(true)
        .screen_mode(Mode::Mode1)
        .locale(locale)
        .build();
    reconstructed_interpreter
        .interpret(&reconstructed_commands, false)
        .map_err(|e| format!("Failed to interpret reconstructed commands: {}", e))?;
    let reconstructed_screen = reconstructed_interpreter.screen().clone();
    let reconstructed_palette = reconstructed_interpreter.palette().clone();

    // Step 7.5: Compare screens and display diff if different
    if screens_are_equal(&original_screen, &reconstructed_screen) {
        info!("✓ Screens are identical!");
        println!("\n=== Reconstructed CatArt Output ===");
        // Use the same code path as test_crtc_catart
        match catalog_to_basic_listing(catalog_bytes, CatalogType::Cat) {
            Ok(catalog_basic_program) => {
                match BasicCommandList::try_from(&catalog_basic_program) {
                    Ok(catalog_basic_command_list) => {
                        match catalog_basic_command_list.to_char_commands() {
                            Ok(commands) => {
                                println!("{}", commands);
                            },
                            Err(e) => {
                                eprintln!(
                                    "Warning: Failed to convert BASIC to CharCommandList: {:?}",
                                    e
                                );
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to convert BASIC program to BasicCommandList: {:?}",
                            e
                        );
                    }
                }
            },
            Err(e) => {
                eprintln!(
                    "Warning: Failed to extract BASIC program from catalog: {}",
                    e
                );
            }
        }
    }
    else {
        info!("⚠ Screens differ! Displaying side-by-side comparison:");
        println!("\n=== CatArt Comparison (Screens Differ!) ===");
        let diff_display = display_screen_diff(
            &original_screen,
            &original_palette,
            &reconstructed_screen,
            &reconstructed_palette
        );
        println!("{}", diff_display);
    }

    // Step 8: Display the generated BASIC program and compare its bytes
    info!("Displaying generated BASIC program from catalog:");
    println!("\n=== Reconstructed BASIC Program ===");
    match catalog_to_basic_listing(catalog_bytes, CatalogType::Cat) {
        Ok(reconstructed_basic) => {
            println!("\n{}", reconstructed_basic);

            // Convert to CharCommandList for byte comparison
            info!("\n=== BASIC PROGRAM BYTE COMPARISON ===");
            match basic_to_char_commands(&reconstructed_basic) {
                Ok(reconstructed_basic_commands) => {
                    // Get bytes from both original and reconstructed BASIC programs
                    let original_basic_bytes = generated_char_commands.bytes();
                    let reconstructed_basic_bytes = reconstructed_basic_commands.bytes();

                    info!(
                        "Original BASIC program: {} commands -> {} bytes",
                        generated_char_commands.len(),
                        original_basic_bytes.len()
                    );
                    info!(
                        "Reconstructed BASIC program: {} commands -> {} bytes",
                        reconstructed_basic_commands.len(),
                        reconstructed_basic_bytes.len()
                    );

                    // Compare byte arrays
                    if original_basic_bytes == reconstructed_basic_bytes {
                        info!("✓ BASIC program bytes match perfectly!");
                    }
                    else {
                        info!("⚠ BASIC program bytes differ!");
                        info!(
                            "  Byte length: original={}, reconstructed={}",
                            original_basic_bytes.len(),
                            reconstructed_basic_bytes.len()
                        );

                        // Find first difference
                        let min_len = original_basic_bytes
                            .len()
                            .min(reconstructed_basic_bytes.len());
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
                            info!(
                                "  Original bytes [{}..{}]: {:02X?}",
                                start,
                                end,
                                &original_basic_bytes[start..end]
                            );
                            info!(
                                "  Reconstructed bytes [{}..{}]: {:02X?}",
                                start,
                                end,
                                &reconstructed_basic_bytes[start..end]
                            );
                        }

                        // Show extra bytes if lengths differ
                        if reconstructed_basic_bytes.len() > original_basic_bytes.len() {
                            let extra_start = original_basic_bytes.len();
                            let extra_end = (extra_start + 20).min(reconstructed_basic_bytes.len());
                            info!(
                                "  Extra bytes in reconstructed [{}..{}]: {:02X?}",
                                extra_start,
                                extra_end,
                                &reconstructed_basic_bytes[extra_start..extra_end]
                            );
                        }
                        else if original_basic_bytes.len() > reconstructed_basic_bytes.len() {
                            let missing_start = reconstructed_basic_bytes.len();
                            let missing_end = (missing_start + 20).min(original_basic_bytes.len());
                            info!(
                                "  Missing bytes from reconstructed [{}..{}]: {:02X?}",
                                missing_start,
                                missing_end,
                                &original_basic_bytes[missing_start..missing_end]
                            );
                        }
                    }
                },
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to convert reconstructed BASIC to commands: {}",
                        e
                    );
                }
            }
        },
        Err(e) => {
            eprintln!("Warning: Failed to generate BASIC listing: {}", e);
        }
    }

    Ok(())
}

fn decode_catalog_command(catalog_fname: &str, output_path: Option<&str>) -> Result<(), String> {
    info!("Decoding catart from: {}", catalog_fname);

    // Load the raw catalog bytes
    let catalog_bytes = load_catalog_bytes(catalog_fname)?;

    match catalog_to_basic_listing(&catalog_bytes, CatalogType::Cat) {
        Ok(basic_program) => {
            if let Some(path) = output_path {
                let mut file = File::create(path)
                    .map_err(|e| format!("Failed to create output file: {}", e))?;
                write!(file, "{}", basic_program)
                    .map_err(|e| format!("Failed to write output file: {}", e))?;
                info!("Saved BASIC listing to {}", path);
            }
            else {
                println!("{}", basic_program);
            }
        },
        Err(e) => {
            error!("Failed to decode catalog: {}", e);
            return Err(format!("Failed to decode catalog: {}", e));
        }
    }

    Ok(())
}
