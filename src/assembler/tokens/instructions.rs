use std::convert::TryFrom;
use std::fmt;

use crate::assembler::assembler::{assemble_opcode,assemble_db_or_dw,assemble_defs,assemble_align,Bytes,SymbolsTableCaseDependent};
use crate::assembler::tokens::expression::*;
use crate::assembler::tokens::data_access::*;
use crate::assembler::parser::*;
use crate::assembler::tokens::Listing;
use crate::assembler::AssemblerError;

use std::fs::File;
use std::io::Read;
use failure::ResultExt;
use either::*;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Mnemonic {
    Adc,
    Add,
    And,
    Call,
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
    Outi,
    Outd,
    Push,
    Pop,
    Rra,
    Res,
    Ret,
    Set,
    Sla,
    Sra,
    Srl,
    Xor
}


impl fmt::Display for Mnemonic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mnemonic::Adc=> write!(f, "ADC"),
            Mnemonic::Add=> write!(f, "ADD"),
            Mnemonic::And => write!(f, "AND"),
            Mnemonic::Call => write!(f, "CALL"),
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
            Mnemonic::Outi => write!(f, "OUTI"),
            Mnemonic::Outd => write!(f, "OUTD"),
            Mnemonic::Or => write!(f, "OR"),
            Mnemonic::Push => write!(f, "PUSH"),
            Mnemonic::Pop => write!(f, "POP"),
            Mnemonic::Rra => write!(f, "RRA"),
            Mnemonic::Res => write!(f, "RES"),
            Mnemonic::Ret => write!(f, "RET"),
            Mnemonic::Set => write!(f, "SET"),
            Mnemonic::Sla => write!(f, "SLA"),
            Mnemonic::Sra => write!(f, "SRA"),
            Mnemonic::Srl => write!(f, "SRL"),
            Mnemonic::Xor => write!(f, "XOR"),
        }
    }
}

/// Stable ticker serves to count nops with the assembler !
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StableTickerAction {
    /// Start of the ticker with its name that will contains its duration
    Start(String),
    Stop
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrunchType {
    LZ48,
    LZ49,
    LZ4,
    LZX7,
    LZEXO
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveType {
    Amsdos,
    Dsk
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Label(String),
    Comment(String),

    OpCode(Mnemonic, Option<DataAccess>, Option<DataAccess>),

    Align(Expr, Option<Expr>),
    Assert(Expr, Option<String>),
    Bank(Expr),
    Bankset(Expr),
    /// Basic code which tokens will be included in the code (imported variables, lines to hide,  code)
    Basic(Option<Vec<String>>, Option<Vec<u16>>, String),
    Breakpoint(Option<Expr>),
    BuildCpr,
    BuildSna(crate::sna::SnapshotVersion),
    Break,
    CrunchedSection(CrunchType, Listing),
    CrunchedBinary(CrunchType, String),
    Defs(Expr, Option<Expr>),
    Defb(Vec<Expr>),
    Defw(Vec<Expr>),
    Equ(String, Expr),
    /// Conditional expression. _0 contains all the expression and the appropriate code, _1 contains the else case
    If(Vec<(Expr, Listing)>, Option<Listing>),
    /// Include of an asm file _0 contains the name of the file, _1 contains the content of the file. It is not loaded at the creation of the Token because there is not enough context to know where to load file
    Include(String, Option<Listing>),
    Incbin(String, Option<Expr>, Option<Expr>, Option<Expr>, Option<Expr>, Option<Vec<u8>>),
    Let(String, Expr),
    List,
    Limit(Expr),
    Macro(String, Vec<String>, String), // Content of the macro is parsed on use
    NoList,
    Org(Expr, Option<Expr>),
    Print(Either<Expr, String>),
    Protect(Expr, Expr),

    /// Duplicate the token stream
    Repeat(Expr, Listing, Option<String>),
    RepeatUntil(Expr, Listing),
    /// Set the value of $ to Expr
    Rorg(Expr, Listing),
    Run(Expr, Option<Expr>),

    Save{
        filename: String, 
        address: Expr, 
        size: Expr, 
        save_type: Option<SaveType>,
        dsk_filename: Option<String>,
        side: Option<Expr>
   },
   SetCrtc(Expr),
   SetCPC(Expr),
    Str(Vec<u8>),
    StableTicker(StableTickerAction),
    Struct(String, Vec<(String, Token)>),
    Switch(Vec<(Expr, Listing)>),
    Undef(String),
    While(Expr, Listing),

    MacroCall(String, Vec<String>) // String are used in order to not be limited to expression and allow opcode/registers use
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

            Token::Org(ref expr, None)
                => write!(f, "ORG {}", expr),
            Token::Org(ref expr, Some(ref expr2))
                => write!(f, "ORG {}, {}", expr, expr2),

            Token::Defs(ref expr, None)
                => write!(f, "DEFS {}", expr),
            Token::Defs(ref expr, Some(ref expr2))
                => write!(f, "DEFS {}, {}", expr, expr2),

            Token::Defb(ref exprs)
                => write!(f, "DB {}", expr_list_to_string(exprs)),

            Token::Defw(ref exprs)
                => write!(f, "DW {}", expr_list_to_string(exprs)),

            Token::Equ(ref name, ref expr)
                => write!(f, "{} EQU {}", name, expr),

            Token::Include(ref fname, _)
                => write!(f, "INCLUDE \"{}\"", fname),

            Token::Print(ref exp)
                => write!(f, "PRINT {}", exp),

            Token::Protect(ref exp1, ref exp2)
                => write!(f, "PROTECT {}, {}", exp1, exp2),

            Token::Repeat(ref exp, ref code, ref label) => {
                if label.is_some() {
                    write!(f, "REPEAT {}, {}\n", exp, label.as_ref().unwrap())?;
                }
                else {
                    write!(f, "REPEAT {}\n", exp)?;
                }
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

            Token::MacroCall(ref name, ref args)
                => {
                    write!(f, "{} {}", name, args.clone().join(", "))?;
                    Ok(())
            },
            _ => unimplemented!()

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
          &Token::Org(ref expr, _)  |  &Token::Equ(_, ref expr)=> Some(expr),
            _ => None
        }
    }

    /// Unroll the tokens when in a repetition loop
    /// TODO return an iterator in order to not produce the vector each time
    pub fn unroll(&self, sym: & SymbolsTableCaseDependent) -> 
        Option<Result<Vec<&Token>, AssemblerError>> {
        if let Token::Repeat(ref expr, ref tokens, ref counter_label) = self {
            let count: Result<i32, AssemblerError> = expr.resolve(sym);
            if count.is_err() {
                return Some(Err(count.err().unwrap()));
            }
            else {
                let count = count.unwrap();
                let mut res =  Vec::with_capacity(count as usize * tokens.len());
                for i in 0..count {
                    // TODO add a specific token to control the loop counter (and change the return type)
                    for t in tokens.iter() {
                        res.push(t);
                    }
                }
                return Some(Ok(res));
            }
        }
        else {
            None
        }
    }

    /// Modify the few tokens that need to read files
    /// TODO refactor file reading of filename search
    pub fn read_referenced_file(&mut self, ctx: &ParserContext) -> Result<(), AssemblerError> {
        match self {
            Token::Include(ref fname, ref mut listing) if listing.is_none() => {

                match ctx.get_path_for(fname) {
                    None => {
                        return Err(AssemblerError::IOError{
                            msg: format!("{:?} not found", fname)
                        });
                    },
                    Some(ref fname) => {
                        let mut f = File::open(&fname)
                                    .map_err(|e|{
                                        AssemblerError::IOError{msg: format!("Unable to open {:?}", fname )}})?;
                        let mut content = String::new();
                        f.read_to_string(&mut content)
                                    .map_err(|e|{AssemblerError::IOError{msg: e.to_string()}})?;
                        
                        let mut new_ctx = ctx.clone();
                        new_ctx.set_current_filename(fname);
                        listing.replace(parse_str_with_context(&content, &new_ctx)?);
                    }
                }
            },

            Token::Incbin(ref fname, _, _, _, _, ref mut data) if data.is_none() => {
                //TODO manage the optional arguments
                match ctx.get_path_for(fname) {
                    None => {
                         return Err(AssemblerError::IOError{
                            msg: format!("{:?} not found", fname)
                        });                       
                    },
                    Some(ref fname) => {
                        let mut f = File::open(&fname)
                                    .map_err(|e|{
                                        AssemblerError::IOError{msg: format!("Unable to open {:?}", fname )}})?;
                        let mut content = Vec::new();
                        f.read_to_end(&mut content)
                                    .map_err(|e|{AssemblerError::IOError{msg: e.to_string()}})?;
                        data.replace(content);
                    }
                }

            },
            _ => {}
        }

        Ok(())
    }

    /// Dummy version that assemble without taking into account the context
    /// TODO find a way to not build a symbol table each time
    pub fn to_bytes(&self) -> Result<Bytes, AssemblerError> {
        let mut table = SymbolsTableCaseDependent::laxist();
        let table = &mut table;
        self.to_bytes_with_context(table)
    }

    /// Assemble the symbol taking into account some context, but never modify this context
    pub fn to_bytes_with_context(&self, table: &mut SymbolsTableCaseDependent) -> Result<Bytes, AssemblerError> {

        let mut env = &mut crate::assembler::assembler::Env::with_table_case_dependent(table);
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

            &Token::Defw(_) | &Token::Defb(_)
                => assemble_db_or_dw(self, env),

            &Token::Label(_) | &Token::Comment(_) | &Token::Org(_, _) | &Token::Assert(_, _)
                => Ok(Bytes::new()),

            &Token::Defs(ref expr, ref fill)
                => assemble_defs(expr, fill.as_ref(), env),

            &Token::Align(ref expr, ref fill)
                => assemble_align(expr, fill.as_ref(), env),

            // Protect and breakpoint directives do not produce any bytes
            Token::Protect(_, _) | Token::Breakpoint(_) | Token::Print(_)
                => Ok(Bytes::new()),

            _
                => Err(format!("Currently unable to generate bytes for {}", self).into())
        }
    }
}


