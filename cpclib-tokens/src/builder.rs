#![allow(missing_docs)]

use cpclib_common::smol_str::SmolStr;
use paste;

/// ! Utility code to build more easily tokens to manipulate in code generators
use crate::tokens::*;

/// NOP instruction
pub fn nop() -> Token {
    token_for_opcode_no_arg(Mnemonic::Nop)
}

pub fn halt() -> Token {
    token_for_opcode_no_arg(Mnemonic::Halt)
}

pub fn di() -> Token {
    token_for_opcode_no_arg(Mnemonic::Di)
}

pub fn ei() -> Token {
    token_for_opcode_no_arg(Mnemonic::Ei)
}

pub fn ind() -> Token {
    token_for_opcode_no_arg(Mnemonic::Ind)
}

pub fn indr() -> Token {
    token_for_opcode_no_arg(Mnemonic::Indr)
}

pub fn ini() -> Token {
    token_for_opcode_no_arg(Mnemonic::Ini)
}

pub fn inir() -> Token {
    token_for_opcode_no_arg(Mnemonic::Inir)
}

pub fn outd() -> Token {
    token_for_opcode_no_arg(Mnemonic::Outd)
}

pub fn outdr() -> Token {
    token_for_opcode_no_arg(Mnemonic::Otdr)
}

pub fn outi() -> Token {
    token_for_opcode_no_arg(Mnemonic::Outi)
}

pub fn outir() -> Token {
    token_for_opcode_no_arg(Mnemonic::Otir)
}

pub fn neg_token() -> Token {
    token_for_opcode_no_arg(Mnemonic::Neg)
}

pub fn exa() -> Token {
    token_for_opcode_no_arg(Mnemonic::ExAf)
}

pub fn ex_hl_de() -> Token {
    token_for_opcode_no_arg(Mnemonic::ExHlDe)
}

/// Generate org directive
pub fn org<E: Into<Expr>>(val: E) -> Token {
    Token::Org {
        val1: val.into(),
        val2: None
    }
}

pub fn equ<S: AsRef<str>, E: Into<Expr>>(label: S, expr: E) -> Token {
    Token::Equ {
        label: label.as_ref().into(),
        expr: expr.into()
    }
}

#[allow(missing_docs)]
pub fn label<S: AsRef<str>>(label: S) -> Token {
    Token::Label(label.as_ref().into())
}

/// Generate an ASSERT token from the string description of the expression
pub fn assert_str<S: AsRef<str>>(expr: S) -> Token {
    Token::Assert(expr.as_ref().into(), None)
}

/// Generate a comment
#[allow(missing_docs)]
pub fn comment<S: AsRef<str>>(label: S) -> Token {
    Token::Comment(label.as_ref().to_owned())
}

pub fn r#if(cond: TestKind, lst: Listing) -> Token {
    IfBuilder::default().condition(cond, lst).build()
}

/// Generate defs directive
pub fn defs_expr<E: Into<Expr>>(expr: E) -> Token {
    Token::Defs(vec![(expr.into(), None)])
}

/// Generate defs directive
pub fn defs_expr_expr<E1: Into<Expr>, E2: Into<Expr>>(count: E1, value: E2) -> Token {
    Token::Defs(vec![(count.into(), value.into().into())])
}

/// Generate defw directive with one argument
pub fn defb<E: Into<Expr>>(val: E) -> Token {
    Token::Defb(vec![val.into()])
}

/// Generate defb directive from a slice of expression
pub fn defb_elements<E>(elements: &[E]) -> Token
where E: Clone + Into<Expr> {
    let mut data = Vec::new();
    for val in elements {
        let val = val.clone();
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
        None,
        None
    )
}

#[allow(missing_docs)]
pub fn push_iy() -> Token {
    Token::OpCode(
        Mnemonic::Push,
        Some(DataAccess::IndexRegister16(IndexRegister16::Iy)),
        None,
        None
    )
}

#[allow(missing_docs)]
pub fn pop_ix() -> Token {
    Token::OpCode(
        Mnemonic::Pop,
        Some(DataAccess::IndexRegister16(IndexRegister16::Ix)),
        None,
        None
    )
}

#[allow(missing_docs)]
pub fn pop_iy() -> Token {
    Token::OpCode(
        Mnemonic::Pop,
        Some(DataAccess::IndexRegister16(IndexRegister16::Iy)),
        None,
        None
    )
}

/// Ret token
pub fn ret() -> Token {
    Token::OpCode(Mnemonic::Ret, None, None, None)
}

#[allow(missing_docs)]
pub fn breakpoint_winape() -> Token {
    Token::Defb(vec![Expr::Value(0xED), Expr::Value(0xFF)])
}

#[allow(missing_docs)]
pub fn breakpoint_snapshot() -> Token {
    todo!()
}

#[allow(missing_docs)]
pub fn jp_label(label: &str) -> Token {
    token_for_opcode_latest_arg(Mnemonic::Jp, label.into())
}

#[allow(missing_docs)]
pub fn jp_ix() -> Token {
    token_for_opcode_latest_arg(
        Mnemonic::Jp,
        DataAccess::MemoryIndexRegister16(IndexRegister16::Ix)
    )
}

#[allow(missing_docs)]
pub fn jp_iy() -> Token {
    token_for_opcode_latest_arg(
        Mnemonic::Jp,
        DataAccess::MemoryIndexRegister16(IndexRegister16::Iy)
    )
}

#[allow(missing_docs)]
pub fn jp_hl() -> Token {
    token_for_opcode_latest_arg(Mnemonic::Jp, DataAccess::MemoryRegister16(Register16::Hl))
}

#[allow(missing_docs)]
pub fn exx() -> Token {
    token_for_opcode_no_arg(Mnemonic::Exx)
}

#[allow(missing_docs)]
pub fn incbin<S: Into<SmolStr>>(fname: S) -> Token {
    Token::Incbin {
        fname: Expr::String(fname.into()),
        transformation: BinaryTransformation::None,
        offset: None,
        length: None,
        extended_offset: None,
        off: false
    }
}

macro_rules! math_op_r8 {
    ($($reg:ident)*) => {$(
        paste::paste! {
            pub fn [<add_ $reg:lower>] () -> Token {
                token_for_opcode_two_args(
                    Mnemonic::Add,
                    Register8::A.into(),
                    Register8::$reg.into()
                )
            }

            pub fn [<sub_ $reg:lower>] () -> Token {
                token_for_opcode_two_args(
                    Mnemonic::Sub,
                    Register8::A.into(),
                    Register8::$reg.into()
                )
            }

        }
    )*}
}
math_op_r8! { A B C D E H L}

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

            /// Generate the opcode dec $reg
            #[allow(missing_docs)] pub fn [<dec_ $reg:lower>] () -> Token {
                token_for_opcode_one_arg(
                    Mnemonic::Dec,
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

            /// Generate the opcode dec $reg
            #[allow(missing_docs)] pub fn [<dec_ $reg:lower>] () -> Token {
                token_for_opcode_one_arg(
                    Mnemonic::Dec,
                    Register16::$reg.into()
                )
            }
        }
    )*}
}
inc_r16! {Af Bc De Hl}

pub fn ld_r8_expr<R: Into<Register8>, E: Into<Expr>>(r: R, e: E) -> Token {
    token_for_opcode_two_args(Mnemonic::Ld, r.into().into(), e.into().into())
}

pub fn ld_r16_expr<R: Into<Register16>, E: Into<Expr>>(r: R, e: E) -> Token {
    token_for_opcode_two_args(Mnemonic::Ld, r.into().into(), e.into().into())
}

/// I have clear doubt that  this exists really
#[allow(missing_docs)]
pub fn ld_l_mem_ix(expr: Expr) -> Token {
    token_for_opcode_two_args(
        Mnemonic::Ld,
        Register8::L.into(),
        DataAccess::IndexRegister16WithIndex(
            IndexRegister16::Ix,
            if expr.is_negated() {
                BinaryOperation::Sub
            }
            else {
                BinaryOperation::Add
            },
            if expr.is_negated() {
                expr.negate()
            }
            else {
                expr
            }
        )
    )
}

pub fn ld_mem_bc_a() -> Token {
    token_for_opcode_two_args(
        Mnemonic::Ld,
        DataAccess::MemoryRegister16(Register16::Bc),
        DataAccess::Register8(Register8::A)
    )
}

pub fn ld_mem_de_a() -> Token {
    token_for_opcode_two_args(
        Mnemonic::Ld,
        DataAccess::MemoryRegister16(Register16::Bc),
        DataAccess::Register8(Register8::A)
    )
}

pub fn ld_a_mem_bc() -> Token {
    token_for_opcode_two_args(
        Mnemonic::Ld,
        DataAccess::Register8(Register8::A),
        DataAccess::MemoryRegister16(Register16::Bc)
    )
}

pub fn ld_a_mem_de() -> Token {
    token_for_opcode_two_args(
        Mnemonic::Ld,
        DataAccess::Register8(Register8::A),
        DataAccess::MemoryRegister16(Register16::De)
    )
}

macro_rules! ld_r16_expr {
    ($($reg:ident)*) => {$(
        paste::paste! {
            /// Generate the opcode LD $reg, expr
            #[allow(missing_docs)] pub fn [<ld_ $reg:lower _expr>]<E:Into<Expr>> (val: E) -> Token {
                token_for_opcode_two_args(
                    Mnemonic::Ld,
                    Register16::$reg.into(),
                    val.into().into()
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
    Sp
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

macro_rules! ld_r8_r8 {
    ($($reg:ident,$reg2:ident)*) => {$(
        paste::paste! {
            /// Generate the opcode LD $reg, reg
            #[allow(missing_docs)] pub fn [<ld_ $reg:lower _ $reg2:lower>]() -> Token {
                token_for_opcode_two_args(
                    Mnemonic::Ld,
                    Register8::$reg.into(),
                    Register8::$reg2.into(),
                )
            }
        }
    )*}
}

ld_r8_r8! {
    A,A A,B A,C A,D A,E A,H A,L
    B,A B,B B,C B,D B,E B,H B,L
    C,A C,B C,C C,D C,E C,H C,L
    D,A D,B D,C D,D D,E D,H D,L
    E,A E,B E,C E,D E,E E,H E,L
    H,A H,B H,C H,D H,E H,H H,L
    L,A L,B L,C L,D L,E L,H L,L
}

#[allow(missing_docs)]
pub fn ld_mem_expr_a<E: Into<Expr>>(e: E) -> Token {
    token_for_opcode_two_args(
        Mnemonic::Ld,
        DataAccess::Memory(e.into()),
        Register8::A.into()
    )
}

#[allow(missing_docs)]
pub fn ld_mem_hl_expr<E: Into<Expr>>(e: E) -> Token {
    let e = e.into();
    token_for_opcode_two_args(
        Mnemonic::Ld,
        DataAccess::MemoryRegister16(Register16::Hl),
        e.into()
    )
}

macro_rules! ld_mem_hl_r8 {
    ($($reg:ident)*) => {$(
        paste::paste! {
            pub fn [<ld_mem_hl_ $reg:lower>]() -> Token {
                ld_mem_hl_r8(Register8::$reg)
            }

            pub fn [<ld_ $reg:lower _mem_hl>]() -> Token {
                ld_register8_mem_hl(Register8::$reg)
            }
        }
    )*}
}

ld_mem_hl_r8! {
    A
    B
    C
    D
    E
    H
    L
}

pub fn ld_mem_hl_r8(reg: Register8) -> Token {
    token_for_opcode_two_args(
        Mnemonic::Ld,
        DataAccess::MemoryRegister16(Register16::Hl),
        reg.into()
    )
}

pub fn ld_register8_mem_hl(reg: Register8) -> Token {
    token_for_opcode_two_args(
        Mnemonic::Ld,
        reg.into(),
        DataAccess::MemoryRegister16(Register16::Hl)
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

macro_rules! set_res_reg8 {
    ($($reg:ident), *) => {$(
        paste::paste! {
            #[allow(missing_docs)]
            #[inline]
            pub fn [<res_ $reg:lower>](bit: u8) -> Token {
                res_reg_pos(Register8::$reg, bit)
            }

            #[allow(missing_docs)]
            #[inline]
            pub fn [<set_ $reg:lower>](bit: u8) -> Token {
                set_reg_pos(Register8::$reg, bit)
            }

        })*
    };
}

set_res_reg8! {A, B, C, D, E, H, L}

#[allow(missing_docs)]
pub fn set_mem_hl(bit: u8) -> Token {
    token_for_opcode_two_args(
        Mnemonic::Set,
        bit.into(),
        DataAccess::MemoryRegister16(Register16::Hl)
    )
}

#[allow(missing_docs)]
pub fn res_mem_hl(bit: u8) -> Token {
    token_for_opcode_two_args(
        Mnemonic::Res,
        bit.into(),
        DataAccess::MemoryRegister16(Register16::Hl)
    )
}

#[allow(missing_docs)]
#[inline]
pub fn res_reg_pos(reg: Register8, bit: u8) -> Token {
    token_for_opcode_two_args(Mnemonic::Res, bit.into(), reg.into())
}

#[allow(missing_docs)]
#[inline]
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

/// Build a token that represents a mnemonic with only one argument BUT positioned in the last position (for jp for example)
#[allow(missing_docs)]
pub fn token_for_opcode_latest_arg(mne: Mnemonic, data2: DataAccess) -> Token {
    Token::OpCode(mne, None, Some(data2), None)
}

/// Build a token that represents a mnemonic with two arguments
#[allow(missing_docs)]
pub fn token_for_opcode_two_args(mne: Mnemonic, data1: DataAccess, data2: DataAccess) -> Token {
    Token::OpCode(mne, Some(data1), Some(data2), None)
}

#[allow(missing_docs)]
pub fn section(section: &str) -> Token {
    Token::Section(section.into())
}

#[derive(Default)]
pub struct IfBuilder {
    conditions: Vec<(TestKind, Listing)>,
    r#else: Option<Listing>
}

impl IfBuilder {
    pub fn build(self) -> Token {
        assert!(!self.conditions.is_empty());
        Token::If(self.conditions, self.r#else)
    }

    pub fn condition(mut self, cond: TestKind, lst: Listing) -> Self {
        assert!(self.r#else.is_none());
        self.conditions.push((cond, lst));
        self
    }

    pub fn r#else(mut self, lst: Listing) -> Self {
        self.r#else = Some(lst);
        self
    }
}

#[derive(Default)]
pub struct ListingBuilder {
    lst: Listing
}

macro_rules! ld_r8_expr_builder {
    ($($reg:ident)*) => {$(
        paste::paste! {
            /// Generate the opcode LD $reg, expr
            pub fn [<ld_ $reg:lower _expr>]<E: Into<Expr>>(mut self, expr: E) -> Self {
                self.lst.add([<ld_ $reg:lower _expr>](expr));
                self
            }


            pub fn [<inc_ $reg:lower>](mut self) -> Self {
                self.lst.add([<inc_ $reg:lower>]());
                self
            }

            pub fn [<dec_ $reg:lower>](mut self) -> Self {
                self.lst.add([<dec_ $reg:lower>]());
                self
            }
        }
    )*}
}

macro_rules! ld_r16_expr_builder {
    ($($reg:ident)*) => {$(
        paste::paste! {
            /// Generate the opcode LD $reg, expr
            pub fn [<ld_ $reg:lower _expr>]<E: Into<Expr>>(mut self, expr: E) -> Self {
                self.lst.add([<ld_ $reg:lower _expr>](expr));
                self
            }


            pub fn [<inc_ $reg:lower>](mut self) -> Self {
                self.lst.add([<inc_ $reg:lower>]());
                self
            }

            pub fn [<dec_ $reg:lower>](mut self) -> Self {
                self.lst.add([<dec_ $reg:lower>]());
                self
            }

        }
    )*}
}

macro_rules! ld_mem_r16_builder {
    ($($reg:ident)*) => {$(
        paste::paste! {

            pub fn [<ld_mem_ $reg:lower _a>](mut self) -> Self {
                self.lst.add([<ld_mem_ $reg:lower _a>]());
                self
            }

            pub fn [<ld_a_ mem_ $reg:lower>](mut self) -> Self {
                self.lst.add([<ld_a_mem_ $reg:lower>]());
                self
            }
        }
    )*}
}

macro_rules! ld_r8_r8_builder {
    ($($reg1:ident,$reg2:ident)*) => {$(
        paste::paste! {
            /// Generate the opcode LD $reg, expr
            pub fn [<ld_ $reg1:lower _ $reg2:lower>](mut self) -> Self {
                self.lst.add([<ld_ $reg1:lower _ $reg2:lower>]());
                self
            }
        }
    )*}
}

macro_rules! ld_mem_hl_r8_builder {
    ($($reg:ident)*) => {$(
        paste::paste! {
            pub fn [<ld_mem_hl_ $reg:lower>](mut self) -> Self {
                self.lst.add(ld_mem_hl_r8(Register8::$reg));
                self
            }

            pub fn [<ld_ $reg:lower _mem_hl>](mut self) -> Self {
                self.lst.add(ld_register8_mem_hl(Register8::$reg));
                self
            }
        }
    )*}
}

macro_rules! no_arg_builder {
    ($($op:ident)*) => {
        $(
            pub fn $op(mut self) -> Self {
                self.lst.add($op());
                self
            }
        )*
    }
}

macro_rules! math_op_r8_builder {
    ($($reg:ident)*) => {$(
        paste::paste! {

            pub fn [<add_ $reg:lower>] (mut self) -> Self {
                self.lst.add([<add_ $reg:lower>] () );
                self
            }

            pub fn [<sub_ $reg:lower>] (mut self) -> Self {
                self.lst.add([<add_ $reg:lower>] () );
                self
            }
        }
    )*}
}

impl ListingBuilder {
    ld_r8_expr_builder! {a b c d e h l}

    ld_r16_expr_builder! {af bc de hl}

    ld_mem_r16_builder! {bc de}

    ld_mem_hl_r8_builder! {A B C D E H L}

    no_arg_builder! {exx nop ldi ldd ldir lddr neg_token exa ex_hl_de halt di ei ind indr outd outdr outi outir ret}

    ld_r8_r8_builder! {
        A,A A,B A,C A,D A,E A,H A,L
        B,A B,B B,C B,D B,E B,H B,L
        C,A C,B C,C C,D C,E C,H C,L
        D,A D,B D,C D,D D,E D,H D,L
        E,A E,B E,C E,D E,E E,H E,L
        H,A H,B H,C H,D H,E H,H H,L
        L,A L,B L,C L,D L,E L,H L,L
    }

    math_op_r8_builder! { A B C D E H L}

    pub fn ld_mem_hl_expr<E: Into<Expr>>(mut self, e: E) -> Self {
        self.lst.add(ld_mem_hl_expr(e));
        self
    }

    pub fn ld_mem_hl_r8<R: Into<Register8>>(mut self, r: R) -> Self {
        self.lst.add(ld_mem_hl_r8(r.into()));
        self
    }

    pub fn ld_r8_expr<R: Into<Register8>, E: Into<Expr>>(mut self, r: R, e: E) -> Self {
        self.lst.add(ld_r8_expr(r, e));
        self
    }

    pub fn ld_r16_expr<R: Into<Register16>, E: Into<Expr>>(mut self, r: R, e: E) -> Self {
        self.lst.add(ld_r16_expr(r, e));
        self
    }

    pub fn call<S: Into<SmolStr>>(self, label: S) -> Self {
        self.call_expr(Expr::Label(label.into()))
    }

    pub fn call_expr<E: Into<Expr>>(mut self, expr: E) -> Self {
        self.lst.add(call_expr(expr));
        self
    }

    pub fn or_expr<E: Into<Expr>>(mut self, expr: E) -> Self {
        let e = expr.into();
        self.lst
            .add(token_for_opcode_one_arg(Mnemonic::Or, e.into()));
        self
    }

    pub fn xor_expr<E: Into<Expr>>(mut self, expr: E) -> Self {
        let e = expr.into();
        self.lst
            .add(token_for_opcode_one_arg(Mnemonic::Xor, e.into()));
        self
    }

    pub fn and_expr<E: Into<Expr>>(mut self, expr: E) -> Self {
        let e = expr.into();
        self.lst
            .add(token_for_opcode_one_arg(Mnemonic::And, e.into()));
        self
    }

    pub fn or_r8(mut self, r: Register8) -> Self {
        self.lst
            .add(token_for_opcode_one_arg(Mnemonic::Or, r.into()));
        self
    }

    pub fn xor_r8(mut self, r: Register8) -> Self {
        self.lst
            .add(token_for_opcode_one_arg(Mnemonic::Xor, r.into()));
        self
    }

    pub fn and_r8(mut self, r: Register8) -> Self {
        self.lst
            .add(token_for_opcode_one_arg(Mnemonic::And, r.into()));
        self
    }

    pub fn comment<S: Into<String>>(mut self, comment: S) -> Self {
        self.lst.add_comment(comment);
        self
    }

    pub fn extend(mut self, other: Listing) -> Self {
        self.lst.inject_listing(&other);
        self
    }

    /// Add a repeating code THAT DOES NOT use the counter
    pub fn repeat<L: Into<Listing>>(mut self, count: i32, code: L) -> Self {
        let test = Expr::Value(count);
        let rpt = Token::Repeat(test, code.into(), None, None);
        self.lst.add(rpt);
        self
    }

    pub fn build(self) -> Listing {
        self.lst
    }
}

/// Code function that generate Listing instead of Tokens
pub mod routines {
    use crate::builder::*;
    use crate::tokens::listing_element::Listing;

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
        ld_af_expr(0);
    }
}
