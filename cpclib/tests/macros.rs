#![feature(proc_macro_hygiene)]
use cpclib_asm::preamble::*;
use cpclib_macros::*;

// TODO reactivate that stuff
// #[test]
// fn test_macro_parse_z80_fname() {
// let listing = parse_z80!(fname: "./cpclib/tests/my_path.asm");
//
// assert_eq!(listing.len(), 3);
// }
fn test_macro_parse_z80_single_instruction() {
    let listing = parse_z80!(" ld a, 0");

    assert_eq!(listing.len(), 1);
    assert_eq!(
        listing[0],
        Token::OpCode(
            Mnemonic::Ld,
            Some(DataAccess::Register8(Register8::A)),
            Some(DataAccess::Expression(0.into())),
            None
        )
    );

    let _bytes = parse_z80!(" push hl");
}

// // Cannot be assemble because of the error
// fn test_macro_parse_z80_single_instruction_fail() {
// let listing = parse_z80!(" ld a, ");
//
// assert_eq!(listing.len(), 1);
// assert_eq!(listing[0], Token::OpCode(Mnemonic::Ld, Some(DataAccess::Register8(Register8::A)), Some(DataAccess::Expression(0.into()))));
//
// let bytes = parse_z80!(" push hl");
//
// }

// #[test]
// fn test_macro_parse_z80_single_instruction2() {
// let listing = parse_z80!(ld a, 0);
//
// assert_eq!(listing.len(), 1);
// assert_eq!(listing[0], Token::OpCode(Mnemonic::Ld, Some(DataAccess::Register8(Register8::A)), Some(DataAccess::Expression(0.into()))));
//
// }

#[test]
fn test_macro_parse_z80_several_instructions_a() {
    let listing = parse_z80!(" ld a, 0 : ld a, 0");

    assert_eq!(listing.len(), 2);
    assert_eq!(
        listing[0],
        Token::OpCode(
            Mnemonic::Ld,
            Some(DataAccess::Register8(Register8::A)),
            Some(DataAccess::Expression(0.into())),
            None
        )
    );
    assert_eq!(
        listing[1],
        Token::OpCode(
            Mnemonic::Ld,
            Some(DataAccess::Register8(Register8::A)),
            Some(DataAccess::Expression(0.into())),
            None
        )
    );
}

#[test]
fn test_macro_parse_z80_several_instructions_b() {
    let listing = parse_z80!(
        " ld a, 0 
     ld a, 0"
    );

    assert_eq!(listing.len(), 2);
    assert_eq!(
        listing[0],
        Token::OpCode(
            Mnemonic::Ld,
            Some(DataAccess::Register8(Register8::A)),
            Some(DataAccess::Expression(0.into())),
            None
        )
    );
    assert_eq!(
        listing[1],
        Token::OpCode(
            Mnemonic::Ld,
            Some(DataAccess::Register8(Register8::A)),
            Some(DataAccess::Expression(0.into())),
            None
        )
    );
}

// #[test]
// fn test_macro_parse_z80_several_instructions_d() {
// let listing = parse_z80!( ld a, 0 : ld a, 0);
//
// assert_eq!(listing.len(), 2);
// assert_eq!(listing[0], Token::OpCode(Mnemonic::Ld, Some(DataAccess::Register8(Register8::A)), Some(DataAccess::Expression(0.into()))));
// assert_eq!(listing[1], Token::OpCode(Mnemonic::Ld, Some(DataAccess::Register8(Register8::A)), Some(DataAccess::Expression(0.into()))));
//
// }

// #[test]
// does not pass yet :(
// fn test_macro_parse_z80_several_instructions_e() {
// let listing = parse_z80!( ld a, 0
// ld a, 0    );
//
// assert_eq!(listing.len(), 2);
// assert_eq!(listing[0], Token::OpCode(Mnemonic::Ld, Some(DataAccess::Register8(Register8::A)), Some(DataAccess::Expression(0.into()))));
// assert_eq!(listing[1], Token::OpCode(Mnemonic::Ld, Some(DataAccess::Register8(Register8::A)), Some(DataAccess::Expression(0.into()))));
//
// }

// #[test]
// fn test_macro_assemble_single_instruction() {
// let bytes = assemble!(" push hl");
//
// assert_eq!(bytes.len(), 1);
// assert_eq!(&bytes[..], &[0xE5]);
// }
//
// #[test]
// fn test_macro_assemble_two_instructions() {
// let bytes = assemble!(" push hl : push de");
//
// assert_eq!(bytes.len(), 2);
// assert_eq!(&bytes[..], &[0xE5, 0xD5]);
// }
//
