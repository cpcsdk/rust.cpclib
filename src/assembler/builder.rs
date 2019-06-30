///! Utility code to build more easily tokens to manipulate in code generators
use crate::assembler::tokens::*;
use casey::{camel, lower, shouty, snake, upper};
use paste;

#[allow(missing_docs)] pub fn equ<S: AsRef<str>, E: Into<Expr>>(label: S, expr: E) -> Token {
    Token::Equ(label.as_ref().to_owned(), expr.into())
}

#[allow(missing_docs)] pub fn label<S: AsRef<str>>(label: S) -> Token {
    Token::Label(label.as_ref().to_owned())
}

#[allow(missing_docs)] pub fn comment<S: AsRef<str>>(label: S) -> Token {
    Token::Comment(label.as_ref().to_owned())
}

#[allow(missing_docs)] pub fn out_c_b() -> Token {
    out_c_register8(Register8::B)
}
#[allow(missing_docs)] pub fn out_c_c() -> Token {
    out_c_register8(Register8::C)
}
#[allow(missing_docs)] pub fn out_c_d() -> Token {
    out_c_register8(Register8::D)
}
#[allow(missing_docs)] pub fn out_c_e() -> Token {
    out_c_register8(Register8::E)
}
#[allow(missing_docs)] pub fn out_c_h() -> Token {
    out_c_register8(Register8::H)
}
#[allow(missing_docs)] pub fn out_c_l() -> Token {
    out_c_register8(Register8::L)
}
#[allow(missing_docs)] pub fn out_c_a() -> Token {
    out_c_register8(Register8::A)
}

#[allow(missing_docs)] pub fn out_c_register8(reg: Register8) -> Token {
    token_for_opcode_two_args(Mnemonic::Out, Register8::C.into(), reg.into())
}

#[allow(missing_docs)] pub fn push_af() -> Token {
    push_or_pop(Mnemonic::Push, Register16::Af)
}

#[allow(missing_docs)] pub fn push_bc() -> Token {
    push_or_pop(Mnemonic::Push, Register16::Bc)
}

#[allow(missing_docs)] pub fn push_de() -> Token {
    push_or_pop(Mnemonic::Push, Register16::De)
}

#[allow(missing_docs)] pub fn push_hl() -> Token {
    push_or_pop(Mnemonic::Push, Register16::Hl)
}

#[allow(missing_docs)] pub fn pop_af() -> Token {
    push_or_pop(Mnemonic::Pop, Register16::Af)
}

#[allow(missing_docs)] pub fn pop_bc() -> Token {
    push_or_pop(Mnemonic::Pop, Register16::Bc)
}

#[allow(missing_docs)] pub fn pop_de() -> Token {
    push_or_pop(Mnemonic::Pop, Register16::De)
}

#[allow(missing_docs)] pub fn pop_hl() -> Token {
    push_or_pop(Mnemonic::Pop, Register16::Hl)
}

#[inline]
fn push_or_pop(op: Mnemonic, reg: Register16) -> Token {
    token_for_opcode_one_arg(op, reg.into())
}

#[allow(missing_docs)] pub fn push_ix() -> Token {
    Token::OpCode(
        Mnemonic::Push,
        Some(DataAccess::IndexRegister16(IndexRegister16::Ix)),
        None,
    )
}

#[allow(missing_docs)] pub fn push_iy() -> Token {
    Token::OpCode(
        Mnemonic::Push,
        Some(DataAccess::IndexRegister16(IndexRegister16::Iy)),
        None,
    )
}

#[allow(missing_docs)] pub fn pop_ix() -> Token {
    Token::OpCode(
        Mnemonic::Pop,
        Some(DataAccess::IndexRegister16(IndexRegister16::Ix)),
        None,
    )
}

#[allow(missing_docs)] pub fn pop_iy() -> Token {
    Token::OpCode(
        Mnemonic::Pop,
        Some(DataAccess::IndexRegister16(IndexRegister16::Iy)),
        None,
    )
}

#[allow(missing_docs)] pub fn breakpoint_winape() -> Token {
    Token::Defb(vec![Expr::Value(0xed), Expr::Value(0xff)])
}

#[allow(missing_docs)] pub fn breakpoint_snapshot() -> Token {
    Token::Breakpoint(None)
}

#[allow(missing_docs)] pub fn jp_label(label: &str) -> Token {
    token_for_opcode_latest_arg(Mnemonic::Jp, label.into())
}

#[allow(missing_docs)] pub fn exx() -> Token {
    token_for_opcode_no_arg(Mnemonic::Exx)
}

#[allow(missing_docs)] pub fn inc_l() -> Token {
    inc_register8(Register8::L)
}

#[allow(missing_docs)] pub fn inc_register8(reg: Register8) -> Token {
    token_for_opcode_one_arg(Mnemonic::Inc, reg.into())
}

/// I have clear doubt that  this exists really
#[allow(missing_docs)] pub fn ld_l_mem_ix(expr: Expr) -> Token {
    token_for_opcode_two_args(
        Mnemonic::Ld,
        Register8::L.into(),
        DataAccess::IndexRegister16WithIndex(IndexRegister16::Ix, expr),
    )
}

macro_rules! ld_r16_expr {
    ($($reg:ident, $name:ident)*) => {$(
        paste::item_with_macros! {
            /// Generate the opcode LD $reg, expr
            #[allow(missing_docs)] pub fn [<ld_ $name _expr>] (val: Expr) -> Token {
                token_for_opcode_two_args(
                    Mnemonic::Ld,
                    Register16::$reg.into(),
                    val.into()
                )
            }
        }
    )*}
}

// TODO remove these extra uneeded arguments
ld_r16_expr! {
    Af,af
    Bc,bc
    De,de
    Hl,hl
}

/*
#[allow(missing_docs)] pub fn ld_de_expr(val: Expr) -> Token {
    token_for_opcode_two_args(
        Mnemonic::Ld,
        Register16::De.into(),
        val.into()
    )
}
*/

#[allow(missing_docs)] pub fn ld_d_mem_hl() -> Token {
    ld_register8_mem_hl(Register8::D)
}

#[allow(missing_docs)] pub fn ld_e_mem_hl() -> Token {
    ld_register8_mem_hl(Register8::E)
}

#[allow(missing_docs)] pub fn ld_register8_mem_hl(reg: Register8) -> Token {
    token_for_opcode_two_args(
        Mnemonic::Ld,
        reg.into(),
        DataAccess::MemoryRegister16(Register16::Hl),
    )
}

#[allow(missing_docs)] pub fn ld_mem_hl_d() -> Token {
    ld_mem_hl_register8(Register8::D)
}

#[allow(missing_docs)] pub fn ld_mem_hl_e() -> Token {
    ld_mem_hl_register8(Register8::E)
}

#[allow(missing_docs)] pub fn ld_mem_hl_b() -> Token {
    ld_mem_hl_register8(Register8::B)
}

#[allow(missing_docs)] pub fn ld_mem_hl_c() -> Token {
    ld_mem_hl_register8(Register8::C)
}

#[allow(missing_docs)] pub fn ld_mem_hl_h() -> Token {
    ld_mem_hl_register8(Register8::H)
}

#[allow(missing_docs)] pub fn ld_mem_hl_l() -> Token {
    ld_mem_hl_register8(Register8::L)
}

#[allow(missing_docs)] pub fn ld_mem_hl_register8(reg: Register8) -> Token {
    token_for_opcode_two_args(
        Mnemonic::Ld,
        DataAccess::MemoryRegister16(Register16::Hl),
        reg.into(),
    )
}

#[allow(missing_docs)] pub fn ldi() -> Token {
    token_for_opcode_no_arg(Mnemonic::Ldi)
}

#[allow(missing_docs)] pub fn ldd() -> Token {
    token_for_opcode_no_arg(Mnemonic::Ldd)
}

#[allow(missing_docs)] pub fn ldir() -> Token {
    token_for_opcode_no_arg(Mnemonic::Ldir)
}

#[allow(missing_docs)] pub fn lddr() -> Token {
    token_for_opcode_no_arg(Mnemonic::Lddr)
}

#[allow(missing_docs)] pub fn res_d(bit: u8) -> Token {
    res_reg_pos(Register8::D, bit)
}

#[allow(missing_docs)] pub fn set_d(bit: u8) -> Token {
    set_reg_pos(Register8::D, bit)
}

#[allow(missing_docs)] pub fn res_reg_pos(reg: Register8, bit: u8) -> Token {
    token_for_opcode_two_args(Mnemonic::Res, bit.into(), reg.into())
}

#[allow(missing_docs)] pub fn set_reg_pos(reg: Register8, bit: u8) -> Token {
    token_for_opcode_two_args(Mnemonic::Set, bit.into(), reg.into())
}

/// Build a token that represents a mnemonic without any argument
#[allow(missing_docs)] pub fn token_for_opcode_no_arg(mne: Mnemonic) -> Token {
    Token::OpCode(mne, None, None)
}

/// Build a token that represents a mnemonic with only one argument
#[allow(missing_docs)] pub fn token_for_opcode_one_arg(mne: Mnemonic, data1: DataAccess) -> Token {
    Token::OpCode(mne, Some(data1), None)
}

/// Build a token that represents a mnemonic with only one argument BUT positionned in the last position (for jp for example)
#[allow(missing_docs)] pub fn token_for_opcode_latest_arg(mne: Mnemonic, data2: DataAccess) -> Token {
    Token::OpCode(mne, None, Some(data2))
}

/// Build a token that represents a mnemonic with two arguments
#[allow(missing_docs)] pub fn token_for_opcode_two_args(mne: Mnemonic, data1: DataAccess, data2: DataAccess) -> Token {
    Token::OpCode(mne, Some(data1), Some(data2))
}
