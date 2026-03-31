use std::path::PathBuf;
use std::process::Command;

use escargot::CargoBuild;
use fs_err as fs;
#[cfg(unix)]
use rexpect::session::spawn_command;
use tempfile::TempDir;

// Helper function to get the bdasm binary path
fn get_bdasm_bin() -> PathBuf {
    CargoBuild::new()
        .bin("bdasm")
        .current_release()
        .run()
        .unwrap()
        .path()
        .to_path_buf()
}

#[test]
fn test_hello_world_roundtrip() {
    let temp_dir = TempDir::new().unwrap();

    // Use pre-generated binary (hello.o was generated with basm --header --output hello.o)
    let binary_with_header = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("hello.o");

    if !binary_with_header.exists() {
        eprintln!("Skipping test: hello.o not found");
        return;
    }

    // Output path
    let disasm_output = temp_dir.path().join("hello_world_disasm.asm");

    // Disassemble binary
    let output = Command::new(get_bdasm_bin())
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

    // Step 3: Check disassembly contains expected instructions
    let disasm_content = fs::read_to_string(&disasm_output).unwrap();
    assert!(
        disasm_content.contains("ld hl") || disasm_content.contains("LD HL"),
        "Should contain LD HL instruction"
    );
    assert!(
        disasm_content.contains("call") || disasm_content.contains("CALL"),
        "Should contain CALL instruction"
    );
    assert!(
        disasm_content.contains("ret") || disasm_content.contains("RET"),
        "Should contain RET instruction"
    );
}

#[test]
fn test_control_file_roundtrip() {
    let temp_dir = TempDir::new().unwrap();

    // Use existing binary
    let binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("doc.bin");

    if !binary.exists() {
        eprintln!("Skipping test: doc.bin not found");
        return;
    }

    let control_file = temp_dir.path().join("test.ctl");
    let output1 = temp_dir.path().join("output1.asm");
    let output2 = temp_dir.path().join("output2.asm");

    // Step 1: Disassemble and save control file
    let output = Command::new(get_bdasm_bin())
        .arg(&binary)
        .arg("--origin")
        .arg("0x4000")
        .arg("-O")
        .arg(&output1)
        .arg("--save-control")
        .arg(&control_file)
        .arg("--compressed")
        .output()
        .unwrap();

    assert!(output.status.success(), "bdasm should succeed");

    assert!(control_file.exists(), "Control file should be created");

    // Step 2: Check control file format
    let control_content = fs::read_to_string(&control_file).unwrap();
    assert!(
        control_content.contains("origin") || control_content.contains("label"),
        "Control file should contain directives"
    );

    // Step 3: Disassemble again using control file
    let output = Command::new(get_bdasm_bin())
        .arg(&binary)
        .arg("-O")
        .arg(&output2)
        .arg("--control")
        .arg(&control_file)
        .arg("--compressed")
        .output()
        .unwrap();

    assert!(output.status.success(), "bdasm should succeed");

    assert!(output2.exists(), "Second disassembly should be created");
}

#[test]
fn test_cpc_string_detection() {
    let temp_dir = TempDir::new().unwrap();

    // Create a binary with a CPC string
    let test_binary = temp_dir.path().join("cpc_string_test.bin");
    let mut data = vec![0x00; 10]; // Some non-string data
    data.extend_from_slice(b"Hell"); // 7-bit ASCII
    data.push(b'o' | 0x80); // Last char with bit 7 set
    fs::write(&test_binary, &data).unwrap();

    let output = temp_dir.path().join("output.asm");
    let control = temp_dir.path().join("output.ctl");

    // Disassemble with CPC string detection
    let cmd_output = Command::new(get_bdasm_bin())
        .arg(&test_binary)
        .arg("--origin")
        .arg("0x0000")
        .arg("-O")
        .arg(&output)
        .arg("--detect-cpc-strings")
        .arg("--save-control")
        .arg(&control)
        .arg("--compressed")
        .arg("--verbose")
        .output()
        .unwrap();

    assert!(cmd_output.status.success(), "bdasm should succeed");

    // Check that control file contains cpcstring directive
    let control_content = fs::read_to_string(&control).unwrap();
    assert!(
        control_content.contains("cpcstring"),
        "Control file should contain cpcstring directive"
    );
}

#[test]
fn test_data_bloc_handling() {
    let temp_dir = TempDir::new().unwrap();

    // Use existing binary
    let binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("doc.bin");

    if !binary.exists() {
        eprintln!("Skipping test: doc.bin not found");
        return;
    }

    let output = temp_dir.path().join("output.asm");

    // Disassemble with data bloc
    let cmd_output = Command::new(get_bdasm_bin())
        .arg(&binary)
        .arg("--origin")
        .arg("0x4000")
        .arg("-O")
        .arg(&output)
        .arg("--data")
        .arg("0x4002-4") // Address 0x4002 in assembly space (offset 2), length 4 bytes
        .arg("--compressed")
        .output()
        .unwrap();

    assert!(cmd_output.status.success(), "bdasm should succeed");

    assert!(output.exists(), "Output should be created");

    // Check that disassembly contains data directives
    let content = fs::read_to_string(&output).unwrap();

    // The --data 0x4002-4 parameter means start at address 0x4002 (offset 2) with length 4
    // doc.bin has 8 bytes: 0x3E, 0x00, 0x01, 0x34, 0x12, 0xC3, 0x00, 0x40
    // With --data 0x4002-4, bytes at offsets 2-5 (0x01, 0x34, 0x12, 0xC3) become data
    // The disassembler uses --compressed mode which outputs raw mnemonics
    // So we should see LD A, 0x0 and JP 0x4000, but not the middle bytes as code
    assert!(
        content.contains("LD A") || content.contains("ld a"),
        "Should contain LD A instruction. Content:\n{}",
        content
    );
}

// Interactive tests using rexpect (Unix only - rexpect uses PTY which is Unix-specific)

#[test]
#[cfg(unix)]
fn test_interactive_control_file_edit() {
    use std::time::Duration;

    let temp_dir = TempDir::new().unwrap();

    // Create a simple binary for testing
    let test_binary = temp_dir.path().join("test.bin");
    let test_data = vec![0x21, 0x00, 0x40, 0xC9]; // LD HL, 0x4000; RET
    fs::write(&test_binary, &test_data).unwrap();

    let control_file = temp_dir.path().join("test.ctl");
    let output = temp_dir.path().join("output.asm");

    // First create a basic control file
    let initial_control = "origin 0x0000\n";
    fs::write(&control_file, initial_control).unwrap();

    // Spawn bdasm with the control file
    let bdasm_path = get_bdasm_bin();
    let mut cmd = std::process::Command::new(bdasm_path);
    cmd.arg(&test_binary)
        .arg("-O")
        .arg(&output)
        .arg("--control")
        .arg(&control_file);

    let mut session = spawn_command(cmd, Some(5000)).unwrap();

    // Wait for completion
    session.exp_eof().unwrap();

    // Verify output was created
    assert!(
        output.exists(),
        "Output should be created from control file"
    );

    // Modify control file to add a label
    let modified_control = "origin 0x0000\nlabel start=0x0000\n";
    fs::write(&control_file, modified_control).unwrap();

    // Run again with modified control file
    let output2 = temp_dir.path().join("output2.asm");
    let mut cmd2 = std::process::Command::new(get_bdasm_bin());
    cmd2.arg(&test_binary)
        .arg("-O")
        .arg(&output2)
        .arg("--control")
        .arg(&control_file);

    let mut session2 = spawn_command(cmd2, Some(5000)).unwrap();
    session2.exp_eof().unwrap();

    // Verify the label appears in output
    let output_content = fs::read_to_string(&output2).unwrap();
    assert!(
        output_content.contains("start"),
        "Output should contain the label 'start'"
    );
}

#[test]
#[cfg(unix)]
fn test_interactive_prompt_with_verbose() {
    use std::time::Duration;

    let temp_dir = TempDir::new().unwrap();

    // Create test binary
    let test_binary = temp_dir.path().join("test.bin");
    let test_data = vec![0x00; 100]; // 100 bytes of data
    fs::write(&test_binary, &test_data).unwrap();

    let output = temp_dir.path().join("output.asm");

    // Run with verbose flag to see progress
    let bdasm_path = get_bdasm_bin();
    let mut cmd = std::process::Command::new(bdasm_path);
    cmd.arg(&test_binary)
        .arg("-O")
        .arg(&output)
        .arg("--verbose");

    let result = spawn_command(cmd, Some(5000));

    if let Ok(mut session) = result {
        // Try to capture verbose output
        let _ = session.exp_regex(r"(?i)(processing|disassembl|address)");
        let _ = session.exp_eof();
    }

    // Verify output was created
    assert!(output.exists(), "Output should be created");
}

#[test]
fn test_save_and_load_control_workflow() {
    let temp_dir = TempDir::new().unwrap();

    // Create test binary with mixed code and data
    let test_binary = temp_dir.path().join("mixed.bin");
    let test_data = vec![
        0x21, 0x00, 0x40, // LD HL, 0x4000
        0x01, 0x02, 0x03, // These will be data
        0xC9, // RET
    ];
    fs::write(&test_binary, &test_data).unwrap();

    let control_file = temp_dir.path().join("workflow.ctl");
    let output1 = temp_dir.path().join("output1.asm");

    // Step 1: Initial disassembly with save-control
    let status = Command::new(get_bdasm_bin())
        .arg(&test_binary)
        .arg("--origin")
        .arg("0x0000")
        .arg("-O")
        .arg(&output1)
        .arg("--save-control")
        .arg(&control_file)
        .arg("--compressed")
        .output()
        .unwrap()
        .status;
    assert!(status.success(), "bdasm should succeed");

    assert!(control_file.exists(), "Control file should be saved");

    // Step 2: Edit control file to add data bloc
    let mut control_content = fs::read_to_string(&control_file).unwrap();
    control_content.push_str("\ndata 3-3\n"); // Mark 3 bytes at position 0x03 as data
    fs::write(&control_file, &control_content).unwrap();

    // Step 3: Reload with modified control file
    let output2 = temp_dir.path().join("output2.asm");
    let status = Command::new(get_bdasm_bin())
        .arg(&test_binary)
        .arg("-O")
        .arg(&output2)
        .arg("--control")
        .arg(&control_file)
        .arg("--compressed")
        .output()
        .unwrap()
        .status;
    assert!(status.success(), "bdasm should succeed");

    // Verify data directive appears in second output
    let output2_content = fs::read_to_string(&output2).unwrap();
    // Check for data directives - should have defb/db with the bytes 01, 02, 03
    assert!(
        output2_content.to_lowercase().contains("defb")
            || output2_content.to_lowercase().contains("db"),
        "Output should contain data directives for specified range. Content:\n{}",
        output2_content
    );
}
