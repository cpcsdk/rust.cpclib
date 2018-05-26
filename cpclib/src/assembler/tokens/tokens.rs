use assembler::tokens::*;
use assembler::tokens::listing::*;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Label(String),
    Comment(String),

    OpCode(Mnemonic, Option<DataAccess>, Option<DataAccess>),

    Assert(Expr),
    Defs(Expr),
    Db(Vec<Expr>),
    Dw(Vec<Expr>),
    Equ(String, Expr),
    Include(String),
    Org(Expr),

    MacroCall(String) // TODO add parameters
}



impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        let expr_list_to_string= |exprs: &Vec<Expr>| {
            exprs
                .iter()
                .map(|expr|{ format!("{}", expr)})
                .collect::<Vec<_>>()
                .join(",")
        };

        match self {
            &Token::Assert(ref expr)
                => write!(f, "ASSERT {}", expr),
            &Token::Label(ref string)
                => write!(f, "{}", string),
            &Token::Comment(ref string)
                => write!(f, " ; {}", string),
            &Token::OpCode(ref mne, None, None)
                => write!(f, "{}", mne),
            &Token::OpCode(ref mne, Some(ref arg1), None)
                => write!(f, "{} {}", mne, arg1),
            &Token::OpCode(ref mne, None, Some(ref arg2)) // JP/JR without flags
                => write!(f, "{} {}", mne, arg2),
            &Token::OpCode(ref mne, Some(ref arg1), Some(ref arg2))
                => write!(f, "{} {}, {}", mne, arg1, arg2),
            &Token::Org(ref expr)
                => write!(f, "ORG {}", expr),
            &Token::Defs(ref expr)
                => write!(f, "DEFS {}", expr),
            &Token::Db(ref exprs)
                => write!(f, "DB {}", expr_list_to_string(exprs)),
            &Token::Dw(ref exprs)
                => write!(f, "DW {}", expr_list_to_string(exprs)),
            &Token::Equ(ref name, ref expr)
                => write!(f, "{} EQU {}", name, expr),
            &Token::Include(ref fname)
                => write!(f, "INCLUDE \"{}\"", fname),
            &Token::MacroCall(ref name)
                => write!(f, "{}", name)

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
            &Token::OpCode(ref mnemonic, ref arg1, ref arg2) => Some(mnemonic),
            _ => None
        }
    }

    pub fn mnemonic_arg1(&self) -> Option<&DataAccess> {
        match self {
            &Token::OpCode(ref mnemonic, ref arg1, ref arg2) => arg1.as_ref(),
            _ => None
        }
    }

    pub fn mnemonic_arg2(&self) -> Option<&DataAccess> {
        match self {
            &Token::OpCode(ref mnemonic, ref arg1, ref arg2) => arg2.as_ref(),
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



    /// Dummy version that assemble without taking into account the context
    /// TODO find a way to not build a symbol table each time
    pub fn to_bytes(&self) -> Result<Bytes, String> {
        let table = &SymbolsTable::laxist();

        match self {
            &Token::OpCode(ref mnemonic, ref arg1, ref arg2)
                => assemble_opcode(mnemonic, arg1, arg2, table),
            &Token::Dw(_) | &Token::Db(_)
                => assemble_db_or_dw(self, table),
            &Token::Label(_) | &Token::Comment(_)
                => Ok(Bytes::new()),
            &Token::Defs(ref expr)
                =>assemble_defs(expr, table),
            _
                => Err(String::from("Currently unable to generate bytes. Need to code that"))
        }
    }
}


impl ListingElement for Token {
    /// Returns an estimation of the duration.
    /// This estimation may be wrong for instruction having several states.
    /// Current version is dumbly simplified : we consider the duration is equal to the size of the
    /// instruction. This is flase of course.
    fn estimated_duration(&self) -> usize {
        match self {
            &Token::Comment(_) | &Token::Label(_) => 0,
            &Token::Defs(ref expr) => expr.eval().ok().unwrap() as usize, // XXX will not work when variables are used
            &Token::OpCode(ref mnemonic, ref arg1, ref arg2) => {
                match mnemonic {

                    &Mnemonic::Inc | &Mnemonic::Dec => {
                        match arg1 {
                            &Some(DataAccess::Register8(_)) => 1,
                            &Some(DataAccess::Register16(_)) => 2,
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

                    &Mnemonic::Nop=> 1,

                    &Mnemonic::Pop => {
                        match arg1 {
                            &Some(DataAccess::Register16(_)) => 3,
                            &Some(DataAccess::IndexedRegister16(_)) => 4,
                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                        }
                    },

                    &Mnemonic::Push => {
                        match arg1 {
                            &Some(DataAccess::Register16(_)) => 4,
                            &Some(DataAccess::IndexedRegister16(_)) => 5,
                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                        }
                    },

                    &Mnemonic::Res | &Mnemonic::Set => {
                        match arg2 {
                            &Some(DataAccess::Register8(_)) => 2,
                            &Some(DataAccess::MemoryRegister16(_)) => 3, // XXX only HL
                            &Some(DataAccess::IndexedRegister16WithIndex(_, _)) => 7,
                            _ => panic!("Impossible case {:?}, {:?}, {:?}", mnemonic, arg1, arg2)
                        }
                    },

                    &Mnemonic::Ret => {
                        match arg1 {
                            none => 3,
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
    fn number_of_bytes(&self) -> Result<usize, &str> {
        let bytes = self.to_bytes();
        if bytes.is_ok() {
            Ok(bytes.ok().unwrap().len())
        }
        else {
            eprintln!("{:?}", bytes);
            Err("Unable to get the bytes of this token")
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
                _ => {write!(f, "\t");}
            }
            write!(f, "{}\n", token);
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


    /// Add additional tokens, that need to be parsed from a string, to the listing
    pub fn add_code(&mut self, code: &str) -> Result<(), String> {
        let res = parser::parse_z80_str(code);

        let tokens = match res {
            Ok((res, local_tokens)) => {
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





