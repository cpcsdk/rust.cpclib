use std::any::Any;
use std::borrow::Cow;
use std::fmt;

use cpclib_common::itertools::Itertools;
use cpclib_common::smol_str::SmolStr;
use cpclib_sna::SnapshotVersion;

use crate::tokens::data_access::*;
use crate::tokens::expression::*;
use crate::{Listing, Register8};

#[derive(Debug, PartialEq, Eq, Clone)]
/// This structures encode the parameters of macros.
/// The usual parameter is a string.
/// However, it can be a list of parameters to allows nested structs
pub enum MacroParam {
    /// Standard argument
    Single(String),
    /// A list of argument that will be provided in a nested macro call
    List(Vec<Box<MacroParam>>)
}

impl ToString for MacroParam {
    fn to_string(&self) -> String {
        match self {
            Self::Single(s) => s.clone(),
            Self::List(l) => {
                format!("[{}]", l.iter().map(|p| p.to_string()).join(","))
            }
        }
    }
}

pub trait MacroParamElement : Clone{
    fn empty() -> Self;

    fn is_single(&self) -> bool;
    fn is_list(&self) -> bool;

    fn single_argument(&self) -> &str;
    fn list_argument(&self) -> &[Box<Self>];
}

impl MacroParamElement for MacroParam {
    fn empty() -> Self {
        Self::Single("".to_owned())
    }

    fn is_single(&self) -> bool {
        matches!(self, MacroParam::Single(_))
    }

    fn is_list(&self) -> bool {
        matches!(self, MacroParam::List(_))
    }

    fn single_argument(&self) -> &str {
        match self {
            MacroParam::Single(s) => s,
            MacroParam::List(_) => unreachable!(),
        }
    }

    fn list_argument(&self) -> &[Box<Self>] {
        match self {
            MacroParam::Single(_) => unreachable!(),
            MacroParam::List(l) => l,
        }
    }
}

impl MacroParam {


    pub fn is_single(&self) -> bool {
        match self {
            Self::Single(_) => true,
            _ => false
        }
    }

    /// Rename the arguments when they are a macro call
    /// XXX I am pretty sure such implementation is faulty when there are nested calls !!! It needs to be checked (maybe nested stuff has to be removed)
    pub fn do_apply_macro_labels_modification(&mut self, seed: usize) {
        match self {
            Self::Single(s) => {
                Expr::do_apply_macro_labels_modification(s, seed);
            }
            Self::List(l) => {
                l.iter_mut().for_each(|m| {
                    m.do_apply_macro_labels_modification(seed);
                })
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Single(s) => s.trim().is_empty(),
            Self::List(_l) => false
        }
    }
}

#[remain::sorted]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[allow(missing_docs)]
pub enum Mnemonic {
    Adc,
    Add,
    And,
    Bit,
    Call,
    Ccf,
    Cp,
    Cpd,
    Cpdr,
    Cpi,
    Cpir,
    Cpl,
    Daa,
    Dec,
    Di,
    Djnz,
    Ei,
    ExAf,
    ExHlDe,
    ExMemSp,
    Exx,
    Halt,
    Im,
    In,
    Inc,
    Ind,
    Indr,
    Ini,
    Inir,
    Jp,
    Jr,
    Ld,
    Ldd,
    Lddr,
    Ldi,
    Ldir,
    Neg,
    Nop,
    Nop2, // Fake instruction that generate a breakpoint on winape
    Or,
    Otdr,
    Otir,
    Out,
    Outd,
    Outi,
    Pop,
    Push,
    Res,
    Ret,
    Reti,
    Retn,
    Rl,
    Rla,
    Rlc,
    Rlca,
    Rld,
    Rr,
    Rra,
    Rrc,
    Rrca,
    Rrd,
    Rst,
    Sbc,
    Scf,
    Set,
    Sl1,
    Sla,
    Sra,
    Srl,
    Sub,
    Xor
}

impl fmt::Display for Mnemonic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[remain::sorted]
        match self {
            Mnemonic::Adc => write!(f, "ADC"),
            Mnemonic::Add => write!(f, "ADD"),
            Mnemonic::And => write!(f, "AND"),
            Mnemonic::Bit => write!(f, "BIT"),
            Mnemonic::Call => write!(f, "CALL"),
            Mnemonic::Ccf => write!(f, "CCF"),
            Mnemonic::Cp => write!(f, "CP"),
            Mnemonic::Cpd => write!(f, "CPD"),
            Mnemonic::Cpdr => write!(f, "CPDR"),
            Mnemonic::Cpi => write!(f, "CPI"),
            Mnemonic::Cpir => write!(f, "CPIR"),
            Mnemonic::Cpl => write!(f, "CPL"),
            Mnemonic::Daa => write!(f, "DAA"),
            Mnemonic::Dec => write!(f, "DEC"),
            Mnemonic::Di => write!(f, "DI"),
            Mnemonic::Djnz => write!(f, "DJNZ"),
            Mnemonic::Ei => write!(f, "EI"),
            Mnemonic::ExAf => write!(f, "EX AF, AF'"),
            Mnemonic::ExHlDe => write!(f, "EX DE, HL"),
            Mnemonic::ExMemSp => write!(f, "EX (SP), "),
            Mnemonic::Exx => write!(f, "EXX"),
            Mnemonic::Halt => write!(f, "HALT"),
            Mnemonic::Im => write!(f, "IM"),
            Mnemonic::In => write!(f, "IN"),
            Mnemonic::Inc => write!(f, "INC"),
            Mnemonic::Ind => write!(f, "IND"),
            Mnemonic::Indr => write!(f, "INDR"),
            Mnemonic::Ini => write!(f, "INI"),
            Mnemonic::Inir => write!(f, "INIR"),
            Mnemonic::Jp => write!(f, "JP"),
            Mnemonic::Jr => write!(f, "JR"),
            Mnemonic::Ld => write!(f, "LD"),
            Mnemonic::Ldd => write!(f, "LDD"),
            Mnemonic::Lddr => write!(f, "LDDR"),
            Mnemonic::Ldi => write!(f, "LDI"),
            Mnemonic::Ldir => write!(f, "LDIR"),
            Mnemonic::Neg => write!(f, "NEG"),
            Mnemonic::Nop => write!(f, "NOP"),
            Mnemonic::Nop2 => write!(f, "DB 0xed, 0xff ; Winape Breakpoint"),
            Mnemonic::Or => write!(f, "OR"),
            Mnemonic::Otdr => write!(f, "OTDR"),
            Mnemonic::Otir => write!(f, "OTIR"),
            Mnemonic::Out => write!(f, "OUT"),
            Mnemonic::Outd => write!(f, "OUTD"),
            Mnemonic::Outi => write!(f, "OUTI"),
            Mnemonic::Pop => write!(f, "POP"),
            Mnemonic::Push => write!(f, "PUSH"),
            Mnemonic::Res => write!(f, "RES"),
            Mnemonic::Ret => write!(f, "RET"),
            Mnemonic::Reti => write!(f, "RETI"),
            Mnemonic::Retn => write!(f, "RETN"),
            Mnemonic::Rl => write!(f, "RL"),
            Mnemonic::Rla => write!(f, "RLA"),
            Mnemonic::Rlc => write!(f, "RLC"),
            Mnemonic::Rlca => write!(f, "RLCA"),
            Mnemonic::Rld => write!(f, "RLD"),
            Mnemonic::Rr => write!(f, "RR"),
            Mnemonic::Rra => write!(f, "RRA"),
            Mnemonic::Rrc => write!(f, "RRC"),
            Mnemonic::Rrca => write!(f, "RRCA"),
            Mnemonic::Rrd => write!(f, "RRD"),
            Mnemonic::Rst => write!(f, "RST"),
            Mnemonic::Sbc => write!(f, "SBC"),
            Mnemonic::Scf => write!(f, "SCF"),
            Mnemonic::Set => write!(f, "SET"),
            Mnemonic::Sl1 => write!(f, "SL1"),
            Mnemonic::Sla => write!(f, "SLA"),
            Mnemonic::Sra => write!(f, "SRA"),
            Mnemonic::Srl => write!(f, "SRL"),
            Mnemonic::Sub => write!(f, "SUB"),
            Mnemonic::Xor => write!(f, "XOR")
        }
    }
}

macro_rules! is_mnemonic {
    ($($mnemonic:ident)*) => {$(
        paste::paste! {
            impl Mnemonic {
                /// Check if this DataAccess corresonds to $mnemonic
                pub fn [<is_ $mnemonic:lower>] (&self) -> bool {
                    match self {
                        Mnemonic::$mnemonic => true,
                        _ => false,
                    }
                }
            }
        }
    )*}
}
is_mnemonic!(
    Adc
    Add
    And
    Bit
    Call
    Ccf
    Cp
    Cpd
    Cpdr
    Cpi
    Cpir
    Cpl
    Daa
    Dec
    Di
    Djnz
    Ei
    ExAf
    ExHlDe
    ExMemSp
    Exx
    Halt
    Im
    In
    Inc
    Ind
    Indr
    Ini
    Inir
    Jp
    Jr
    Ld
    Ldd
    Lddr
    Ldi
    Ldir
    Neg
    Nop
    Nop2
    Or
    Otdr
    Otir
    Out
    Outd
    Outi
    Pop
    Push
    Res
    Ret
    Reti
    Retn
    Rl
    Rla
    Rlc
    Rlca
    Rld
    Rr
    Rra
    Rrc
    Rrca
    Rrd
    Rst
    Sbc
    Scf
    Set
    Sla
    Sl1
    Sra
    Srl
    Sub
    Xor
);

/// Stable ticker serves to count nops with the assembler !
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum StableTickerAction {
    /// Start of the ticker with its name that will contains its duration
    Start(SmolStr),
    Stop
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[allow(missing_docs)]
pub enum CrunchType {
    LZ48,
    LZ49,
    LZ4,
    LZX7,
    LZEXO,
    LZAPU
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[allow(missing_docs)]
pub enum SaveType {
    AmsdosBin,
    AmsdosBas,
    Dsk,
    Tape
}

/// Encode the kind of test done in if/elif/else cases
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum TestKind {
    // Test succeed if it is an expression that returns True
    True(Expr),
    // Test succeed if it is an expression that returns False
    False(Expr),
    // Test succeed if it is an existing label
    LabelExists(SmolStr),
    // Test succeed if it is a missing label
    LabelDoesNotExist(SmolStr),
    LabelUsed(SmolStr),
    LabelNused(SmolStr)
}


pub trait TestKindElement {
    fn is_true_test(&self) -> bool;
    fn is_false_test(&self) -> bool;

    fn is_label_used_test(&self) -> bool;
    fn is_label_nused_test(&self) -> bool;

    fn is_label_exists_test(&self) -> bool;
    fn is_label_nexists_test(&self) -> bool;

    fn expr_unchecked(&self) -> &Expr;
    fn label_unchecked(&self) -> &str;
}

impl TestKindElement for TestKind {
    fn is_true_test(&self) -> bool {
        todo!()
    }

    fn is_false_test(&self) -> bool {
        todo!()
    }

    fn is_label_used_test(&self) -> bool {
        todo!()
    }

    fn is_label_nused_test(&self) -> bool {
        todo!()
    }

    fn is_label_exists_test(&self) -> bool {
        todo!()
    }

    fn is_label_nexists_test(&self) -> bool {
        todo!()
    }

    fn expr_unchecked(&self) -> &Expr {
        todo!()
    }

    fn label_unchecked(&self) -> &str {
        todo!()
    }
}

/// List of transformations that can be applied to an imported binary file
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[allow(missing_docs)]
pub enum BinaryTransformation {
    // Raw include of the data
    None,
    // Compression with exomizer
    Exomizer,
    // Compression with lz49
    Lz49,
    Lz48,
    // compression with aplib
    Aplib
}

impl BinaryTransformation {
    pub fn crunch_type(&self) -> Option<CrunchType> {
        match self {
            BinaryTransformation::None => None,
            BinaryTransformation::Exomizer => Some(CrunchType::LZEXO),
            BinaryTransformation::Lz49 => Some(CrunchType::LZ49),
            BinaryTransformation::Aplib => Some(CrunchType::LZAPU),
            BinaryTransformation::Lz48 => Some(CrunchType::LZ48)
        }
    }
}

/// Define characeters encoding
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharsetFormat {
    /// Reset the encoding knowledge
    Reset,
    /// Specify all chars in a row
    CharsList(Vec<char>, Expr),
    /// Attribute the code to a single char
    Char(char, Expr),
    /// Specify for a given interval
    Interval(char, char, Expr)
}

/// TODO use a more complete type that can use a subset of functions to generate a string
pub type Filename = String;



pub trait ToSimpleToken {
	/// Convert the token in its simplest form
	fn as_simple_token(&self) -> Cow<Token>;
}

impl ToSimpleToken for Token {
	fn as_simple_token(&self) -> Cow<Token> {
		Cow::Borrowed(self)
	}
}

/// The embeded Listing can be of several kind (with the token or with decorated version of the token)
#[remain::sorted]
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Token {
    Align(Expr, Option<Expr>),
    Assert(Expr, Option<Vec<FormattedExpr>>),
    Assign(SmolStr, Expr),

    /// Configure the bank - completely incompatible with rasm behavior
    /// The expression corresponds to the GATE ARRAY value to select the bank of interest
    Bank(Option<Expr>),
    Bankset(Expr),
    /// Basic code which tokens will be included in the code (imported variables, lines to hide,  code)
    Basic(Option<Vec<SmolStr>>, Option<Vec<u16>>, String),
    Break,
    Breakpoint(Option<Expr>),
    BuildCpr,
    BuildSna(Option<SnapshotVersion>),
    Charset(CharsetFormat),
    Comment(String),
    CrunchedBinary(CrunchType, SmolStr),
    CrunchedSection(CrunchType, Listing),
    Defb(Vec<Expr>),
    Defs(Vec<(Expr, Option<Expr>)>),
    Defw(Vec<Expr>),

    Equ(SmolStr, Expr),
    Export(Vec<SmolStr>),

    Fail(Vec<FormattedExpr>),
    For {
        label: SmolStr,
        start: Expr,
        stop: Expr,
        step: Option<Expr>,
        listing: Listing
    },

    /// Function embeds a listing with a limited number of possible instructions and return a value
    Function(SmolStr, Vec<SmolStr>, Listing),

    /// Conditional expression. _0 contains all the expression and the appropriate code, _1 contains the else case
    If(Vec<(TestKind, Listing)>, Option<Listing>),

    /// Include of an asm file _0 contains the name of the file, _1 contains the content of the file. It is not loaded at the creation of the Token because there is not enough context to know where to load file
    Incbin {
        fname: Filename,
        offset: Option<Expr>,
        length: Option<Expr>,
        extended_offset: Option<Expr>,
        off: bool,
        transformation: BinaryTransformation
    },
    // file may or may not be read during parse. If not, it is read on demand when assembling
    Include(Filename, Option<SmolStr>, bool),
    Iterate(SmolStr, Vec<Expr>, Listing),

    Label(SmolStr),
    Let(SmolStr, Expr),
    Limit(Expr),
    List,

    Macro(SmolStr, Vec<SmolStr>, String), // Content of the macro is parsed on use
    // macro call can be used for struct too
    MacroCall(SmolStr, Vec<MacroParam>), /* String are used in order to not be limited to expression and allow opcode/registers use */

    // Fake pop directive with several arguments
    MultiPop(Vec<DataAccess>),
    // Fake push directive with several arguments
    MultiPush(Vec<DataAccess>),

    Next(SmolStr, SmolStr, Option<Expr>),
    NoExport(Vec<SmolStr>),
    NoList,

    /// Very last argument concerns only few undocumented instructions that save their results in a register
    OpCode(
        Mnemonic,
        Option<DataAccess>,
        Option<DataAccess>,
        Option<Register8>
    ),
    Org(Expr, Option<Expr>),
    Pause,
    Print(Vec<FormattedExpr>),
    Protect(Expr, Expr),
    /// Define a named section in the current page
    Range(String, Expr, Expr),
    /// Duplicate the token stream
    Repeat(
        // number of loops
        Expr,
        // code to execute
        Listing,
        // name of the counter if any
        Option<SmolStr>,
        // start value
        Option<Expr>
    ),
    RepeatUntil(Expr, Listing),
    /// Return value from a function
    Return(Expr),
    /// Set the value of $ to Expr
    Rorg(Expr, Listing),
    Run(Expr, Option<Expr>),

    Save {
        filename: Filename,
        address: Option<Expr>,
        size: Option<Expr>,
        save_type: Option<SaveType>,
        dsk_filename: Option<String>,
        side: Option<Expr>
    },
    Section(SmolStr),
    SetCPC(Expr),
    SetCrtc(Expr),
    SetN(SmolStr, SmolStr, Option<Expr>),
    /// This directive setup a value for a given flag of the snapshot
    SnaInit(Filename),
    SnaSet(
        cpclib_sna::flags::SnapshotFlag,
        cpclib_sna::flags::FlagValue
    ),
    StableTicker(StableTickerAction),
    Str(Vec<Expr>),
    Struct(SmolStr, Vec<(SmolStr, Token)>),
    Switch(Expr, Vec<(Expr, Listing, bool)>, Option<Listing>),

    Undef(SmolStr),
    WaitNops(Expr),
    While(Expr, Listing)
}

impl Clone for Token {
    fn clone(&self) -> Self {
        match self {
            Token::Align(a, b) => Token::Align(a.clone(), b.clone()),
            Token::Assert(a, b) => Token::Assert(a.clone(), b.clone()),
            Token::Assign(a, b) => Token::Assign(a.clone(), b.clone()),
            Token::Bank(b) => Token::Bank(b.clone()),
            Token::Bankset(b) => Token::Bankset(b.clone()),
            Token::Basic(a, b, c) => Token::Basic(a.clone(), b.clone(), c.clone()),
            Token::Break => Token::Break,
            Token::Breakpoint(a) => Token::Breakpoint(a.clone()),
            Token::BuildCpr => Token::BuildCpr,
            Token::BuildSna(a) => Token::BuildSna(a.clone()),
            Token::Charset(a) => Token::Charset(a.clone()),
            Token::Comment(c) => Token::Comment(c.clone()),
            Token::CrunchedBinary(a, b) => Token::CrunchedBinary(a.clone(), b.clone()),
            Token::CrunchedSection(a, b) => Token::CrunchedSection(a.clone(), b.clone()),
            Token::Defb(l) => Token::Defb(l.clone()),
            Token::Defs(l) => Token::Defs(l.clone()),
            Token::Defw(l) => Token::Defw(l.clone()),
            Token::Equ(a, b) => Token::Equ(a.clone(), b.clone()),
            Token::Export(a) => Token::Export(a.clone()),
            Token::Fail(a) => Token::Fail(a.clone()),
            Token::Function(a, b, c) => Token::Function(a.clone(), b.clone(), c.clone()),
            Token::If(a, b) => Token::If(a.clone(), b.clone()),
            Token::Incbin {
                fname,
                offset,
                length,
                extended_offset,
                off,
                transformation
            } => {
                Token::Incbin {
                    fname: fname.clone(),
                    offset: offset.clone(),
                    length: length.clone(),
                    extended_offset: extended_offset.clone(),
                    off: off.clone(),
                    transformation: transformation.clone()
                }
            }
            Token::Include(a, b, c) => {
                Token::Include(
                    a.clone(),
                    b.clone(),
                    c.clone()
                )
            }
            Token::Iterate(a, b, c) => Token::Iterate(a.clone(), b.clone(), c.clone()),
            Token::Label(a) => Token::Label(a.clone()),
            Token::Let(a, b) => Token::Let(a.clone(), b.clone()),
            Token::Limit(a) => Token::Limit(a.clone()),
            Token::List => Token::List,
            Token::Macro(a, b, c) => Token::Macro(a.clone(), b.clone(), c.clone()),
            Token::MacroCall(n, p) => Token::MacroCall(n.clone(), p.clone()),
            Token::MultiPop(a) => Token::MultiPop(a.clone()),
            Token::MultiPush(b) => Token::MultiPush(b.clone()),
            Token::Next(a, b, c) => Token::Next(a.clone(), b.clone(), c.clone()),
            Token::NoExport(a) => Token::NoExport(a.clone()),
            Token::NoList => Token::NoList,
            Token::OpCode(mne, arg1, arg2, arg3) => {
                Self::OpCode(mne.clone(), arg1.clone(), arg2.clone(), arg3.clone())
            }
            Token::Org(a, b) => Token::Org(a.clone(), b.clone()),
            Token::Pause => Token::Pause,
            Token::Print(a) => Token::Print(a.clone()),
            Token::Protect(a, b) => Token::Protect(a.clone(), b.clone()),
            Token::Range(a, b, c) => Token::Range(a.clone(), b.clone(), c.clone()),
            Token::Repeat(a, b, c, d) => Token::Repeat(a.clone(), b.clone(), c.clone(), d.clone()),
            Token::RepeatUntil(a, b) => Token::RepeatUntil(a.clone(), b.clone()),
            Token::Return(a) => Token::Return(a.clone()),
            Token::Rorg(a, b) => Token::Rorg(a.clone(), b.clone()),
            Token::Run(a, b) => Token::Run(a.clone(), b.clone()),
            Token::Save {
                filename,
                address,
                size,
                save_type,
                dsk_filename,
                side
            } => {
                Token::Save {
                    filename: filename.clone(),
                    address: address.clone(),
                    size: size.clone(),
                    save_type: save_type.clone(),
                    dsk_filename: dsk_filename.clone(),
                    side: side.clone()
                }
            }
            Token::Section(a) => Token::Section(a.clone()),
            Token::SetCPC(b) => Token::SetCPC(b.clone()),
            Token::SetCrtc(c) => Token::SetCrtc(c.clone()),
            Token::SetN(a, b, c) => Token::SetN(a.clone(), b.clone(), c.clone()),
            Token::SnaInit(a) => Token::SnaInit(a.clone()),
            Token::SnaSet(a, b) => Token::SnaSet(a.clone(), b.clone()),
            Token::StableTicker(a) => Token::StableTicker(a.clone()),
            Token::Str(a) => Token::Str(a.clone()),
            Token::Struct(a, b) => Token::Struct(a.clone(), b.clone()),
            Token::Switch(a, b, c) => Token::Switch(a.clone(), b.clone(), c.clone()),
            Token::Undef(a) => Token::Undef(a.clone()),
            Token::WaitNops(b) => Token::WaitNops(b.clone()),
            Token::While(a, b) => Token::While(a.clone(), b.clone()),
            Token::For {
                label,
                start,
                stop,
                step,
                listing
            } => {
                Token::For {
                    label: label.clone(),
                    start: start.clone(),
                    stop: stop.clone(),
                    step: step.clone(),
                    listing: listing.clone()
                }
            }
        }
    }
}

impl PartialEq for Token {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Token::OpCode(a1, b1, c1, d1), Token::OpCode(a2, b2, c2, d2)) => {
                a1 == a2 && b1 == b2 && c1 == c2 && d1 == d2
            }

            (Token::Print(a1), Token::Print(a2)) => a1 == a2,

            (Token::Defb(a), Token::Defb(b)) => a == b,

            _ => unimplemented!("{:?}, {:?}", self, other)
        }
    }
}

impl Eq for Token {}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let expr_list_to_string = |exprs: &Vec<Expr>| {
            exprs
                .iter()
                .map(|expr| expr.to_simplified_string())
                .collect::<Vec<_>>()
                .join(",")
        };

        let data_access_list_to_string = |data: &Vec<DataAccess>| {
            data.iter()
                .map(|d| format!("{}", d))
                .collect::<Vec<_>>()
                .join(",")
        };

        #[remain::sorted]
        match self {

            Token::Align(ref expr, None)
                => write!(f, "ALIGN {}", expr.to_simplified_string()),
            Token::Align(ref expr, Some(ref fill))
                => write!(f, "ALIGN {}, {}", expr.to_simplified_string(), fill),
            Token::Assert(ref expr, None)
                => write!(f, "ASSERT {}", expr.to_simplified_string()),
            Token::Assert(ref expr, Some(ref text))
                => write!(f, "ASSERT {}, {}", expr.to_simplified_string(), text.iter().map(|e|e.to_string()).join(",")),

            Token::Breakpoint(None)
                => write!(f, "BREAKPOINT"),
            Token::Breakpoint(Some(ref expr))
                 => write!(f, "BREAKPOINT {}", expr.to_simplified_string()),

            Token::Comment(ref string)
                 => write!(f, " ; {}", string.replace("\n","\n;")),
 
                 Token::Defb(ref exprs)
                 => write!(f, "DB {}", expr_list_to_string(exprs)),
            Token::Defs(ref vals)
                 => write!(f, "DEFS {}", vals.iter()
                    .map(|p| {
                        match &p.1 {
                            Some(ref v) => format!("{}, {}", p.0.to_simplified_string(), v.to_simplified_string()),
                            None => format!("{}", p.0.to_simplified_string())
                        }
                    })
                    .join(", ")
                ),

            Token::Defw(ref exprs)
                 => write!(f, "DW {}", expr_list_to_string(exprs)),
 
            Token::Equ(ref name, ref expr)
                 => write!(f, "{} EQU {}", name, expr.to_simplified_string()),

            

             Token::Incbin{
                 fname, 
                 offset, 
                 length, 
                 extended_offset, 
                 off, 
                 transformation
             } 
                 => {

                    let directive = match transformation {
                        BinaryTransformation::None => "INCBIN",
                        BinaryTransformation::Exomizer => "INCEXO",
                        BinaryTransformation::Lz49 => "INCL49",
                        BinaryTransformation::Aplib => "INCAPU",
                        BinaryTransformation::Lz48 =>"INCL48",
                    };

                     write!(f, "{} \"{}\"", directive, fname)?;
                     if offset.is_some() {
                         write!(f, ", {}", offset.as_ref().unwrap())?;

                         if length.is_some() {
                            write!(f, ", {}", length.as_ref().unwrap())?;

                            if extended_offset.is_some() {
                                write!(f, ", {}", extended_offset.as_ref().unwrap())?;

                                if *off {
                                    write!(f, ", OFF")?;
    
                                 }
                             }
                         }
                     }
                     Ok(())

                 }
 

                 Token::Include(ref fname, Some(module), once)
                 => write!(f, "INCLUDE {}\"{}\" namespace {}", fname, module.as_str(), if *once {"ONCE "} else {""}),

                 Token::Include(ref fname, None, once)
                 => write!(f, "INCLUDE {}\"{}\"", fname, if *once {"ONCE "} else {""}),
 
            Token::Label(ref string)
                => write!(f, "{}", string),


            Token::MacroCall(ref name, ref args)
                => {use cpclib_common::itertools::Itertools;
                    write!(f, "{} {}", name, args.clone()
                                                .iter()
                                                .map(|a|{a.to_string()})
                                                .join(", "))?;
                    Ok(())
            },

            Token::MultiPush(ref regs) => {
                write!(f, "PUSH {}", data_access_list_to_string(regs))
            },
            Token::MultiPush(ref regs) => {
                write!(f, "POP {}", data_access_list_to_string(regs))
            },

                // TODO remove this one / it is not coherent as we have the PortC
            Token::OpCode(ref mne, Some(DataAccess::Register8(_)), Some(ref arg2), None) if &Mnemonic::Out == mne
                => write!(f, "{} (C), {}", mne, arg2),
            Token::OpCode(ref mne, None, None, None)
                => write!(f, "{}", mne),
            Token::OpCode(ref mne, Some(ref arg1), None, None)
                => write!(f, "{} {}", mne, arg1),
            Token::OpCode(ref mne, None, Some(ref arg2), None) // JP/JR without flags
               => write!(f, "{} {}", mne, arg2),
            Token::OpCode(ref mne, Some(ref arg1), Some(ref arg2), None)
                => write!(f, "{} {}, {}", mne, arg1, arg2),

            Token::OpCode(ref mne, Some(ref arg1), Some(ref arg2), Some(arg3))
                => write!(f, "{} {}, {}, {}", mne, arg1, arg2, arg3),    

            Token::Org(ref expr, None)
                => write!(f, "ORG {}", expr),
            Token::Org(ref expr, Some(ref expr2))
                => write!(f, "ORG {}, {}", expr, expr2),


            Token::Print(ref exp)
                => write!(f, "PRINT {}", exp.iter().map(|e|e.to_string()).join(",")),

            Token::Protect(ref exp1, ref exp2)
                => write!(f, "PROTECT {}, {}", exp1, exp2),

            Token::Repeat(ref exp, ref code, ref label, ref start) => {
                
                write!(f, "REPEAT {}", exp)?;
                if label.is_some() {
                    write!(f, " {}", label.as_ref().unwrap())?;
                }
                if start.is_some() {
                    write!(f, ", {}", start.as_ref().unwrap())?;
                }
                writeln!(f, "")?;

                for token in code.iter() {
                    writeln!(f, "\t{}", token)?;
                }
                write!(f, "\tENDREPEAT")
            },

            Token::StableTicker(ref ticker)
                => {
                    match ticker {
                        StableTickerAction::Start(ref label) => {
                            write!(f, "STABLETICKER START {}", label)
                        },
                        StableTickerAction::Stop => {
                            write!(f, "STABLETICKER STOP")
                        }
                    }
            },

            _ => unimplemented!()

        }
    }
}

impl From<u8> for Token {
    fn from(byte: u8) -> Self {
        Self::Defb(vec![byte.into()])
    }
}

#[allow(missing_docs)]
impl Token {
    pub fn is_directive(&self) -> bool {
        match self {
            Self::OpCode(..) => false,
            _ => true
        }
    }

    pub fn new_opcode(mne: Mnemonic, arg1: Option<DataAccess>, arg2: Option<DataAccess>) -> Self {
        Token::OpCode(mne, arg1, arg2, None)
    }

    /// When diassembling code, the token with relative information are not appropriate
    pub fn fix_relative_jumps_after_disassembling(&mut self) {
        if self.is_opcode() {
            let expression = match self {
                Self::OpCode(Mnemonic::Jr, _, Some(DataAccess::Expression(exp)), _) => Some(exp),
                Self::OpCode(Mnemonic::Djnz, Some(DataAccess::Expression(exp)), ..) => Some(exp),
                //          Self::OpCode(_, Some(DataAccess::IndexRegister16WithIndex(_, exp)), _) => Some(exp),
                //         Self::OpCode(_, _, Some(DataAccess::IndexRegister16WithIndex(_, exp))) => Some(exp),
                _ => None
            };

            if let Some(expr) = expression {
                expr.fix_relative_value();
            };
        }
    }

    pub fn is_opcode(&self) -> bool {
        self.mnemonic().is_some()
    }

    pub fn is_output_opcode(&self) -> bool {
        match self {
            Token::OpCode(Mnemonic::Out, ..)
            | Token::OpCode(Mnemonic::Outd, ..)
            | Token::OpCode(Mnemonic::Outi, ..)
            | Token::OpCode(Mnemonic::Otdr, ..)
            | Token::OpCode(Mnemonic::Otir, ..) => true,
            _ => false
        }
    }

    pub fn is_input_opcode(&self) -> bool {
        match self {
            Token::OpCode(Mnemonic::In, ..)
            | Token::OpCode(Mnemonic::Ind, ..)
            | Token::OpCode(Mnemonic::Ini, ..)
            | Token::OpCode(Mnemonic::Indr, ..)
            | Token::OpCode(Mnemonic::Inir, ..) => true,
            _ => false
        }
    }

    pub fn is_retlike_opcode(&self) -> bool {
        match self {
            Token::OpCode(Mnemonic::Ret, ..)
            | Token::OpCode(Mnemonic::Reti, ..)
            | Token::OpCode(Mnemonic::Retn, ..) => true,
            _ => false
        }
    }

    /// Check if it is an undocumented instruction that makes a copy of the data to save in an additional register
    pub fn is_autocopy_opcode(&self) -> bool {
        matches!(
            self,
            Self::OpCode(
                Mnemonic::Rlc
                    | Mnemonic::Rrc
                    | Mnemonic::Rl
                    | Mnemonic::Rr
                    | Mnemonic::Sla
                    | Mnemonic::Sra
                    | Mnemonic::Sl1
                    | Mnemonic::Srl,
                Some(DataAccess::IndexRegister16WithIndex(_, _)),
                Some(DataAccess::Register8(_)),
                None
            ) | Self::OpCode(
                Mnemonic::Set | Mnemonic::Res,
                Some(DataAccess::Expression(_)),
                Some(DataAccess::IndexRegister16WithIndex(_, _)),
                Some(_)
            )
        )
    }

    pub fn label(&self) -> Option<&str> {
        match self {
            Token::Label(ref value) | Token::Equ(ref value, _) => Some(value),
            _ => None
        }
    }

    pub fn is_label(&self) -> bool {
        match self {
            Self::Label(_) => true,
            _ => false
        }
    }

    pub fn mnemonic(&self) -> Option<&Mnemonic> {
        match self {
            Token::OpCode(ref mnemonic, ..) => Some(mnemonic),
            _ => None
        }
    }

    pub fn mnemonic_arg1(&self) -> Option<&DataAccess> {
        match self {
            Token::OpCode(_, ref arg1, ..) => arg1.as_ref(),
            _ => None
        }
    }

    pub fn mnemonic_arg2(&self) -> Option<&DataAccess> {
        match self {
            Token::OpCode(_, _, ref arg2, _) => arg2.as_ref(),
            _ => None
        }
    }

    pub fn mnemonic_arg1_mut(&mut self) -> Option<&mut DataAccess> {
        match self {
            Token::OpCode(_, ref mut arg1, ..) => arg1.as_mut(),
            _ => None
        }
    }

    pub fn mnemonic_arg2_mut(&mut self) -> Option<&mut DataAccess> {
        match self {
            Token::OpCode(_, _, ref mut arg2, _) => arg2.as_mut(),
            _ => None
        }
    }

    pub fn macro_name(&self) -> Option<&str> {
        match self {
            Self::Macro(name, _args, _content) => Some(name),
            Self::MacroCall(name, _params) => Some(name),
            _ => None
        }
    }

    pub fn macro_arguments(&self) -> Option<&[SmolStr]> {
        match self {
            Self::Macro(_name, args, _content) => Some(args.as_ref()),
            _ => None
        }
    }

    pub fn macro_content(&self) -> Option<&str> {
        match self {
            Self::Macro(_name, _args, content) => Some(content),
            _ => None
        }
    }

    /// Rename the @labels in macros
    /// XXX no more needed - to remove later
    // pub fn fix_local_macro_labels_with_seed(&mut self, seed: usize) {
    // match self {
    // Self::Align(a, b)  | Self::Org(a, b) | Self::Run(a, b) => {
    // a.fix_local_macro_labels_with_seed(seed);
    // b.as_mut().map(|b| b.fix_local_macro_labels_with_seed(seed));
    // }
    //
    // Self::Defs(a) => {
    // a.iter_mut()
    // .for_each(|p| {
    // match &mut p.1 {
    // Some(ref mut v) => {
    // p.0.fix_local_macro_labels_with_seed(seed);
    // v.fix_local_macro_labels_with_seed(seed);
    // },
    // None => {
    // p.0.fix_local_macro_labels_with_seed(seed);
    // }
    // }
    // })
    // }
    //
    // Self::Protect(a, b) => {
    // a.fix_local_macro_labels_with_seed(seed);
    // b.fix_local_macro_labels_with_seed(seed);
    // }
    //
    // Self::Assert(a, _)
    // | Self::Bank(a)
    // | Self::Bankset(a)
    // | Self::Breakpoint(Some(a))
    // | Self::Limit(a)
    // | Self::SetCPC(a)
    // | Self::SetCrtc(a) => {
    // a.fix_local_macro_labels_with_seed(seed);
    // }
    //
    // Self::Defb(v) | Self::Defw(v) => {
    // v.iter_mut()
    // .for_each(|e| e.fix_local_macro_labels_with_seed(seed));
    // }
    //
    // Self::Assign(a, b)  | Self::Equ(a, b) | Self::Let(a, b) => {
    // Expr::do_apply_macro_labels_modification(a, seed);
    // b.fix_local_macro_labels_with_seed(seed);
    // }
    //
    // Self::Save {
    // address,
    // size,
    // side,
    // ..
    // } => {
    // address.fix_local_macro_labels_with_seed(seed);
    // size.fix_local_macro_labels_with_seed(seed);
    // side.as_mut()
    // .map(|s| s.fix_local_macro_labels_with_seed(seed));
    // }
    //
    // Self::Basic(_, _, _)
    // | Self::Break
    // | Self::BuildCpr
    // | Self::BuildSna(_)
    // | Self::Comment(_)
    // | Self::CrunchedBinary(_, _)
    // | Self::CrunchedSection(_, _)
    // | Self::List
    // | Self::MultiPop(_)
    // | Self::MultiPush(_)
    // | Self::NoList
    // | Self::SnaSet(_, _)
    // | Self::StableTicker(_)
    // | Self::Str(_)
    // | Self::Struct(_, _) => {}
    //
    // Self::If(v, o) => {
    // v.iter_mut()
    // .map(|(t, l)| l)
    // .for_each(|l| l.fix_local_macro_labels_with_seed(seed));
    // o.as_mut().map(|l| l.fix_local_macro_labels_with_seed(seed));
    // }
    //
    // Self::Label(s) => {
    // Expr::do_apply_macro_labels_modification(s, seed);
    // }
    //
    // Self::MacroCall(_n, v) => {
    // v.iter_mut()
    // .for_each(|p| p.do_apply_macro_labels_modification(seed));
    // }
    //
    // Self::OpCode(_m, a, b, _) => {
    // a.as_mut().map(|d| d.fix_local_macro_labels_with_seed(seed));
    // b.as_mut().map(|d| d.fix_local_macro_labels_with_seed(seed));
    // }
    //
    //
    // Self::RepeatUntil(e, l)
    // | Self::Rorg(e, l)
    // | Self::While(e, l) => {
    // e.fix_local_macro_labels_with_seed(seed);
    // l.fix_local_macro_labels_with_seed(seed);
    // }
    //
    // Self::Repeat(e, l, _, s) => {
    //
    // e.fix_local_macro_labels_with_seed(seed);
    // l.fix_local_macro_labels_with_seed(seed);
    // s.as_mut().map(|e| e.fix_local_macro_labels_with_seed(seed));
    // }
    //
    // Self::Switch(l) => {
    // l.iter_mut().for_each(|(e, l)| {
    // e.fix_local_macro_labels_with_seed(seed);
    // l.fix_local_macro_labels_with_seed(seed);
    // });
    // }
    //
    // Self::Print(ref mut vec) => {
    // vec.iter_mut().for_each(|e| e.fix_local_macro_labels_with_seed(seed))
    // }
    // _ => unimplemented!("{:?}", self),
    // }
    // }

    #[deprecated(
        since = "0.1.1",
        note = "please use `expr` instead as other token need it"
    )]
    pub fn org_expr(&self) -> Option<&Expr> {
        self.expr()
    }

    pub fn expr(&self) -> Option<&Expr> {
        match self {
            Token::Org(ref expr, _) | Token::Equ(_, ref expr) => Some(expr),
            _ => None
        }
    }

    /// Return true for directives that can emebed some listing information
    pub fn has_at_least_one_listing(&self) -> bool {
        match self {
            Self::CrunchedSection(..)
            | Self::Include(..)
            | Self::If(..)
            | Self::Repeat(..)
            | Self::RepeatUntil(..)
            | Self::Rorg(..)
            | Self::Switch(..)
            | Self::While(..) => true,
            _ => false
        }
    }
}
