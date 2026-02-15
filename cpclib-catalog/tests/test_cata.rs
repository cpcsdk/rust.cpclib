mod test_helpers;

use std::fs;

use cpclib_catalog::{catalog_to_basic_listing, catalog_to_catart_commands};
use cpclib_catart::basic_command::{BasicCommand, BasicCommandList};
use cpclib_catart::entry::CatalogType;
use cpclib_catart::interpret::{Interpreter, Locale, Mode};
use cpclib_disc::AnyDisc;
use cpclib_disc::amsdos::AmsdosManagerNonMut;
use cpclib_disc::disc::Disc;
use cpclib_disc::edsk::Head;
use test_helpers::compare_memory_with_visual_diff;

#[test]
fn test_cata_catart() {
    eprintln!("\n=== CATA.DSK Catalog Test ===\n");

    // Load catalog from DSK and extract BASIC program
    let dsk = AnyDisc::open("tests/discs/CATA/CATA.DSK").expect("Failed to read CATA.DSK file");
    let manager = AmsdosManagerNonMut::new_from_disc(&dsk, Head::A);
    let binary_catalog = manager.catalog_slice();

    eprintln!("Binary catalog size: {} bytes", binary_catalog.len());

    // Test with CAT (not DIR) - adjust if needed
    let catalog_type = CatalogType::Cat;
    eprintln!("Catalog type: {:?}", catalog_type);

    let catalog_basic_program = catalog_to_basic_listing(&binary_catalog, catalog_type)
        .expect("Unable to extract BASIC program from catalog");

    eprintln!("BASIC program extracted successfully");

    let catalog_basic_command_list = BasicCommandList::try_from(&catalog_basic_program)
        .expect("Unable to get cat art commands from catalog BASIC");

    eprintln!(
        "BASIC commands parsed: {} commands",
        catalog_basic_command_list.iter().count()
    );

    // Print command summary
    eprintln!("\n=== BASIC Command Summary ===");
    let mode_cmds = catalog_basic_command_list
        .iter()
        .filter(|c| matches!(c, BasicCommand::Mode(..)))
        .count();
    let paper_cmds = catalog_basic_command_list
        .iter()
        .filter(|c| matches!(c, BasicCommand::Paper(..)))
        .count();
    let pen_cmds = catalog_basic_command_list
        .iter()
        .filter(|c| matches!(c, BasicCommand::Pen(..)))
        .count();
    let ink_cmds = catalog_basic_command_list
        .iter()
        .filter(|c| matches!(c, BasicCommand::Ink(..)))
        .count();
    let window_cmds = catalog_basic_command_list
        .iter()
        .filter(|c| matches!(c, BasicCommand::Window(..)))
        .count();
    let cls_cmds = catalog_basic_command_list
        .iter()
        .filter(|c| matches!(c, BasicCommand::Cls))
        .count();
    let locate_cmds = catalog_basic_command_list
        .iter()
        .filter(|c| matches!(c, BasicCommand::Locate(..)))
        .count();
    let symbol_cmds = catalog_basic_command_list
        .iter()
        .filter(|c| matches!(c, BasicCommand::Symbol(..)))
        .count();

    eprintln!("  Mode commands:        {}", mode_cmds);
    eprintln!("  Paper commands:       {}", paper_cmds);
    eprintln!("  Pen commands:         {}", pen_cmds);
    eprintln!("  Ink commands:         {}", ink_cmds);
    eprintln!("  Window commands:      {}", window_cmds);
    eprintln!("  Cls commands:         {}", cls_cmds);
    eprintln!("  Locate commands:      {}", locate_cmds);
    eprintln!("  Symbol commands:      {}", symbol_cmds);

    // Print first 10 Window/Locate commands for debugging
    eprintln!("\n=== First 10 Window/Locate Commands ===");
    let window_locate: Vec<&BasicCommand> = catalog_basic_command_list
        .iter()
        .filter(|cmd| matches!(cmd, BasicCommand::Window(..) | BasicCommand::Locate(..)))
        .collect();

    for (i, cmd) in window_locate.iter().take(10).enumerate() {
        eprintln!("  [{}] {:?}", i, cmd);
    }
    eprintln!(
        "  ... (total {} Window/Locate commands)",
        window_locate.len()
    );

    // === Screen Memory Comparison ===
    eprintln!("\n=== Screen Memory Comparison ===");

    // Load expected screen memory from CATA.SCR
    let expected_screen_bytes =
        fs::read("tests/discs/CATA/CATA.SCR").expect("Failed to read CATA.SCR file");

    eprintln!(
        "Expected screen size: {} bytes",
        expected_screen_bytes.len()
    );

    // Execute catalog BASIC program to generate screen memory
    let catalog_char_commands = catalog_to_catart_commands(&binary_catalog, CatalogType::Cat).expect("Unable to build the catalog");

    eprintln!(
        "Total catalog CharCommands: {}",
        catalog_char_commands.iter().count()
    );

    eprintln!("\n=== CharCommand Summary ===");
    eprintln!(
        "Mode commands:        {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Mode(..)))
            .count()
    );
    eprintln!(
        "Paper commands:       {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Paper(..)))
            .count()
    );
    eprintln!(
        "Pen commands:         {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Pen(..)))
            .count()
    );
    eprintln!(
        "Ink commands:         {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Ink(..)))
            .count()
    );
    eprintln!(
        "Window commands:      {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Window(..)))
            .count()
    );
    eprintln!(
        "Cls commands:         {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Cls))
            .count()
    );
    eprintln!(
        "Locate commands:      {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Locate(..)))
            .count()
    );
    eprintln!(
        "Char commands:        {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Char(..)))
            .count()
    );
    eprintln!(
        "Symbol commands:      {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Symbol(..)))
            .count()
    );

    // Initialize as CPC 6128 with startup message as user requested
    let mut interpreter = Interpreter::new_6128();

    
    interpreter
        .interpret(catalog_char_commands.iter(), true)
        .expect("Failed to interpret catalog BASIC commands");
    let palette = interpreter.palette();

    eprintln!("\nFinal palette state: {:?}", palette);

    let actual_screen_memory = interpreter.memory_screen().memory();
    eprintln!("Actual screen size: {} bytes", actual_screen_memory.len());

    // Convert expected bytes (Vec<u8>) to fixed-size array
    let expected_screen_memory: [u8; 16384] = expected_screen_bytes
        .try_into()
        .expect("CATA.SCR file must be exactly 16384 bytes");

    // Use visual diff comparison
    eprintln!("\n=== Generating visual diff ===");
    if let Err(msg) = compare_memory_with_visual_diff(
        palette,
        &expected_screen_memory,
        actual_screen_memory,
        "cata_screen_diff",
        "tests/discs/CATA"
    ) {
        eprintln!("{}", msg);
        panic!("Screen memory mismatch - see PNG file for visual comparison");
    }


    
    eprintln!("\n✅ Test passed - screen memory matches perfectly!");
}
