///! Utility code to build more easily tokens to manipulate in code generators

use crate::assembler::tokens::*;

pub fn push_af() -> Token {
    push_or_pop(Mnemonic::Push, Register16::Af)
}

pub fn push_bc() -> Token {
    push_or_pop(Mnemonic::Push, Register16::Bc)
}

pub fn push_de() -> Token {
    push_or_pop(Mnemonic::Push, Register16::De)
}

pub fn push_hl() -> Token {
    push_or_pop(Mnemonic::Push, Register16::Hl)
}

pub fn pop_af() -> Token {
    push_or_pop(Mnemonic::Pop, Register16::Af)
}

pub fn pop_bc() -> Token {
    push_or_pop(Mnemonic::Pop, Register16::Bc)
}

pub fn pop_de() -> Token {
    push_or_pop(Mnemonic::Pop, Register16::De)
}

pub fn pop_hl() -> Token {
    push_or_pop(Mnemonic::Pop, Register16::Hl)
}

#[inline]
fn push_or_pop(op: Mnemonic, reg: Register16) -> Token {
    token_for_opcode_one_arg(
        op,
        reg.into()
    )
}

pub fn push_ix() -> Token {
    Token::OpCode(
        Mnemonic::Push,
        Some(DataAccess::IndexRegister16(IndexRegister16::Ix)),
        None
    )
}

pub fn push_iy() -> Token {
    Token::OpCode(
        Mnemonic::Push,
        Some(DataAccess::IndexRegister16(IndexRegister16::Iy)),
        None
    )
}

pub fn pop_ix() -> Token {
    Token::OpCode(
        Mnemonic::Pop,
        Some(DataAccess::IndexRegister16(IndexRegister16::Ix)),
        None
    )
}

pub fn pop_iy() -> Token {
    Token::OpCode(
        Mnemonic::Pop,
        Some(DataAccess::IndexRegister16(IndexRegister16::Iy)),
        None
    )
}

pub fn breakpoint_winape() -> Token {
    Token::Defb(vec![Expr::Value(0xed), Expr::Value(0xff)])
}

pub fn breakpoint_snapshot() -> Token {
    Token::Breakpoint(None)
}


pub fn jp_label(label: &str) -> Token {
    token_for_opcode_one_arg(
        Mnemonic::Jp,
        label.into()
    )
}

/// Build a token that represents a mnemonic without any argument
pub fn token_for_opcode_no_arg(mne: Mnemonic) -> Token {
    Token::OpCode(
        mne,
        None,
        None
    )
}

/// Build a token that represents a mnemonic with only one argument
pub fn token_for_opcode_one_arg(mne: Mnemonic, data1: DataAccess) -> Token {
    Token::OpCode(
        mne,
        Some(data1),
        None
    )
}

/// Build a token that represents a mnemonic with two arguments
pub fn token_for_opcode_two_args(mne: Mnemonic, data1: DataAccess, data2: DataAccess) -> Token {
    Token::OpCode(
        mne,
        Some(data1),
        Some(data2)
    )
}





