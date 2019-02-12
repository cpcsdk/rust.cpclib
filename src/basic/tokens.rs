use num_enum::CustomTryInto;
use num_enum::IntoPrimitive;
use std::fmt;

#[derive(IntoPrimitive, CustomTryInto, Copy, Clone, PartialEq, Debug)]
#[repr(u8)]
pub enum BasicTokenNoPrefix {
    EndOfTokenisedLine = 0,

    StatementSeparator = 1,

    IntegerVariableDefinition = 2,
    StringVariableDefinition = 3,
    FloatingPointVariableDefinition = 4,

    VarUnknown1 = 6,
    VarUnknown2 = 7,
    VarUnknown3 = 8,
    VarUnknown4 = 9,
    VarUnknown5 = 0xa,

    VariableDefinition1 = 0xb,
    VariableDefinition2 = 0xc,
    VariableDefinition3 = 0xd,

    ConstantNumber0 = 0x0e,
    ConstantNumber1 = 0x0f,
    ConstantNumber2 = 0x10,
    ConstantNumber3 = 0x11,
    ConstantNumber4 = 0x12,
    ConstantNumber5 = 0x13,
    ConstantNumber6 = 0x14,
    ConstantNumber7 = 0x15,
    ConstantNumber8 = 0x16,
    ConstantNumber9 = 0x17,
    ConstantNumber10 = 0x18,

    ValueIntegerDecimal8bits = 0x19,

    ValueIntegerDecimal16bits = 0x1a,
    ValueIntegerBinary16bits = 0x1b,
    ValueIntegerHexadecimal16bits = 0x1c,

    LineMemoryAddressPointer = 0x1d,
    LineNumber = 0x1e,

    ValueFloatingPoint = 0x1f,

    CharSpace = 0x20,
    CharExclamation = 0x21,

    ValueQuotedString = 0x22,

    // TODO add all ascii symbols from 23 to 7b
    CharUpperA = 65,
    CharUpperB,
    CharUpperC,
    CharUpperD,
    CharUpperE,
    CharUpperF,
    CharUpperG,
    CharUpperH,
    CharUpperI,
    CharUpperJ,
    CharUpperK,
    CharUpperL,
    CharUpperM,
    CharUpperN,
    CharUpperO,
    CharUpperP,
    CharUpperQ,
    CharUpperR,
    CharUpperS,
    CharUpperT,
    CharUpperU,
    CharUpperV,
    CharUpperW,
    CharUpperX,
    CharUpperY,
    CharUpperZ,

    CharLowerA = 97,
    CharLowerB,
    CharLowerC,
    CharLowerD,
    CharLowerE,
    CharLowerF,
    CharLowerG,
    CharLowerH,
    CharLowerI,
    CharLowerJ,
    CharLowerK,
    CharLowerL,
    CharLowerM,
    CharLowerN,
    CharLowerO,
    CharLowerP,
    CharLowerQ,
    CharLowerR,
    CharLowerS,
    CharLowerT,
    CharLowerU,
    CharLowerV,
    CharLowerW,
    CharLowerX,
    CharLowerY,
    CharLowerZ,

    Pipe = 0x7c,

    Unused7d = 0x7d,
    Unused7e = 0x7e,
    Unused7f = 0x7f,

    After = 0x80,
    Auto,
    Border,
    Call,
    Cat,
    Chain,
    Clear,
    Clg,
    Closein,
    Closeout,
    Cls,
    Cont,
    Data,
    Def,
    Defint,
    Defreal,
    Defstr,
    Deg,
    Delete,
    Dim,
    Draw,
    Drawr,
    Edit,
    Else,
    End,
    Ent,
    Env,
    Erase,
    Error,
    Every,
    For,
    Gosub,
    Goto,
    If,
    Ink,
    Input,
    Key,
    Let,
    Line,
    List,
    Load,
    Locate,
    Memory,
    Merge,
    MidDollar,
    Mode,
    Move,
    Mover,
    Next,
    New,
    On,
    OnBreak,
    OnErrorGoto,
    Sq,
    Openin,
    Openout,
    Origin,
    Out,
    Paper,
    Pen,
    Plot,
    Plotr,
    Poke,
    Print,
    SymbolQuote,
    Rad,
    Randomize,
    Read,
    Release,
    Rem,
    Renum,
    Restore,
    Resume,
    Return,
    Run,
    Save,
    Sound,
    Speed,
    Stop,
    Symbol,
    Tag,
    Tagoff,
    Troff,
    Tron,
    Wait,
    Wend,
    While,
    Width,
    Window,
    Write,
    Zone,
    Di,
    Ei,
    Fill,
    Graphics,
    Mask,
    Frame,
    Cursor,
    UnusedE2,
    Erl,
    Fn,
    Spc,
    Step,
    Swap,
    UnusedE8,
    UnusedE9,
    Tab,
    Then,
    To,
    Using,
    GreaterThan,
    Equal,
    GreaterOrEqual,
    LessThan,
    NotEqual,
    LessThanOrEqual,
    Addition,
    SubstractionOrUnaryMinus,
    Multiplication,
    Division,
    Power,
    IntegerDivision,
    And,
    Mod,
    Or,
    Xor,
    AdditionalTokenMarker,
}

impl fmt::Display for BasicTokenNoPrefix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tag = match self {
            Self::Call => "CALL",
            Self::Print => "PRINT",
            Self::Rem => "REM",

            Self::SymbolQuote => "'",
            Self::StatementSeparator => ":",

            Self::CharSpace => " ",

            _ => unimplemented!("{:?}", self),
        };
        write!(f, "{}", tag)
    }
}

impl From<u8> for BasicTokenNoPrefix {
    fn from(val: u8) -> BasicTokenNoPrefix {
        val.try_into_BasicTokenNoPrefix().unwrap()
    }
}

impl BasicTokenNoPrefix {
    pub fn value(&self) -> u8 {
        (*self).into()
    }
}

#[derive(IntoPrimitive, CustomTryInto, Copy, Clone, PartialEq, Debug)]
#[repr(u8)]
pub enum BasicTokenPrefixed {
    Abs = 0,
    Asc,
    Atn,
    CharDollar,
    Cint,
    Cos,
    Creal,
    Exp,
    Fix,
    Fre,
    Inkey,
    Inp,
    Joy,
    Len,
    Log,
    Log10,
    LowerDollar,
    Peek,
    Remain,
    Sign,
    SpaceDollar,
    Sq,
    Sqr,
    StrDollar,
    Tan,
    Unt,
    UpperDollar,
    Val = 0x1d,

    Eof = 0x40,
    Err,
    Himem,
    InkeyDollar,
    Pi,
    Rnd,
    Time,
    Xpos,
    Ypos,
    Derr = 0x49,

    BinDollar = 0x71,
    DecDollar,
    HexDollar,
    Instr,
    LeftDollar,
    Max,
    Min,
    Pos,
    RightDollar,
    Round,
    StringDollar,
    Test,
    Teststr,
    CopycharDollar,
    Vpos = 0x7f,
}

impl fmt::Display for BasicTokenPrefixed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tag = match self {
            Self::Abs => "ABS",
            _ => unimplemented!("{}", self),
        };
        write!(f, "{}", tag)
    }
}

impl From<u8> for BasicTokenPrefixed {
    fn from(val: u8) -> BasicTokenPrefixed {
        val.try_into_BasicTokenPrefixed().unwrap()
    }
}

impl BasicTokenPrefixed {
    pub fn value(&self) -> u8 {
        (*self).into()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BasicValue {
    Integer(u8, u8),
    Float(u8, u8, u8, u8, u8),
    String(String),
}

impl BasicValue {
    pub fn new_integer(word: u16) -> BasicValue {
        BasicValue::Integer((word % 256) as u8, (word / 256) as u8)
    }

    pub fn new_string(value: &str) -> BasicValue {
        unimplemented!()
    }

    pub fn new_float(value: i32) -> BasicValue {
        unimplemented!()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            BasicValue::Integer(ref low, ref high) => vec![*low, *high],
            _ => unimplemented!(),
        }
    }

    /// Return the integer value when it is an integer
    pub fn as_integer(&self) -> Option<u16> {
        match self {
            Self::Integer(ref low, ref high) => Some((*low) as u16 + 256 * (*high) as u16),
            _ => None,
        }
    }

    pub fn int_hexdecimal_representation(&self) -> Option<String> {
        self.as_integer().map(|i| format!("&{:X}", i))
    }

    pub fn int_decimal_representation(&self) -> Option<String> {
        self.as_integer().map(|i| format!("{}", i))
    }
}

/// Represents any kind of token
#[derive(Debug, Clone, PartialEq)]
pub enum BasicToken {
    /// Simple tokens.
    SimpleToken(BasicTokenNoPrefix),
    /// Tokens prefixed by 0xff
    PrefixedToken(BasicTokenPrefixed),
    /// Encode a RSX call
    Rsx(String),
    /// Encode a variable set
    Variable(String, BasicValue),
    /// Encode a constant. The first field can only take ValueIntegerDecimal8bits, ValueIntegerDecimal16bits, ValueIntegerBinary16bits, ValueIntegerHexadecimal16bits
    Constant(BasicTokenNoPrefix, BasicValue),
    Comment(BasicTokenNoPrefix, Vec<u8>),
}

impl fmt::Display for BasicToken {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BasicToken::SimpleToken(ref tok) => {
                write!(f, "{}", tok.to_string())?;
            }
            BasicToken::PrefixedToken(ref tok) => {
                write!(f, "{}", tok.to_string())?;
            }
            BasicToken::Comment(ref tok, ref comment) => {
                write!(f, "{}", tok.to_string())?;
                write!(f, "{},", String::from_utf8(comment.to_vec()).unwrap())?;
            }
            BasicToken::Constant(ref kind, ref constant) => {
                let repr = match kind {
                    BasicTokenNoPrefix::ValueIntegerHexadecimal16bits => {
                        constant.int_hexdecimal_representation().unwrap()
                    }
                    BasicTokenNoPrefix::ValueIntegerDecimal16bits => {
                        constant.int_decimal_representation().unwrap()
                    }
                    _ => unimplemented!("{:?}", kind),
                };
                write!(f, "{}", repr)?;
            }
            _ => unimplemented!("{:?}", self),
        }

        Ok(())
    }
}

impl BasicToken {
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            BasicToken::SimpleToken(ref tok) => vec![tok.value()],

            BasicToken::PrefixedToken(ref tok) => vec![
                BasicTokenNoPrefix::AdditionalTokenMarker.value(),
                tok.value(),
            ],

            BasicToken::Rsx(ref name) => {
                let encoded_name = self.rsx_encoded_name().unwrap();
                let mut data = vec![BasicTokenNoPrefix::Pipe.value(), encoded_name.len() as u8];
                data.extend_from_slice(&encoded_name);
                data
            }

            BasicToken::Constant(ref kind, ref constant) => {
                let mut data = vec![kind.value()];
                data.extend_from_slice(&constant.as_bytes());
                data
            }

            BasicToken::Comment(ref comment_type, ref comment) => {
                let mut data = vec![comment_type.value()];
                data.extend_from_slice(&comment);
                data
            }

            _ => unimplemented!(),
        }
    }

    /// Returns the encoded version of the rsx name (bit 7 to 1 of last char)
    pub fn rsx_encoded_name(&self) -> Option<Vec<u8>> {
        match self {
            BasicToken::Rsx(ref name) => Some(Self::encode_string(name)),
            _ => None,
        }
    }

    pub fn variable_encoded_name(&self) -> Option<Vec<u8>> {
        match self {
            BasicToken::Variable(ref name, _) => Some(Self::encode_string(name)),
            _ => None,
        }
    }

    /// Encode a string by setting the bit 7 of last char. Returns a vector of bytes.
    fn encode_string(name: &str) -> Vec<u8> {
        let mut copy = name.as_bytes().to_vec();
        copy.pop(); // Remove \0
        copy.last_mut().map(|c| *c += 0b10000000); // Set bit 7 to last char
        copy
    }
}

#[cfg(test)]
mod test {
    use crate::basic::tokens::*;

    #[test]
    fn test_conversion() {
        assert_eq!(BasicTokenNoPrefix::Pipe.value(), 0x7c);
        assert_eq!(BasicTokenNoPrefix::After.value(), 0x80);

        assert_eq!(BasicTokenNoPrefix::Goto.value(), 0xa0);

        assert_eq!(BasicTokenNoPrefix::SymbolQuote.value(), 0xc0);

        assert_eq!(BasicTokenNoPrefix::Frame.value(), 0xe0);

        assert_eq!(BasicTokenNoPrefix::GreaterOrEqual.value(), 0xf0);

        assert_eq!(BasicTokenNoPrefix::Division.value(), 0xf7);

        assert_eq!(BasicTokenNoPrefix::from(0xf7), BasicTokenNoPrefix::Division);
    }
}
