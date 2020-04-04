
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
fn test_macro_parse_assembly_single_instruction2() {
    let listing = parse_assembly!(ld a, 0);

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


#[test]
fn test_macro_parse_assembly_several_instructions_d() {
    let listing = parse_assembly!( ld a, 0 : ld a, 0);

    assert_eq!(listing.len(), 2);
    assert_eq!(listing[0], Token::OpCode(Mnemonic::Ld, Some(DataAccess::Register8(Register8::A)), Some(DataAccess::Expression(0.into()))));
    assert_eq!(listing[1], Token::OpCode(Mnemonic::Ld, Some(DataAccess::Register8(Register8::A)), Some(DataAccess::Expression(0.into()))));

}

#[test]
/// does not pass yet :(
fn test_macro_parse_assembly_several_instructions_e() {
    let listing = parse_assembly!( ld a, 0 
     ld a, 0    );

    assert_eq!(listing.len(), 2);
    assert_eq!(listing[0], Token::OpCode(Mnemonic::Ld, Some(DataAccess::Register8(Register8::A)), Some(DataAccess::Expression(0.into()))));
    assert_eq!(listing[1], Token::OpCode(Mnemonic::Ld, Some(DataAccess::Register8(Register8::A)), Some(DataAccess::Expression(0.into()))));

}



#[test]
fn test_macro_assemble_single_instruction() {
    let bytes = assemble!(" push hl");

    assert_eq!(bytes.len(), 1);
    assert_eq!(&bytes[..], &[0xe5]);

}

#[test]
fn test_macro_assemble_two_instructions() {
    let bytes = assemble!(" push hl : push de");

    assert_eq!(bytes.len(), 2);
    assert_eq!(&bytes[..], &[0xe5, 0xd5]);

}