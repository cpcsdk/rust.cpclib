use std::convert::TryFrom;
use std::fmt;

use crate::assembler::assembler::{assemble_opcode,assemble_db_or_dw,assemble_defs,assemble_align,Bytes,SymbolsTable};
use crate::assembler::tokens::expression::*;
use crate::assembler::tokens::data_access::*;
use crate::assembler::parser::*;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Mnemonic {
    Adc,
    Add,
    And,
    Dec,
    Di,
    Djnz,
    Ei,
    ExAf,
    Exx,
    Halt,
    In,
    Inc,
    Jp,
    Jr,
    Ld,
    Ldd,
    Ldi,
    Lddr,
    Ldir,
    Nop,
    Nops2, // Fake instruction that generate a breakpoint on winape
    Or,
    Out,
    Push,
    Pop,
    Rra,
    Res,
    Ret,
    Set,
    Xor
}


impl fmt::Display for Mnemonic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mnemonic::Adc=> write!(f, "ADC"),
            Mnemonic::Add=> write!(f, "ADD"),
            Mnemonic::And => write!(f, "AND"),
            Mnemonic::Dec => write!(f, "DEC"),
            Mnemonic::Di => write!(f, "DI"),
            Mnemonic::Djnz => write!(f, "DJNZ"),
            Mnemonic::Ei => write!(f, "EI"),
            Mnemonic::Exx => write!(f, "EXX"),
            Mnemonic::ExAf => write!(f, "EX AF, AF'"),
            Mnemonic::Halt => write!(f, "HALT"),
            Mnemonic::In => write!(f, "IN"),
            Mnemonic::Inc => write!(f, "INC"),
            Mnemonic::Jp => write!(f, "JP"),
            Mnemonic::Jr => write!(f, "JR"),
            Mnemonic::Ld => write!(f, "LD"),
            Mnemonic::Ldi => write!(f, "LDI"),
            Mnemonic::Ldd => write!(f, "LDD"),
            Mnemonic::Ldir => write!(f, "LDIR"),
            Mnemonic::Lddr => write!(f, "LDDR"),
            Mnemonic::Nop => write!(f, "NOP"),
            Mnemonic::Nops2 => write!(f, "DB 0xed, 0xff ; Winape Breakpoint"),
            Mnemonic::Out => write!(f, "OUT"),
            Mnemonic::Or => write!(f, "OR"),
            Mnemonic::Push => write!(f, "PUSH"),
            Mnemonic::Pop => write!(f, "POP"),
            Mnemonic::Rra => write!(f, "RRA"),
            Mnemonic::Res => write!(f, "RES"),
            Mnemonic::Ret => write!(f, "RET"),
            Mnemonic::Set => write!(f, "SET"),
            Mnemonic::Xor => write!(f, "XOR"),
        }
    }
}

/// Stable ticker serves to count nops with the assembler !
#[derive(Debug, Clone, PartialEq)]
pub enum StableTickerAction {
    /// Start of the ticker with its name that will contains its duration
    Start(String),
    Stop
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Label(String),
    Comment(String),

    OpCode(Mnemonic, Option<DataAccess>, Option<DataAccess>),

    Align(Expr),
    Assert(Expr),
    Defs(Expr),
    Db(Vec<Expr>),
    Dw(Vec<Expr>),
    Equ(String, Expr),
    Include(String),
    Org(Expr),
    Protect(Expr, Expr),

    /// Duplicate the token stream
    Repeat(Expr, Vec<Token>),

    StableTicker(StableTickerAction),
    MacroCall(String) // TODO add parameters
}



impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        let expr_list_to_string= |exprs: &Vec<Expr>| {
            exprs
                .iter()
                .map(|expr|{ format!("{}", expr)})
                .collect::<Vec<_>>()
                .join(",")
        };

        match *self {
            Token::OpCode(ref mne, Some(DataAccess::Register8(_)), Some(ref arg2)) if &Mnemonic::Out == mne
                => write!(f, "{} (C), {}", mne, arg2),

            Token::Align(ref expr)
                => write!(f, "ALIGN {}", expr),
            Token::Assert(ref expr)
                => write!(f, "ASSERT {}", expr),
            Token::Label(ref string)
                => write!(f, "{}", string),
            Token::Comment(ref string)
                => write!(f, " ; {}", string),
            Token::OpCode(ref mne, None, None)
                => write!(f, "{}", mne),
            Token::OpCode(ref mne, Some(ref arg1), None)
                => write!(f, "{} {}", mne, arg1),

            Token::OpCode(ref mne, None, Some(ref arg2)) // JP/JR without flags
               => write!(f, "{} {}", mne, arg2),
            Token::OpCode(ref mne, Some(ref arg1), Some(ref arg2))
                => write!(f, "{} {}, {}", mne, arg1, arg2),
            Token::Org(ref expr)
                => write!(f, "ORG {}", expr),
            Token::Defs(ref expr)
                => write!(f, "DEFS {}", expr),
            Token::Db(ref exprs)
                => write!(f, "DB {}", expr_list_to_string(exprs)),
            Token::Dw(ref exprs)
                => write!(f, "DW {}", expr_list_to_string(exprs)),
            Token::Equ(ref name, ref expr)
                => write!(f, "{} EQU {}", name, expr),
            Token::Include(ref fname)
                => write!(f, "INCLUDE \"{}\"", fname),
            Token::Protect(ref exp1, ref exp2)
                => write!(f, "PROTECT {}, {}", exp1, exp2),
            Token::Repeat(ref exp, ref code) => {
                write!(f, "REPEAT {}\n", exp)?;
                for token in code.iter() {
                    write!(f, "\t{}\n", token)?;
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
            Token::MacroCall(ref name)
                => write!(f, "{}", name)

        }
    }
}



impl<'a> TryFrom<&'a str> for Token {
    type Error = String;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let tokens = {
            let res = parse_z80_str(value);
            match res {
                Ok(tokens) => tokens.1,
                Err(e) => {
                    return Err(decode_parsing_error(value, e));
                }
            }

        };
        if tokens.len() > 1 {
            Err(format!("{} tokens are present instead of one", tokens.len()))
        }
        else {
            Ok(tokens[0].clone())
        }

    }
}


impl Token {
    pub fn label(&self) -> Option<&String> {
        match self {
            &Token::Label(ref value) |  &Token::Equ(ref value, _) => Some(value),
            _ => None
        }
    }

    pub fn mnemonic(&self) -> Option<&Mnemonic> {
        match self {
            &Token::OpCode(ref mnemonic, _, _) => Some(mnemonic),
            _ => None
        }
    }

    pub fn mnemonic_arg1(&self) -> Option<&DataAccess> {
        match self {
            &Token::OpCode(_, ref arg1, _) => arg1.as_ref(),
            _ => None
        }
    }

    pub fn mnemonic_arg2(&self) -> Option<&DataAccess> {
        match self {
            &Token::OpCode(_, _, ref arg2) => arg2.as_ref(),
            _ => None
        }
    }

    #[deprecated(since="0.1.1", note="please use `expr` instead as other token need it")]
    pub fn org_expr(&self) -> Option<&Expr> {
        self.expr()
    }

    pub fn expr(&self) -> Option<&Expr> {
        match self {
          &Token::Org(ref expr)  |  &Token::Equ(_, ref expr)=> Some(expr),
            _ => None
        }
    }

    /// Unroll the tokens when in a repetition loop
    /// TODO return an iterator in order to not produce the vector each time
    pub fn unroll(&self, sym: & SymbolsTable) -> Option<Result<Vec<&Token>, String>> {
        match self {
            Token::Repeat(ref expr, ref tokens) => {
                match expr.resolve(sym) {
                    Ok(count) => {
                        let mut res =  Vec::with_capacity(count as usize * tokens.len());
                        for i in 0..count {
                            // TODO add a specific token to control the loop counter (and change the return type)
                            for t in tokens.iter() {
                                res.push(t);
                            }
                        }
                        Some(Ok(res))
                    },
                    Err(msg) => Some(Err(msg))
                }
            },
            _ => None
        }
    }


    /// Dummy version that assemble without taking into account the context
    /// TODO find a way to not build a symbol table each time
    pub fn to_bytes(&self) -> Result<Bytes, String> {
        let mut table = SymbolsTable::laxist();
        let table = &mut table;
        self.to_bytes_with_context(table)
    }

    /// Assemble the symbol taking into account some context, but never modify this context
    pub fn to_bytes_with_context(&self, table: &mut SymbolsTable) -> Result<Bytes, String> {

        let mut env = &mut crate::assembler::assembler::Env::with_table(table);
                match self {
            &Token::OpCode(ref mnemonic, ref arg1, ref arg2)
                => assemble_opcode(
                    mnemonic, 
                    arg1, 
                    arg2, 
                    env // Modification to the environment are lost
                ),

            &Token::Equ(_, _)
                => Ok(Bytes::new()),

            &Token::Dw(_) | &Token::Db(_)
                => assemble_db_or_dw(self, env),

            &Token::Label(_) | &Token::Comment(_) | &Token::Org(_) | &Token::Assert(_)
                => Ok(Bytes::new()),

            &Token::Defs(ref expr)
                => assemble_defs(expr, env),

            &Token::Align(ref expr)
                => assemble_align(expr, table),

            // Protect directive does not produce any bytes
            &Token::Protect(_, _)
                => Ok(Bytes::new()),

            _
                => Err(format!("Currently unable to generate bytes for {}", self))
        }
    }
}


