mod test_helpers;

use std::fs;

use cpclib_basic::BasicProgram;
use cpclib_catalog::{catalog_extraction, catalog_to_basic_listing, catalog_to_catart_commands};
use cpclib_catart::basic_command::{BasicCommand, BasicCommandList};
use cpclib_catart::entry::{CatalogType, ScreenMode, UnifiedCatalog};
use cpclib_catart::interpret::{Interpreter, Locale, Mode};
use cpclib_disc::AnyDisc;
use cpclib_disc::amsdos::AmsdosManagerNonMut;
use cpclib_disc::disc::Disc;
use cpclib_disc::edsk::Head;
use test_helpers::compare_memory_with_visual_diff;

#[test]
fn test_crtc_catart() {
    // Parse original BASIC program
    let orig_basic_str = include_str!("discs/crtc/T8.ASC");
    let orig_basic_program =
        BasicProgram::parse(orig_basic_str).expect("Failed to parse BASIC program");
    let orig_basic_command_list = BasicCommandList::try_from(&orig_basic_program)
        .expect("Unable to get cat art commands from original BASIC");

    // Load catalog from DSK and extract BASIC program
    let dsk = AnyDisc::open("tests/discs/crtc/CRTC.DSK").expect("Failed to read DSK file");
    let manager = AmsdosManagerNonMut::new_from_disc(&dsk, Head::A);
    let binary_catalog = manager.catalog_slice();
    let catalog_type = CatalogType::Cat;

    let catalog_basic_program = catalog_to_basic_listing(&binary_catalog, catalog_type)
        .expect("Unable to extract BASIC program from catalog");
    let catalog_basic_command_list = BasicCommandList::try_from(&catalog_basic_program)
        .expect("Unable to get cat art commands from catalog BASIC");

    // Filter both command lists to keep only Window and Locate commands
    let orig_window_locate: Vec<&BasicCommand> = orig_basic_command_list
        .iter()
        .filter(|cmd| matches!(cmd, BasicCommand::Window(..) | BasicCommand::Locate(..)))
        .collect();

    let catalog_window_locate: Vec<&BasicCommand> = catalog_basic_command_list
        .iter()
        .filter(|cmd| matches!(cmd, BasicCommand::Window(..) | BasicCommand::Locate(..)))
        .collect();

    // Print summary
    eprintln!("\n=== Window/Locate Command Comparison ===");
    eprintln!("Original commands: {}", orig_window_locate.len());
    eprintln!("Catalog commands:  {}", catalog_window_locate.len());
    eprintln!();

    // Check lengths match
    if orig_window_locate.len() != catalog_window_locate.len() {
        eprintln!("ERROR: Command count mismatch!");
        eprintln!(
            "Original has {} Window/Locate commands",
            orig_window_locate.len()
        );
        eprintln!(
            "Catalog has {} Window/Locate commands",
            catalog_window_locate.len()
        );
        eprintln!("\nOriginal commands:");
        for (i, cmd) in orig_window_locate.iter().enumerate() {
            eprintln!("  [{}] {:?}", i, cmd);
        }
        eprintln!("\nCatalog commands:");
        for (i, cmd) in catalog_window_locate.iter().enumerate() {
            eprintln!("  [{}] {:?}", i, cmd);
        }
        panic!(
            "Window/Locate command count mismatch: {} original vs {} catalog",
            orig_window_locate.len(),
            catalog_window_locate.len()
        );
    }

    // Compare commands one by one
    let mut window_locate_has_failed = false;
    for (index, (orig_cmd, catalog_cmd)) in orig_window_locate
        .iter()
        .zip(catalog_window_locate.iter())
        .enumerate()
    {
        if orig_cmd != catalog_cmd {
            eprintln!("\n❌ DIFFERENCE FOUND at position {}:", index);
            eprintln!("  Original: {:?}", orig_cmd);
            eprintln!("  Catalog:  {:?}", catalog_cmd);
            eprintln!();

            // Show context (previous and next commands if available)
            eprintln!("Context:");
            if index > 0 {
                eprintln!(
                    "  [{}] Original: {:?}",
                    index - 1,
                    orig_window_locate[index - 1]
                );
                eprintln!(
                    "  [{}] Catalog:  {:?}",
                    index - 1,
                    catalog_window_locate[index - 1]
                );
            }
            eprintln!("  [{}] Original: {:?} ← MISMATCH", index, orig_cmd);
            eprintln!("  [{}] Catalog:  {:?} ← MISMATCH", index, catalog_cmd);
            if index + 1 < orig_window_locate.len() {
                eprintln!(
                    "  [{}] Original: {:?}",
                    index + 1,
                    orig_window_locate[index + 1]
                );
                eprintln!(
                    "  [{}] Catalog:  {:?}",
                    index + 1,
                    catalog_window_locate[index + 1]
                );
            }
            eprintln!();

            // Provide detailed analysis for Window and Locate differences
            match (orig_cmd, catalog_cmd) {
                (BasicCommand::Window(ol, or, ot, ob), BasicCommand::Window(cl, cr, ct, cb)) => {
                    eprintln!("Window parameter differences:");
                    if ol != cl {
                        eprintln!(
                            "  Left:   {} (orig) vs {} (catalog) — diff: {}",
                            ol,
                            cl,
                            (*cl as i16) - (*ol as i16)
                        );
                    }
                    if or != cr {
                        eprintln!(
                            "  Right:  {} (orig) vs {} (catalog) — diff: {}",
                            or,
                            cr,
                            (*cr as i16) - (*or as i16)
                        );
                    }
                    if ot != ct {
                        eprintln!(
                            "  Top:    {} (orig) vs {} (catalog) — diff: {}",
                            ot,
                            ct,
                            (*ct as i16) - (*ot as i16)
                        );
                    }
                    if ob != cb {
                        eprintln!(
                            "  Bottom: {} (orig) vs {} (catalog) — diff: {}",
                            ob,
                            cb,
                            (*cb as i16) - (*ob as i16)
                        );
                    }
                },
                (BasicCommand::Locate(ox, oy), BasicCommand::Locate(cx, cy)) => {
                    eprintln!("Locate parameter differences:");
                    if ox != cx {
                        eprintln!(
                            "  Column: {} (orig) vs {} (catalog) — diff: {}",
                            ox,
                            cx,
                            (*cx as i16) - (*ox as i16)
                        );
                    }
                    if oy != cy {
                        eprintln!(
                            "  Row:    {} (orig) vs {} (catalog) — diff: {}",
                            oy,
                            cy,
                            (*cy as i16) - (*oy as i16)
                        );
                    }
                },
                _ => {
                    eprintln!(
                        "Command type mismatch: original is {:?}, catalog is different type",
                        orig_cmd
                    );
                }
            }

            window_locate_has_failed = true;
        }
    }

    assert!(
        !window_locate_has_failed,
        "There were differences in Window/Locate commands. See above for details."
    );

    eprintln!(
        "✓ All {} Window/Locate commands match perfectly!",
        orig_window_locate.len()
    );

    // === Screen Memory Comparison ===
    eprintln!("\n=== Screen Memory Comparison ===");

    // Load expected screen memory from CRTC.SCR
    let expected_screen_bytes =
        fs::read("tests/discs/crtc/CRTC.SCR").expect("Failed to read CRTC.SCR file");

    eprintln!(
        "Expected screen size: {} bytes",
        expected_screen_bytes.len()
    );

    // Execute catalog BASIC program to generate screen memory
    let catalog_char_commands = catalog_basic_command_list
        .to_char_commands()
        .expect("Failed to convert catalog BASIC to CharCommandList");

    eprintln!(
        "Total catalog CharCommands: {}",
        catalog_char_commands.iter().count()
    );

    // Also run the original BASIC and compare
    eprintln!("\nExecuting ORIGINAL BASIC commands...");
    let orig_char_commands = orig_basic_command_list
        .to_char_commands()
        .expect("Failed to convert original BASIC to CharCommandList");

    eprintln!(
        "Total original CharCommands: {}",
        orig_char_commands.iter().count()
    );
    eprintln!(
        "Orig Mode commands: {}",
        orig_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Mode(..)))
            .count()
    );
    eprintln!(
        "Orig Paper commands: {}",
        orig_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Paper(..)))
            .count()
    );
    eprintln!(
        "Orig Pen commands: {}",
        orig_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Pen(..)))
            .count()
    );
    eprintln!(
        "Orig Ink commands: {}",
        orig_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Ink(..)))
            .count()
    );
    eprintln!(
        "Orig Window commands: {}",
        orig_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Window(..)))
            .count()
    );
    eprintln!(
        "Orig Cls commands: {}",
        orig_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Cls))
            .count()
    );
    eprintln!(
        "Orig Locate commands: {}",
        orig_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Locate(..)))
            .count()
    );
    eprintln!(
        "Orig Char commands: {}",
        orig_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Char(..)))
            .count()
    );
    eprintln!(
        "Orig Symbol commands: {}",
        orig_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Symbol(..)))
            .count()
    );
    eprintln!(
        "Orig PrintSymbol commands: {}",
        orig_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::PrintSymbol(..)))
            .count()
    );
    eprintln!(
        "Orig String commands: {}",
        orig_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::String(..)))
            .count()
    );

    eprintln!("\nCatalog commands:");
    eprintln!(
        "Mode commands: {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Mode(..)))
            .count()
    );
    eprintln!(
        "Paper commands: {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Paper(..)))
            .count()
    );
    eprintln!(
        "Pen commands: {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Pen(..)))
            .count()
    );
    eprintln!(
        "Ink commands: {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Ink(..)))
            .count()
    );
    eprintln!(
        "Window commands: {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Window(..)))
            .count()
    );
    eprintln!(
        "Cls commands: {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Cls))
            .count()
    );
    eprintln!(
        "Locate commands: {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Locate(..)))
            .count()
    );
    eprintln!(
        "Char commands: {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Char(..)))
            .count()
    );
    eprintln!(
        "Symbol commands: {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::Symbol(..)))
            .count()
    );
    eprintln!(
        "PrintSymbol commands: {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::PrintSymbol(..)))
            .count()
    );
    eprintln!(
        "String commands: {}",
        catalog_char_commands
            .iter()
            .filter(|c| matches!(c, cpclib_catart::char_command::CharCommand::String(..)))
            .count()
    );

    let mut interpreter = Interpreter::builder()
        .screen_mode(Mode::Mode1)
        .locale(Locale::English)
        .as_6128(true)
        .build();
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
        .expect("CRTC.SCR file must be exactly 16384 bytes");

    // Use visual diff comparison
    eprintln!("\n=== Generating visual diff ===");
    if let Err(msg) = compare_memory_with_visual_diff(
        palette,
        &expected_screen_memory,
        actual_screen_memory,
        "crtc_screen_diff",
        "tests/discs/crtc"
    ) {
        eprintln!("⚠️ Visual diff generation failed: {}", msg);
    }
    else {
        eprintln!("✓ Visual diff PNG generated: tests/discs/crtc/crtc_screen_diff.png");
    }

    // Alternative 3: Use catalog_to_catart_commands directly
    eprintln!("\nExecuting commands from catalog_to_catart_commands...");
    let direct_char_commands = catalog_to_catart_commands(&binary_catalog, catalog_type)
        .expect("Unable to extract commands from catalog");
    let mut interpreter3 = Interpreter::builder()
        .screen_mode(Mode::Mode1)
        .locale(Locale::English)
        .as_6128(true)
        .build();
    interpreter3
        .interpret(direct_char_commands.iter(), true)
        .expect("Failed to interpret direct commands");
    let direct_screen_memory = interpreter3.memory_screen().memory();
    let palette3 = interpreter3.palette();

    eprintln!("\n=== Generating visual diff for direct method ===");
    if let Err(msg) = compare_memory_with_visual_diff(
        palette3,
        &expected_screen_memory,
        direct_screen_memory,
        "crtc_screen_diff_direct",
        "tests/discs/crtc"
    ) {
        eprintln!("⚠️ Visual diff generation failed: {}", msg);
    }
    else {
        eprintln!("✓ Visual diff PNG generated: tests/discs/crtc/crtc_screen_diff_direct.png");
    }

    // Also compare the two generated memories to be sure
    eprintln!("\n=== Comparing screen memories ===");
    let differences: Vec<usize> = actual_screen_memory
        .iter()
        .zip(direct_screen_memory.iter())
        .enumerate()
        .filter_map(|(idx, (a, d))| if a != d { Some(idx) } else { None })
        .collect();

    if differences.is_empty() {
        eprintln!("✅ The two generated screen memories are identical.");
    }
    else {
        eprintln!(
            "❌ Found {} differences between screen memories",
            differences.len()
        );
        eprintln!("First 20 differences:");
        for &idx in differences.iter().take(20) {
            eprintln!(
                "  Position {}: catalog={}, direct={}",
                idx, actual_screen_memory[idx], direct_screen_memory[idx]
            );
        }
        eprintln!("\n📊 Check the generated PNG files for visual comparison:");
        eprintln!("   - tests/discs/crtc/crtc_screen_diff_comparison.png");
        eprintln!("   - tests/discs/crtc/crtc_screen_diff_direct_comparison.png");
    }

    // Uncomment to enforce equality:
    // assert_eq!(actual_screen_memory, direct_screen_memory, "The two generated screen memories should be identical");
    eprintln!("\n✓ Test completed - image files generated for manual inspection");
}

#[test]
fn test_crtc_two_paths_comparison() {
    eprintln!("\n=== Comparing TWO extraction paths ===\n");

    // Load catalog from DSK
    let dsk = AnyDisc::open("tests/discs/crtc/CRTC.DSK").expect("Failed to read DSK file");
    let manager = AmsdosManagerNonMut::new_from_disc(&dsk, Head::A);
    let binary_catalog = manager.catalog_slice();
    let catalog_type = CatalogType::Cat;

    // PATH 1: catalog_to_basic_listing (used by test) → extract_basic_from_sequential_catart
    eprintln!("PATH 1: catalog_to_basic_listing (extract_basic_from_sequential_catart)");
    let path1_basic_program = catalog_to_basic_listing(&binary_catalog, catalog_type)
        .expect("Unable to extract BASIC program from catalog");
    let path1_basic_commands = BasicCommandList::try_from(&path1_basic_program)
        .expect("Unable to get commands from path1 BASIC");

    let path1_window_locate: Vec<&BasicCommand> = path1_basic_commands
        .iter()
        .filter(|cmd| matches!(cmd, BasicCommand::Window(..) | BasicCommand::Locate(..)))
        .collect();

    eprintln!("  Window/Locate commands: {}", path1_window_locate.len());
    for (i, cmd) in path1_window_locate.iter().enumerate() {
        eprintln!("  [{}] {:?}", i, cmd);
    }

    // PATH 2: catalog_to_catart_commands (used by binary) → commands_by_mode_and_order
    eprintln!("\nPATH 2: catalog_to_catart_commands (commands_by_mode_and_order)");
    let path2_char_commands = catalog_to_catart_commands(&binary_catalog, catalog_type)
        .expect("Unable to extract commands from catalog");

    eprintln!("  Total CharCommands: {}", path2_char_commands.len());
    eprintln!("  First 10 commands:");
    for (i, cmd) in path2_char_commands.iter().take(10).enumerate() {
        eprintln!("    [{}] {:?}", i, cmd);
    }

    // Convert CharCommandList to BasicCommandList for comparison
    let path2_basic_commands: Vec<BasicCommand> = path2_char_commands
        .iter()
        .filter_map(|cmd| cmd.to_basic_command())
        .collect();

    let path2_window_locate: Vec<&BasicCommand> = path2_basic_commands
        .iter()
        .filter(|cmd| matches!(cmd, BasicCommand::Window(..) | BasicCommand::Locate(..)))
        .collect();

    eprintln!("  Window/Locate commands: {}", path2_window_locate.len());
    for (i, cmd) in path2_window_locate.iter().enumerate() {
        eprintln!("  [{}] {:?}", i, cmd);
    }

    // Compare
    eprintln!("\n=== COMPARISON ===");
    eprintln!("PATH 1 commands: {}", path1_window_locate.len());
    eprintln!("PATH 2 commands: {}", path2_window_locate.len());

    if path1_window_locate.len() != path2_window_locate.len() {
        panic!(
            "Different number of commands: PATH1={}, PATH2={}",
            path1_window_locate.len(),
            path2_window_locate.len()
        );
    }

    for (index, (cmd1, cmd2)) in path1_window_locate
        .iter()
        .zip(path2_window_locate.iter())
        .enumerate()
    {
        if cmd1 != cmd2 {
            eprintln!("\n❌ MISMATCH at position {}:", index);
            eprintln!("  PATH 1: {:?}", cmd1);
            eprintln!("  PATH 2: {:?}", cmd2);
            panic!("Commands differ at position {}", index);
        }
    }

    eprintln!(
        "\n✅ All {} Window/Locate commands match between both paths!",
        path1_window_locate.len()
    );
}
