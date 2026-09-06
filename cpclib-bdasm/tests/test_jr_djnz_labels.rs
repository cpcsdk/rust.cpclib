//! Regression test for JR/DJNZ label injection, after
//! `resolve_jr_djnz_target`'s arithmetic moved out of this crate and into
//! `cpclib_asm::disass` as a shared function, with `cpclib-bdasm` now calling
//! it instead of carrying its own copy.
//!
//! The bytes below reproduce the `DEC SP ; JR NZ,e ; JR C,e ; JR NZ,e`
//! sequence that opens `test_no_origin.asm` (from line 34 on) - an existing
//! fixture that already contains `JR NZ, label_005f`. Reproducing just that
//! sequence and checking the same line comes out unchanged is enough to show
//! the extraction moved the arithmetic without moving the behaviour.
use cpclib_bdasm::{BdAsmCli, process};
use cpclib_common::camino::Utf8PathBuf;
use fs_err as fs;
use tempfile::TempDir;

#[test]
fn jr_and_djnz_label_injection_is_unchanged_by_the_extraction() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("jr_targets.bin");
    let output_path = temp_dir.path().join("jr_targets.asm");

    // DEC SP ; JR NZ,+92 ; JR C,+50 ; JR NZ,+74 - targets 0x5f, 0x37, 0x51,
    // exactly the addresses `test_no_origin.asm` names `label_005f`,
    // `label_0037` and `label_0051`. Padded with NOPs past those targets: a
    // label is only generated for an address inside the disassembled
    // binary's own range (`BdAsmEnv::valid_range`), same as the fixture's
    // much larger source binary.
    let mut program = vec![0x3B, 0x20, 0x5C, 0x38, 0x32, 0x20, 0x4A];
    program.resize(0x60, 0x00);
    fs::write(&input_path, &program).unwrap();

    let cli = BdAsmCli {
        input: Utf8PathBuf::from_path_buf(input_path).unwrap(),
        origin: None,
        data_bloc: Vec::new(),
        label: Vec::new(),
        skip: None,
        length: None,
        compress: false,
        output: Some(Utf8PathBuf::from_path_buf(output_path.clone()).unwrap()),
        save_control: None,
        control: None,
        detect_cpc_strings: false,
        verbose: false
    };

    process(&cli, &()).unwrap();

    let disasm = fs::read_to_string(&output_path).unwrap();

    // The exact lines `test_no_origin.asm` already has (36-38) - unchanged by
    // moving the arithmetic into `cpclib_asm::disass`.
    assert!(
        disasm
            .lines()
            .any(|line| line.trim() == "JR NZ, label_005f"),
        "{disasm}"
    );
    assert!(
        disasm.lines().any(|line| line.trim() == "JR C, label_0037"),
        "{disasm}"
    );
    assert!(
        disasm
            .lines()
            .any(|line| line.trim() == "JR NZ, label_0051"),
        "{disasm}"
    );
}
