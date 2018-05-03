///! Utility code to build more easily tokens to manipulate in code generators

use assembler::tokens::*;

pub fn push_de() -> Token {
    Token::OpCode(
        Mnemonic::Push,
        Some(DataAccess::Register16(Register16::De)),
        None
    )
}






