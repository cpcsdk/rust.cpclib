use std::path::PathBuf;
use std::process::Command;

use escargot::CargoBuild;
use fs_err as fs;
use tempfile::TempDir;

#[test]
fn test_label_generation() {
    let temp_dir = TempDir::new().unwrap();

    // Read the test binary
    let binary_with_header = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("hello.o");

    if !binary_with_header.exists() {
        eprintln!("Skipping test: hello.o not found");
        return;
    }

    // Output path
    let disasm_output = temp_dir.path().join("hello_disasm.asm");

    // Build and get the binary path using escargot
    let bdasm_bin = CargoBuild::new()
        .bin("bdasm")
        .current_release()
        .run()
        .unwrap();

    // Disassemble binary
    let output = Command::new(bdasm_bin.path())
        .arg(&binary_with_header)
        .arg("--origin")
        .arg("0x1200")
        .arg("-O")
        .arg(&disasm_output)
        .arg("--compressed")
        .output()
        .unwrap();

    assert!(output.status.success(), "bdasm should succeed");
    assert!(
        disasm_output.exists(),
        "Disassembly output should be created"
    );

    // Read and check the output
    let disasm_content = fs::read_to_string(&disasm_output).unwrap();

    println!("\n=== DISASSEMBLY OUTPUT ===");
    for (i, line) in disasm_content.lines().enumerate().take(30) {
        println!("{:3}: {}", i + 1, line);
    }

    // Check specific cases
    let has_ld_hl_label = disasm_content
        .lines()
        .any(|line| line.to_uppercase().contains("LD HL") && line.contains("label_1211"));

    let has_call_with_label = disasm_content
        .lines()
        .any(|line| line.to_uppercase().contains("CALL") && line.contains("label_1207"));

    let no_bb5a_label = !disasm_content.contains("label_bb5a");

    println!("\n=== ANALYSIS ===");
    println!("Has LD HL, label_1211: {}", has_ld_hl_label);
    println!("Has CALL label_1207: {}", has_call_with_label);
    println!("No label_bb5a (external): {}", no_bb5a_label);

    // These are the fixes we're testing:
    assert!(
        has_ld_hl_label,
        "LD HL should use label_1211 for the message address"
    );
    assert!(has_call_with_label, "CALL should use label_1207");
    assert!(
        no_bb5a_label,
        "Should not create labels for external addresses like 0xbb5a"
    );
}
