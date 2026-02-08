//! Integration tests for parsing real CSL files

use fs_err as fs;
use std::path::PathBuf;

use cpclib_csl::parse_csl_with_rich_errors;

/// Helper function to find all CSL test files
fn find_csl_files() -> Vec<PathBuf> {
    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/CSL");

    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(&test_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                // Recursively search subdirectories
                if let Ok(sub_entries) = fs::read_dir(entry.path()) {
                    for sub_entry in sub_entries.flatten() {
                        if sub_entry.path().is_dir() {
                            // Another level (MODULE A, MODULE B, etc.)
                            if let Ok(module_entries) = fs::read_dir(sub_entry.path()) {
                                for module_entry in module_entries.flatten() {
                                    let path = module_entry.path();
                                    if path.extension().and_then(|s| s.to_str()) == Some("CSL") {
                                        files.push(path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    files.sort();
    files
}

#[test]
fn test_parse_all_csl_files() {
    let files = find_csl_files();

    assert!(!files.is_empty(), "No CSL test files found");

    let mut errors = Vec::new();
    let mut success_count = 0;

    for file_path in &files {
        let file_name = file_path.file_name().unwrap().to_string_lossy();

        match fs::read_to_string(file_path) {
            Ok(content) => {
                match parse_csl_with_rich_errors(&content, Some(file_name.to_string())) {
                    Ok(script) => {
                        println!("✓ {} ({} instructions)", file_name, script.len());
                        success_count += 1;
                    },
                    Err(e) => {
                        // Try to give more context
                        let mut debug_content = content.clone();
                        if debug_content.len() > 500 {
                            debug_content.truncate(500);
                        }
                        let error_msg = format!("✗ {}: Parse error: {}", file_name, e);
                        eprintln!("{}", error_msg);
                        errors.push((file_name.to_string(), error_msg));
                    }
                }
            },
            Err(e) => {
                let error_msg = format!("✗ {}: Failed to read file: {}", file_name, e);
                eprintln!("{}", error_msg);
                errors.push((file_name.to_string(), error_msg));
            }
        }
    }

    eprintln!("\n=== Summary ===");
    eprintln!("Total files: {}", files.len());
    eprintln!("Successfully parsed: {}", success_count);
    eprintln!("Failed: {}", errors.len());

    if !errors.is_empty() {
        eprintln!("\n=== Errors ===");
        for (file, error) in &errors {
            eprintln!("{}: {}", file, error);
        }
        panic!(
            "Failed to parse {} out of {} CSL files",
            errors.len(),
            files.len()
        );
    }
}

#[test]
fn test_parse_specific_module_a_file() {
    let file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/CSL/Shaker_CSL/MODULE A/SHAKE26A-0.CSL");

    let content = fs::read_to_string(&file_path).expect("Failed to read SHAKE26A-0.CSL");

    eprintln!("Content length: {} bytes", content.len());
    eprintln!("First 200 bytes: {:?}", &content[..content.len().min(200)]);

    // Try parsing progressively larger chunks
    for chunk_size in [500, 1000, 2000, 5000, content.len()] {
        let chunk = &content[..chunk_size.min(content.len())];
        let result = parse_csl_with_rich_errors(chunk, None);
        if result.is_err() {
            eprintln!("\nFailed at chunk size {}", chunk_size);
            eprintln!(
                "Last 100 bytes of chunk: {:?}",
                &chunk[chunk.len().saturating_sub(100)..]
            );
            break;
        }
        else {
            eprintln!("✓ Parsed {} bytes successfully", chunk_size);
        }
    }

    let script = parse_csl_with_rich_errors(&content, Some("SHAKE26A-0.CSL".to_string()))
        .expect("Failed to parse SHAKE26A-0.CSL");

    // Verify it has instructions
    assert!(script.len() > 0);

    // Count non-empty, non-comment instructions
    let real_instructions: Vec<_> = script
        .iter()
        .filter(|i| {
            !matches!(
                i,
                cpclib_csl::CslInstruction::Comment(_) | cpclib_csl::CslInstruction::Empty
            )
        })
        .collect();

    assert!(
        real_instructions.len() > 10,
        "Should have multiple real instructions"
    );
}

#[test]
fn test_parse_first_lines_only() {
    let content = ";\r\n; Fichier de script CSL \r\ncsl_version 1.0\r\nreset\r\n";

    eprintln!("Testing: {:?}", content);

    let result = parse_csl_with_rich_errors(content, None);
    assert!(
        result.is_ok(),
        "Failed to parse first lines: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_first_500_bytes() {
    // Test parsing the actual first 500 bytes from the file
    let file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/CSL/Shaker_CSL/MODULE A/SHAKE26A-0.CSL");

    let content = fs::read_to_string(&file_path).expect("Failed to read SHAKE26A-0.CSL");

    let chunk = &content[..500.min(content.len())];
    eprintln!("Parsing {} bytes", chunk.len());
    eprintln!(
        "Last 50 chars: {:?}",
        &chunk[chunk.len().saturating_sub(50)..]
    );

    let result = parse_csl_with_rich_errors(chunk, None);
    assert!(
        result.is_ok(),
        "Failed to parse first 500 bytes: {:?}",
        result.err()
    );
}

#[test]
fn test_module_directories_exist() {
    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/CSL/Shaker_CSL");

    // Check that the test directory structure exists
    assert!(
        test_dir.exists(),
        "tests/CSL/Shaker_CSL directory should exist"
    );

    let modules = ["MODULE A", "MODULE B", "MODULE C", "MODULE E"];

    for module in &modules {
        let module_dir = test_dir.join(module);
        if module_dir.exists() {
            println!("Found {}", module);
        }
    }
}
