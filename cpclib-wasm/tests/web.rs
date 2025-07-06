//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use std::println;

use cpclib_asm::preamble::EnvOptions;
use cpclib_sna::SnapshotFlag;
use cpclib_wasm::*;
use wasm_bindgen_test::*;

// TODO find a way to init the thread pool...
wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn asm_parse_failure() {
    let source = "ld hl, 1234  push hl";
    let config = asm::asm_create_parser_config("test.asm");
    let result = asm::asm_parse_source(&source, &config);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn asm_parse_success() {
    let source = "ld hl, 1234 :  push hl";
    let config = asm::asm_create_parser_config("test.asm");
    let result = asm::asm_parse_source(&source, &config);
    assert!(result.is_ok());
}

#[wasm_bindgen_test]
fn asm_assemble_failure() {
    let source = "ld hl, 1234  push hl";
    let config = asm::asm_create_parser_config("test.asm");
    let result = asm::asm_assemble_snapshot(&source, &config);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn asm_assemble_success() {
    let source = "ld hl, 1234 :  push hl";
    let config = asm::asm_create_parser_config("test.asm");
    let result = asm::asm_assemble_snapshot(&source, &config);
    assert!(result.is_ok());
}

#[wasm_bindgen_test]
fn asm_fail_save() {
    let source = " SAVE \"test\"";
    let config = asm::asm_create_parser_config("test.asm");
    let result = asm::asm_assemble_snapshot(&source, &config);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn asm_fail_include() {
    let source = " include \"test.asm\"";
    let config = asm::asm_create_parser_config("test.asm");
    let result = asm::asm_assemble_snapshot(&source, &config);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn basic_parse_success_one_line() {
    let source = "10 PRINT \"HELLO WORLD\"";
    let result = basic::basic_parse_program(source);
    assert!(result.is_ok());
}

#[wasm_bindgen_test]
fn basic_parse_success_two_lines() {
    let source = "10 PRINT \"HELLO\":20 PRINT \"WORLD\"";
    let result = basic::basic_parse_program(source);
    assert!(result.is_ok());

    let sna = result.unwrap().sna().unwrap();
}

#[wasm_bindgen_test]
// this test is a copy past of generate_loop4000.rs
fn manually_generated_snapshot() {
    use cpclib_asm::AssemblingOptions;
    use cpclib_asm::assembler::visit_tokens_all_passes_with_options;
    use cpclib_asm::preamble::{ParserContextBuilder, parse_z80_with_context_builder};
    use cpclib_sna::{Snapshot, SnapshotFlag};

    let asm = "
		org 0x4000
		run $
		jp $
	";

    let mut ctx = ParserContextBuilder::default();
    let mut options: EnvOptions = AssemblingOptions::default().into();
    let listing = parse_z80_with_context_builder(asm, ctx).expect("Unable to parse z80 code");
    let (_, env) = visit_tokens_all_passes_with_options(&listing, options)
        .expect("Unable to assemble z80 code");
    let sna = env.sna().clone();
    assert_eq!(
        sna.get_value(&SnapshotFlag::Z80_PC).as_u16().unwrap(),
        0x4000
    );
    assert_eq!(sna.get_byte(0x4000), 0xC3);
    assert_eq!(sna.get_byte(0x4001), 0x00);
    assert_eq!(sna.get_byte(0x4002), 0x40);

    println!("Manual generation of snapshot succeeds");
}

#[wasm_bindgen_test]
// this test is a copy past of generate_loop4000.rs
fn playground_generated_snapshot() {
    let source = "
        org 0x4000
        run $
        jp $
    ";
    let config = asm::asm_create_parser_config("loop4000.asm");
    let sna = asm::asm_assemble_snapshot(&source, &config).expect("Unable to build the snapshot");
    assert_eq!(
        sna.get_value(&SnapshotFlag::Z80_PC).as_u16().unwrap(),
        0x4000
    );

    assert_eq!(sna.get_byte(0x4000), 0xC3);
    assert_eq!(sna.get_byte(0x4001), 0x00);
    assert_eq!(sna.get_byte(0x4002), 0x40);

    let bytes = sna.bytes();
    assert_eq!(bytes.get_index(0x10), 2); // we currently want only snapshot v2
    assert_eq!(bytes.get_index(0x23), 0x00); // pc
    assert_eq!(bytes.get_index(0x24), 0x40);

    let header_size = 0x100;
    assert_eq!(bytes.get_index(header_size + 0x4000), 0xC3);
    assert_eq!(bytes.get_index(header_size + 0x4001), 0x00);
    assert_eq!(bytes.get_index(header_size + 0x4002), 0x40);
}
