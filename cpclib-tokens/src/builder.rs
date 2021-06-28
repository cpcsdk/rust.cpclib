///! Utility code to build more easily tokens to manipulate in code generators
use crate::tokens::*;

use paste;

/// NOP instruction
pub fn nop() -> Token {
    token_for_opcode_no_arg(Mnemonic::Nop)
}

/// Generate org directive
pub fn org<E: Into<Expr>>(val: E) -> Token {
    Token::Org(val.into(), None)
}

#[allow(missing_docs)]
pub fn equ<S: AsRef<str>, E: Into<Expr>>(label: S, expr: E) -> Token {
    Token::Equ(label.as_ref().to_owned(), expr.into())
}

#[allow(missing_docs)]
pub fn label<S: AsRef<str>>(label: S) -> Token {
    Token::Label(label.as_ref().to_owned())
}

/**
 * Generate an ASSERT token from the string description of the expression
 */
pub fn assert_str<S: AsRef<str>>(expr: S) -> Token {
    Token::Assert(expr.as_ref().into(), None)
}

/// Generate a call

#[allow(missing_docs)]
pub fn comment<S: AsRef<str>>(label: S) -> Token {
    Token::Comment(label.as_ref().to_owned())
}

/// Generate defs directive
pub fn defs_expr<E: Into<Expr>>(expr: E) -> Token {
    Token::Defs(expr.into(), None)
}

/// Generate defs directive
pub fn defs_expr_expr<E1: Into<Expr>, E2: Into<Expr>>(count: E1, value: E2) -> Token {
    Token::Defs(count.into(), value.into().into())
}

/// Generate defw directive with one argument
pub fn defb<E: Into<Expr>>(val: E) -> Token {
    Token::Defb(vec![val.into()])
}

/// Generate defb directive from a slice of expression
pub fn defb_elements<E: Into<Expr>>(elements: &[E]) -> Token
where
    E: Copy,
{
    let mut data = Vec::new();
    for val in elements {
        let val = *val;
        let expr = val.into();
        data.push(expr);
    }
    Token::Defb(data)
}

/// Generate defw directive with one argument
pub fn defw<E: Into<Expr>>(val: E) -> Token {
    Token::Defw(vec![val.into()])
}

/// DJNZ opcode
pub fn djnz_expr<E: Into<Expr>>(expr: E) -> Token {
    mnemonic_with_single_expr(Mnemonic::Djnz, expr)
}

/// Call opcode
pub fn call_expr<E: Into<Expr>>(expr: E) -> Token {
    mnemonic_with_single_expr(Mnemonic::Call, expr)
}

/// Use this function to generate tokens having a mnemonic with a single expression argument
/// TODO write a macro instead and automatically generate all the cases
fn mnemonic_with_single_expr<E: Into<Expr>>(mne: Mnemonic, expr: E) -> Token {
    Token::OpCode(mne, Some(expr.into().into()), None, None)
}


#[allow(missing_docs)]
pub fn out_c_b() -> Token {
    out_c_register8(Register8::B)
}
#[allow(missing_docs)]
pub fn out_c_c() -> Token {
    out_c_register8(Register8::C)
}
#[allow(missing_docs)]
pub fn out_c_d() -> Token {
    out_c_register8(Register8::D)
}
#[allow(missing_docs)]
pub fn out_c_e() -> Token {
    out_c_register8(Register8::E)
}
#[allow(missing_docs)]
pub fn out_c_h() -> Token {
    out_c_register8(Register8::H)
}
#[allow(missing_docs)]
pub fn out_c_l() -> Token {
    out_c_register8(Register8::L)
}
#[allow(missing_docs)]
pub fn out_c_a() -> Token {
    out_c_register8(Register8::A)
}

#[allow(missing_docs)]
pub fn out_c_register8(reg: Register8) -> Token {
    token_for_opcode_two_args(Mnemonic::Out, DataAccess::PortC, reg.into())
}

#[allow(missing_docs)]
pub fn push_af() -> Token {
    push_or_pop(Mnemonic::Push, Register16::Af)
}

#[allow(missing_docs)]
pub fn push_bc() -> Token {
    push_or_pop(Mnemonic::Push, Register16::Bc)
}

#[allow(missing_docs)]
pub fn push_de() -> Token {
    push_or_pop(Mnemonic::Push, Register16::De)
}

#[allow(missing_docs)]
pub fn push_hl() -> Token {
    push_or_pop(Mnemonic::Push, Register16::Hl)
}

#[allow(missing_docs)]
pub fn pop_af() -> Token {
    push_or_pop(Mnemonic::Pop, Register16::Af)
}

#[allow(missing_docs)]
pub fn pop_bc() -> Token {
    push_or_pop(Mnemonic::Pop, Register16::Bc)
}

#[allow(missing_docs)]
pub fn pop_de() -> Token {
    push_or_pop(Mnemonic::Pop, Register16::De)
}

#[allow(missing_docs)]
pub fn pop_hl() -> Token {
    push_or_pop(Mnemonic::Pop, Register16::Hl)
}

#[inline]
fn push_or_pop(op: Mnemonic, reg: Register16) -> Token {
    token_for_opcode_one_arg(op, reg.into())
}

#[allow(missing_docs)]
pub fn push_ix() -> Token {
    Token::OpCode(
        Mnemonic::Push,
        Some(DataAccess::IndexRegister16(IndexRegister16::Ix)),
        None, None
    )
}

#[allow(missing_docs)]
pub fn push_iy() -> Token {
    Token::OpCode(
        Mnemonic::Push,
        Some(DataAccess::IndexRegister16(IndexRegister16::Iy)),
        None, None
    )
}

#[allow(missing_docs)]
pub fn pop_ix() -> Token {
    Token::OpCode(
        Mnemonic::Pop,
        Some(DataAccess::IndexRegister16(IndexRegister16::Ix)),
        None, None
    )
}

#[allow(missing_docs)]
pub fn pop_iy() -> Token {
    Token::OpCode(
        Mnemonic::Pop,
        Some(DataAccess::IndexRegister16(IndexRegister16::Iy)),
        None, None
    )
}

/// Ret token
pub fn ret() -> Token {
    Token::OpCode(Mnemonic::Ret, None, None, None)
}

#[allow(missing_docs)]
pub fn breakpoint_winape() -> Token {
    Token::Defb(vec![Expr::Value(0xed), Expr::Value(0xff)])
}

#[allow(missing_docs)]
pub fn breakpoint_snapshot() -> Token {
    Token::Breakpoint(None)
}

#[allow(missing_docs)]
pub fn jp_label(label: &str) -> Token {
    token_for_opcode_latest_arg(Mnemonic::Jp, label.into())
}

#[allow(missing_docs)]
pub fn exx() -> Token {
    token_for_opcode_no_arg(Mnemonic::Exx)
}

#[allow(missing_docs)]
pub fn incbin<S: AsRef<str>>(fname: S) -> Token {
    Token::Incbin {
        fname: fname.as_ref().to_string(),
        transformation: BinaryTransformation::None,
        offset: None,
        length: None,
        extended_offset: None,
        off: false,
        content: None,
    }
}

macro_rules! inc_r8 {
    ($($reg:ident)*) => {$(
        paste::paste! {
            /// Generate the opcode inc $reg
            #[allow(missing_docs)] pub fn [<inc_ $reg:lower>] () -> Token {
                token_for_opcode_one_arg(
                    Mnemonic::Inc,
                    Register8::$reg.into()
                )
            }
        }
    )*}
}
inc_r8! { A B C D E H L}

macro_rules! inc_r16 {
    ($($reg:ident)*) => {$(
        paste::paste! {
            /// Generate the opcode inc $reg
            #[allow(missing_docs)] pub fn [<inc_ $reg:lower>] () -> Token {
                token_for_opcode_one_arg(
                    Mnemonic::Inc,
                    Register16::$reg.into()
                )
            }
        }
    )*}
}
inc_r16! {Af Bc De Hl}

/// I have clear doubt that  this exists really
#[allow(missing_docs)]
pub fn ld_l_mem_ix(expr: Expr) -> Token {
    token_for_opcode_two_args(
        Mnemonic::Ld,
        Register8::L.into(),
        DataAccess::IndexRegister16WithIndex(IndexRegister16::Ix, expr),
    )
}

macro_rules! ld_r16_expr {
    ($($reg:ident)*) => {$(
        paste::paste! {
            /// Generate the opcode LD $reg, expr
            #[allow(missing_docs)] pub fn [<ld_ $reg:lower _expr>] (val: Expr) -> Token {
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
    Af
    Bc
    De
    Hl
}

macro_rules! ld_r8_expr {
    ($($reg:ident)*) => {$(
        paste::paste! {
            /// Generate the opcode LD $reg, expr
            #[allow(missing_docs)] pub fn [<ld_ $reg:lower _expr>]<E: Into<Expr>> (val: E) -> Token {
                token_for_opcode_two_args(
                    Mnemonic::Ld,
                    Register8::$reg.into(),
                    val.into().into()
                )
            }
        }
    )*}
}

ld_r8_expr! {
    A
    B
    C
    D
    E
    H
    L
}

#[allow(missing_docs)]
pub fn ld_d_mem_hl() -> Token {
    ld_register8_mem_hl(Register8::D)
}

#[allow(missing_docs)]
pub fn ld_e_mem_hl() -> Token {
    ld_register8_mem_hl(Register8::E)
}

#[allow(missing_docs)]
pub fn ld_register8_mem_hl(reg: Register8) -> Token {
    token_for_opcode_two_args(
        Mnemonic::Ld,
        reg.into(),
        DataAccess::MemoryRegister16(Register16::Hl),
    )
}

#[allow(missing_docs)]
pub fn ld_mem_hl_d() -> Token {
    ld_mem_hl_register8(Register8::D)
}

#[allow(missing_docs)]
pub fn ld_mem_hl_e() -> Token {
    ld_mem_hl_register8(Register8::E)
}

#[allow(missing_docs)]
pub fn ld_mem_hl_b() -> Token {
    ld_mem_hl_register8(Register8::B)
}

#[allow(missing_docs)]
pub fn ld_mem_hl_c() -> Token {
    ld_mem_hl_register8(Register8::C)
}

#[allow(missing_docs)]
pub fn ld_mem_hl_h() -> Token {
    ld_mem_hl_register8(Register8::H)
}

#[allow(missing_docs)]
pub fn ld_mem_hl_l() -> Token {
    ld_mem_hl_register8(Register8::L)
}

#[allow(missing_docs)]
pub fn ld_mem_hl_register8(reg: Register8) -> Token {
    token_for_opcode_two_args(
        Mnemonic::Ld,
        DataAccess::MemoryRegister16(Register16::Hl),
        reg.into(),
    )
}

#[allow(missing_docs)]
pub fn ldi() -> Token {
    token_for_opcode_no_arg(Mnemonic::Ldi)
}

#[allow(missing_docs)]
pub fn ldd() -> Token {
    token_for_opcode_no_arg(Mnemonic::Ldd)
}

#[allow(missing_docs)]
pub fn ldir() -> Token {
    token_for_opcode_no_arg(Mnemonic::Ldir)
}

#[allow(missing_docs)]
pub fn lddr() -> Token {
    token_for_opcode_no_arg(Mnemonic::Lddr)
}

#[allow(missing_docs)]
pub fn res_d(bit: u8) -> Token {
    res_reg_pos(Register8::D, bit)
}

#[allow(missing_docs)]
pub fn set_d(bit: u8) -> Token {
    set_reg_pos(Register8::D, bit)
}

#[allow(missing_docs)]
pub fn res_reg_pos(reg: Register8, bit: u8) -> Token {
    token_for_opcode_two_args(Mnemonic::Res, bit.into(), reg.into())
}

#[allow(missing_docs)]
pub fn set_reg_pos(reg: Register8, bit: u8) -> Token {
    token_for_opcode_two_args(Mnemonic::Set, bit.into(), reg.into())
}

/// Build a token that represents a mnemonic without any argument
#[allow(missing_docs)]
pub fn token_for_opcode_no_arg(mne: Mnemonic) -> Token {
    Token::OpCode(mne, None, None, None)
}

/// Build a token that represents a mnemonic with only one argument
#[allow(missing_docs)]
pub fn token_for_opcode_one_arg(mne: Mnemonic, data1: DataAccess) -> Token {
    Token::OpCode(mne, Some(data1), None, None)
}

/// Build a token that represents a mnemonic with only one argument BUT positionned in the last position (for jp for example)
#[allow(missing_docs)]
pub fn token_for_opcode_latest_arg(mne: Mnemonic, data2: DataAccess) -> Token {
    Token::OpCode(mne, None, Some(data2), None)
}

/// Build a token that represents a mnemonic with two arguments
#[allow(missing_docs)]
pub fn token_for_opcode_two_args(mne: Mnemonic, data1: DataAccess, data2: DataAccess) -> Token {
    Token::OpCode(mne, Some(data1), Some(data2), None)
}

/// Code function that generate Listing instead of Tokens
pub mod routines {
    use crate::builder::*;
    use crate::tokens::tokens::Listing;

    /// Generate the listing that handle a wait loop
    /// Idea comes from Rhino/Batman Group http://cpcrulez.fr/forum/viewtopic.php?p=15827#p15827

    #[allow(dead_code)]
    pub fn wait(mut duration: u32) -> Listing {
        let wait_code_for = |l_duration| {
            assert!(l_duration > 0);
            let loops = (l_duration - 1) / 4;
            let loopsx4 = loops * 4;
            let nops = l_duration - loopsx4 - 1;

            let mut listing = Listing::default();
            if loops != 0 {
                listing.push(ld_b_expr(loops));
                listing.push(djnz_expr("$"));
            }

            listing.push(defs_expr_expr(nops, 0));
            listing
        };

        let mut full_code = Listing::new();
        while duration > 1024 {
            full_code.inject_listing(&wait_code_for(1024));
            duration -= 1024;
        }
        if duration != 0 {
            full_code.inject_listing(&wait_code_for(duration));
        }

        full_code
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_ld_r16() {
        use super::*;
        // just check if it compiles
        ld_af_expr(0.into());
    }
}
