
#![feature(proc_macro_hygiene)]
use cpclib_macros::*;
use cpclib_asm::preamble::*;

#[test]
fn test_macro_parse_assembly_single_instruction() {
    let listing = parse_assembly!(" ld a, 0");

    assert_eq!(listing.len(), 1);
    assert_eq!(listing[0], Token::OpCode(Mnemonic::Ld, Some(DataAccess::Register8(Register8::A)), Some(DataAccess::Expression(0.into()))));

}


#[test]
fn test_macro_parse_assembly_several_instructions_a() {
    let listing = parse_assembly!(" ld a, 0 : ld a, 0");

    assert_eq!(listing.len(), 2);
    assert_eq!(listing[0], Token::OpCode(Mnemonic::Ld, Some(DataAccess::Register8(Register8::A)), Some(DataAccess::Expression(0.into()))));
    assert_eq!(listing[1], Token::OpCode(Mnemonic::Ld, Some(DataAccess::Register8(Register8::A)), Some(DataAccess::Expression(0.into()))));

}

#[test]
fn test_macro_parse_assembly_several_instructions_b() {
    let listing = parse_assembly!(" ld a, 0 
     ld a, 0");

    assert_eq!(listing.len(), 2);
    assert_eq!(listing[0], Token::OpCode(Mnemonic::Ld, Some(DataAccess::Register8(Register8::A)), Some(DataAccess::Expression(0.into()))));
    assert_eq!(listing[1], Token::OpCode(Mnemonic::Ld, Some(DataAccess::Register8(Register8::A)), Some(DataAccess::Expression(0.into()))));

}
