///! Utility code to build more easily tokens to manipulate in code generators

use assembler::tokens::*;

pub fn push_de() -> Token {
    Token::OpCode(
        Mnemonic::Push,
        Some(DataAccess::Register16(Register16::De)),
        None
    )
}


pub fn breakpoint_winape() -> Token {
    Token::Db(vec![Expr::Value(0xed), Expr::Value(0xff)])
}






