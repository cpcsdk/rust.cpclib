use assembler::tokens::*;
use assembler::tokens::listing::*;
use std::str::FromStr;

impl ListingElement for Token {
    /// Returns an estimation of the duration.
    /// This estimation may be wrong for instruction having several states.
    /// Current version is dumbly simplified : we consider the duration is equal to the size of the
    /// instruction. This is flase of course.
    fn estimated_duration(&self) -> usize {
        match self {
            &Token::Comment(_) | &Token::Label(_)  => 0,
            &Token::Defs(ref expr) => expr.eval().ok().unwrap() as usize, // XXX will not work when variables are used
            &Token::OpCode(ref mnemonic, ref arg1, ref arg2) => {
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
                            &Some(DataAccess::Register8(_)) => {
                                match arg2 {
                                    &Some(DataAccess::Register8(_)) => 1,
                                    &Some(DataAccess::Expression(_)) => 2,
                                    _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                                }

                            },

                            // Dest in 16bits reg
                            &Some(DataAccess::Register16(_)) => {
                                match arg2 {
                                    &Some(DataAccess::Expression(_)) => 3,
                                    _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                                }

                            },

                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                        }
                    },

                    &Mnemonic::Ldi | &Mnemonic::Ldd => 5,

                    &Mnemonic::Nop | &Mnemonic::Exx => 1,

                    &Mnemonic::Out => {
                        match arg1 {
                            &Some(DataAccess::Register8(Register8::C)) => 4, // XXX Not sure for out (c), 0
                            &Some(DataAccess::Expression(_)) => 3,
                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                        }
                    }

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
            _ => 0
        }
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


}











/// Standard listing is a specific implementation
pub type Listing = BaseListing<Token>;

impl fmt::Display for Listing {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        for token in self.listing().iter() {
            match token {
                &Token::Label(_) |
                    &Token::Equ(_, _) |
                    &Token::Comment(_) => (),
                _ => {
                    let res = write!(f, "\t");
                    if res.is_err() {return res;}
                }
            }
            let res = write!(f, "{}\n", token);
            if res.is_err(){return res;}
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

    /// Add a new label to the listing
    pub fn add_label(&mut self, label:&str) {
        self.mut_listing().push(Token::Label(String::from(label)));
    }

    /// Add a new comment to the listing
    pub fn add_comment(&mut self, comment: &str) {
        self.mut_listing().push(Token::Comment(String::from(comment)));
    }

    /// Add additional tokens, that need to be parsed from a string, to the listing
    pub fn add_code(&mut self, code: &str) -> Result<(), String> {
        let res = parser::parse_z80_str(code);

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
}





