use cpclib_tokens::symbols::*;
use cpclib_tokens::tokens::*;

use crate::assembler::{assemble_align, assemble_defs, assemble_opcode, Bytes};
use crate::error::*;

use crate::implementation::expression::ExprEvaluationExt;
use crate::implementation::listing::ListingExt;

use crate::AssemblingOptions;

/// Needed methods for the Token defined in cpclib_tokens
pub trait TokenExt: ListingElement {
    fn estimated_duration(&self) -> Result<usize, AssemblerError>;
    fn number_of_bytes(&self) -> Result<usize, String>;
    fn number_of_bytes_with_context(
        &self,
        table: &mut SymbolsTableCaseDependent,
    ) -> Result<usize, String>;

    /// Unroll the tokens when it represents a loop
    fn unroll(&self, env: &crate::Env)
        -> Option<Result<Vec<&Self>, AssemblerError>>;

    /// Generate the listing of opcodes for directives that embed bytes
    fn disassemble_data(&self) -> Result<Listing, String>;

    /// Assemble the token to a stream of bytes
    fn to_bytes(&self) -> Result<Vec<u8>, AssemblerError>;

    /// Assemble the token to a streal of bytes .Can use the symbols context
    #[deprecated]
    fn to_bytes_with_context(
        &self,
        table: &mut SymbolsTableCaseDependent,
    ) -> Result<Vec<u8>, AssemblerError>;

    fn to_bytes_with_options(&self, option: &AssemblingOptions) -> Result<Vec<u8>, AssemblerError>;

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
        env: &crate::Env,
    ) -> Option<Result<Vec<&Self>, AssemblerError>> {
        if let Token::Repeat(ref expr, ref tokens, ref _counter_label, ref _counter_start) = self {
            let count: Result<i32, AssemblerError> = expr.resolve(env);
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
                    Token::Defb(_) | Token::Defw(_) | Token::Defs(_, _) => {
                        return Err(format!("{} as not been disassembled", token))
                    }
                    _ => {}
                }
            }

            return Ok(lst);
        };

        match self {
            Token::Defs(ref expr, ref value) => {
                use crate::assembler::Env;

                assemble_defs(expr, value.as_ref(), &Env::default())
                    .or_else(|err| Err(format!("Unable to assemble {}: {:?}", self, err)))
                    .and_then(|b| wrap(&b))
            }

            Token::Defb(_) | Token::Defw(_) => {
                use crate::assembler::Env;
                use crate::assembler::visit_db_or_dw;

                let mut env = Env::default();
                visit_db_or_dw(self, &mut env)
                    .map_err(|err| format!("Unable to assemble {}: {:?}", self, err))?;
                wrap(&env.produced_bytes())
            }

            _ => {
                let mut lst = Listing::new();
                lst.push(self.clone());
                Ok(lst)
            }
        }
    }

    /// Dummy version that assemble without taking into account the context
    /// TODO find a way to not build a symbol table each time
    fn to_bytes(&self) -> Result<Vec<u8>, AssemblerError> {
        let mut table = SymbolsTableCaseDependent::laxist();
        let table = &mut table;
        self.to_bytes_with_context(table)
    }

    /// Assemble the symbol taking into account some context, but never modify this context
    #[allow(clippy::match_same_arms)]
    fn to_bytes_with_context(
        &self,
        table: &mut SymbolsTableCaseDependent,
    ) -> Result<Vec<u8>, AssemblerError> {
        let mut options = if table.is_case_sensitive() {
            AssemblingOptions::new_case_sensitive()
        } else {
            AssemblingOptions::new_case_insensitive()
        };
        options.set_symbols(table.table());
        self.to_bytes_with_options(&options)
    }

    fn to_bytes_with_options(&self, option: &AssemblingOptions) -> Result<Vec<u8>, AssemblerError> {
        let listing: Listing = self.clone().into();
        listing.to_bytes_with_options(option)
    }

    /// Returns an estimation of the duration.
    /// This estimation may be wrong for instruction having several states.
    #[allow(clippy::match_same_arms)]
    fn estimated_duration(&self) -> Result<usize, AssemblerError> {
        let duration = match self {
            Token::Assert(_, _)
            | Token::Breakpoint(_)
            | Token::Comment(_)
            | Token::Label(_)
            | Token::Equ(_, _)
            | Token::Protect(_, _) => 0,

            // Here, there is a strong limitation => it will works only if no symbols are used
            Token::Defw(_) | Token::Defb(_) | Token::Defs(_, _) => self
                .disassemble_data()
                .map_err(|e| AssemblerError::DisassemblerError{msg:e})
                .and_then(|lst| lst.estimated_duration())?,

            Token::OpCode(ref mnemonic, ref arg1, ref arg2, ref arg3) => {
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
            _ => return Err(AssemblerError::BugInAssembler{msg: format!("Duration computation for {:?} not yet coded", self)}),
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

#[cfg(test)]
#[allow(clippy::pedantic)]
#[allow(warnings)]
mod tests {
    use crate::preamble::*;

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
        assert!(!Token::OpCode(
            Mnemonic::Out,
            Some(DataAccess::Register8(Register8::C)),
            Some(DataAccess::Register8(Register8::A)),
            None
        )
        .is_valid());
    }

    #[cfg(test)]
    mod test {

        use super::*;
        use ParseToken;
        #[test]
        fn fixup_duration() {
            assert_eq!(
                Token::parse_token(" di")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                1
            );
            assert_eq!(
                Token::parse_token(" add a,c ")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                1
            );
            assert_eq!(
                Token::parse_token(" ld l, a")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                1
            );
            assert_eq!(
                Token::parse_token(" ld b, e")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                1
            );
            assert_eq!(
                Token::parse_token(" ld e, b")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                1
            );
            assert_eq!(
                Token::parse_token(" exx")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                1
            );
            assert_eq!(
                Token::parse_token(" push bc")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                4
            );
            assert_eq!(
                Token::parse_token(" pop bc")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                3
            );
            assert_eq!(
                Token::parse_token(" push ix")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                5
            );
            assert_eq!(
                Token::parse_token(" pop ix")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                4
            );
            assert_eq!(
                Token::parse_token(" ld b, nnn")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                2
            );
            assert_eq!(
                Token::parse_token(" ld e, (hl)")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                2
            );
            assert_eq!(
                Token::parse_token(" ld d, (hl)")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                2
            );
            assert_eq!(
                Token::parse_token(" ld a, (hl)")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                2
            );
            assert_eq!(
                Token::parse_token(" ld a, (dd)")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                4
            );
            assert_eq!(
                Token::parse_token(" ld hl, (dd)")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                5
            );
            println!("{:?}", Token::parse_token(" ld de, (dd)").unwrap());
            assert_eq!(
                Token::parse_token(" ld de, (dd)")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                6
            );
            assert_eq!(
                Token::parse_token(" ld a, (ix+0)")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                5
            );
            assert_eq!(
                Token::parse_token(" ld l, (ix+0)")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                5
            );
            assert_eq!(
                Token::parse_token(" ldi")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                5
            );
            assert_eq!(
                Token::parse_token(" inc c")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                1
            );
            assert_eq!(
                Token::parse_token(" inc l")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                1
            );
            assert_eq!(
                Token::parse_token(" dec c")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                1
            );
            assert_eq!(
                Token::parse_token(" out (c), d")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                4
            );
            assert_eq!(
                Token::parse_token(" out (c), c")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                4
            );
            assert_eq!(
                Token::parse_token(" out (c), e")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                4
            );
            assert_eq!(
                Token::parse_token(" ld b, 0x7f")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                2
            );

            assert!(Token::Basic(None, None, "".to_owned())
                .estimated_duration()
                .is_err());
        }
    }
}
