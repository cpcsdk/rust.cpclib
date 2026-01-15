use std::fmt;

use crate::BasicError;

impl TryFrom<u8> for BasicTokenNoPrefix {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, String> {
        match value {
            5 => Err(format!("{value} is invalid")),
            _ => Ok(unsafe { std::mem::transmute::<u8, BasicTokenNoPrefix>(value) })
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
    CharOpenSquareBracket,    // [
    CharBackslash,            // \
    CharCloseSquareBracket,   // ]
    CharCaret,                // ^
    CharUnderscore,           // _
    CharBacktick,             // `

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

    CharOpenBrace,     // { (ASCII 123)
    Pipe = 0x7C,       // | (ASCII 124)
    CharCloseBrace,    // } (ASCII 125)
    CharTilde,         // ~ (ASCII 126)
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
    Not,
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

            '!' => BasicTokenNoPrefix::CharExclamation,
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
            '-' => BasicTokenNoPrefix::CharHyphen,
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
            '[' => BasicTokenNoPrefix::CharOpenSquareBracket,
            '\\' => BasicTokenNoPrefix::CharBackslash,
            ']' => BasicTokenNoPrefix::CharCloseSquareBracket,
            '^' => BasicTokenNoPrefix::CharCaret,
            '_' => BasicTokenNoPrefix::CharUnderscore,
            '`' => BasicTokenNoPrefix::CharBacktick,
            '{' => BasicTokenNoPrefix::CharOpenBrace,
            '|' => BasicTokenNoPrefix::Pipe,
            '}' => BasicTokenNoPrefix::CharCloseBrace,
            '~' => BasicTokenNoPrefix::CharTilde,

            '\t' => BasicTokenNoPrefix::CharTab,

            _ => unimplemented!("'{}'", c)
        }
    }
}

impl fmt::Display for BasicTokenNoPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // BASIC statement keywords
            Self::After => write!(f, "AFTER"),
            Self::Auto => write!(f, "AUTO"),
            Self::Border => write!(f, "BORDER"),
            Self::Call => write!(f, "CALL"),
            Self::Cat => write!(f, "CAT"),
            Self::Chain => write!(f, "CHAIN"),
            Self::Clear => write!(f, "CLEAR"),
            Self::Clg => write!(f, "CLG"),
            Self::Closein => write!(f, "CLOSEIN"),
            Self::Closeout => write!(f, "CLOSEOUT"),
            Self::Cls => write!(f, "CLS"),
            Self::Cont => write!(f, "CONT"),
            Self::Cursor => write!(f, "CURSOR"),
            Self::Data => write!(f, "DATA"),
            Self::Def => write!(f, "DEF"),
            Self::Defint => write!(f, "DEFINT"),
            Self::Defreal => write!(f, "DEFREAL"),
            Self::Defstr => write!(f, "DEFSTR"),
            Self::Deg => write!(f, "DEG"),
            Self::Delete => write!(f, "DELETE"),
            Self::Di => write!(f, "DI"),
            Self::Dim => write!(f, "DIM"),
            Self::Draw => write!(f, "DRAW"),
            Self::Drawr => write!(f, "DRAWR"),
            Self::Edit => write!(f, "EDIT"),
            Self::Ei => write!(f, "EI"),
            Self::Else => write!(f, "ELSE"),
            Self::End => write!(f, "END"),
            Self::Ent => write!(f, "ENT"),
            Self::Env => write!(f, "ENV"),
            Self::Erase => write!(f, "ERASE"),
            Self::Erl => write!(f, "ERL"),
            Self::Error => write!(f, "ERROR"),
            Self::Every => write!(f, "EVERY"),
            Self::Fill => write!(f, "FILL"),
            Self::Fn => write!(f, "FN"),
            Self::For => write!(f, "FOR"),
            Self::Frame => write!(f, "FRAME"),
            Self::Gosub => write!(f, "GOSUB"),
            Self::Goto => write!(f, "GOTO"),
            Self::Graphics => write!(f, "GRAPHICS"),
            Self::If => write!(f, "IF"),
            Self::Ink => write!(f, "INK"),
            Self::Input => write!(f, "INPUT"),
            Self::Key => write!(f, "KEY"),
            Self::Let => write!(f, "LET"),
            Self::Line => write!(f, "LINE"),
            Self::List => write!(f, "LIST"),
            Self::Load => write!(f, "LOAD"),
            Self::Locate => write!(f, "LOCATE"),
            Self::Mask => write!(f, "MASK"),
            Self::Memory => write!(f, "MEMORY"),
            Self::Merge => write!(f, "MERGE"),
            Self::MidDollar => write!(f, "MID$"),
            Self::Mode => write!(f, "MODE"),
            Self::Move => write!(f, "MOVE"),
            Self::Mover => write!(f, "MOVER"),
            Self::New => write!(f, "NEW"),
            Self::Next => write!(f, "NEXT"),
            Self::On => write!(f, "ON"),
            Self::OnBreak => write!(f, "ON BREAK"),
            Self::OnErrorGoto => write!(f, "ON ERROR GOTO"),
            Self::Openin => write!(f, "OPENIN"),
            Self::Openout => write!(f, "OPENOUT"),
            Self::Origin => write!(f, "ORIGIN"),
            Self::Out => write!(f, "OUT"),
            Self::Paper => write!(f, "PAPER"),
            Self::Pen => write!(f, "PEN"),
            Self::Plot => write!(f, "PLOT"),
            Self::Plotr => write!(f, "PLOTR"),
            Self::Poke => write!(f, "POKE"),
            Self::Print => write!(f, "PRINT"),
            Self::Rad => write!(f, "RAD"),
            Self::Randomize => write!(f, "RANDOMIZE"),
            Self::Read => write!(f, "READ"),
            Self::Release => write!(f, "RELEASE"),
            Self::Rem => write!(f, "REM"),
            Self::Renum => write!(f, "RENUM"),
            Self::Restore => write!(f, "RESTORE"),
            Self::Resume => write!(f, "RESUME"),
            Self::Return => write!(f, "RETURN"),
            Self::Run => write!(f, "RUN"),
            Self::Save => write!(f, "SAVE"),
            Self::Sound => write!(f, "SOUND"),
            Self::Spc => write!(f, "SPC"),
            Self::Speed => write!(f, "SPEED"),
            Self::Sq => write!(f, "SQ"),
            Self::Step => write!(f, "STEP"),
            Self::Stop => write!(f, "STOP"),
            Self::Swap => write!(f, "SWAP"),
            Self::Symbol => write!(f, "SYMBOL"),
            Self::Tab => write!(f, "TAB"),
            Self::Tag => write!(f, "TAG"),
            Self::Tagoff => write!(f, "TAGOFF"),
            Self::Then => write!(f, "THEN"),
            Self::To => write!(f, "TO"),
            Self::Troff => write!(f, "TROFF"),
            Self::Tron => write!(f, "TRON"),
            Self::Using => write!(f, "USING"),
            Self::Wait => write!(f, "WAIT"),
            Self::Wend => write!(f, "WEND"),
            Self::While => write!(f, "WHILE"),
            Self::Width => write!(f, "WIDTH"),
            Self::Window => write!(f, "WINDOW"),
            Self::Write => write!(f, "WRITE"),
            Self::Zone => write!(f, "ZONE"),
            
            // Operators
            Self::GreaterThan => write!(f, ">"),
            Self::Equal => write!(f, "="),
            Self::GreaterOrEqual => write!(f, ">="),
            Self::LessThan => write!(f, "<"),
            Self::NotEqual => write!(f, "<>"),
            Self::LessThanOrEqual => write!(f, "<="),
            Self::Addition => write!(f, "+"),
            Self::SubstractionOrUnaryMinus => write!(f, "-"),
            Self::Multiplication => write!(f, "*"),
            Self::Division => write!(f, "/"),
            Self::Power => write!(f, "^"),
            Self::IntegerDivision => write!(f, "\\"),
            Self::And => write!(f, " AND "),
            Self::Not => write!(f, " NOT "),
            Self::Mod => write!(f, " MOD "),
            Self::Or => write!(f, " OR "),
            Self::Xor => write!(f, " XOR "),

            Self::SymbolQuote => write!(f, "'"),
            Self::StatementSeparator => write!(f, ":"),

            Self::EndOfTokenisedLine => Ok(()),
            
            // Constant numbers 0-10
            Self::ConstantNumber0 => write!(f, "0"),
            Self::ConstantNumber1 => write!(f, "1"),
            Self::ConstantNumber2 => write!(f, "2"),
            Self::ConstantNumber3 => write!(f, "3"),
            Self::ConstantNumber4 => write!(f, "4"),
            Self::ConstantNumber5 => write!(f, "5"),
            Self::ConstantNumber6 => write!(f, "6"),
            Self::ConstantNumber7 => write!(f, "7"),
            Self::ConstantNumber8 => write!(f, "8"),
            Self::ConstantNumber9 => write!(f, "9"),
            Self::ConstantNumber10 => write!(f, "10"),
            
            // Value tokens - these should not appear in source reconstruction
            // They contain binary data that needs special handling
            Self::ValueQuotedString => write!(f, "\""),
            
            Self::ValueIntegerDecimal8bits |
            Self::ValueIntegerDecimal16bits |
            Self::ValueIntegerBinary16bits |
            Self::ValueIntegerHexadecimal16bits |
            Self::ValueFloatingPoint |
            Self::LineMemoryAddressPointer |
            Self::LineNumber => {
                // These tokens should be followed by data bytes that need to be decoded
                // For now, just indicate their presence
                write!(f, "<value:{:?}>", self)
            },

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
            _ => Ok(unsafe { std::mem::transmute::<u8, BasicTokenPrefixed>(value) })
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
            Self::Asc => "ASC",
            Self::Atn => "ATN",
            Self::ChrDollar => "CHR$",
            Self::Cint => "CINT",
            Self::Cos => "COS",
            Self::Creal => "CREAL",
            Self::Exp => "EXP",
            Self::Fix => "FIX",
            Self::Fre => "FRE",
            Self::Inkey => "INKEY",
            Self::Inp => "INP",
            Self::Int => "INT",
            Self::Joy => "JOY",
            Self::Len => "LEN",
            Self::Log => "LOG",
            Self::Log10 => "LOG10",
            Self::LowerDollar => "LOWER$",
            Self::Peek => "PEEK",
            Self::Remain => "REMAIN",
            Self::Sign => "SIGN",
            Self::Sin => "SIN",
            Self::SpaceDollar => "SPACE$",
            Self::Sq => "SQ",
            Self::Sqr => "SQR",
            Self::StrDollar => "STR$",
            Self::Tan => "TAN",
            Self::Unt => "UNT",
            Self::UpperDollar => "UPPER$",
            Self::Val => "VAL",
            Self::Eof => "EOF",
            Self::Err => "ERR",
            Self::Himem => "HIMEM",
            Self::InkeyDollar => "INKEY$",
            Self::Pi => "PI",
            Self::Rnd => "RND",
            Self::Time => "TIME",
            Self::Xpos => "XPOS",
            Self::Ypos => "YPOS",
            Self::Derr => "DERR",
            Self::BinDollar => "BIN$",
            Self::DecDollar => "DEC$",
            Self::HexDollar => "HEX$",
            Self::Instr => "INSTR",
            Self::LeftDollar => "LEFT$",
            Self::Max => "MAX",
            Self::Min => "MIN",
            Self::Pos => "POS",
            Self::RightDollar => "RIGHT$",
            Self::Round => "ROUND",
            Self::StringDollar => "STRING$",
            Self::Test => "TEST",
            Self::Teststr => "TESTSTR",
            Self::CopycharDollar => "COPYCHR$",
            Self::Vpos => "VPOS",
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

/// Amstrad BASIC floating-point number representation
/// Stores a 5-byte floating-point number in the format used by Locomotive BASIC
/// Format: mantissa (4 bytes) + exponent (1 byte)
/// - Bytes 0-3: 32-bit mantissa (normalized, MSB always 1, implied)
/// - Byte 3, bit 7: sign (1=negative, 0=positive)
/// - Byte 4: exponent (biased by 128, 0=zero, 1-127=negative, 128-255=positive)
#[derive(Debug, Clone, PartialEq)]
pub struct BasicFloat {
    /// Raw 5-byte representation: [mantissa_byte0, mantissa_byte1, mantissa_byte2, mantissa_byte3, exponent]
    bytes: [u8; 5]
}

impl BasicFloat {
    /// Create a BasicFloat from 5 bytes in Amstrad BASIC format
    pub fn from_bytes(bytes: [u8; 5]) -> Self {
        Self { bytes }
    }
    
    /// Create a BasicFloat from individual byte values
    pub fn new(b0: u8, b1: u8, b2: u8, b3: u8, b4: u8) -> Self {
        Self { bytes: [b0, b1, b2, b3, b4] }
    }
    
    /// Get the raw bytes
    pub fn as_bytes(&self) -> [u8; 5] {
        self.bytes
    }
    
    /// Convert to f64
    pub fn to_f64(&self) -> f64 {
        let exponent = self.bytes[4];
        
        // Special case: exponent 0 means the number is zero
        if exponent == 0 {
            return 0.0;
        }
        
        // Extract sign from bit 7 of byte 3
        let sign_bit = (self.bytes[3] & 0x80) != 0;
        let sign = if sign_bit { -1.0 } else { 1.0 };
        
        // Extract mantissa (32 bits)
        // The mantissa is normalized, so bit 31 is always 1 (implied/reconstructed)
        // Byte 3 bits 6-0: mantissa bits 30-24 (bit 7 is used for sign, bit 31 is implied 1)
        // Byte 2: mantissa bits 23-16
        // Byte 1: mantissa bits 15-8
        // Byte 0: mantissa bits 7-0
        let mantissa_bits = 
            (1u32 << 31) |                            // bit 31 (implied leading 1)
            (((self.bytes[3] & 0x7F) as u32) << 24) | // bits 30-24
            ((self.bytes[2] as u32) << 16) |          // bits 23-16
            ((self.bytes[1] as u32) << 8) |           // bits 15-8
            (self.bytes[0] as u32);                   // bits 7-0
        
        // Convert mantissa to floating point
        // The mantissa represents a fraction in the range [0.5, 1.0) after dividing by 2^32
        let mantissa_value = (mantissa_bits as f64) / (1u64 << 32) as f64;
        
        // Calculate the actual exponent (biased by 128)
        let actual_exponent = (exponent as i16) - 128;
        
        // Result = sign × mantissa × 2^exponent
        sign * mantissa_value * 2f64.powi(actual_exponent as i32)
    }
    
    /// Create a BasicFloat from an f64 value
    pub fn from_f64(value: f64) -> Self {
        Self::try_from(value).unwrap_or_else(|_| Self::new(0, 0, 0, 0, 0))
    }
}

impl TryFrom<f64> for BasicFloat {
    type Error = BasicError;
    
    /// Convert f64 to Amstrad BASIC floating-point format
    /// Implementation from https://github.com/EdouardBERGE/rasm/blob/master/rasm.c#L2295
    fn try_from(nb: f64) -> Result<Self, Self::Error> {
        let mut bits = [false; 32];
        let mut res = [0; 5];

        let (is_pos, nb) = if nb >= 0f64 { (true, nb) } else { (false, -nb) };

        let deci = nb.trunc() as u64;
        let _fract = nb.fract();

        let mut bitpos = 0;
        let mut exp: i32 = 0;
        let mut mantissa: u64;
        let mut mask: u64;

        if deci >= 1 {
            // nb is >=1
            mask = 0x80000000;

            // search for the first (from the left) bit to 1
            while (deci & mask) == 0 {
                mask /= 2;
            }
            // count the number of remaining bits
            while mask > 0 {
                exp += 1;
                mask /= 2;
            }
            // build the mantissa part of the decimal value
            mantissa = (nb * 2f64.powi(32 - exp) + 0.5) as _;
            if (mantissa & 0xFF00000000) != 0 {
                mantissa = 0xFFFFFFFF
            };

            mask = 0x80000000;
            while mask != 0 {
                bits[bitpos] = (mantissa & mask) != 0;
                bitpos += 1;
                mask /= 2;
            }
        }
        else {
            // <1
            if nb == 0.0 {
                exp = -128;
            }
            else {
                mantissa = (nb * 4294967296.0 + 0.5) as _; // as v is ALWAYS <1.0 we never reach the 32 bits maximum
                if (mantissa & 0xFF00000000) != 0 {
                    mantissa = 0xFFFFFFFF;
                }

                mask = 0x80000000;
                // find first significant bit of fraction part
                while (mantissa & mask) == 0 {
                    mask /= 2;
                    exp -= 1;
                }

                mantissa = (nb * 2.0f64.powi(32 - exp) + 0.5) as _; // as v is ALWAYS <1.0 we never reach the 32 bits maximum
                if (mantissa & 0xFF00000000) != 0 {
                    mantissa = 0xFFFFFFFF;
                }

                mask = 0x80000000;
                while mask != 0 {
                    bits[bitpos] = (mantissa & mask) != 0;
                    bitpos += 1;
                    mask /= 2;
                }
            }
        }

        {
            // generate the mantissa bytes
            let mut ib: usize = 3;
            let mut ibb: u8 = 0x80;
            for (j, &bit) in bits.iter().enumerate().take(bitpos) {
                if bit {
                    res[ib] |= ibb;
                }
                ibb /= 2;
                if ibb == 0 {
                    ibb = 0x80;
                    if ib != 0 {
                        ib -= 1
                    }
                    else {
                        debug_assert!(j == bitpos - 1);
                    }
                }
            }
        }

        {
            // generate the exponent
            exp += 128;
            if !(0..=255).contains(&exp) {
                return Err(BasicError::ExponentOverflow);
            }
            else {
                res[4] = exp as _;
            }
        }

        {
            // Generate the sign bit
            if is_pos {
                res[3] &= 0x7F;
            }
            else {
                res[3] |= 0x80;
            }
        }

        Ok(BasicFloat::from_bytes(res))
    }
}

impl TryFrom<&str> for BasicFloat {
    type Error = BasicError;
    
    /// Parse a decimal string directly to Amstrad BASIC floating-point format
    /// Converts to f64 first for correct algorithm, then uses TryFrom<f64>
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        // Parse the string as f64 to get the correct value
        // This ensures we use the proven conversion algorithm
        let value = s.trim().parse::<f64>()
            .map_err(|_| BasicError::InvalidFloat)?;
        
        // Use the TryFrom<f64> implementation which has the correct algorithm
        BasicFloat::try_from(value)
    }
}

impl fmt::Display for BasicFloat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = self.to_f64();
        
        // Special case for zero
        if value == 0.0 {
            return write!(f, "0");
        }
        
        // Format to 9 decimal places
        let formatted = format!("{:.9}", value);
        
        // Remove trailing zeros and unnecessary decimal point
        let trimmed = formatted
            .trim_end_matches('0')
            .trim_end_matches('.');
        
        write!(f, "{}", trimmed)
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Encode a Basic value
pub enum BasicValue {
    /// 16bits integer value in saved order
    Integer(u8, u8),
    /// 5bytes float value in Amstrad BASIC format
    Float(BasicFloat),
    /// String
    String(String)
}

#[allow(missing_docs)]
impl BasicValue {
    pub fn new_integer(word: i16) -> Self {
        let word: u16 = i16::cast_unsigned(word);
        BasicValue::Integer((word % 256) as u8, (word / 256) as u8)
    }

    pub fn new_integer_by_bytes(low: u8, high: u8) -> Self {
        BasicValue::Integer(low, high)
    }

    pub fn new_string(_value: &str) -> Self {
        unimplemented!()
    }

    pub fn new_float(value: f64) -> Self {
        BasicValue::Float(BasicFloat::from_f64(value))
    }
    
    pub fn new_float_by_bytes(b0: u8, b1: u8, b2: u8, b3: u8, b4: u8) -> Self {
        BasicValue::Float(BasicFloat::new(b0, b1, b2, b3, b4))
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            Self::Integer(low, high) => vec![*low, *high],
            Self::Float(f) => f.as_bytes().to_vec(),
            _ => unimplemented!()
        }
    }

    /// Return the integer value when it is an integer
    pub fn as_integer(&self) -> Option<u16> {
        match self {
            Self::Integer(low, high) => Some(u16::from(*low) + 256 * u16::from(*high)),
            _ => None
        }
    }
    
    /// Return the float value when it is a float
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(f.to_f64()),
            _ => None
        }
    }

    pub fn int_hexdecimal_representation(&self) -> Option<String> {
        self.as_integer().map(|i| format!("&{i:X}"))
    }

    pub fn int_decimal_representation(&self) -> Option<String> {
        self.as_integer().map(|i| format!("{i}"))
    }
    
    pub fn int_binary_representation(&self) -> Option<String> {
        self.as_integer().map(|i| format!("&X{i:b}"))
    }
    
    pub fn float_representation(&self) -> Option<String> {
        self.as_float().map(|f| {
            // Format according to BASIC rules
            let basic_float = if let Self::Float(bf) = self {
                bf.clone()
            } else {
                BasicFloat::from_f64(f)
            };
            basic_float.to_string()
        })
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
            BasicToken::SimpleToken(tok) => {
                write!(f, "{tok}")?;
            },
            BasicToken::PrefixedToken(tok) => {
                write!(f, "{tok}")?;
            },
            BasicToken::Comment(tok, comment) => {
                write!(f, "{tok}")?;
                write!(f, "{}", String::from_utf8_lossy(comment))?;
            },
            BasicToken::Constant(kind, constant) => {
                let repr = match kind {
                    BasicTokenNoPrefix::ValueIntegerHexadecimal16bits => {
                        constant.int_hexdecimal_representation().unwrap()
                    },
                    BasicTokenNoPrefix::ValueIntegerDecimal16bits |
                    BasicTokenNoPrefix::ValueIntegerDecimal8bits => {
                        constant.int_decimal_representation().unwrap()
                    },
                    BasicTokenNoPrefix::ValueIntegerBinary16bits => {
                        constant.int_binary_representation().unwrap()
                    },
                    BasicTokenNoPrefix::ValueFloatingPoint => {
                        constant.float_representation().unwrap_or_else(|| "<float?>".to_string())
                    },
                    _ => format!("<const:{:?}>", kind)
                };
                write!(f, "{repr}")?;
            },
            BasicToken::Variable(name, _value) => {
                // Just write the variable name
                write!(f, "{name}")?;
            },
            BasicToken::Rsx(name) => {
                write!(f, "|{name}")?;
            },
        }

        Ok(())
    }
}

#[allow(missing_docs)]
impl BasicToken {
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            BasicToken::SimpleToken(tok) => vec![tok.value()],

            BasicToken::PrefixedToken(tok) => {
                vec![
                    BasicTokenNoPrefix::AdditionalTokenMarker.value(),
                    tok.value(),
                ]
            },

            BasicToken::Rsx(_name) => {
                let encoded_name = self.rsx_encoded_name().unwrap();
                let mut data = vec![BasicTokenNoPrefix::Pipe.value(), encoded_name.len() as u8];
                data.extend_from_slice(&encoded_name);
                data
            },

            BasicToken::Constant(kind, constant) => {
                let mut data = vec![kind.value()];
                data.extend_from_slice(&constant.as_bytes());
                data
            },

            BasicToken::Comment(comment_type, comment) => {
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
            BasicToken::Rsx(name) => Some(Self::encode_string(name)),
            _ => None
        }
    }

    pub fn variable_encoded_name(&self) -> Option<Vec<u8>> {
        match self {
            BasicToken::Variable(name, _) => Some(Self::encode_string(name)),
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
