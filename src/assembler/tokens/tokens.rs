use crate::assembler::tokens::*;
use crate::assembler::tokens::listing::*;
use std::str::FromStr;
use crate::assembler::assembler::SymbolsTable;

impl ListingElement for Token {
    /// Returns an estimation of the duration.
    /// This estimation may be wrong for instruction having several states.
    fn estimated_duration(&self) -> Result<usize, String> {
        let duration = match self {
            Token::Breakpoint(_) |
            Token::Comment(_) | 
                Token::Label(_) |
                Token::Equ(_, _) |
                Token::Protect(_, _) => 0,
            Token::Defs(ref expr, ref value) => {
                if value.is_some() {
                    unimplemented!(); // A disassembler is needed there to get the instruction
                }
                expr.eval().ok().unwrap() as usize
            }, 
            Token::OpCode(ref mnemonic, ref arg1, ref arg2) => {
                match mnemonic {

                    &Mnemonic::Add => {
                        match arg1 {
                            &Some(DataAccess::Register8(_)) => {
                                match arg2 {
                                    &Some(DataAccess::Register8(_)) => 1,
                                    &Some(DataAccess::IndexRegister16WithIndex(_, _, _)) => 5,
                                    _ => 2,
                                }
                            },
                            &Some(DataAccess::Register16(_)) => 4,
                            &Some(DataAccess::IndexRegister16(_)) => 5,
                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                        }
                    },

                    &Mnemonic::And | &Mnemonic::Or | &Mnemonic::Xor => {
                        match arg1 {
                            &Some(DataAccess::Register8(_)) => 1,
                            &Some(DataAccess::IndexRegister8(_)) => 2,
                            &Some(DataAccess::Expression(_)) => 2,
                            &Some(DataAccess::MemoryRegister16(_)) => 2,
                            &Some(DataAccess::IndexRegister16WithIndex(_, _, _)) => 5,
                            _ => unreachable!()
                        }
                    },


                    /// XXX Not stable timing
                    &Mnemonic::Djnz => 3, // or 4

                    &Mnemonic::ExAf => 1, 

                    &Mnemonic::Inc | &Mnemonic::Dec => {
                        match arg1 {
                            &Some(DataAccess::Register8(_)) => 1,
                            &Some(DataAccess::Register16(_)) => 2,
                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                        }
                    },

                    &Mnemonic::Jp => {
                        match arg1 {
                            &None => {
                                match arg2 {
                                    &Some(DataAccess::Expression(_)) => 3,
                                    &Some(DataAccess::MemoryRegister16(Register16::Hl)) => 1,
                                    &Some(DataAccess::IndexRegister16(_)) => 2,
                                    _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                                }
                            },

                            &Some(DataAccess::FlagTest(_)) => {
                                match arg2 {
                                    &Some(DataAccess::Expression(_)) => 3,
                                    _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                                }
                            },

                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                        }
                    },

                    // Always give the fastest
                    &Mnemonic::Jr => {
                       match arg1 {
                            &None => {
                                match arg2 {
                                    &Some(DataAccess::Expression(_)) => 3,
                                    _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                                }
                            },

                            &Some(DataAccess::FlagTest(_)) => {
                                match arg2 {
                                    &Some(DataAccess::Expression(_)) => 2, // or 3
                                    _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                                }
                            },

                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                        }
                    },

                    &Mnemonic::Ld => {
                        match arg1 {
                            // Dest in memory pointed by register
                            &Some(DataAccess::MemoryRegister16(_)) => {
                                match arg2 {
                                    &Some(DataAccess::Register8(_)) => 2,
                                    &Some(DataAccess::Expression(_)) => 3, // XXX Valid only for HL
                                    _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                                }
                            },

                            // Dest in 8bits reg
                            &Some(DataAccess::Register8(ref _dst)) => {
                                match arg2 {
                                    &Some(DataAccess::Register8(_)) => 1,
                                    &Some(DataAccess::MemoryRegister16(Register16::Hl)) => 2,
                                    &Some(DataAccess::Expression(_)) => 2,
                                    &Some(DataAccess::Memory(_)) => 4,
                                    &Some(DataAccess::IndexRegister16WithIndex(_, _, _))=> 5,
                                    _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                                }

                            },

                            // Dest in 16bits reg
                            &Some(DataAccess::Register16(ref dst)) => {
                                match arg2 {
                                    &Some(DataAccess::Expression(_)) => 3,
                                    &Some(DataAccess::Memory(_)) if dst == &Register16::Hl => 5,
                                    &Some(DataAccess::Memory(_)) => 6,
                                    _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                                }

                            },

                            &Some(DataAccess::IndexRegister16(_)) => {
                                match arg2 {
                                    &Some(DataAccess::Expression(_)) => 4,
                                    _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                                }


                            }


                            &Some(DataAccess::Memory(_)) => {
                                match arg2 {
                                    &Some(DataAccess::Register8(Register8::A)) => 4,
                                    &Some(DataAccess::Register16(Register16::Hl)) => 5,
                                    &Some(DataAccess::Register16(_)) => 6,
                                    &Some(DataAccess::IndexRegister16(_)) => 6,
                                    _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                                }


                            }

                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                        }
                    },

                    &Mnemonic::Ldi | &Mnemonic::Ldd => 5,

                    &Mnemonic::Nop | &Mnemonic::Exx | &Mnemonic::Di => 1,
                    &Mnemonic::Nops2 => 2,

                    &Mnemonic::Out => {
                        match arg1 {
                            &Some(DataAccess::Register8(Register8::C)) => 4, // XXX Not sure for out (c), 0
                            &Some(DataAccess::Expression(_)) => 3,
                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                        }
                    },

                    Mnemonic::Outi | Mnemonic::Outd => 5,

                    &Mnemonic::Pop => {
                        match arg1 {
                            &Some(DataAccess::Register16(_)) => 3,
                            &Some(DataAccess::IndexRegister16(_)) => 4,
                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                        }
                    },

                    &Mnemonic::Push => {
                        match arg1 {
                            &Some(DataAccess::Register16(_)) => 4,
                            &Some(DataAccess::IndexRegister16(_)) => 5,
                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                        }
                    },

                    &Mnemonic::Res | &Mnemonic::Set => {
                        match arg2 {
                            &Some(DataAccess::Register8(_)) => 2,
                            &Some(DataAccess::MemoryRegister16(_)) => 3, // XXX only HL
                            &Some(DataAccess::IndexRegister16WithIndex(_, _, _)) => 7,
                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                        }
                    },

                    &Mnemonic::Ret => {
                        match arg1 {
                            None => 3,
                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                        }
                    },

                    _ => panic!("Duration not set for {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                }
            }
            _ => return Err(format!("Duration computation for {:?} not yet coded", self))
        };
        Ok(duration)
    }



    /// Return the number of bytes of the token
    fn number_of_bytes(&self) -> Result<usize, String> {
        let bytes = self.to_bytes();
        if bytes.is_ok() {
            Ok(bytes.ok().unwrap().len())
        }
        else {
            eprintln!("{:?}", bytes);
            Err(format!("Unable to get the bytes of this token: {:?}", self))
        }
    }



    /// Return the number of bytes of the token given the provided context
    fn number_of_bytes_with_context(&self, table: &mut SymbolsTable) -> Result<usize, String> {
        let bytes = self.to_bytes_with_context(table);
        if bytes.is_ok() {
            Ok(bytes.ok().unwrap().len())
        }
        else {
            eprintln!("{:?}", bytes);
            Err(format!("Unable to get the bytes of this token: {:?}", self))
        }
    }


}










/// Standard listing is a specific implementation
pub type Listing = BaseListing<Token>;

impl fmt::Display for Listing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        for token in self.listing().iter() {
            match token {
                &Token::Label(_) |
                    &Token::Equ(_, _) |
                    &Token::Comment(_) => (),
                _ => {
                   write!(f, "\t")?;
                }
            }
            //write!(f, "{} ; {:?} {:?} nops {:?} bytes\n", token, token, token.estimated_duration(), token.number_of_bytes())?;
            writeln!(f, "{}", token)?;
        }

        Ok(())
    }
}

/// To create a listing from a string correspond to assemble the string to produce the Tokens
impl FromStr for Listing{

    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let res = parser::parse_z80_str(s);
         match res {
            Err(e) => Err(String::from_str(e.into_error_kind().description()).ok().unwrap()),
            Ok( (_, opcodes) ) => {;
                let mut listing = Listing::new();
                listing.extend_from_slice(&opcodes);
                Ok(listing)
            }
        }
    }
}



/// Main usage of listing is related to Tokens.. Here are the methods strongly liked to Token
impl Listing {

    /// Save the listing on disc
    pub fn save(&self, path: &::std::path::Path) -> ::std::io::Result<()> {
        use std::io::prelude::*;
        use std::fs::File;

        // Open a file in write-only mode, returns `io::Result<File>`
        let mut file = File::create(path)?;
        file.write_all(self.to_string().as_bytes())?;

        Ok(())
    }

    /// Add a new label to the listing
    pub fn add_label(&mut self, label:&str) {
        self.mut_listing().push(Token::Label(String::from(label)));
    }

    /// Add a new comment to the listing
    pub fn add_comment(&mut self, comment: &str) {
        self.mut_listing().push(Token::Comment(String::from(comment)));
    }

    /// Add a list of bytes to the listing
    pub fn add_bytes(&mut self, bytes: &[u8]) {
        let exp = bytes.iter().map(|pu8|{Expr::Value(*pu8 as _)}).collect::<Vec<_>>();
        let tok = Token::Defb(exp);
        self.push(tok);
    }

    /// Add additional tokens, that need to be parsed from a string, to the listing
    pub fn add_code<S: AsRef<str> + core::fmt::Display>(&mut self, code: S) -> Result<(), String> {
        let res = parser::parse_z80_str(code.as_ref());

        let tokens = match res {
            Ok((_res, local_tokens)) => {
                Ok(local_tokens)
            },
            Err(e) => {
                Err(e)
            }
        };


        if tokens.is_ok() {
            self.mut_listing().extend_from_slice(&tokens.ok().unwrap());
            Ok(())
        }
        else {
            Err(format!("Unable to assemble '{}'", code))
        }

    }




    /// Compute the size of the listing.
    /// The listing has a size only if its tokens has a size
    pub fn number_of_bytes(&self) -> Result<usize, String> {
        let mut count = 0;
        let mut current_address : Option<usize>= None;
        let mut sym = SymbolsTable::default();

        for token in self.listing().iter() {
            if current_address.is_some() {
                sym.set_current_address(current_address.unwrap() as u16);
            }


            let mut current_size = 0;
            if let  &Token::Org(ref expr, _) = token {
                current_address = Some(expr.resolve(&sym)? as usize);
                println!("Set address to {:?}", current_address);
            }
            else if let &Token::Align(ref _expr, _) = token {
                if current_address.is_none() {
                    return Err("Unable to guess align size if current address is unknown".to_owned())
                }

                current_size = token.number_of_bytes_with_context(&mut sym)?;
            }
            else {
                current_size = token.number_of_bytes()?;
            }


            if current_address.is_some() {
                current_address = Some(current_address.unwrap() as usize + current_size);
            }
            count += current_size;
        }

        Ok(count)

    }
}





#[cfg(test)]
mod tests {
    use crate::assembler::tokens::*;
    use std::convert::TryFrom;
    use crate::assembler::builder::*;

    #[test]
    fn fixup_duration() {
        assert_eq!(Token::try_from(" di").unwrap().estimated_duration().unwrap(), 1);
        assert_eq!(Token::try_from(" add a,c ").unwrap().estimated_duration().unwrap(), 1);
        assert_eq!(Token::try_from(" ld l, a").unwrap().estimated_duration().unwrap(), 1);
        assert_eq!(Token::try_from(" ld b, e").unwrap().estimated_duration().unwrap(), 1);
        assert_eq!(Token::try_from(" ld e, b").unwrap().estimated_duration().unwrap(), 1);
        assert_eq!(Token::try_from(" exx").unwrap().estimated_duration().unwrap(), 1);
        assert_eq!(Token::try_from(" push bc").unwrap().estimated_duration().unwrap(), 4);
        assert_eq!(Token::try_from(" pop bc").unwrap().estimated_duration().unwrap(), 3);
        assert_eq!(Token::try_from(" push ix").unwrap().estimated_duration().unwrap(), 5);
        assert_eq!(Token::try_from(" pop ix").unwrap().estimated_duration().unwrap(), 4);
        assert_eq!(Token::try_from(" ld b, nnn").unwrap().estimated_duration().unwrap(), 2);
        assert_eq!(Token::try_from(" ld e, (hl)").unwrap().estimated_duration().unwrap(), 2);
        assert_eq!(Token::try_from(" ld d, (hl)").unwrap().estimated_duration().unwrap(), 2);
        assert_eq!(Token::try_from(" ld a, (hl)").unwrap().estimated_duration().unwrap(), 2);
        assert_eq!(Token::try_from(" ld a, (dd)").unwrap().estimated_duration().unwrap(), 4);
        assert_eq!(Token::try_from(" ld hl, (dd)").unwrap().estimated_duration().unwrap(), 5);
        assert_eq!(Token::try_from(" ld de, (dd)").unwrap().estimated_duration().unwrap(), 6);
        assert_eq!(Token::try_from(" ld a, (ix+0)").unwrap().estimated_duration().unwrap(), 5);
        assert_eq!(Token::try_from(" ld l, (ix+0)").unwrap().estimated_duration().unwrap(), 5);
        assert_eq!(Token::try_from(" ldi").unwrap().estimated_duration().unwrap(), 5);
        assert_eq!(Token::try_from(" inc c").unwrap().estimated_duration().unwrap(), 1);
        assert_eq!(Token::try_from(" inc l").unwrap().estimated_duration().unwrap(), 1);
        assert_eq!(Token::try_from(" dec c").unwrap().estimated_duration().unwrap(), 1);
        assert_eq!(Token::try_from(" out (c), d").unwrap().estimated_duration().unwrap(), 4);
        assert_eq!(Token::try_from(" out (c), c").unwrap().estimated_duration().unwrap(), 4);
        assert_eq!(Token::try_from(" out (c), e").unwrap().estimated_duration().unwrap(), 4);
        assert_eq!(Token::try_from(" ld b, 0x7f").unwrap().estimated_duration().unwrap(), 2);
    }



    #[test]
    fn test_timing2() {

        assert_eq!(
            exx().estimated_duration().unwrap(),
            1
        );

        assert_eq!(
            pop_de().estimated_duration().unwrap(),
            3
        );

        assert_eq!(
            inc_l().estimated_duration().unwrap(),
            1
        );

        assert_eq!(
            jp_label("XX").estimated_duration().unwrap(),
            3
        );

        assert_eq!(
            ld_l_mem_ix(14.into()).estimated_duration().unwrap(),
            5
        );

        assert_eq!(
            ld_mem_hl_e().estimated_duration().unwrap(),
            2
        );

        assert_eq!(
            ld_e_mem_hl().estimated_duration().unwrap(),
            2
        );

        assert_eq!(
            ld_d_mem_hl().estimated_duration().unwrap(),
            2
        );

        assert_eq!(
            out_c_d().estimated_duration().unwrap(),
            4
        );
    }
}
