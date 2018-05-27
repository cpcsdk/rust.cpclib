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


mod tokens;
mod listing;
mod expression;

pub use self::tokens::*;
pub use self::listing::*;
pub use self::expression::*;


/// Represent the type of the input elements.
pub type InputElement = u8;

/// Represent the type of the input.
pub type Input<'a> = &'a [InputElement];


use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use assembler::parser;
use assembler::assembler::{assemble_opcode,assemble_db_or_dw,assemble_defs,Bytes,SymbolsTable};



#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
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
pub enum IndexRegister16{
    Ix,
    Iy
}

impl fmt::Display for IndexRegister16 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::*;
        let code = match self {
            &IndexRegister16::Ix => "IX",
            &IndexRegister16::Iy => "IY"
        };
        write!(f, "{}", code)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Hash)]
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
    IndexRegister16WithIndex(IndexRegister16, Oper, Expr),
    IndexRegister16(IndexRegister16),
    IndexRegister8(IndexRegister8),
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
            &DataAccess::IndexRegister16WithIndex(ref reg, ref op,  ref delta) =>
                write!(f, "({} {} {})", reg, op, delta),
            &DataAccess::IndexRegister16(ref reg) =>
                write!(f, "{}", reg),
            &DataAccess::Register16(ref reg) =>
                write!(f, "{}", reg),
            &DataAccess::IndexRegister8(ref reg) =>
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

    pub fn is_indexregister16(&self) -> bool {
        match self {
            &DataAccess::IndexRegister16(_) => true,
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

    pub fn get_indexregister16(&self) -> Option<IndexRegister16> {
        match self {
            &DataAccess::IndexRegister16(ref reg) => Some(reg.clone()),
            _ => None
        }
    }

    pub fn get_register8(&self) -> Option<Register8> {
        match self {
            &DataAccess::Register8(ref reg) => Some(reg.clone()),
            _ => None
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Mnemonic {
    Adc,
    Add,
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
    Out,
    Push,
    Pop,
    Res,
    Ret,
    Set
}


impl fmt::Display for Mnemonic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Mnemonic::Adc=> write!(f, "ADC"),
            &Mnemonic::Add=> write!(f, "ADD"),
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
            &Mnemonic::Out => write!(f, "OUT"),
            &Mnemonic::Push => write!(f, "PUSH"),
            &Mnemonic::Pop => write!(f, "POP"),
            &Mnemonic::Res => write!(f, "RES"),
            &Mnemonic::Ret => write!(f, "RET"),
            &Mnemonic::Set => write!(f, "SET"),
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
    use assembler::tokens::{Token,Mnemonic,DataAccess,Expr, Register16, Register8, FlagTest, Listing, ListingElement};
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
