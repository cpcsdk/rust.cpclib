use std::fmt;

impl TryFrom<u8> for BasicTokenNoPrefix {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, String> {
        match value {
            5 => Err(format!("{value} is invalid")),
            _ => Ok(unsafe { std::mem::transmute(value) })
        }
    }
}

impl From<BasicTokenNoPrefix> for u8 {
    fn from(val: BasicTokenNoPrefix) -> Self {
        val as u8
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum BasicTokenNoPrefix {
    EndOfTokenisedLine = 0,

    StatementSeparator = 1,

    IntegerVariableDefinition = 2,
    StringVariableDefinition = 3,
    FloatingPointVariableDefinition = 4,

    VarUnknown1 = 6,
    VarUnknown2 = 7,
    VarUnknown3 = 8,
    CharTab = 9, // XXX Not sure of that
    VarUnknown5 = 0xA,

    VariableDefinition1 = 0xB,
    VariableDefinition2 = 0xC,
    VariableDefinition3 = 0xD,

    ConstantNumber0 = 0x0E,
    ConstantNumber1 = 0x0F,
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

    ValueIntegerDecimal16bits = 0x1A,
    ValueIntegerBinary16bits = 0x1B,
    ValueIntegerHexadecimal16bits = 0x1C,

    LineMemoryAddressPointer = 0x1D,
    LineNumber = 0x1E,

    ValueFloatingPoint = 0x1F,

    CharSpace = 0x20,
    CharExclamation = 0x21,

    ValueQuotedString = 0x22,

    CharNumber,
    CharDollar,
    CharPerCent,
    CharAmpersand,
    CharSingleQuote,
    CharOpenParenthesis,
    CharCloseParenthesis,
    CharAsterix,
    CharPlus,
    CharComma,
    CharHyphen,
    CharDot,
    CharSlash,
    Char0,
    Char1,
    Char2,
    Char3,
    Char4,
    Char5,
    Char6,
    Char7,
    Char8,
    Char9,
    CharColon,
    CharSemiColon,
    CharLess,
    CharEquals,
    CharGreater,
    CharQuestionMark,
    CharAt,

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

    Pipe = 0x7C,

    Unused7d = 0x7D,
    Unused7e = 0x7E,
    Unused7f = 0x7F,

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
    AdditionalTokenMarker
}

impl From<char> for BasicTokenNoPrefix {
    fn from(c: char) -> Self {
        match c {
            // ':' => (BasicTokenNoPrefix::StatementSeparator),
            ' ' => BasicTokenNoPrefix::CharSpace,
            'A' => BasicTokenNoPrefix::CharUpperA,
            'B' => BasicTokenNoPrefix::CharUpperB,
            'C' => BasicTokenNoPrefix::CharUpperC,
            'D' => BasicTokenNoPrefix::CharUpperD,
            'E' => BasicTokenNoPrefix::CharUpperE,
            'F' => BasicTokenNoPrefix::CharUpperF,
            'G' => BasicTokenNoPrefix::CharUpperG,
            'H' => BasicTokenNoPrefix::CharUpperH,
            'I' => BasicTokenNoPrefix::CharUpperI,
            'J' => BasicTokenNoPrefix::CharUpperJ,
            'K' => BasicTokenNoPrefix::CharUpperK,
            'L' => BasicTokenNoPrefix::CharUpperL,
            'M' => BasicTokenNoPrefix::CharUpperM,
            'N' => BasicTokenNoPrefix::CharUpperN,
            'O' => BasicTokenNoPrefix::CharUpperO,
            'P' => BasicTokenNoPrefix::CharUpperP,
            'Q' => BasicTokenNoPrefix::CharUpperQ,
            'R' => BasicTokenNoPrefix::CharUpperR,
            'S' => BasicTokenNoPrefix::CharUpperS,
            'T' => BasicTokenNoPrefix::CharUpperT,
            'U' => BasicTokenNoPrefix::CharUpperU,
            'V' => BasicTokenNoPrefix::CharUpperV,
            'W' => BasicTokenNoPrefix::CharUpperW,
            'X' => BasicTokenNoPrefix::CharUpperX,
            'Y' => BasicTokenNoPrefix::CharUpperY,
            'Z' => BasicTokenNoPrefix::CharUpperZ,
            'a' => BasicTokenNoPrefix::CharLowerA,
            'b' => BasicTokenNoPrefix::CharLowerB,
            'c' => BasicTokenNoPrefix::CharLowerC,
            'd' => BasicTokenNoPrefix::CharLowerD,
            'e' => BasicTokenNoPrefix::CharLowerE,
            'f' => BasicTokenNoPrefix::CharLowerF,
            'g' => BasicTokenNoPrefix::CharLowerG,
            'h' => BasicTokenNoPrefix::CharLowerH,
            'i' => BasicTokenNoPrefix::CharLowerI,
            'j' => BasicTokenNoPrefix::CharLowerJ,
            'k' => BasicTokenNoPrefix::CharLowerK,
            'l' => BasicTokenNoPrefix::CharLowerL,
            'm' => BasicTokenNoPrefix::CharLowerM,
            'n' => BasicTokenNoPrefix::CharLowerN,
            'o' => BasicTokenNoPrefix::CharLowerO,
            'p' => BasicTokenNoPrefix::CharLowerP,
            'q' => BasicTokenNoPrefix::CharLowerQ,
            'r' => BasicTokenNoPrefix::CharLowerR,
            's' => BasicTokenNoPrefix::CharLowerS,
            't' => BasicTokenNoPrefix::CharLowerT,
            'u' => BasicTokenNoPrefix::CharLowerU,
            'v' => BasicTokenNoPrefix::CharLowerV,
            'w' => BasicTokenNoPrefix::CharLowerW,
            'x' => BasicTokenNoPrefix::CharLowerX,
            'y' => BasicTokenNoPrefix::CharLowerY,
            'z' => BasicTokenNoPrefix::CharLowerZ,

            '#' => BasicTokenNoPrefix::CharNumber,
            '$' => BasicTokenNoPrefix::CharDollar,
            '%' => BasicTokenNoPrefix::CharPerCent,
            '&' => BasicTokenNoPrefix::CharAmpersand,
            '\'' => BasicTokenNoPrefix::CharSingleQuote,
            '(' => BasicTokenNoPrefix::CharOpenParenthesis,
            ')' => BasicTokenNoPrefix::CharCloseParenthesis,
            '*' => BasicTokenNoPrefix::CharAsterix,
            '+' => BasicTokenNoPrefix::CharPlus,
            ',' => BasicTokenNoPrefix::CharComma,
            '_' => BasicTokenNoPrefix::CharHyphen,
            '.' => BasicTokenNoPrefix::CharDot,
            '/' => BasicTokenNoPrefix::CharSlash,
            '0' => BasicTokenNoPrefix::Char0,
            '1' => BasicTokenNoPrefix::Char1,
            '2' => BasicTokenNoPrefix::Char2,
            '3' => BasicTokenNoPrefix::Char3,
            '4' => BasicTokenNoPrefix::Char4,
            '5' => BasicTokenNoPrefix::Char5,
            '6' => BasicTokenNoPrefix::Char6,
            '7' => BasicTokenNoPrefix::Char7,
            '8' => BasicTokenNoPrefix::Char8,
            '9' => BasicTokenNoPrefix::Char9,
            ':' => BasicTokenNoPrefix::CharColon,
            ';' => BasicTokenNoPrefix::CharSemiColon,
            '<' => BasicTokenNoPrefix::CharLess,
            '=' => BasicTokenNoPrefix::CharEquals,
            '>' => BasicTokenNoPrefix::CharGreater,
            '?' => BasicTokenNoPrefix::CharQuestionMark,
            '@' => BasicTokenNoPrefix::CharAt,

            '\t' => BasicTokenNoPrefix::CharTab,

            _ => unimplemented!("'{}'", c)
        }
    }
}

impl fmt::Display for BasicTokenNoPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Call => write!(f, "CALL"),
            Self::For => write!(f, "FOR"),
            Self::Load => write!(f, "LOAD"),
            Self::Memory => write!(f, "MEMORY"),
            Self::Print => write!(f, "PRINT"),
            Self::Rem => write!(f, "REM"),

            Self::SymbolQuote => write!(f, "'"),
            Self::StatementSeparator => write!(f, ":"),

            Self::EndOfTokenisedLine => Ok(()),

            _ => {
                let c = (*self as u8) as char;
                match c {
                    ' '..='z' => write!(f, "{c}"),

                    _ => unimplemented!("{:?}", self)
                }
            }
        }
    }
}

impl BasicTokenNoPrefix {
    /// Returns the 8bit code that represents the token
    pub fn value(self) -> u8 {
        self.into()
    }
}

impl TryFrom<u8> for BasicTokenPrefixed {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, String> {
        match value {
            0x1E..0x40 | 0x50..0x71 | 0x80.. => Err(format!("{value} is invalid")),
            _ => Ok(unsafe { std::mem::transmute(value) })
        }
    }
}

impl From<BasicTokenPrefixed> for u8 {
    fn from(val: BasicTokenPrefixed) -> Self {
        val as u8
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum BasicTokenPrefixed {
    Abs = 0,
    Asc,
    Atn,
    ChrDollar,
    Cint,
    Cos,
    Creal,
    Exp,
    Fix,
    Fre,
    Inkey,
    Inp,
    Int,
    Joy,
    Len,
    Log,
    Log10,
    LowerDollar,
    Peek,
    Remain,
    Sign,
    Sin,
    SpaceDollar,
    Sq,
    Sqr,
    StrDollar,
    Tan,
    Unt,
    UpperDollar,
    Val = 0x1D,

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
    Vpos = 0x7F
}

impl fmt::Display for BasicTokenPrefixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag = match self {
            Self::Abs => "ABS",
            _ => unimplemented!("{}", self)
        };
        write!(f, "{tag}")
    }
}

// impl From<u8> for BasicTokenPrefixed {
// fn from(val: u8) -> BasicTokenPrefixed {
// val.try_into().unwrap()
// }
// }

impl BasicTokenPrefixed {
    /// Returns the 8bits code that represents the prefixed token
    pub fn value(self) -> u8 {
        self.into()
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Encode a Basic value
pub enum BasicValue {
    /// 16bits integer value in saved order
    Integer(u8, u8),
    /// 5bytes float value in saved order
    Float(u8, u8, u8, u8, u8),
    /// String
    String(String)
}

#[allow(missing_docs)]
impl BasicValue {
    pub fn new_integer(word: i16) -> Self {
        let word: u16 = unsafe { i16::cast_unsigned(word) };
        BasicValue::Integer((word % 256) as u8, (word / 256) as u8)
    }

    pub fn new_integer_by_bytes(low: u8, high: u8) -> Self {
        BasicValue::Integer(low, high)
    }

    pub fn new_string(_value: &str) -> Self {
        unimplemented!()
    }

    pub fn new_float(_value: i32) -> Self {
        unimplemented!()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            Self::Integer(ref low, ref high) => vec![*low, *high],
            _ => unimplemented!()
        }
    }

    /// Return the integer value when it is an integer
    pub fn as_integer(&self) -> Option<u16> {
        match self {
            Self::Integer(ref low, ref high) => Some(u16::from(*low) + 256 * u16::from(*high)),
            _ => None
        }
    }

    pub fn int_hexdecimal_representation(&self) -> Option<String> {
        self.as_integer().map(|i| format!("&{i:X}"))
    }

    pub fn int_decimal_representation(&self) -> Option<String> {
        self.as_integer().map(|i| format!("{i}"))
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
    /// Encode a comment
    Comment(BasicTokenNoPrefix, Vec<u8>)
}

impl fmt::Display for BasicToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BasicToken::SimpleToken(ref tok) => {
                write!(f, "{tok}")?;
            },
            BasicToken::PrefixedToken(ref tok) => {
                write!(f, "{tok}")?;
            },
            BasicToken::Comment(ref tok, ref comment) => {
                write!(f, "{tok}")?;
                write!(f, "{},", String::from_utf8(comment.to_vec()).unwrap())?;
            },
            BasicToken::Constant(ref kind, ref constant) => {
                let repr = match kind {
                    BasicTokenNoPrefix::ValueIntegerHexadecimal16bits => {
                        constant.int_hexdecimal_representation().unwrap()
                    },
                    BasicTokenNoPrefix::ValueIntegerDecimal16bits => {
                        constant.int_decimal_representation().unwrap()
                    },
                    _ => unimplemented!("{:?}", kind)
                };
                write!(f, "{repr}")?;
            },
            _ => unimplemented!("{:?}", self)
        }

        Ok(())
    }
}

#[allow(missing_docs)]
impl BasicToken {
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            BasicToken::SimpleToken(ref tok) => vec![tok.value()],

            BasicToken::PrefixedToken(ref tok) => {
                vec![
                    BasicTokenNoPrefix::AdditionalTokenMarker.value(),
                    tok.value(),
                ]
            },

            BasicToken::Rsx(ref _name) => {
                let encoded_name = self.rsx_encoded_name().unwrap();
                let mut data = vec![BasicTokenNoPrefix::Pipe.value(), encoded_name.len() as u8];
                data.extend_from_slice(&encoded_name);
                data
            },

            BasicToken::Constant(ref kind, ref constant) => {
                let mut data = vec![kind.value()];
                data.extend_from_slice(&constant.as_bytes());
                data
            },

            BasicToken::Comment(ref comment_type, ref comment) => {
                let mut data = vec![comment_type.value()];
                data.extend_from_slice(comment);
                data
            },

            _ => unimplemented!()
        }
    }

    /// Returns the encoded version of the rsx name (bit 7 to 1 of last char)
    pub fn rsx_encoded_name(&self) -> Option<Vec<u8>> {
        match self {
            BasicToken::Rsx(ref name) => Some(Self::encode_string(name)),
            _ => None
        }
    }

    pub fn variable_encoded_name(&self) -> Option<Vec<u8>> {
        match self {
            BasicToken::Variable(ref name, _) => Some(Self::encode_string(name)),
            _ => None
        }
    }

    /// Encode a string by setting the bit 7 of last char. Returns a vector of bytes.
    fn encode_string(name: &str) -> Vec<u8> {
        let mut copy = name.as_bytes().to_vec();
        copy.pop(); // Remove \0
        if let Some(c) = copy.last_mut() {
            *c += 0b1000_0000; // Set bit 7 to last char
        }
        copy
    }
}

#[cfg(test)]
mod test {
    use std::convert::TryInto;

    use crate::tokens::*;

    #[test]
    fn test_conversion() {
        assert_eq!(BasicTokenNoPrefix::Pipe.value(), 0x7C);
        assert_eq!(BasicTokenNoPrefix::After.value(), 0x80);

        assert_eq!(BasicTokenNoPrefix::Goto.value(), 0xA0);

        assert_eq!(BasicTokenNoPrefix::SymbolQuote.value(), 0xC0);

        assert_eq!(BasicTokenNoPrefix::Frame.value(), 0xE0);

        assert_eq!(BasicTokenNoPrefix::GreaterOrEqual.value(), 0xF0);

        assert_eq!(BasicTokenNoPrefix::Division.value(), 0xF7);

        let token: BasicTokenNoPrefix = 0xF7.try_into().unwrap();
        assert_eq!(token, BasicTokenNoPrefix::Division);
    }
}
