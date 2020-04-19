use cpclib_tokens::tokens::*;
use cpclib_tokens::symbols::*;

use core::iter::FromIterator;

use crate::assembler::{assemble_align, assemble_db_or_dw, assemble_defs, assemble_opcode, Bytes};
use crate::parser::*;
use crate::error::*;

use cpclib_tokens::tokens::*;
use cpclib_tokens::symbols::*;
use crate::implementation::expression::ExprEvaluationExt;
use crate::implementation::listing::ListingExt;

use std::fs::File;
use std::io::Read;

/// Needed methods for the Token defined in cpclib_tokens
pub trait TokenExt : ListingElement {

    
    fn estimated_duration(&self) -> Result<usize, String>;
    fn number_of_bytes(&self) -> Result<usize, String>;
    fn number_of_bytes_with_context(
        &self,
        table: &mut SymbolsTableCaseDependent,
    )-> Result<usize, String>;



    /// Unroll the tokens when it represents a loop
    fn unroll(
        &self,
        sym: &SymbolsTableCaseDependent,
    ) -> Option<Result<Vec<&Self>, AssemblerError>>;

    /// Generate the listing of opcodes for directives that embed bytes
    fn disassemble_data(&self) -> Result<Listing, String>;

    /// Modify the few tokens that need to read files. We consider they are empty at this point
    fn read_referenced_file(&mut self, ctx: &ParserContext) -> Result<(), AssemblerError>;

    /// Assemble the token to a stream of bytes
    fn to_bytes(&self) -> Result<Bytes, AssemblerError>;

    /// Assemble the token to a streal of bytes .Can use the symbols context
    fn to_bytes_with_context(
        &self,
        table: &mut SymbolsTableCaseDependent,
    ) -> Result<Bytes, AssemblerError>;

    /// Check if the token is valid. We consider a token vlaid if it is possible to assemble it
    fn is_valid(&self) -> bool {
        self.to_bytes().is_ok()
    }
}


impl TokenExt for Token {
     /// Unroll the tokens when in a repetition loop
    /// TODO return an iterator in order to not produce the vector each time
    fn unroll(
        &self,
        sym: &SymbolsTableCaseDependent,
    ) -> Option<Result<Vec<&Self>, AssemblerError>> {
        if let Token::Repeat(ref expr, ref tokens, ref _counter_label) = self {
            let count: Result<i32, AssemblerError> = expr.resolve(sym);
            if count.is_err() {
                Some(Err(count.err().unwrap()))
            } else {
                let count = count.unwrap();
                let mut res = Vec::with_capacity(count as usize * tokens.len());
                for _i in 0..count {
                    // TODO add a specific token to control the loop counter (and change the return type)
                    for t in tokens.iter() {
                        res.push(t);
                    }
                }
                Some(Ok(res))
            }
        } else {
            None
        }
    }

    /// Generate the listing of opcodes for directives that contain data Defb/defw/Defs in order to have
    /// mnemonics. Fails when some values are not opcodes
    fn disassemble_data(&self) -> Result<Listing, String> {

        // Disassemble the bytes and return the listing ONLY if it has no more defb/w/s directives
        let wrap = |bytes: &[u8]| {
            use crate::disass::disassemble;

            let lst = disassemble(&bytes);
            for token in lst.listing() {
                match token {
                    Token::Defb(_)| Token::Defw(_) | Token::Defs(_, _) => {
                        return Err(format!("{} as not been disassembled", token))
                    },
                    _ => {}
                }
            }

            return Ok(lst);
        };

        match self {
            Token::Defs(ref expr, ref value) => {
                use crate::assembler::Env;
                use crate::disass::disassemble;

                assemble_defs(expr, value.as_ref(), &Env::default())
                            .or_else(|err|{ Err(format!("Unable to assemble {}: {:?}", self, err))})
                            .and_then(|b| wrap(&b))
            },

            Token::Defb(_) | Token::Defw(_) => {
                use crate::assembler::Env;
                use crate::disass::disassemble;

                assemble_db_or_dw(self, &Env::default())
                            .or_else(|err|{ Err(format!("Unable to assemble {}: {:?}", self, err))})
                            .and_then(|b| wrap(&b))

            },

            _ => {
                let mut lst = Listing::new();
                lst.push(self.clone());
                Ok(lst)
            }
        }

    }

    /// Modify the few tokens that need to read files
    /// TODO refactor file reading of filename search
    fn read_referenced_file(&mut self, ctx: &ParserContext) -> Result<(), AssemblerError> {
        match self {
            Token::Include(ref fname, ref mut listing) if listing.is_none() => {
                match ctx.get_path_for(fname) {
                    None => {
                        return Err(AssemblerError::IOError {
                            msg: format!("{:?} not found", fname),
                        });
                    }
                    Some(ref fname) => {
                        let mut f = File::open(&fname).map_err(|_e| AssemblerError::IOError {
                            msg: format!("Unable to open {:?}", fname),
                        })?;
                        let mut content = String::new();
                        f.read_to_string(&mut content)
                            .map_err(|e| AssemblerError::IOError { msg: e.to_string() })?;

                        let mut new_ctx = ctx.clone();
                        new_ctx.set_current_filename(fname);
                        listing.replace(parse_str_with_context(&content, &new_ctx)?);
                    }
                }
            }

            Token::Incbin(ref fname, _, _, _, _, ref mut data, ref transformation)
                if data.is_none() =>
            {
                //TODO manage the optional arguments
                match ctx.get_path_for(fname) {
                    None => {
                        return Err(AssemblerError::IOError {
                            msg: format!("{:?} not found", fname),
                        });
                    }
                    Some(ref fname) => {
                        let mut f = File::open(&fname).map_err(|_e| AssemblerError::IOError {
                            msg: format!("Unable to open {:?}", fname),
                        })?;
                        let mut content = Vec::new();
                        f.read_to_end(&mut content)
                            .map_err(|e| AssemblerError::IOError { msg: e.to_string() })?;

                        match transformation {
                            BinaryTransformation::None => {
                                data.replace(content);
                            }
                            BinaryTransformation::Exomizer => {
                                unimplemented!("Need to implement exomizer crunching")
                            }
                        }
                    }
                }
            }

            // Rorg may embed some instructions that read files
            Token::Rorg(_, ref mut listing) => {
                for token in listing.iter_mut() {
                    token.read_referenced_file(ctx)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Dummy version that assemble without taking into account the context
    /// TODO find a way to not build a symbol table each time
    fn to_bytes(&self) -> Result<Bytes, AssemblerError> {
        let mut table = SymbolsTableCaseDependent::laxist();
        let table = &mut table;
        self.to_bytes_with_context(table)
    }


    /// Assemble the symbol taking into account some context, but never modify this context
    #[allow(clippy::match_same_arms)]
    fn to_bytes_with_context(
        &self,
        table: &mut SymbolsTableCaseDependent,
    ) -> Result<Bytes, AssemblerError> {
        let env = &mut crate::assembler::Env::with_table_case_dependent(table);
        match self {
            Token::OpCode(ref mnemonic, ref arg1, ref arg2) => assemble_opcode(
                *mnemonic, arg1, arg2, env, // Modification to the environment are lost
            ),

            Token::Equ(_, _) => Ok(Bytes::new()),

            Token::Defw(_) | Token::Defb(_) => assemble_db_or_dw(self, env),

            Token::Label(_) | Token::Comment(_) | Token::Org(_, _) | Token::Assert(_, _) => {
                Ok(Bytes::new())
            }

            Token::Defs(ref expr, ref fill) => assemble_defs(expr, fill.as_ref(), env),

            Token::Align(ref expr, ref fill) => assemble_align(expr, fill.as_ref(), env),

            // Protect and breakpoint directives do not produce any bytes
            Token::Protect(_, _) | Token::Breakpoint(_) | Token::Print(_) => Ok(Bytes::new()),

            _ => Err(format!("Currently unable to generate bytes for {}", self).into()),
        }
    }


    /// Returns an estimation of the duration.
    /// This estimation may be wrong for instruction having several states.
    #[allow(clippy::match_same_arms)]
    fn estimated_duration(&self) -> Result<usize, String> {
        let duration = match self {
            Token::Assert(_, _)
            | Token::Breakpoint(_)
            | Token::Comment(_)
            | Token::Label(_)
            | Token::Equ(_, _)
            | Token::Protect(_, _) => 0,

            // Here, there is a strong limitation => it will works only if no symbols are used
            Token::Defw(_) | Token::Defb(_) | Token::Defs(_, _) => {

                self.disassemble_data()
                    .and_then(|lst|{
                        lst.estimated_duration()
                    })?
            }

            Token::OpCode(ref mnemonic, ref arg1, ref arg2) => {
                match mnemonic {
                    &Mnemonic::Add => match arg1 {
                        Some(DataAccess::Register8(_)) => match arg2 {
                            Some(DataAccess::Register8(_)) => 1,
                            Some(DataAccess::IndexRegister16WithIndex(_, _)) => 5,
                            _ => 2,
                        },
                        Some(DataAccess::Register16(_)) => 4,
                        Some(DataAccess::IndexRegister16(_)) => 5,
                        _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2),
                    },

                    &Mnemonic::And | &Mnemonic::Or | &Mnemonic::Xor => match arg1 {
                        Some(DataAccess::Register8(_)) => 1,
                        Some(DataAccess::IndexRegister8(_)) => 2,
                        Some(DataAccess::Expression(_)) => 2,
                        Some(DataAccess::MemoryRegister16(_)) => 2,
                        Some(DataAccess::IndexRegister16WithIndex(_, _)) => 5,
                        _ => unreachable!(),
                    },

                    // XXX Not stable timing
                    &Mnemonic::Djnz => 3, // or 4

                    &Mnemonic::ExAf => 1,

                    &Mnemonic::Inc | &Mnemonic::Dec => match arg1 {
                        Some(DataAccess::Register8(_)) => 1,
                        Some(DataAccess::Register16(_)) => 2,
                        _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2),
                    },

                    &Mnemonic::Jp => match arg1 {
                        &None => match arg2 {
                            Some(DataAccess::Expression(_)) => 3,
                            Some(DataAccess::MemoryRegister16(Register16::Hl)) => 1,
                            Some(DataAccess::IndexRegister16(_)) => 2,
                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2),
                        },

                        Some(DataAccess::FlagTest(_)) => match arg2 {
                            Some(DataAccess::Expression(_)) => 3,
                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2),
                        },

                        _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2),
                    },

                    // Always give the fastest
                    &Mnemonic::Jr => {
                        match arg1 {
                            &None => match arg2 {
                                Some(DataAccess::Expression(_)) => 3,
                                _ => {
                                    panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                                }
                            },

                            Some(DataAccess::FlagTest(_)) => {
                                match arg2 {
                                    Some(DataAccess::Expression(_)) => 2, // or 3
                                    _ => panic!(
                                        "Impossible case {:?}, {:?}, {:?}",
                                        mnemonic, arg1, arg2
                                    ),
                                }
                            }

                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2),
                        }
                    }

                    &Mnemonic::Ld => {
                        match arg1 {
                            // Dest in memory pointed by register
                            Some(DataAccess::MemoryRegister16(_)) => {
                                match arg2 {
                                    Some(DataAccess::Register8(_)) => 2,
                                    Some(DataAccess::Expression(_)) => 3, // XXX Valid only for HL
                                    _ => panic!(
                                        "Impossible case {:?}, {:?}, {:?}",
                                        mnemonic, arg1, arg2
                                    ),
                                }
                            }

                            // Dest in 8bits reg
                            Some(DataAccess::Register8(ref _dst)) => match arg2 {
                                Some(DataAccess::Register8(_)) => 1,
                                Some(DataAccess::MemoryRegister16(Register16::Hl)) => 2,
                                Some(DataAccess::Expression(_)) => 2,
                                Some(DataAccess::Memory(_)) => 4,
                                Some(DataAccess::IndexRegister16WithIndex(_, _)) => 5,
                                _ => {
                                    panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                                }
                            },

                            // Dest in 16bits reg
                            Some(DataAccess::Register16(ref dst)) => match arg2 {
                                Some(DataAccess::Expression(_)) => 3,
                                Some(DataAccess::Memory(_)) if dst == &Register16::Hl => 5,
                                Some(DataAccess::Memory(_)) => 6,
                                _ => {
                                    panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                                }
                            },

                            Some(DataAccess::IndexRegister16(_)) => match arg2 {
                                Some(DataAccess::Expression(_)) => 4,
                                _ => {
                                    panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                                }
                            },

                            Some(DataAccess::Memory(_)) => match arg2 {
                                Some(DataAccess::Register8(Register8::A)) => 4,
                                Some(DataAccess::Register16(Register16::Hl)) => 5,
                                Some(DataAccess::Register16(_)) => 6,
                                Some(DataAccess::IndexRegister16(_)) => 6,
                                _ => {
                                    panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                                }
                            },

                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2),
                        }
                    }

                    &Mnemonic::Ldi | &Mnemonic::Ldd => 5,

                    &Mnemonic::Nop | &Mnemonic::Exx | &Mnemonic::Di | &Mnemonic::ExHlDe => 1,
                    &Mnemonic::Nops2 => 2,

                    &Mnemonic::Out => {
                        match arg1 {
                            Some(DataAccess::PortC) => 4, // XXX Not sure for out (c), 0
                            Some(DataAccess::Expression(_)) => 3,
                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2),
                        }
                    }

                    Mnemonic::Outi | Mnemonic::Outd => 5,

                    &Mnemonic::Pop => match arg1 {
                        Some(DataAccess::Register16(_)) => 3,
                        Some(DataAccess::IndexRegister16(_)) => 4,
                        _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2),
                    },

                    &Mnemonic::Push => match arg1 {
                        Some(DataAccess::Register16(_)) => 4,
                        Some(DataAccess::IndexRegister16(_)) => 5,
                        _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2),
                    },

                    &Mnemonic::Res | &Mnemonic::Set => {
                        match arg2 {
                            Some(DataAccess::Register8(_)) => 2,
                            Some(DataAccess::MemoryRegister16(_)) => 3, // XXX only HL
                            Some(DataAccess::IndexRegister16WithIndex(_, _)) => 7,
                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2),
                        }
                    }

                    &Mnemonic::Ret => match arg1 {
                        None => 3,
                        _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2),
                    },

                    &Mnemonic::Sub => match arg1 {
                        Some(DataAccess::Register8(_)) => 1,
                        Some(DataAccess::IndexRegister8(_)) => 2,
                        Some(DataAccess::Expression(_)) => 2,
                        Some(DataAccess::MemoryRegister16(Register16::Hl)) => 2,
                        Some(DataAccess::IndexRegister16WithIndex(_, _)) => 5,
                        _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2),
                    },

                    _ => panic!(
                        "Duration not set for {:?}, {:?}, {:?}",
                        mnemonic, arg1, arg2
                    ),
                }
            }
            _ => return Err(format!("Duration computation for {:?} not yet coded", self)),
        };
        Ok(duration)
    }

    /// Return the number of bytes of the token
    fn number_of_bytes(&self) -> Result<usize, String> {
        let bytes = self.to_bytes();
        if bytes.is_ok() {
            Ok(bytes.ok().unwrap().len())
        } else {
            eprintln!("{:?}", bytes);
            Err(format!("Unable to get the bytes of this token: {:?}", self))
        }
    }



    /// Return the number of bytes of the token given the provided context
    fn number_of_bytes_with_context(
        &self,
        table: &mut SymbolsTableCaseDependent,
    ) -> Result<usize, String> {
        let bytes = self.to_bytes_with_context(table);
        if bytes.is_ok() {
            Ok(bytes.ok().unwrap().len())
        } else {
            eprintln!("{:?}", bytes);
            Err(format!("Unable to get the bytes of this token: {:?}", self))
        }
    }
}


pub trait TokenTryFrom<T> {
    fn try_from(value: T) -> Result<Token, String>;
}

impl TokenTryFrom<&str> for Token {

    fn try_from(value: &str) -> Result<Self, String> {
        let tokens = {
            let res = parse_z80_str(value);
            match res {
                Ok(tokens) => tokens.1,
                Err(_e) => {
                    return Err("ERROR -- need to code why ...".to_owned());
                }
            }
        };

        match tokens.len() {
            0 => Err("No ASM found.".to_owned()),
            1 => Ok(tokens[0].clone()),
            _ => Err(format!(
                "{} tokens are present instead of one",
                tokens.len()
            ))
        }
    }
}


impl TokenTryFrom<String> for Token {

    fn try_from(value: String) -> Result<Self, String> {
        Token::try_from(&value[..])
    }
}


#[cfg(test)]
#[allow(clippy::pedantic)]
#[allow(warnings)]
mod tests {
    use crate::preamble::*;
    #[test]
    fn fixup_duration() {
        assert_eq!(
            Token::try_from(" di")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            1
        );
        assert_eq!(
            Token::try_from(" add a,c ")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            1
        );
        assert_eq!(
            Token::try_from(" ld l, a")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            1
        );
        assert_eq!(
            Token::try_from(" ld b, e")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            1
        );
        assert_eq!(
            Token::try_from(" ld e, b")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            1
        );
        assert_eq!(
            Token::try_from(" exx")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            1
        );
        assert_eq!(
            Token::try_from(" push bc")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            4
        );
        assert_eq!(
            Token::try_from(" pop bc")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            3
        );
        assert_eq!(
            Token::try_from(" push ix")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            5
        );
        assert_eq!(
            Token::try_from(" pop ix")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            4
        );
        assert_eq!(
            Token::try_from(" ld b, nnn")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            2
        );
        assert_eq!(
            Token::try_from(" ld e, (hl)")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            2
        );
        assert_eq!(
            Token::try_from(" ld d, (hl)")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            2
        );
        assert_eq!(
            Token::try_from(" ld a, (hl)")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            2
        );
        assert_eq!(
            Token::try_from(" ld a, (dd)")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            4
        );
        assert_eq!(
            Token::try_from(" ld hl, (dd)")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            5
        );
        println!("{:?}", Token::try_from(" ld de, (dd)").unwrap());
        assert_eq!(
            Token::try_from(" ld de, (dd)")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            6
        );
        assert_eq!(
            Token::try_from(" ld a, (ix+0)")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            5
        );
        assert_eq!(
            Token::try_from(" ld l, (ix+0)")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            5
        );
        assert_eq!(
            Token::try_from(" ldi")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            5
        );
        assert_eq!(
            Token::try_from(" inc c")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            1
        );
        assert_eq!(
            Token::try_from(" inc l")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            1
        );
        assert_eq!(
            Token::try_from(" dec c")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            1
        );
        assert_eq!(
            Token::try_from(" out (c), d")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            4
        );
        assert_eq!(
            Token::try_from(" out (c), c")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            4
        );
        assert_eq!(
            Token::try_from(" out (c), e")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            4
        );
        assert_eq!(
            Token::try_from(" ld b, 0x7f")
                .unwrap()
                .estimated_duration()
                .unwrap(),
            2
        );

        assert!(Token::Basic(None, None, "".to_owned())
            .estimated_duration()
            .is_err());
    }

    #[test]
    fn test_timing2() {
        // We are only able to disassemble nop ...
        assert_eq!(defs_expr_expr(10, 0).estimated_duration().unwrap(), 10);
        assert_eq!(defw(0).estimated_duration().unwrap(), 2);
        assert_eq!(defb(0).estimated_duration().unwrap(), 1);

        assert_eq!(exx().estimated_duration().unwrap(), 1);

        assert_eq!(pop_de().estimated_duration().unwrap(), 3);

        assert_eq!(inc_l().estimated_duration().unwrap(), 1);

        assert_eq!(jp_label("XX").estimated_duration().unwrap(), 3);

        assert_eq!(ld_l_mem_ix(14.into()).estimated_duration().unwrap(), 5);

        assert_eq!(ld_mem_hl_e().estimated_duration().unwrap(), 2);

        assert_eq!(ld_e_mem_hl().estimated_duration().unwrap(), 2);

        assert_eq!(ld_d_mem_hl().estimated_duration().unwrap(), 2);

        assert_eq!(out_c_d().estimated_duration().unwrap(), 4);

    }


    #[test]
    fn is_valid_ok() {
        assert!(out_c_d().is_valid());
    }

    #[test]
    fn is_valid_nok() {
        assert!(!Token::OpCode(Mnemonic::Out, Some(DataAccess::Register8(Register8::C)), Some(DataAccess::Register8(Register8::A))).is_valid());
    }
}
