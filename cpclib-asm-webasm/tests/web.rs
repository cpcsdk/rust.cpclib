//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use wasm_bindgen_test::*;
use cpclib_asm_webasm::*;

// TODO find a way to init the thread pool...
wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn asm_parse_failure() {

    let source =  "ld hl, 1234  push hl";
    let config = asm::create_parser_config("test.asm");
    let result = asm::parse_source(&source, &config);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn asm_parse_success() {
    let source =  "ld hl, 1234 :  push hl";
    let config = asm::create_parser_config("test.asm");
    let result = asm::parse_source(&source, &config);
    assert!(result.is_ok());
}

#[wasm_bindgen_test]
fn asm_assemble_failure() {
    let source =  "ld hl, 1234  push hl";
    let config = asm::create_parser_config("test.asm");
    let result = asm::assemble_snapshot(&source, &config);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn asm_assemble_success() {
    let source =  "ld hl, 1234 :  push hl";
    let config = asm::create_parser_config("test.asm");
    let result = asm::assemble_snapshot(&source, &config);
    assert!(result.is_ok());
}

#[wasm_bindgen_test]
fn asm_fail_save() {
    let source =  " SAVE \"test\"";
    let config = asm::create_parser_config("test.asm");
    let result = asm::assemble_snapshot(&source, &config);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn asm_fail_include() {
    let source =  " include \"test.asm\"";
    let config = asm::create_parser_config("test.asm");
    let result = asm::assemble_snapshot(&source, &config);
    assert!(result.is_err());
}


#[wasm_bindgen_test]
fn basic_parse_success() {
    let source =  "10 PRINT \"HELLO WORLD\"";
    let result = basic::parse_basic_program(source);
    assert!(result.is_ok());
}