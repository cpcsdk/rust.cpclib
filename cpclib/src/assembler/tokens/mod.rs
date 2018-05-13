use nom::{
    AtEof,
    Compare,
    CompareResult,
    FindSubstring,
    InputIter,
    InputLength,
    Offset,
    Slice,
    ErrorKind
};
use nom::types::{CompleteStr, CompleteByteSlice};
use std::ops::Deref;
use std::ops::DerefMut;
use std::str::FromStr;
use std::slice::Iter;
use std::iter::Enumerate;
use memchr;


/// Represent the type of the input elements.
pub type InputElement = u8;

/// Represent the type of the input.
pub type Input<'a> = &'a [InputElement];


use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use assembler::parser;
use assembler::assembler::{assemble_opcode,assemble_db_or_dw,assemble_defs,Bytes,SymbolsTable};

///TODO add the ability to manipulate value of different symbol
#[derive(PartialEq, Eq, Clone)]
pub enum Expr {
// types to manipulate
  Value(i32),
  Label(String),

  // Arithmetic operations
  Add(Box<Expr>, Box<Expr>),
  Sub(Box<Expr>, Box<Expr>),
  Mul(Box<Expr>, Box<Expr>),
  Div(Box<Expr>, Box<Expr>),
  Mod(Box<Expr>, Box<Expr>),

  Paren(Box<Expr>),

  // Boolean operations
  Equal(Box<Expr>, Box<Expr>),
  LowerOrEqual(Box<Expr>, Box<Expr>),
  GreaterOrEqual(Box<Expr>, Box<Expr>),
  StrictlyGreater(Box<Expr>, Box<Expr>),
  StrictlyLower(Box<Expr>, Box<Expr>),
}



impl Expr {

    /// Simple evaluation without context => can only evaluate number based operations.
    pub fn eval(&self) -> Result<i32, String> {
        let sym = SymbolsTable::default();
        self.resolve(&sym)
    }


    pub fn resolve(&self, sym: &SymbolsTable) -> Result<i32, String> {
        use self::Expr::*;

        let oper = |left: &Expr, right: &Expr, oper:Oper| -> Result<i32, String>{
            let res_left = left.resolve(sym);
            let res_right = right.resolve(sym);

            match (res_left, res_right) {
                (Ok(a), Ok(b)) => {
                    match oper {
                        Oper::Add => Ok(a+b),
                        Oper::Sub => Ok(a-b),
                        Oper::Mul => Ok(a*b),
                        Oper::Div => Ok(a/b),
                        Oper::Mod => Ok(a%b),

                        Oper::Equal => Ok( (a == b) as i32),

                        Oper::LowerOrEqual => Ok( (a <= b) as i32),
                        Oper::StrictlyLower=> Ok( (a < b) as i32),
                        Oper::GreaterOrEqual => Ok( (a >= b) as i32),
                        Oper::StrictlyGreater=> Ok( (a > b) as i32),
                    }
                },
                (Err(a), Ok(b))  => Err(format!("Unable to make the operation {:?}: {}", oper, a)),
                (Ok(a), Err(b))  => Err(format!("Unable to make the operation {:?}: {}", oper, b)),
                (Err(a), Err(b))  => Err(format!("Unable to make the operation {:?}: {} & {}", oper, a, b))
            }

        };

        match *self {
            Value(val) => Ok(val),

            Label(ref label) => {
                let val = sym.value(label);
                if val.is_some() {
                    Ok(val.unwrap())
                }
                else {
                    Err(format!("{} not found in symbol table", label))
                }
            }

            Add(ref left, ref right) => oper(left, right, Oper::Add),
            Sub(ref left, ref right) => oper(left, right, Oper::Sub),
            Mul(ref left, ref right) => oper(left, right, Oper::Mul),
            Div(ref left, ref right) => oper(left, right, Oper::Div),
            Mod(ref left, ref right) => oper(left, right, Oper::Mod),


            Equal(ref left, ref right) => oper(left, right, Oper::Equal),
            LowerOrEqual(ref left, ref right) => oper(left, right, Oper::LowerOrEqual),
            GreaterOrEqual(ref left, ref right) => oper(left, right, Oper::GreaterOrEqual),
            StrictlyGreater(ref left, ref right) => oper(left, right, Oper::StrictlyGreater),
            StrictlyLower(ref left, ref right) => oper(left, right, Oper::StrictlyLower),

            Paren(ref e) => e.resolve(sym),

            _ => Err(String::from("Need to implement the operation"))
        }
    }





    /// Check if it is necessary to read within a symbol table
    pub fn is_context_independant(&self) -> bool {
        use self::Expr::*;
        match *self {
            Label(ref label) => false,
            _ => true
        }
    }

}

#[derive(Debug)]
pub enum Oper {
  Add,
  Sub,
  Mul,
  Div,
  Mod,

  Equal,
  LowerOrEqual,
  GreaterOrEqual,
  StrictlyGreater,
  StrictlyLower,
}

impl Display for Expr {
  fn fmt(&self, format: &mut Formatter) -> fmt::Result {
    use self::Expr::*;
    match *self {
      Value(val) => write!(format, "0x{:x}", val),
      Label(ref label) => write!(format, "{}", label),

      Add(ref left, ref right) => write!(format, "{} + {}", left, right),
      Sub(ref left, ref right) => write!(format, "{} - {}", left, right),
      Mul(ref left, ref right) => write!(format, "{} * {}", left, right),
      Mod(ref left, ref right) => write!(format, "{} % {}", left, right),
      Div(ref left, ref right) => write!(format, "{} / {}", left, right),

      Paren(ref expr) => write!(format, "({})", expr),


      Equal(ref left, ref right) => write!(format, "{} == {}", left, right),
      GreaterOrEqual(ref left, ref right) => write!(format, "{} >= {}", left, right),
      StrictlyGreater(ref left, ref right) => write!(format, "{} > {}", left, right),
      StrictlyLower(ref left, ref right) => write!(format, "{} < {}", left, right),
      LowerOrEqual(ref left, ref right) => write!(format, "{} <= {}", left, right),
    }
  }
}

impl Debug for Expr {
  fn fmt(&self, format: &mut Formatter) -> fmt::Result {
    use self::Expr::*;
    match *self {
      Value(val) => write!(format, "{}", val),
      Label(ref label) => write!(format, "{}", label),

      Add(ref left, ref right) => write!(format, "({:?} + {:?})", left, right),
      Sub(ref left, ref right) => write!(format, "({:?} - {:?})", left, right),
      Mul(ref left, ref right) => write!(format, "({:?} * {:?})", left, right),
      Mod(ref left, ref right) => write!(format, "({:?} % {:?})", left, right),
      Div(ref left, ref right) => write!(format, "({:?} / {:?})", left, right),

      Paren(ref expr) => write!(format, "[{:?}]", expr),


      Equal(ref left, ref right) => write!(format, "{:?} == {:?}", left, right),
      GreaterOrEqual(ref left, ref right) => write!(format, "{:?} >= {:?}", left, right),
      StrictlyGreater(ref left, ref right) => write!(format, "{:?} > {:?}", left, right),
      StrictlyLower(ref left, ref right) => write!(format, "{:?} < {:?}", left, right),
      LowerOrEqual(ref left, ref right) => write!(format, "{:?} <= {:?}", left, right),

    }
  }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Register16 {
    Af,
    Hl,
    De,
    Bc,
    Sp
}
impl fmt::Display for Register16 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let code = match self {
            &Register16::Af => "AF",
            &Register16::Bc => "BC",
            &Register16::De => "DE",
            &Register16::Hl => "HL",
            &Register16::Sp => "SP"
        };
        write!(f, "{}", code)
    }
}



#[derive(Debug, PartialEq, Eq, Clone)]
pub enum IndexedRegister16{
    Ix,
    Iy
}

impl fmt::Display for IndexedRegister16 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::*;
        let code = match self {
            &IndexedRegister16::Ix => "IX",
            &IndexedRegister16::Iy => "IY"
        };
        write!(f, "{}", code)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Register8 {
    A,
    B,
    C,
    D,
    E,
    H,
    L

}

impl Register8 {

    pub fn is_high(&self) -> bool {
        match self {
            &Register8::A | &Register8::B | &Register8::D | &Register8::H => true,
            _ => false
        }
    }


    pub fn is_low(&self) -> bool  {
        !self.is_high()
    }

    pub fn neighbourg(&self) -> Option<Register8> {
        match self {
            &Register8::A => None,
            &Register8::B => Some(Register8::C),
            &Register8::C => Some(Register8::B),
            &Register8::D => Some(Register8::E),
            &Register8::E => Some(Register8::D),
            &Register8::H => Some(Register8::L),
            &Register8::L => Some(Register8::H),
        }
    }


    /// Return the 16bit register than contains this one
    pub fn complete(&self) -> Register16 {
        match self {
            &Register8::A => Register16::Af,
            &Register8::B => Register16::Bc,
            &Register8::C => Register16::Bc,
            &Register8::D => Register16::De,
            &Register8::E => Register16::De,
            &Register8::H => Register16::Hl,
            &Register8::L => Register16::Hl,
        }
    }
}

impl fmt::Display for Register8 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::*;
        let code = match self {
            &Register8::A => "A",
            &Register8::B => "B",
            &Register8::C => "C",
            &Register8::D => "D",
            &Register8::E => "E",
            &Register8::H => "H",
            &Register8::L => "L"
        };
        write!(f, "{}", code)
    }
}



#[derive(Debug, PartialEq, Eq, Clone)]
pub enum IndexRegister8 {
    Ixh,
    Ixl,
    Iyh,
    Iyl
}


impl fmt::Display for IndexRegister8 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::*;
        let code = match self {
            &IndexRegister8::Ixh => "IXH",
            &IndexRegister8::Ixl => "IXL",
            &IndexRegister8::Iyh => "IYH",
            &IndexRegister8::Iyl => "IYL"
        };
        write!(f, "{}", code)
    }
}

        /*
#[derive(Debug, PartialEq, Eq)]
pub struct Label;

#[derive(Debug, PartialEq, Eq)]
pub enum Value{
    Label(),
    Constant
}
*/

// TODO add missing flags
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FlagTest {
    NZ,
    Z,
    NC,
    C,
    PO,
    PE,
    P,
    M
}

impl fmt::Display for FlagTest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let code = match self {
            &FlagTest::NZ => "NZ",
            &FlagTest::Z => "Z",
            &FlagTest::NC => "NC",
            &FlagTest::C => "C",
            &FlagTest::PO => "PO",
            &FlagTest::PE => "PE",
            &FlagTest::P => "P",
            &FlagTest::M => "M"
        };
        write!(f, "{}", code)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
/// Encode the way mnemonics access to data
pub enum DataAccess {
    /// We are using an indexed register associated to its index
    IndexedRegister16WithIndex(IndexedRegister16, i8),
    IndexedRegister16(IndexedRegister16),
    /// Represents a standard 16 bits register
    Register16(Register16),
    /// Represents a standard 8 bits register
    Register8(Register8),
    /// Represents a memory access indexed by a register
    MemoryRegister16(Register16),
    /// Represents any expression
    Expression(Expr),
    /// Represents an address
    Memory(Expr),
    /// Represnts the test of bit flag
    FlagTest(FlagTest)
}



impl fmt::Display for DataAccess {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &DataAccess::IndexedRegister16WithIndex(ref reg, ref delta) =>
                write!(f, "({} + {})", reg, delta),
            &DataAccess::IndexedRegister16(ref reg) =>
                write!(f, "{}", reg),
            &DataAccess::Register16(ref reg) =>
                write!(f, "{}", reg),
            &DataAccess::Register8(ref reg) =>
                write!(f, "{}", reg),
            &DataAccess::MemoryRegister16(ref reg) =>
                write!(f, "({})", reg),
            &DataAccess::Expression(ref exp) =>
                write!(f, "{}", exp),
            &DataAccess::Memory(ref exp) =>
                write!(f, "({})", exp),
            &DataAccess::FlagTest(ref test) =>
                write!(f, "{}", test)
        }
    }
}


impl DataAccess {
    pub fn expr(&self) -> Option<&Expr>{
        match self {
            &DataAccess::Expression(ref expr) => Some(expr),
            _ => None
        }
    }

    pub fn is_register8(&self) -> bool {
        match self {
            &DataAccess::Register8(_) => true,
            _ => false
        }
    }

    pub fn is_register16(&self) -> bool {
        match self {
            &DataAccess::Register16(_) => true,
            _ => false
        }
    }

    pub fn is_memory(&self) -> bool {
        match self {
            &DataAccess::Memory(_) => true,
            _ => false
        }
    }

    pub fn is_address_in_register16(&self) -> bool {
        match self {
            &DataAccess::MemoryRegister16(_) => true,
            _ => false
        }
    }

    pub fn get_register16(&self) -> Option<Register16> {
        match self {
            &DataAccess::Register16(ref reg) => Some(reg.clone()),
            &DataAccess::MemoryRegister16(ref reg) => Some(reg.clone()),
            _ => None
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Mnemonic {
    Dec,
    Di,
    Ei,
    Inc,
    Jp,
    Jr,
    Ld,
    Ldd,
    Ldi,
    Nop,
    Push,
    Pop,
    Res,
    Ret,
    Set
}


impl fmt::Display for Mnemonic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Mnemonic::Dec => write!(f, "DEC"),
            &Mnemonic::Di => write!(f, "DI"),
            &Mnemonic::Ei => write!(f, "EI"),
            &Mnemonic::Inc => write!(f, "INC"),
            &Mnemonic::Jp => write!(f, "JP"),
            &Mnemonic::Jr => write!(f, "JR"),
            &Mnemonic::Ld => write!(f, "LD"),
            &Mnemonic::Ldi => write!(f, "LDI"),
            &Mnemonic::Ldd => write!(f, "LDD"),
            &Mnemonic::Nop => write!(f, "NOP"),
            &Mnemonic::Push => write!(f, "PUSH"),
            &Mnemonic::Pop => write!(f, "POP"),
            &Mnemonic::Res => write!(f, "RES"),
            &Mnemonic::Ret => write!(f, "RET"),
            &Mnemonic::Set => write!(f, "SET"),
        }
    }
}


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

    /// Return the number of bytes of the token
    pub fn number_of_bytes(&self) -> Result<usize, &str> {
        let bytes = self.to_bytes();
        if bytes.is_ok() {
            Ok(bytes.ok().unwrap().len())
        }
        else {
            eprintln!("{:?}", bytes);
            Err("Unable to get the bytes of this token")
        }
    }

    /// Returns an estimation of the duration.
    /// This estimation may be wrong for instruction having several states.
    /// Current version is dumbly simplified : we consider the duration is equal to the size of the
    /// instruction. This is flase of course.
    pub fn estimated_duration(&self) -> usize {
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



}


impl Token {

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


/// A listing is simply a list of token
#[derive(Debug,Clone)]
pub struct Listing {
    /// Ordered list of the tokens
    listing: Vec<Token>,
    /// Duration of the listing execution. Manually set by user
    duration: Option<usize>
}

impl fmt::Display for Listing {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        for token in self.listing.iter() {
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

impl Deref for Listing {
    type Target = Vec<Token>;

    fn deref(&self) -> &Self::Target {
        &self.listing
    }
}


impl DerefMut for Listing {
    fn deref_mut(&mut self) -> &mut Vec<Token>{
        &mut self.listing
    }
}


impl Listing {

    /// Create an empty listing without duration
    pub fn new() -> Listing {
        Listing{
            listing: Vec::new(),
            duration: None
        }
    }


    /// Add a new token to the listing
    pub fn add(&mut self, token:Token) {
        self.listing.push(token);
    }

    /// Add a new labl to the listing
    pub fn add_label(&mut self, label:&str) {
        self.listing.push(Token::Label(String::from(label)));
    }

    /// Consume another listing by injecting it
    pub fn inject_listing(&mut self, other:Listing) {
        self.listing.extend_from_slice(&other.listing);
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
            self.listing.extend_from_slice(&tokens.ok().unwrap());
            Ok(())
        }
        else {
            Err(format!("Unable to assemble '{}'", code))
        }

    }

    /// Compute the size of the listing.
    /// The listing has a size only if its tokens has a size
    pub fn number_of_bytes(&self) -> Result<usize, &str> {
        self
            .listing
            .iter()
            .map(|token|{token.number_of_bytes()})
            .fold(
                Ok(0),
                |sum, val| {
                    if sum.is_err() || val.is_err() {
                        Err("Unable to compute the number of bytes")
                    }
                    else {
                        Ok(sum.unwrap() + val.unwrap())
                    }
                }
            )
    }


    /// Get the execution duration.
    /// If field `duration` is set, returns it. Otherwise, compute it
    pub fn estimated_duration(&self) -> usize {
        if self.duration.is_some() {
            self.duration.unwrap()
        }
        else {
            self.listing
                .iter()
                .map(|token|{token.estimated_duration()})
                .sum()
        }
    }


    pub fn set_duration(&mut self, duration: usize) {
        self.duration = Some(duration);
    }

    /// Get the token at the required position
    pub fn get(&self, idx: usize) -> Option<&Token> {
        self.listing.get(idx)
    }
}






/// To create a listing from a string correspond to assemble the string to produce the Tokens
impl FromStr for Listing{

    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let res = parser::parse_z80_str(s);
         match res {
            Err(e) => Err(String::from_str(e.into_error_kind().description()).ok().unwrap()),
            Ok( (_, opcodes) ) => {Ok(Listing{listing: opcodes, duration: None})}
        }
    }
}


// Stolen code from https://github.com/tagua-vm/parser/blob/737e8625e51580cb6d8aaecea5b2f04fefbccaa5/source/tokens.rs


/// A span is a set of meta information about a token.
///
/// The `Span` structure can be used as an input of the nom parsers.
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Span<'a> {
    /// The offset represents the position of the slice relatively to
    /// the input of the parser. It starts at offset 0.
    pub offset: usize,

    /// The line number of the slice relatively to the input of the
    /// parser. It starts at line 1.
    pub line: u32,

    /// The column number of the slice relatively to the input of the
    /// parser. It starts at column 1.
    pub column: u32,

    /// The slice that is spanned.
    slice: Input<'a>
}

impl<'a> Span<'a> {
    /// Create a span for a particular input with default `offset`,
    /// `line`, and `column` values.
    ///
    /// `offset` starts at 0, `line` starts at 1, and `column` starts at 1.
    ///
    pub fn new(input: Input<'a>) -> Self {
        Span {
            offset: 0,
            line  : 1,
            column: 1,
            slice : input
        }
    }

    /// Create a span for a particular input at a particular offset, line, and column.
    ///
    /// # Examples
    ///
    pub fn new_at(input: Input<'a>, offset: usize, line: u32, column: u32) -> Self {
        Span {
            offset: offset,
            line  : line,
            column: column,
            slice : input
        }
    }

    /// Create a blank span.
    /// This is strictly equivalent to `Span::new(b"")`.
    ///
    /// # Examples
    pub fn empty() -> Self {
        Self::new(b"")
    }

    /// Extract the entire slice of the span.
    ///
    /// # Examples
    pub fn as_slice(&self) -> Input<'a> {
        self.slice
    }
}

/// Implement `InputLength` from nom to be able to use the `Span`
/// structure as an input of the parsers.
///
/// This trait aims at computing the length of the input.
impl<'a> InputLength for Span<'a> {
    /// Compute the length of the slice in the span.
    ///
    /// # Examples
    ///
    fn input_len(&self) -> usize {
        self.slice.len()
    }
}

/// Implement `AtEof` from nom to be able to use the `Span` structure
/// as an input of the parsers.
///
/// This trait aims at determining whether the current span is at the
/// end of the input.
impl<'a> AtEof for Span<'a> {
    fn at_eof(&self) -> bool {
        self.slice.at_eof()
    }
}

/// Implement `InputIter` from nom to be able to use the `Span`
/// structure as an input of the parsers.
///
/// This trait aims at iterating over the input.
impl<'a> InputIter for Span<'a> {
    /// Type of an element of the span' slice.
    type Item     = &'a InputElement;

    /// Type of a raw element of the span' slice.
    type RawItem  = InputElement;

    /// Type of the enumerator iterator.
    type Iter     = Enumerate<Iter<'a, Self::RawItem>>;

    /// Type of the iterator.
    type IterElem = Iter<'a, Self::RawItem>;

    /// Return an iterator that enumerates the byte offset and the
    /// element of the slice in the span.
    fn iter_indices(&self) -> Self::Iter {
        self.slice.iter().enumerate()
    }

    /// Return an iterator over the elements of the slice in the span.
    ///
    fn iter_elements(&self) -> Self::IterElem {
        self.slice.iter()
    }

    /// Find the byte position of an element in the slice of the span.
    ///
    fn position<P>(&self, predicate: P) -> Option<usize>
        where P: Fn(Self::RawItem) -> bool {
        self.slice.iter().position(|x| predicate(*x))
    }

    /// Get the byte offset from the element's position in the slice
    /// of the span.
    ///
    fn slice_index(&self, count: usize) -> Option<usize> {
        if self.slice.len() >= count {
            Some(count)
        } else {
            None
        }
    }
}

/// Implement `FindSubstring` from nom to be able to use the `Span`
/// structure as an input of the parsers.
///
/// This traits aims at finding a substring in an input.
impl<'a, 'b> FindSubstring<Input<'b>> for Span<'a> {
    /// Find the position of a substring in the current span.
    ///
    fn find_substring(&self, substring: Input<'b>) -> Option<usize> {
        let substring_length = substring.len();

        if substring_length == 0 {
            None
        } else if substring_length == 1 {
            memchr::memchr(substring[0], self.slice)
        } else {
            let max          = self.slice.len() - substring_length;
            let mut offset   = 0;
            let mut haystack = self.slice;

            while let Some(position) = memchr::memchr(substring[0], haystack) {
                offset += position;

                if offset > max {
                    return None
                }

                if &haystack[position..position + substring_length] == substring {
                    return Some(offset);
                }

                haystack  = &haystack[position + 1..];
                offset   += 1;
            }

            None
        }
    }
}

/// Implement `Compare` from nom to be able to use the `Span`
/// structure as an input of the parsers.
///
/// This trait aims at comparing inputs.
impl<'a, 'b> Compare<Input<'b>> for Span<'a> {
    /// Compare self to another input for equality.
    ///
    fn compare(&self, element: Input<'b>) -> CompareResult {
        self.slice.compare(element)
    }

    /// Compare self to another input for equality independently of the case.
    fn compare_no_case(&self, element: Input<'b>) -> CompareResult {
        self.slice.compare_no_case(element)
    }
}



#[cfg(test)]
mod test {
    use assembler::tokens::{Token,Mnemonic,DataAccess,Expr, Register16, Register8, FlagTest, Listing};
    use std::str::FromStr;
    #[test]
    fn test_size (){

        assert_eq!(
            Token::OpCode(Mnemonic::Jp, None, Some(DataAccess::Expression(Expr::Value(0))))
                .number_of_bytes(),
            Ok(3)
        );

        assert_eq!(
            Token::OpCode(Mnemonic::Jr, None, Some(DataAccess::Expression(Expr::Value(0))))
                .number_of_bytes(),
            Ok(2)
        );

        assert_eq!(
            Token::OpCode(Mnemonic::Jr, Some(DataAccess::FlagTest(FlagTest::NC)), Some(DataAccess::Expression(Expr::Value(0))))
                .number_of_bytes(),
            Ok(2)
        );

        assert_eq!(
            Token::OpCode(Mnemonic::Push, Some(DataAccess::Register16(Register16::De)), None)
                .number_of_bytes(),
            Ok(1)
        );

        assert_eq!(
            Token::OpCode(Mnemonic::Dec, Some(DataAccess::Register8(Register8::A)), None)
                .number_of_bytes(),
            Ok(1)
        );
    }


    #[test]
    fn test_listing () {
        let mut listing = Listing::from_str("   nop").expect("unable to assemble");
        assert_eq!(listing.estimated_duration(), 1);
        listing.set_duration(100);
        assert_eq!(listing.estimated_duration(), 100);
    }



    #[test]
    fn test_duration() {
         let mut listing = Listing::from_str("
            pop de      ; 3
            inc l       ; 1
            ld (hl), e  ; 2
            inc l       ; 1
            ld (hl), d  ; 2
        ").expect("Unable to assemble this code");
        println!("{}", listing);
        assert_eq!(listing.estimated_duration(), (3+1+2+1+2));
    }
}
