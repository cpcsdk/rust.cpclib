use std::convert::TryFrom;
use std::fmt;

use itertools::Itertools;
use crate::tokens::data_access::*;
use crate::tokens::expression::*;
use crate::tokens::Listing;

use cpclib_sna::SnapshotVersion;

use either::*;

use std::fs::File;
use std::io::Read;

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
    Nops2, // Fake instruction that generate a breakpoint on winape
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
    Sla,
    Sll,
    Sra,
    Srl,
    Sub,
    Xor,
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
            Mnemonic::Nops2 => write!(f, "DB 0xed, 0xff ; Winape Breakpoint"),
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
            Mnemonic::Rst => write!(f,"RST"),
            Mnemonic::Sbc => write!(f, "SBC"),
            Mnemonic::Scf => write!(f, "SCF"),
            Mnemonic::Set => write!(f, "SET"),
            Mnemonic::Sla => write!(f, "SLA"),
            Mnemonic::Sll => write!(f, "SLL"),
            Mnemonic::Sra => write!(f, "SRA"),
            Mnemonic::Srl => write!(f, "SRL"),
            Mnemonic::Sub => write!(f, "SUB"),
            Mnemonic::Xor => write!(f, "XOR"),
        }
    }
}


macro_rules! is_mnemonic {
    ($($mnemonic:ident)*) => {$(
        paste::item_with_macros! {
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
    Nops2
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
    Sll
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
    Start(String),
    Stop,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[allow(missing_docs)]
pub enum CrunchType {
    LZ48,
    LZ49,
    LZ4,
    LZX7,
    LZEXO,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[allow(missing_docs)]
pub enum SaveType {
    Amsdos,
    Dsk,
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
    LabelExists(String),
    // Test succeed if it is a missing label
    LabelDoesNotExist(String),
}

/// List of transformations that can be applied to an imported binary file
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[allow(missing_docs)]
pub enum BinaryTransformation {
    None,
    Exomizer,
}

#[remain::sorted]
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Token {
    Align(Expr, Option<Expr>),
    Assert(Expr, Option<String>),

    Bank(Expr),
    Bankset(Expr),
    /// Basic code which tokens will be included in the code (imported variables, lines to hide,  code)
    Basic(Option<Vec<String>>, Option<Vec<u16>>, String),
    Break,
    Breakpoint(Option<Expr>),
    BuildCpr,
    BuildSna(SnapshotVersion),
    Comment(String),
    CrunchedBinary(CrunchType, String),
    CrunchedSection(CrunchType, Listing),
    Defb(Vec<Expr>),
    Defs(Expr, Option<Expr>),
    Defw(Vec<Expr>),

    Equ(String, Expr),

    /// Conditional expression. _0 contains all the expression and the appropriate code, _1 contains the else case
    If(Vec<(TestKind, Listing)>, Option<Listing>),

    /// Include of an asm file _0 contains the name of the file, _1 contains the content of the file. It is not loaded at the creation of the Token because there is not enough context to know where to load file
    Incbin(
        // TODO name arguments to ease manipulation
        String,
        Option<Expr>,
        Option<Expr>,
        Option<Expr>,
        Option<Expr>,
        Option<Vec<u8>>,
        BinaryTransformation,
    ),
    Include(String, Option<Listing>),

    Label(String),
    Let(String, Expr),
    Limit(Expr),
    List,

    Macro(String, Vec<String>, String), // Content of the macro is parsed on use
    MacroCall(String, Vec<Expr>), // String are used in order to not be limited to expression and allow opcode/registers use

    NoList,

    OpCode(Mnemonic, Option<DataAccess>, Option<DataAccess>),
    Org(Expr, Option<Expr>),

    Print(Vec<FormattedExpr>),
    Protect(Expr, Expr),

    /// Duplicate the token stream
    Repeat(Expr, Listing, Option<String>),
    RepeatUntil(Expr, Listing),
    /// Set the value of $ to Expr
    Rorg(Expr, Listing),
    Run(Expr, Option<Expr>),

    Save {
        filename: String,
        address: Expr,
        size: Expr,
        save_type: Option<SaveType>,
        dsk_filename: Option<String>,
        side: Option<Expr>,
    },
    SetCPC(Expr),
    SetCrtc(Expr),
    StableTicker(StableTickerAction),
    Str(Vec<u8>),
    Struct(String, Vec<(String, Token)>),
    Switch(Vec<(Expr, Listing)>),

    Undef(String),

    While(Expr, Listing),

}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let expr_list_to_string = |exprs: &Vec<Expr>| {
            exprs
                .iter()
                .map(|expr| format!("{}", expr))
                .collect::<Vec<_>>()
                .join(",")
        };

        #[remain::sorted]
        match *self {

            Token::Align(ref expr, None)
                => write!(f, "ALIGN {}", expr),
            Token::Align(ref expr, Some(ref fill))
                => write!(f, "ALIGN {}, {}", expr, fill),
            Token::Assert(ref expr, None)
                => write!(f, "ASSERT {}", expr),
            Token::Assert(ref expr, Some(ref text))
                => write!(f, "ASSERT {}, {}", expr, text),

            Token::Breakpoint(None)
                => write!(f, "BREAKPOINT"),
            Token::Breakpoint(Some(ref expr))
                 => write!(f, "BREAKPOINT {}", expr),

            Token::Comment(ref string)
                 => write!(f, " ; {}", string.replace("\n","\n;")),
 
                 Token::Defb(ref exprs)
                 => write!(f, "DB {}", expr_list_to_string(exprs)),
            Token::Defs(ref expr, None)
                 => write!(f, "DEFS {}", expr),
            Token::Defs(ref expr, Some(ref expr2))
                 => write!(f, "DEFS {}, {}", expr, expr2),

 
            Token::Defw(ref exprs)
                 => write!(f, "DW {}", expr_list_to_string(exprs)),
 
            Token::Equ(ref name, ref expr)
                 => write!(f, "{} EQU {}", name, expr),

            

             Token::Incbin(ref fname, None, None, None, None, None, BinaryTransformation::None) 
                 => write!(f, "INCBIN \"{}\"", fname),
 

                 Token::Include(ref fname, _)
                 => write!(f, "INCLUDE \"{}\"", fname),
 
            Token::Label(ref string)
                => write!(f, "{}", string),


                Token::MacroCall(ref name, ref args)
                => {use itertools::Itertools;
                    write!(f, "{} {}", name, args.clone()
                                                .iter()
                                                .map(|a|{a.to_string()})
                                                .join(", "))?;
                    Ok(())
            },

                // TODO remove this one / it is not coherent as we have the PortC
            Token::OpCode(ref mne, Some(DataAccess::Register8(_)), Some(ref arg2)) if &Mnemonic::Out == mne
                => write!(f, "{} (C), {}", mne, arg2),
            Token::OpCode(ref mne, None, None)
                => write!(f, "{}", mne),
            Token::OpCode(ref mne, Some(ref arg1), None)
                => write!(f, "{} {}", mne, arg1),
            Token::OpCode(ref mne, None, Some(ref arg2)) // JP/JR without flags
               => write!(f, "{} {}", mne, arg2),
            Token::OpCode(ref mne, Some(ref arg1), Some(ref arg2))
                => write!(f, "{} {}, {}", mne, arg1, arg2),

            Token::Org(ref expr, None)
                => write!(f, "ORG {}", expr),
            Token::Org(ref expr, Some(ref expr2))
                => write!(f, "ORG {}, {}", expr, expr2),


            Token::Print(ref exp)
                => write!(f, "PRINT {}", exp.iter().map(|e|e.to_string()).join(",")),

            Token::Protect(ref exp1, ref exp2)
                => write!(f, "PROTECT {}, {}", exp1, exp2),

            Token::Repeat(ref exp, ref code, ref label) => {
                if label.is_some() {
                    writeln!(f, "REPEAT {}, {}", exp, label.as_ref().unwrap())?;
                }
                else {
                    writeln!(f, "REPEAT {}", exp)?;
                }
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

    /// When diassembling code, the token with relative information are not appropriate
    pub fn fix_relative_jumps_after_disassembling(&mut self) {
        if self.is_opcode() {

            let expression = match self {
                Self::OpCode(Mnemonic::Jr, _, Some(DataAccess::Expression(exp))) => Some(exp),
                Self::OpCode(Mnemonic::Djnz, Some(DataAccess::Expression(exp)), _) => Some(exp),
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

    pub fn label(&self) -> Option<&String> {
        match self {
            Token::Label(ref value) | Token::Equ(ref value, _) => Some(value),
            _ => None,
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
            Token::OpCode(ref mnemonic, _, _) => Some(mnemonic),
            _ => None,
        }
    }

    pub fn mnemonic_arg1(&self) -> Option<&DataAccess> {
        match self {
            Token::OpCode(_, ref arg1, _) => arg1.as_ref(),
            _ => None,
        }
    }

    pub fn mnemonic_arg2(&self) -> Option<&DataAccess> {
        match self {
            Token::OpCode(_, _, ref arg2) => arg2.as_ref(),
            _ => None,
        }
    }

    pub fn mnemonic_arg1_mut(&mut self) -> Option<&mut DataAccess> {
        match self {
            Token::OpCode(_,  ref mut arg1, _) => arg1.as_mut(),
            _ => None,
        }
    }

    pub fn mnemonic_arg2_mut(&mut self) -> Option<&mut DataAccess> {
        match self {
            Token::OpCode(_, _, ref mut arg2) => arg2.as_mut(),
            _ => None,
        }
    }

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
            _ => None,
        }
    }

   
}
