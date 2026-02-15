use std::fs;
use std::path::PathBuf;

use cpclib_catart::entry::{
    Catalog, CatalogType, PrintableEntry, PrintableEntryFileName, ScreenMode, UnifiedCatalog
};

/// Helper to create PrintableEntryFileName from ASCII string (for testing)
fn make_fname(name: &str) -> PrintableEntryFileName {
    let parts: Vec<&str> = name.split('.').collect();
    let filename = parts[0].as_bytes();
    let extension = if parts.len() > 1 {
        parts[1].as_bytes()
    }
    else {
        &[]
    };

    let mut f = [b' '; 8];
    let mut e = [b' '; 3];

    for (i, &b) in filename.iter().take(8).enumerate() {
        f[i] = b;
    }
    for (i, &b) in extension.iter().take(3).enumerate() {
        e[i] = b;
    }

    PrintableEntryFileName {
        f1: f[0],
        f2: f[1],
        f3: f[2],
        f4: f[3],
        f5: f[4],
        f6: f[5],
        f7: f[6],
        f8: f[7],
        e1: e[0],
        e2: e[1],
        e3: e[2]
    }
}

#[test]
fn test_simple_entries_distribution() {
    println!("\n=== Testing Grid Distribution with Simple Entries ===");

    // Create mock entries A through F  (alphabetically sorted)
    let entries: Vec<PrintableEntry> = (b'A'..=b'F')
        .map(|c| {
            let mut name = String::from(c as char);
            name.push_str(".BAS");
            PrintableEntry {
                user: 0,
                fname: make_fname(&name),
                pieces: [0; 4],
                sectors: [0; 16]
            }
        })
        .collect();

    println!("\n--- Input Entries (Alphabetical) ---");
    for (i, entry) in entries.iter().enumerate() {
        let fname_str = entry.fname.display_name();
        println!("{}: {}", i, fname_str);
    }

    // Create catalog and unified catalog
    let catalog = Catalog::try_from(entries.as_slice()).expect("Failed to create catalog");
    let unified_catalog = UnifiedCatalog::from(catalog);

    // Check what sorted_entries returns
    println!("\n=== Sorted Entries for CAT ===");
    let sorted = unified_catalog.visible_sorted_entries(CatalogType::Cat);
    for (i, entry) in sorted.iter().enumerate() {
        let bytes = entry.fname().all_generated_bytes();
        let display = entry.fname().display_name();
        println!("{}: {} (bytes: {:?})", i, display, bytes);
    }

    // Test Mode 1 (2 columns) grid creation with CAT type
    println!("\n=== CAT: Mode 1 (2 columns) Grid Distribution ===");
    let mode1_grid =
        unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode1, CatalogType::Cat);

    println!(
        "Grid dimensions: {} columns, {} max rows",
        mode1_grid.num_columns(),
        mode1_grid.max_num_rows()
    );

    // Show how entries are distributed in columns
    for col in 0..mode1_grid.num_columns() {
        println!("\n--- Column {} ---", col);
        if let Some(column) = mode1_grid.column(col) {
            for (row, entry) in column.iter().enumerate() {
                let fname_bytes = entry.fname().all_generated_bytes();
                let fname_str = String::from_utf8_lossy(&fname_bytes);
                println!("  Row {}: {}", row, fname_str.trim());
            }
        }
    }

    // Show display order (how it would appear on screen reading left-to-right)
    println!("\n=== Actual Display Order (Row-by-Row, Left-to-Right) ===");
    let displayed: Vec<String> = mode1_grid
        .entries_display_order()
        .map(|entry| {
            let fname_bytes = entry.fname().all_generated_bytes();
            String::from_utf8_lossy(&fname_bytes).trim().to_string()
        })
        .collect();

    for (i, name) in displayed.iter().enumerate() {
        let row = i / mode1_grid.num_columns();
        let col = i % mode1_grid.num_columns();
        println!("Position {} (row {}, col {}): {}", i, row, col, name);
    }

    // Expected behavior: CAT fills columns VERTICALLY, so after alphabetical sorting:
    // Sorted: [A, B, C, D, E, F]
    // Column 0 gets first half: A, B, C
    // Column 1 gets second half: D, E, F
    // Display reads left-to-right: A, D, B, E, C, F
    println!("\n=== Expected Behavior for CAT (Vertical Column Fill) ===");
    println!("CAT sorts entries alphabetically, then fills columns VERTICALLY:");
    println!("Sorted order: A, B, C, D, E, F");
    println!("Column 0 (rows 0-2): A, B, C");
    println!("Column 1 (rows 0-2): D, E, F");
    println!("Display (reading left-to-right, top-to-bottom):");
    println!("  Row 0: A    D");
    println!("  Row 1: B    E");
    println!("  Row 2: C    F");

    // Verification - the sorted order should be alphabetical
    println!("\n=== Verification: Check Sorted Order ===");
    let sorted_for_check = unified_catalog.visible_sorted_entries(CatalogType::Cat);
    let sorted_names: Vec<String> = sorted_for_check
        .iter()
        .map(|e| e.fname().display_name())
        .collect();

    let expected_sorted = vec!["A       .BAS", "B       .BAS", "C       .BAS", "D       .BAS", "E       .BAS", "F       .BAS"];
    println!("Sorted entries: {:?}", sorted_names);
    println!("Expected sorted: {:?}", expected_sorted);

    let sort_correct = sorted_names
        .iter()
        .zip(expected_sorted.iter())
        .all(|(a, b)| a.trim() == *b);

    if !sort_correct {
        println!("\n❌ BUG: Sorting is incorrect!");
        println!("The entries are not being sorted alphabetically.");
        panic!("Sorting bug detected!");
    }

    // Now verify the display order matches the vertical fill pattern
    let expected_display = vec!["A       .BAS", "D       .BAS", "B       .BAS", "E       .BAS", "C       .BAS", "F       .BAS"];
    println!("\n=== Verification: Check Display Order (After Vertical Fill) ===");
    println!("Expected display: {:?}", expected_display);
    println!(
        "Actual display: {:?}",
        displayed.iter().map(|s| s.trim()).collect::<Vec<_>>()
    );

    let mut all_match = true;
    for (i, (actual, expected)) in displayed.iter().zip(expected_display.iter()).enumerate() {
        let actual_trimmed = actual.trim();
        if actual_trimmed != *expected {
            println!(
                "❌ Position {}: Got '{}', Expected '{}'",
                i, actual_trimmed, expected
            );
            all_match = false;
        }
        else {
            println!("✅ Position {}: {}", i, actual_trimmed);
        }
    }

    if all_match {
        println!("\n✅ All entries match expected order!");
    }
    else {
        println!("\n❌ BUG DETECTED: Display order does NOT match expected vertical fill!");
        panic!("Grid distribution bug detected!");
    }
}

#[test]
fn test_blocus_manual_ordering() {
    println!("\n=== BLOCUS Manual Entry Ordering Test ===");

    // Load the BLOCUS catalog using raw DSK parsing
    let dsk_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("cpclib-catalog")
        .join("tests")
        .join("discs")
        .join("BLOCUS")
        .join("BLOCUS.DSK");

    if !dsk_path.exists() {
        println!("BLOCUS.DSK not found, skipping test");
        return;
    }

    println!("Reading BLOCUS.DSK from: {}", dsk_path.display());
    let dsk_bytes = fs::read(&dsk_path).expect("Failed to read BLOCUS.DSK");

    // Extract catalog bytes from DSK file
    // Standard DSK format has catalog in Track 0, starting at sector data
    // DIB is 256 bytes, then track data begins
    // Each track has a Track Information Block followed by sector data
    // For a standard CPC disk, catalog is in Track 0, sectors C1-C4

    // Simplified: Look for the catalog pattern
    // Catalog entries start after the disk header
    // Let's try to find it by looking for the pattern of entries

    println!("\nSearching for catalog data in DSK...");
    println!("DSK size: {} bytes", dsk_bytes.len());

    // Try different offsets to find the catalog
    let possible_offsets = vec![
        0x100, // After 256-byte header
        0x200, // After track info
        0x300, // Another common offset
    ];

    for &offset in &possible_offsets {
        if offset + 2048 > dsk_bytes.len() {
            continue;
        }

        println!("\nTrying offset 0x{:04X}:", offset);
        let candidate_catalog = &dsk_bytes[offset..offset + 2048];

        // Check if this looks like a catalog (count non-empty entries)
        let mut non_empty = 0;
        for i in 0..64 {
            let entry_offset = i * 32;
            let status = candidate_catalog[entry_offset];
            if status != 0xE5 {
                non_empty += 1;
            }
        }

        println!("  Found {} non-empty entries", non_empty);

        if non_empty > 5 {
            println!("\n=== Likely Catalog Found at 0x{:04X} ===", offset);

            // Print entries
            for i in 0..64 {
                let entry_offset = i * 32;
                let entry_bytes = &candidate_catalog[entry_offset..entry_offset + 32];
                let status = entry_bytes[0];

                if status != 0xE5 {
                    // Get filename bytes (bytes 1-11)
                    let name_bytes: Vec<u8> = entry_bytes[1..12].to_vec();

                    println!("\nEntry {:2} (offset 0x{:04X}):", i, offset + entry_offset);
                    println!("  Status: 0x{:02X}", status);
                    println!("  Name bytes: {:02X?}", name_bytes);

                    // Try to interpret as text/commands
                    let as_chars: String = name_bytes
                        .iter()
                        .map(|&b| {
                            if b >= 32 && b < 127 {
                                format!("{}", b as char)
                            }
                            else {
                                format!("[{:02X}]", b)
                            }
                        })
                        .collect();
                    println!("  Interpreted: {}", as_chars);

                    // Check if it's a catart entry (starts with control codes)
                    if name_bytes[0] < 0x20 {
                        println!("  -> Likely catart entry (starts with control code)");
                    }
                }
            }

            println!("\n=== Creating Catalog from Raw Bytes ===");
            let entries_vec: Vec<PrintableEntry> = (0..64)
                .map(|i| {
                    let entry_offset = i * 32;
                    let entry_bytes = &candidate_catalog[entry_offset..entry_offset + 32];

                    PrintableEntry {
                        user: entry_bytes[0],
                        fname: PrintableEntryFileName {
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
                        },
                        pieces: [
                            entry_bytes[12],
                            entry_bytes[13],
                            entry_bytes[14],
                            entry_bytes[15]
                        ],
                        sectors: entry_bytes[16..32].try_into().unwrap()
                    }
                })
                .collect();

            let catalog =
                Catalog::try_from(entries_vec.as_slice()).expect("Failed to create catalog");
            let unified_catalog = UnifiedCatalog::from(catalog);

            let sorted = unified_catalog.visible_sorted_entries(CatalogType::Cat);
            println!("\n=== Sorted Entries (Alphabetical by Raw Bytes) ===");
            for (i, entry) in sorted.iter().enumerate() {
                if !entry.fname().is_empty() {
                    let bytes = entry.fname().all_generated_bytes();
                    let display = String::from_utf8_lossy(&bytes);
                    println!("{:2}: {}", i, display);
                }
            }

            println!("\n=== Entries in Disk Order (as stored) ===");
            for (i, entry) in unified_catalog.visible_entries().enumerate() {
                if !entry.fname().is_empty() {
                    let bytes = entry.fname().all_generated_bytes();
                    let display = String::from_utf8_lossy(&bytes);
                    println!("{:2}: {} (user: 0x{:02X})", i, display, entry.user);
                }
            }

            println!("\n=== Analysis ===");
            println!("Notice: The first byte of each entry name is used as part of the sort key.");
            println!(
                "Entries with user bytes like 0x21, 0x22, 0x23... (!, \", #...) are sequentially"
            );
            println!(
                "numbered in disk order, which suggests they should be displayed in that order"
            );
            println!("to form coherent text, NOT alphabetically sorted!");

            break;
        }
    }
}

#[test]
fn test_sorting_with_control_codes() {
    println!("\n=== Testing Sorting with Control Code Entries ===");

    // Create entries that simulate catart entries with control codes
    // These entries have control codes in the filename bytes
    let mut entries = vec![];

    // Entry 1: Creates "Z" display (but has low control codes first)
    entries.push(PrintableEntry {
        user: 0,
        fname: PrintableEntryFileName {
            f1: 0x06,
            f2: 0x1F,
            f3: 0x01,
            f4: 0x01, // Control codes
            f5: 0x0F,
            f6: 0x01,
            f7: b'Z',
            f8: 0x17,
            e1: b'.',
            e2: b'T',
            e3: b'X'
        },
        pieces: [0; 4],
        sectors: [0; 16]
    });

    // Entry 2: Creates "A" display (but has low control codes first)
    entries.push(PrintableEntry {
        user: 0,
        fname: PrintableEntryFileName {
            f1: 0x06,
            f2: 0x1F,
            f3: 0x01,
            f4: 0x01,
            f5: 0x0F,
            f6: 0x01,
            f7: b'A',
            f8: 0x17,
            e1: b'.',
            e2: b'T',
            e3: b'X'
        },
        pieces: [0; 4],
        sectors: [0; 16]
    });

    // Entry 3: Creates "M" display
    entries.push(PrintableEntry {
        user: 0,
        fname: PrintableEntryFileName {
            f1: 0x06,
            f2: 0x1F,
            f3: 0x01,
            f4: 0x01,
            f5: 0x0F,
            f6: 0x01,
            f7: b'M',
            f8: 0x17,
            e1: b'.',
            e2: b'T',
            e3: b'X'
        },
        pieces: [0; 4],
        sectors: [0; 16]
    });

    println!("\n--- Before Sorting (in disk order) ---");
    for (i, entry) in entries.iter().enumerate() {
        let bytes = entry.fname.all_generated_bytes();
        println!("{}: Raw bytes: {:02X?}", i, &bytes[0..8]);
        println!(
            "   Display char position: f7 = '{}' (0x{:02X})",
            bytes[6] as char, bytes[6]
        );
    }

    // Create catalog and get sorted entries
    let catalog = Catalog::try_from(entries.as_slice()).expect("Failed to create catalog");
    let unified_catalog = UnifiedCatalog::from(catalog);
    let sorted = unified_catalog.visible_sorted_entries(CatalogType::Cat);

    println!("\n--- After Sorting (alphabetical by raw bytes) ---");
    for (i, entry) in sorted.iter().enumerate() {
        let bytes = entry.fname().all_generated_bytes();
        println!("{}: Raw bytes: {:02X?}", i, &bytes[0..8]);
        println!(
            "   Display char position: f7 = '{}' (0x{:02X})",
            bytes[6] as char, bytes[6]
        );
    }

    println!("\n--- Analysis ---");
    println!(
        "When sorting by raw bytes (comparing [0x06, 0x1F, 0x01, 0x01, 0x0F, 0x01, f7, ...]):"
    );
    println!("The entries are sorted by f7 (the displayable character).");
    println!("So even with control codes, alphabetical sorting should work.");
    println!("Expected order: A, M, Z");

    // Verify the order
    let sorted_chars: Vec<char> = sorted
        .iter()
        .map(|e| e.fname().all_generated_bytes()[6] as char)
        .collect();
    println!("\nActual sorted order: {:?}", sorted_chars);

    assert_eq!(
        sorted_chars,
        vec!['A', 'M', 'Z'],
        "Entries should be sorted by display character"
    );

    println!("✅ Sorting with control codes works correctly!");
}

#[test]
fn test_cat_vs_dir_filling_logic() {
    println!("\n=== Testing CAT vs DIR Filling Logic ===");

    // Create simple A-F entries
    let entries: Vec<PrintableEntry> = (b'A'..=b'F')
        .map(|c| {
            let mut name = String::from(c as char);
            name.push_str(".BAS");
            PrintableEntry {
                user: 0,
                fname: make_fname(&name),
                pieces: [0; 4],
                sectors: [0; 16]
            }
        })
        .collect();

    let catalog = Catalog::try_from(entries.as_slice()).expect("Failed to create catalog");
    let unified_catalog = UnifiedCatalog::from(catalog);

    // Test CAT mode
    println!("\n--- CAT Mode (Current Implementation) ---");
    let cat_grid =
        unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode1, CatalogType::Cat);

    println!(
        "Column 0: {:?}",
        cat_grid
            .column(0)
            .unwrap()
            .iter()
            .map(|e| e.fname().display_name())
            .collect::<Vec<_>>()
    );
    println!(
        "Column 1: {:?}",
        cat_grid
            .column(1)
            .unwrap()
            .iter()
            .map(|e| e.fname().display_name())
            .collect::<Vec<_>>()
    );

    let cat_display: Vec<String> = cat_grid
        .entries_display_order()
        .map(|e| e.fname().display_name())
        .collect();
    println!("CAT Display order (row-by-row): {:?}", cat_display);

    // Test DIR mode
    println!("\n--- DIR Mode (Current Implementation) ---");
    let dir_grid =
        unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode1, CatalogType::Dir);

    println!(
        "Column 0: {:?}",
        dir_grid
            .column(0)
            .unwrap()
            .iter()
            .map(|e| e.fname().display_name())
            .collect::<Vec<_>>()
    );
    println!(
        "Column 1: {:?}",
        dir_grid
            .column(1)
            .unwrap()
            .iter()
            .map(|e| e.fname().display_name())
            .collect::<Vec<_>>()
    );

    let dir_display: Vec<String> = dir_grid
        .entries_display_order()
        .map(|e| e.fname().display_name())
        .collect();
    println!("DIR Display order (row-by-row): {:?}", dir_display);

    // Analysis
    println!("\n--- Analysis ---");
    println!("Current CAT implementation:");
    println!("  - Fills column-by-column: Col0=[A,B,C], Col1=[D,E,F]");
    println!("  - Reading left-to-right gives: A D B E C F");
    println!("\nCurrent DIR implementation:");
    println!("  - Fills row-by-row with modulo: Col0=[A,C,E], Col1=[B,D,F]");
    println!("  - Reading left-to-right gives: A B C D E F");
    println!("\nQuestion: Which behavior should CAT have?");
    println!("If CAT should display alphabetically when reading left-to-right,");
    println!("then CAT and DIR implementations might be swapped!");
}
