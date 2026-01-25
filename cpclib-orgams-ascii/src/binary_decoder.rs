//! Winnow-based parser for Orgams binary format
//!
//! This parser uses winnow combinators and models semantic structures.

use std::borrow::Cow;
use std::ops::Deref;
use std::fmt::{Display, Formatter};

use cpclib_common::smallvec::SmallVec;
use cpclib_common::itertools::Itertools;
use cpclib_common::parse;
use cpclib_common::winnow::combinator::{cut_err, eof, preceded, repeat, terminated, trace};
use cpclib_common::winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use cpclib_common::winnow::stream::Offset;
use cpclib_common::winnow::{self, LocatingSlice};


use cpclib_tokens::opcode_table::{TABINSTR, TABINSTRCB, TABINSTRDD, TABINSTRDDCB, TABINSTRFD, TABINSTRFDCB, TABINSTRED};

use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::prelude::*;
use winnow::token::{any, literal, rest, take};

pub type Input<'a> = LocatingSlice<&'a [u8]>;
pub type OrgamsParseResult<T> = ModalResult<T>;


fn byte2code_naive(bytes: &[u8]) -> String {
    let mut results: Vec<Cow<'static, str>> = Vec::with_capacity(bytes.len());
    let mut was_7f = false;
    for b in bytes.iter().cloned() {

        let repr = if was_7f {
            was_7f = false;
            match b {
                CMD_ASIS => "ASIS".into(),
                CMD_STORE_PC_LINE => "STORE_PC_LINE".into(),
                CMD_STORE_PC_INSTR => "STORE_PC_INSTR".into(),
                CMD_ORG => "ORG".into(),
                CMD_ORG2 => "ORG2".into(),
                CMD_ENT => "ENT".into(),
                CMD_FILL => "FILL".into(),
                CMD_SKIP => "SKIP".into(),
                CMD_IF => "IF".into(),
                CMD_ELSE => "ELSE".into(),
                CMD_END => "END".into(),
                CMD_FACTOR_BLOC => "FACTOR_BLOC".into(),
                CMD_FACTOR_BLOC_END => "FACTOR_BLOC_END".into(),
                CMD_END_BIS => "END_BIS".into(),
                CMD_BRK => "BRK".into(),
                CMD_BRK_SET => "BRK_SET".into(),
                CMD_RESTORE => "RESTORE".into(),
                CMD_BANK  => "BANK".into(),
                CMD_ENDM => "ENDM".into(),
                CMD_MACRO_USE => "MACRO_USE".into(),
                CMD_LOAD => "LOAD".into(),
                CMD_IMPORT => "IMPORT".into(),
                CMD_STR => "STR".into(),
                CMD_SAVE => "SAVE".into(),
                CMD_SAVEA => "SAVEA".into(),
                CMD_REPEAT => "REPEAT".into(),

                _ => format!("0x7F 0x{:02X}", b).into()
            }
        }
        else {
            was_7f = b == MARKER_ESCAPE;
            match b {
                MARKER_NEWLINE => "NL".into(),
                MARKER_INDENT => "IND".into(),
                MARKER_ESCAPE => "ESC".into(),
                MARKER_COMMENT => "COMT".into(),
                MARKER_ASSIGN => "ASS".into(),
                MARKER_WORD => "WORD".into(),
                MARKER_BYTE => "BYTE".into(),
                MARKER_LOCAL_LABEL => "LOCAL_LABEL".into(),
                MARKER_LABEL_ADDR => "LABEL_ADDR".into(),
                MARKER_MACRO_DEF => "MACRO_DEF".into(),

                _ => format!("0x{:02X}", b).into(),
            }
        };

        results.push(repr);
    }

    results.iter().join(",")
}

fn debug_slice(bytes: &[u8]) {
    eprintln!("debug: bytes: [{}]", bytes.iter().take(20).map(|b| format!("{:02x}", b)).join(" "));
    eprint!("debug: codes: [{}]", byte2code_naive(&bytes[..bytes.len().min(20)]));
}




const CHUNK_MAX_SIZE: u8 = 222;

// Marker bytes
const MARKER_NEWLINE: u8 = 0x4A; // 'J'
const MARKER_INDENT: u8 = 0x49; // 'I'
const MARKER_ESCAPE: u8 = 0x7F; // escape code for commands
const MARKER_COMMENT: u8 = 0x43; // 'C' - introduces a comment
const MARKER_ASSIGN: u8 = 0x64; // 'd' - introduces an assignment
const MARKER_WORD: u8 = 0xD7;
const MARKER_BYTE:u8 = 0xcf;
const MARKER_LOCAL_LABEL: u8 = 0x51;
const MARKER_LABEL_ADDR: u8 = 0x40;
const MARKER_MACRO_DEF: u8 = 0x6d;


const TAB_INSTR: u8 = 10;
const TAB_COMMAND: u8 = 6;
const TAB_COMMENT: u8 =  24;

// Label reference ranges (from ORGAMS format docs)
const SHORT_LABEL_START: u8 = 0x60; // Short labels: 0x60-0xDF (128 labels)
const SHORT_LABEL_END: u8 = 0xDF;
const LONG_LABEL_START: u8 = 0xE0; // Long labels: 0xE0-0xFF (256 labels)

const EXP_MULTI_TERM_BEGIN: u8 = 0x42; // 'B'
const EXP_MULTI_TERM_END: u8 = 0x45; // 'E'
const EXP_SHORT_DECIMAL_MAX_VALUE: u8 = 0x1F; // 0x00-0x1F

const EXP_SPACE: u8 = 0x20;
const EXP_OP_PLUS: u8 = 0x2B;
const EXP_OP_MINUS: u8 = 0x2D;
const EXP_OP_MULT: u8 = 0x2A;
const EXP_OP_DIV: u8 = 0x2F;
const EXP_OP_MOD: u8 = 0x25;
const EXP_OP_PAREN_OPEN: u8 = 0x28;
const EXP_OP_PAREN_CLOSE: u8 = 0x29;

const EXP_AND: u8 = 0x26; // '&';

const EXP_DECIMAL_8: u8 = 0x30; // '0'
const EXP_DECIMAL_16: u8 = 0x31;
const EXP_DECIMAL_CUSTOM: u8 = 0x32;
const EXP_DECIMAL_CUSTOM_LONG: u8 = 0x33;

const EXP_HEXDECIMAL_8: u8 = 0x34; // '4'
const EXP_HEXDECIMAL_16: u8 = 0x35;
const EXP_HEXDECIMAL_CUSTOM: u8 = 0x36;
const EXP_HEXDECIMAL_CUSTOM_LONG: u8 = 0x37;

const EXP_BINARY_8: u8 = 0x38;
const EXP_BINARY_16: u8 = 0x39;
const EXP_BINARY_CUSTOM: u8 = 0x3A;
const EXP_BINARY_CUSTOM_LONG: u8 = 0x3B;

const SHORT_LABEL: u8 = SHORT_LABEL_START;

// Command opcodes after 0x7f
const CMD_ASIS: u8 = 1;
const CMD_STORE_PC_LINE: u8 = 0x02;
const CMD_STORE_PC_INSTR: u8 = 0x03;
const CMD_ORG: u8 = 0x04;
const CMD_ORG2: u8 = 0x05;
const CMD_ENT: u8 = 0x06;
const CMD_FILL: u8 = 0x07;
const CMD_SKIP: u8 = 0x08;
const CMD_IF: u8 = 0x09;
const CMD_ELSE: u8 = 0x0A;
const CMD_END: u8 = 0x0C;
const CMD_FACTOR_BLOC: u8 = 13;
const CMD_FACTOR_BLOC_END: u8 = 14;
const CMD_END_BIS: u8 = 0x0F;
const CMD_BRK: u8 = 16;
const CMD_BRK_SET: u8 = 17;
const CMD_RESTORE: u8 = 18;
const CMD_BANK : u8 = 19;
const CMD_ENDM: u8 = 0x14; // Renamed from CMD_MACRO
const CMD_MACRO_USE: u8 = 0x15; // 0x15 used to be called Endm in decoder2, but decoder.rs calls it MacroUse or If. Assuming unused for now.
const CMD_LOAD: u8 = 22;
const CMD_IMPORT: u8 = 0x17; // This seems to be the escaped version?
const CMD_STR: u8 = 24;
const CMD_SAVE: u8 = 25;
const CMD_SAVEA: u8 = 26;
const CMD_REPEAT: u8 = 0x5B;

const IX_CODE: u8 = 0xdd;
const IY_CODE: u8 = 0xfd;


const END_MARKER: u8 = 0x41;

#[inline]
fn consume_marker(marker: u8) -> impl Fn(&mut Input) -> OrgamsParseResult<()> + 'static {
    move |input: &mut Input| -> OrgamsParseResult<()> {
        literal(marker).void().parse_next(input)
    }
}

/// Main program structure
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub chunks : Vec<Chunk>,
    pub labels: StringTable
}

#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub items: Vec<Item>
}

impl Deref for Line {
    type Target = Vec<Item>;
    fn deref(&self) -> &Self::Target {
        &self.items
    }
}


impl Line {
    pub fn bytes(&self, labels: &StringTable) -> Vec<u8> {
        self.items.iter()
         .flat_map(|i| i.bytes(labels).into_iter())
         .collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Item> {
        self.items.iter()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub lines: Vec<Line>,
}


impl Deref for Chunk {
    type Target = Vec<Line>;

    fn deref(&self) -> &Self::Target {
        &self.lines
    }
}

impl Chunk {

    /// Get the bytes of the source part
    pub fn bytes(&self, labels: &StringTable) -> Vec<u8> {
        let mut bytes: Vec<u8> = self.lines
            .iter()
            .flat_map(|line| line.items.iter())
            .flat_map(|item| item.bytes(labels).into_iter())
            .collect();
        let computed_size = bytes.len() as u8;
        bytes.insert(0, computed_size); // prepend size byte
        bytes
    }

    pub fn items(&self) -> impl Iterator<Item = &Item> {
        self.lines.iter().flat_map(|line| line.items.iter())
    }

}

impl Program {

    pub fn items(&self) -> impl Iterator<Item = &Item> {
        self.chunks.iter().flat_map(|chunk| chunk.items())
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.header_bytes()
            .into_iter()
            .chain(self.src_bytes().into_iter())
            .chain(self.labels_bytes().into_iter())
            .chain(self.checksum_bytes().into_iter())
            .collect()
    }

    /// Get the bytes of the source part
    pub fn src_bytes(&self) -> Vec<u8> {
        self.chunks
            .iter()
            .flat_map(|chunk| chunk.bytes(&self.labels).into_iter())
            .chain(vec![0u8].into_iter()) // null terminator
            .collect()
    }

    pub fn header_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"ORGA");
        bytes.extend_from_slice(&[0u8; 0x67 - 4]); // rest of header
        bytes.extend_from_slice(b"SRCc");
        bytes.push(2); // version
        bytes
    }

    pub fn labels_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"LBLs");
        bytes.push(2); // version
        for s in &self.labels.strings {
            bytes.extend_from_slice(&s.bytes());
        }
        bytes.push(0); // null terminator
        bytes
    }

    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"ChCk");
        bytes.extend_from_slice(&[0u8; 4]); // placeholder for checksum
        bytes
    }
}

/// Top-level items
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// Comment text (without the marker)
    Comment(Comment),
    /// Newline marker
    NewLine,
    /// Indent marker with space count
    Indent(Indent),
    /// Assignment: label = expression
    Assign(Assign),
    /// Standalone label reference
    Label(LabelRef),
    /// Macro definition
    Macro(MacroDef),
    /// Statement
    Statement(Statement),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Indent(pub u8);

#[derive(Debug, Clone, PartialEq)]
pub struct Comment(pub SizedString);

impl Deref for Comment {
    type Target = SizedString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Represent a string where the bit 7 is on on the last char.
/// The stored string DEOS NOT have the bit 7 on.
#[derive(Debug, Clone, PartialEq)]
pub struct Bit7OnString(String);

impl Bit7OnString {
    // Create a string from a array of bytes where the last char has bit 7 on
    // accept ONLY ASCII strings
    pub fn new(bytes: &[u8]) -> Self {
        let mut content = bytes.to_vec();
        if let Some(last) = content.last_mut() {
            *last &= !(1 << 7); // clear high bit
        }
        Self(String::from_utf8_lossy(&content).to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut bytes = self.0.as_bytes().to_vec();
        if let Some(last) = bytes.last_mut() {
            *last |= 1 << 7; // set bit 7 on last char
        }
        bytes
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SizedString(String);

impl PartialEq<str> for Bit7OnString {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for Bit7OnString {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl SizedString {
    pub fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.0.len() + 1);
        bytes.push(self.0.len() as u8);
        bytes.extend_from_slice(self.0.as_bytes());
        bytes
    }
}

impl Deref for SizedString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Item {
    pub fn bytes(&self, table: &StringTable) -> Vec<u8> {
        match self {
            Item::Comment(text) => {
                let mut bytes = Vec::with_capacity(text.len() + 1 + 1);
                bytes.push(MARKER_COMMENT);
                bytes.extend_from_slice(text.bytes().as_slice());
                bytes
            },
            Item::NewLine => vec![MARKER_NEWLINE],
            Item::Indent(count) => vec![MARKER_INDENT, count.0],
            Item::Assign(assign) => assign.bytes(table),
            Item::Label(label) => {
                let mut bytes = Vec::new();
                bytes.push(MARKER_LABEL_ADDR);
                bytes.extend_from_slice(&label.bytes(table));
                bytes
            },
            Item::Macro(m) => m.bytes(table),
            Item::Statement(s) => s.bytes(table),
        }
    }
}

/// Label reference (index into string table)
#[derive(Debug, Clone, PartialEq)]
pub enum LabelRef {
    Short(u8),
    Long(u8, u8),
}

impl LabelRef {
    const fn new_short_from_stream(byte: u8) -> Self {
        debug_assert!(byte >= SHORT_LABEL_START && byte <= SHORT_LABEL_END);
        LabelRef::Short(byte - SHORT_LABEL_START)
    }
    
    const fn new_long_from_stream(long: u8, byte: u8) -> Self {
        debug_assert!(long >= LONG_LABEL_START && long <= 0xFF); // XXX I do not know if there is a range of values here
        LabelRef::Long(long, byte)
    }

    pub fn get(&self, table: &StringTable) -> Cow<'_, str> {
        table
            .label(self)
            .map(|s| Cow::Owned(s))
            .unwrap_or(Cow::Borrowed("<unknown label>"))
    }

    pub fn bytes(&self, _table: &StringTable) -> Vec<u8> {
        match self {
            LabelRef::Short(index) => vec![index + SHORT_LABEL_START],
            LabelRef::Long(long, byte) => vec![*long, *byte],
        }
    }

    pub fn display(&self, table: &StringTable) -> String {
        self.get(table).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StringTable {
    strings: Vec<Bit7OnString>
}

impl StringTable {
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Bit7OnString> {
        self.strings.iter()
    }

    fn empty() -> Self {
        Self {
            strings: Vec::with_capacity(0)
        }
    }

    pub fn from_vec_bit7on_texts(strings: Vec<Bit7OnString>) -> Self {
        Self { strings }
    }

    fn get(&self, index: usize) -> Option<&Bit7OnString> {
        self.strings.get(index)
    }

    fn add(&mut self, s: Vec<u8>) -> usize {
        self.strings.push(Bit7OnString::new(&s));
        self.strings.len() - 1
    }

    fn label(&self, label_ref: &LabelRef) -> Option<String> {
        let index = match label_ref {
            LabelRef::Short(idx) => *idx as usize,
            LabelRef::Long(long, byte) => {
                // Compute index from long label encoding
                // Long labels use 2 bytes to encode larger indices
                // 0xE0 corresponds to the start of long labels.
                // 0x60 - 0xDF are short labels (indices 0 - 127)
                // So index starts at 128
                let idx = 128 +(((*long as usize) << 8) | (*byte as usize)) - 0xE000 ;
                idx
            }
        };
        self.get(index).map(|s| s.as_str().to_string())
    }
}

/// Expression (encoded bytes)
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    MultiTerm(Vec<ExpressionMember>),
    SingleTerm(ExpressionMember)
}



#[derive(Debug, Clone, PartialEq)]
pub struct SizedExpression(Expression);

impl Deref for SizedExpression {
    type Target = Expression;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionMember {
    ShortDecimal(u8), // number between 0-31
    Value(Value),     // decimal/hexadecimal/binary 8/16/custom
    Operator(Operator),
    LabelRef(LabelRef), // Label reference 0x60-0xFF
    Space,              // 0x20
    Dollar,             // 0x24 ($)
    DoubleDollar        // 0x44 ($$)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    And,
    Plus,       // 0x2b
    Minus,      // 0x2d
    Multiply,   // 0x2a
    Divide,     // 0x2f
    Modulo,     // 0x25
    ParenOpen,  // 0x28
    ParenClose  // 0x29
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValueBasis {
    Decimal,
    Hexadecimal,
    Binary
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValueContent {
    EightBits(u8),
    SixteenBits(u16),
    Custom(Vec<u8>)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    pub basis: ValueBasis,
    pub content: ValueContent
}

impl Value {
    pub fn bytes(&self, _table: &StringTable) -> Vec<u8> {
        // TODO everything is false here : we need to output the code and content
        match (&self.basis, &self.content) {
            (ValueBasis::Decimal, ValueContent::EightBits(v)) => vec![0x30, *v],
            (ValueBasis::Hexadecimal, ValueContent::EightBits(v)) => vec![0x34, *v],
            (ValueBasis::Binary, ValueContent::EightBits(v)) => vec![0x38, *v],
            (ValueBasis::Decimal, ValueContent::SixteenBits(v)) => {
                let mut result = vec![0x31];
                result.extend_from_slice(&v.to_le_bytes());
                result
            },
            (ValueBasis::Hexadecimal, ValueContent::SixteenBits(v)) => {
                let mut result = vec![0x35];
                result.extend_from_slice(&v.to_le_bytes());
                result
            },
            (ValueBasis::Binary, ValueContent::SixteenBits(v)) => {
                let mut result = vec![0x39];
                result.extend_from_slice(&v.to_le_bytes());
                result
            },
            (_, ValueContent::Custom(b)) => b.clone()
        }
    }
}

impl ExpressionMember {
    pub fn bytes(&self, table: &StringTable) -> Vec<u8> {
        match self {
            ExpressionMember::ShortDecimal(v) => vec![*v],
            ExpressionMember::Value(val) => val.bytes(table),
            ExpressionMember::Operator(op) => {
                let code = match op {
                    Operator::Plus => 0x2B,
                    Operator::Minus => 0x2D,
                    Operator::Multiply => 0x2A,
                    Operator::Divide => 0x2F,
                    Operator::Modulo => 0x25,
                    Operator::ParenOpen => 0x28,
                    Operator::ParenClose => 0x29,
                    Operator::And => EXP_AND,
                };
                vec![code]
            },
            ExpressionMember::LabelRef(label_ref) => label_ref.bytes(table),
            ExpressionMember::Space => vec![0x20],
            ExpressionMember::Dollar => vec![0x24],
            ExpressionMember::DoubleDollar => vec![0x44],
        }
    }
}

impl Expression {
    pub fn bytes(&self, table: &StringTable) -> Vec<u8> {
        let mut result = Vec::new();

        match self {
            Expression::MultiTerm(members) => {
                result.push(EXP_MULTI_TERM_BEGIN);
                for member in members {
                    result.extend(member.bytes(table));
                }
                result.push(EXP_MULTI_TERM_END);
            },
            Expression::SingleTerm(member) => {
                result.extend(member.bytes(table));
            }
        }

        result
    }
}

impl SizedExpression {
    pub fn bytes(&self, table: &StringTable) -> Vec<u8> {
        let mut result= self.0.bytes(table);
        // Prepend size byte
        let size = result.len() as u8;
        let mut with_size = vec![size];
        with_size.extend(result);
        with_size
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assign {
    pub label: LabelRef,
    pub expression: SizedExpression,
}

impl Assign {
    pub fn bytes(&self, table: &StringTable) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(MARKER_ASSIGN);
        
        bytes
            .into_iter()
            .chain(
                self.label
                    .bytes(table)
                    .into_iter()
                    .chain(self.expression.bytes(table).into_iter())
            )
            .collect()
    }
}


impl Expression {
    pub fn display(&self, table: &StringTable) -> String {
        match self {
            Expression::MultiTerm(members) => {
                members.iter().map(|m| m.display(table)).collect::<Vec<_>>().join("")
            },
            Expression::SingleTerm(member) => member.display(table)
        }
    }
}

impl ExpressionMember {
    pub fn display(&self, table: &StringTable) -> String {
        match self {
            ExpressionMember::ShortDecimal(v) => format!("{}", v),
            ExpressionMember::Value(v) => v.display(),
            ExpressionMember::Operator(op) => op.to_string(),
            ExpressionMember::LabelRef(l) => l.get(table).to_string(),
            ExpressionMember::Space => " ".to_string(),
            ExpressionMember::Dollar => "$".to_string(),
            ExpressionMember::DoubleDollar => "$$".to_string()
        }
    }
}

impl Operator {
    pub fn to_string(&self) -> String {
        match self {
            Operator::Plus => "+".to_string(),
            Operator::Minus => "-".to_string(),
            Operator::Multiply => "*".to_string(),
            Operator::Divide => "/".to_string(),
            Operator::Modulo => "%".to_string(),
            Operator::ParenOpen => "[".to_string(),
            Operator::ParenClose => "]".to_string(),
            Operator::And => "AND".to_string(),
        }
    }
}

impl Value {
    pub fn display(&self) -> String {
         match &self.content {
            ValueContent::EightBits(val) => {
                match self.basis {
                    ValueBasis::Decimal => format!("{}", val),
                    ValueBasis::Hexadecimal => format!("&{:X}", val),
                    ValueBasis::Binary => format!("%{:b}", val)
                }
            },
            ValueContent::SixteenBits(val) => {
                match self.basis {
                    ValueBasis::Decimal => format!("{}", val),
                    ValueBasis::Hexadecimal => format!("&{:04X}", val),
                    ValueBasis::Binary => format!("%{:b}", val)
                }
            },
            ValueContent::Custom(_bytes) => {
                 String::from("<custom>")
            }
         }
    }
}

impl Assign {
    pub fn display(&self, table: &StringTable) -> String {
        format!("{} = {}", self.label.get(table), self.expression.display(table))
    }
}

impl Item {
    pub fn display(&self, table: &StringTable) -> String {
         match self {
            Item::Comment(text) => format!(";{}", text.as_str()),
            Item::NewLine => "\n".to_string(),
            Item::Indent(count) => " ".repeat(count.0 as usize),
            Item::Assign(assign) => assign.display(table),
            Item::Label(label) => format!("{}", label.get(table)),
            Item::Macro(m) => m.display(table),
            Item::Statement(s) => s.display(table),
        }
    }
}





/// Macro definition
#[derive(Debug, Clone, PartialEq)]
pub struct MacroDef {
    pub def_block_len: u8, // TODO do not store it but recompute on the fly
    pub name: LabelRef,
    pub params: Vec<LabelRef>,
}

impl MacroDef {
    pub fn bytes(&self, table: &StringTable) -> Vec<u8> {
        let mut content = Vec::new();
        content.extend_from_slice(&self.name.bytes(table));
        
        for param in &self.params {
            content.extend_from_slice(&param.bytes(table));
        }

        // Respect the original length if possible, padding with 0 (or 0x41?) if we are short.
        // The original parser saw 0x41 as separator.
        // If we strictly recompute:
        let computed_len = content.len() as u8;
        
        // If we want to be "pure", we use computed_len.
        // But to pass tests requiring exact binary reconstruction of existing files that might have padding/separators:
        let mut bytes = Vec::new();
        bytes.push(MARKER_MACRO_DEF);
        
        if self.def_block_len > computed_len {
             bytes.push(self.def_block_len);
             bytes.extend_from_slice(&content);
             // Fill gap. END_MARKER was the observed separator.
             for _ in 0..(self.def_block_len - computed_len) {
                 bytes.push(END_MARKER); 
             }
        } else {
             bytes.push(computed_len);
             bytes.extend_from_slice(&content);
        }
        
        bytes
    }

    pub fn display(&self, table: &StringTable) -> String {
        let name = self.name.get(table);
        if self.params.is_empty() {
            format!("MACRO {}", name)
        } else {
            let mut params = String::new();
            for p in &self.params {
                params.push(' ');
                params.push_str(&p.get(table));
            }
            format!("MACRO {}{}", name, params)
        }
    }
}

/// Statements
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    If(SizedExpression),
    Else,
    End,
    EndBis,
    EndMacro,
    Ent(SizedExpression),
    Import(String, bool), 
    Org2(SizedExpression, SizedExpression),
    Org(SizedExpression),
    Byte(Vec<SizedExpression>),
    Word(Vec<SizedExpression>),
    Skip(SizedExpression),
    StorePcInstr, // hidden instruction
    StorePcLine, // hidden instruction
    StarRepeat( Box<SizedExpression>, Box<Item>),
    RawString(String),
    MacroUse(Expression, Vec<Expression>),
    Instruction(Instruction)
}

impl Statement {
    pub fn bytes(&self, table: &StringTable) -> Vec<u8> {
        let mut bytes = Vec::new();
        match self {
            Statement::Byte(exprs) => {
                let exprs = exprs.iter().flat_map(|expr| expr.bytes(table)).collect::<Vec<u8>>();
                bytes.push(MARKER_BYTE);
                bytes.push(exprs.len() as u8+2);
                bytes.push(1.max(exprs.len() as u8));
                bytes.extend_from_slice(&exprs);
                bytes.push(END_MARKER);
            }
            Statement::Word(exprs) => {
                let exprs = exprs.iter().flat_map(|expr| expr.bytes(table)).collect::<Vec<u8>>();
                bytes.push(MARKER_WORD);
                bytes.push(exprs.len() as u8+2);
                bytes.push((1.max(exprs.len() as u8) *2));
                bytes.extend_from_slice(&exprs);
                bytes.push(END_MARKER);
            }
            Statement::If(condition) => {
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_IF);
                bytes.extend_from_slice(&condition.bytes(table));
            },
            Statement::Else => {
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_ELSE);
            },
            Statement::End => {
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_END);
            },
            Statement::EndBis => {
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_END_BIS);
            },
            Statement::EndMacro => {
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_ENDM);
            },
            Statement::Import(s, escaped) => {
                if *escaped {
                    bytes.push(MARKER_ESCAPE);
                }
                bytes.push(CMD_IMPORT); // Not escaped usually?
                bytes.push(s.len() as u8);
                bytes.extend_from_slice(s.as_bytes());
            },
            Statement::RawString(s) => {
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_ASIS); // Raw string command
                let content = s.as_bytes();
                bytes.push(content.len() as u8);
                bytes.extend_from_slice(content);
            },
            Statement::Ent(exp) => {
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_ENT); // ENT command
                bytes.extend_from_slice(&exp.bytes(table));
            },
            Statement::Org(exp) => {
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_ORG); // ORG command
                bytes.extend_from_slice(&exp.bytes(table));
            },
            Statement::Org2(exp1, exp2) => {
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_ORG2); // ORG2 command
                bytes.extend_from_slice(&exp1.bytes(table));
                bytes.extend_from_slice(&exp2.bytes(table));
            },

            Statement::Skip(exp) => {
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_SKIP); // SKIP command
                bytes.extend_from_slice(&exp.bytes(table));
            },
            Statement::StorePcInstr => {
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_STORE_PC_INSTR); 
                
            },
            Statement::StorePcLine => {
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_STORE_PC_LINE); 
            },
            
            Statement::StarRepeat(exprs, token) => {
                bytes.push(CMD_REPEAT);
                bytes.extend_from_slice(&exprs.bytes(table));
                bytes.extend_from_slice(&token.bytes(table));
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_END_BIS);
            },

            Statement::MacroUse(name, args) => {
                let mut content = name.bytes(table);
                for arg in args {
                    content.extend(arg.bytes(table));
                }
                content.push(END_MARKER); // endmarker
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_MACRO_USE);
                bytes.push(content.len() as u8);
                bytes.extend(content);
            },

            Statement::Instruction(instr) => {
                let instr_bytes = instr.bytes(table);
                bytes.extend(instr_bytes);
            }
        }
        bytes
    }

    pub fn display(&self, table: &StringTable) -> String {
        match self {
            Statement::If(expr) => format!("IF {}", expr.display(table)),
            Statement::Else => "ELSE".to_string(),
            Statement::End => "END".to_string(),
            Statement::EndBis => "".to_string(),
            Statement::EndMacro => "ENDM".to_string(), 
            Statement::Import(s, escaped) => {
                if *escaped && s.starts_with('"') && s.len() >= 3 {
                    // Start is " + user number
                    // End is A (maybe access rights/encoding ?)
                    let stripped = &s[2..s.len()-1];
                    format!("IMPORT \"{}\"", stripped)
                } else {
                    format!("IMPORT \"{}\"", s)
                }
            },
            Statement::RawString(s) => s.clone(),
            Statement::Ent(e) => format!("ENT {}", e.display(table)),
            Statement::Org(e) => format!("ORG {}", e.display(table)),
            Statement::Org2(e1, e2) => format!("ORG {},{}", e1.display(table), e2.display(table)),
            Statement::Skip(e) => format!("SKIP {}", e.display(table)),
            Statement::Byte(exprs) => {
                let exprs = exprs.iter().map(|e| e.display(table)).join(",");
                format!("BYTE {}", exprs)
            },
            Statement::Word(exprs) => {
                format!("WORD {}", exprs.iter().map(|e| e.display(table)).join(","))
            }
            Statement::StorePcInstr | Statement::StorePcLine => {
                // norepresentation is expected
                "".to_owned()
            },
            Statement::StarRepeat(expr, item) => {
                format!("{} ** {}",  expr.display(table), item.display(table))
            },
            Statement::MacroUse(name, args) => {
                //let name_str = name.get(table).to_string();
                let name_str = name.display(table);
                let args_str = args.iter().map(|e| e.display(table)).collect::<Vec<_>>().join(",");

                format!("{}({})", name_str, args_str)
            },
            Statement::Instruction(instr) => {
                instr.display(table)
            }
            
        }
    }
}


#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    // prefix for IX/IY related instructions
    pub prefix: Option<u8>,
    // opcode of the instruction. Can explicitely encode operands
    pub opcode: u8,
    // operands as expressions
    pub coded_operands: Vec<SizedExpression>,
}


impl Instruction {
    pub fn bytes(&self, table: &StringTable) -> Vec<u8> {
        let mut bytes = Vec::new();

        if let Some(prefix) = self.prefix {
            bytes.push(prefix);
        }
        bytes.push(self.opcode);
        for expr in &self.coded_operands {
            bytes.extend_from_slice(&expr.bytes(table));
        }
        bytes
    }

    pub fn display(&self, table: &StringTable) -> String {
        let tab = if let Some(prefix) = self.prefix {
            match prefix {
                IX_CODE => TABINSTRDD,
                IY_CODE => TABINSTRFD,
                other => panic!("Unknown prefix code: {}", other),
            }
        } else {
            TABINSTR
        };

        let mut result = tab[self.opcode as usize].to_lowercase();
        let mut i = 0;
        while i < self.coded_operands.len() {
            if let Some(pos) = result.find("nnnn") {
                let expr_str = self.coded_operands[i].display(table);
                result.replace_range(pos..pos + 4, &expr_str);
                i += 1;
            } else if let Some(pos) = result.find("nn") {
                let expr_str = self.coded_operands[i].display(table);
                result.replace_range(pos..pos + 2, &expr_str);
                i += 1;
            } else {
                break;
            }
        }
        result
    }
}

#[derive(Debug, Clone)]
enum ExpressionKind {
    EightBits,
    SixteenBits
}

fn z80str_to_expressions_list(repr: &str) -> SmallVec<[ExpressionKind; 2]> {
    let mut kinds = SmallVec::new();
    let mut i = 0;
    while i < repr.len() {
        if repr[i..].starts_with("nnnn") {
            kinds.push(ExpressionKind::SixteenBits);
            i += 4;
        } else if repr[i..].starts_with("nn") {
            kinds.push(ExpressionKind::EightBits);
            i += 2;
        } else {
            i += 1;
        }
    }
    kinds
}

fn parse_indexed_instruction(prefix: u8) -> impl Fn(&mut Input) -> OrgamsParseResult<Instruction> {
    move |input: &mut Input| {
        let opcode = any.parse_next(input)?;
        let repr = if prefix == IX_CODE {
            TABINSTRDD
        } else if prefix == IY_CODE {
            TABINSTRFD
        } else {
            panic!("Unsupported prefix: 0x{:02X}", prefix);
        }[opcode as usize];
        let kinds = z80str_to_expressions_list(repr);
        let mut coded_operands = Vec::with_capacity(kinds.len());
        for _kind in kinds {
            let expr = parse_sized_expression.parse_next(input)?;
            coded_operands.push(expr);
        }
        Ok(Instruction { prefix: Some(prefix), opcode, coded_operands })
    }
}

        pub struct DisplayState<'f, 'g> {
            pub(crate) f: Option<&'f mut std::fmt::Formatter<'g>>,
            pub(crate) current_line: String,
            pub(crate) line_number: usize,
            pub(crate) next_comment_requires_alignment: bool,
            pub(crate) previous_was_label: bool,
            pub(crate) last_generated_line: Option<String>
        }

        impl<'f, 'g> DisplayState<'f, 'g> {
            pub fn new(f: Option<&'f mut std::fmt::Formatter<'g>>) -> Self {
            Self {
                f,
                current_line: String::new(),
                line_number: 1,
                next_comment_requires_alignment: false,
                previous_was_label: false,
                last_generated_line: None,
            }
            }

            pub fn last_line(&self) -> Option<&str> {
                self.last_generated_line.as_deref()
            }

            pub fn line_number(&self) -> usize {
                self.line_number
            }
        }

        impl<'f,'g>  DisplayState<'f,'g> {
            fn emit_line(&mut self) -> std::fmt::Result {
                if let Some(f) = self.f.as_mut() {
                    write!(f, "{}\r\n", self.current_line)?;
                }

                self.last_generated_line = Some(self.current_line.clone());
                self.current_line.clear();
                self.line_number += 1;
                self.next_comment_requires_alignment = false;
                self.previous_was_label = false;
                Ok(())
            }

            fn auto_format(&mut self, is_instruction: bool) {
                if self.current_line.is_empty() {
                    let nb_tabs = if is_instruction { TAB_INSTR } else { TAB_COMMAND };
                    for _ in 0..nb_tabs {
                        self.current_line.push(' ');
                    }
                } else if !self.has_only_indents() {
                    if !self.current_line.ends_with(' ') {
                        if !self.previous_was_label{
                            self.current_line.push(':');
                        } else {
                            const INDENT_AFTER_LABEL: usize = TAB_INSTR as _;
                            if self.col_number() <= INDENT_AFTER_LABEL {
                                self.current_line.push_str(" ".repeat(INDENT_AFTER_LABEL - self.col_number()).as_str());
                            } else {
                                self.current_line.push(' ');
                            }
                        }
                    }
                }
            }

            
            fn append_instruction_representation(&mut self, s: String) {
                self.append_token_or_instruction_representation(true, s);
            }

            fn append_token_representation(&mut self, s: String) {
                self.append_token_or_instruction_representation(false, s);
            }

            fn append_token_or_instruction_representation(&mut self, is_instruction: bool, s: String) {
                if !self.current_line.is_empty() && (s.starts_with("WORD") || s.starts_with("BYTE") || s.starts_with("SKIP")){
                    self.current_line.push(' ')
                } else if !s.starts_with(' ') {
                    // I wonder if it is a hack
                    self.auto_format(is_instruction); 
                }
                self.append_string_no_indent(s);
                self.previous_was_label = false;
            }

            fn append_comment(&mut self, c: &str) -> std::fmt::Result {
              if (self.has_only_indents() || self.is_empty()) && !self.next_comment_requires_alignment{
                        // nothing to do
                    } else {
                        // handle some indentation
                         let current_line_len = self.col_number();
                         let indent = if current_line_len < TAB_COMMENT as usize {
                             format!("{}", " ".repeat(TAB_COMMENT as usize - current_line_len))
                         } else {
                             " ".into()
                         };
                         self.append_string_no_indent(indent);
                    }

                self.append_string_no_indent(format!(";{}", c));
                self.emit_line()
              }

            fn append_label(&mut self, label: &str) {
                self.append_string_no_indent(label.to_string());
                self.previous_was_label = true;
            }

            /// the string already has its indentation. maybe it has to change
            fn append_assign(&mut self, assignment: &str)  {
                self.append_string_next_comment_aligned(assignment.to_string());
                self.previous_was_label = false;
            }

            fn append_string_no_indent(&mut self, s: String) {
                self.current_line.push_str(&s);
            }
            fn append_string_next_comment_aligned(&mut self, s: String) {
                self.append_string_no_indent(s);
                self.next_comment_requires_alignment = true;
            }

            fn col_number(&self) -> usize {
                self.current_line.len()
            }

            fn has_only_indents(&self) -> bool {
                self.current_line.chars().all(|c| c == ' ')
            }

            fn is_empty(&self) -> bool {
                self.current_line.is_empty()
            }

            pub fn render_items<'i>(&mut self, items : impl IntoIterator<Item=&'i Item>, labels: &StringTable) -> std::fmt::Result {
                for item in items {
                    self.render_item(item, labels)?;
                }
                Ok(())
            }

        pub fn render_item(&mut self, item: &Item, labels: &StringTable) -> std::fmt::Result {

            let state = self;
            match item {
                Item::Comment(comment) => {
                    let comment = comment.as_str();
                    state.append_comment(comment)?;
                },
                Item::NewLine => {
                    state.emit_line()?;
                },
                Item::Indent(count) => {
                    state.append_string_no_indent(" ".repeat(count.0 as usize));
                },
                Item::Assign(assign) => {
                    let label = labels.label(&assign.label).unwrap_or(format!("<unknown:{:?}>", assign.label));
                    let expr_repr = assign.expression.display(labels);
                    
                    // Heuristic: Left padding for short labels
                    let label_len = label.len();
                    let repr = if label_len < 5 {
                         format!("{}{} = {}", label, " ".repeat(5 - label_len), expr_repr)
                    } else {
                         format!("{} = {}", label, expr_repr)
                    };  
                    state.append_assign(&repr);

                },
                Item::Label(label) => {
                    let label_str = label.get(labels);
                    state.append_label(label_str.as_str());
                },
                Item::Macro(m) => {
                    let repr = m.display(labels);
                    state.append_token_representation(repr);
                },
                Item::Statement(s) => {
                    match s {

                        Statement::RawString(s) => {
                            state.append_string_no_indent(s.clone());
                        },

                        Statement::Instruction(ins) => {
                            let repr = ins.display(labels);
                            state.append_instruction_representation(repr);
                        },

                        _ => {
                            let repr: String = s.display(labels);
                            if !repr.is_empty() {
                                state.append_token_representation(repr);
                            }
                        }
                    }
                },

            }
                Ok(())
        }
        }

        impl<'f, 'g> Drop for DisplayState<'f, 'g> {
            fn drop(&mut self) {
                if !self.current_line.is_empty() {
                    self.emit_line();
                }
            }
        }



impl std::fmt::Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut iter = self.chunks.iter().flat_map(|c| c.items()).peekable();

        let mut state = DisplayState {
            f: Some(f),
            current_line: String::new(),
            line_number: 1,
            next_comment_requires_alignment: false,
            last_generated_line: None,
            previous_was_label: false,
        };


        Ok(())
    }
}

/// Parse complete Orgams file
pub fn parse_orgams_file(input: &mut Input) -> OrgamsParseResult<Program> {
    const ORGA: &[u8] = b"ORGA";
    const SRCC: &[u8] = b"SRCc";
    const LBLS: &[u8] = b"LBLs";
    const CHCK: &[u8] = b"ChCk";

    // Verify ORGA magic bytes
    cut_err(literal(ORGA).context(StrContext::Expected(StrContextValue::StringLiteral("ORGA"))))
        .parse_next(input)?;

    // Skip rest of header (0x67 bytes total, already read 4)
    let _header = take(0x67 - ORGA.len()).context(StrContext::Expected(StrContextValue::Description("header data"))).parse_next(input)?;

    eprintln!("debug: checking SRCC");
    cut_err(literal(SRCC).context(StrContext::Expected(StrContextValue::StringLiteral("SRCc"))))
        .parse_next(input)?;

    let _version = any.context(StrContext::Expected(StrContextValue::Description("version 2 expected"))).verify(|&b| b==2).parse_next(input)?;

    // Parse items
    eprintln!("debug: parsing items");
    let chunks = parse_all_code.context(StrContext::Expected(StrContextValue::Description("chunks"))).parse_next(input)?;

    // Skip null byte separator before LBLs
    let _null_separator = literal([0x00]).context(StrContext::Expected(StrContextValue::Description("null after chunks"))).parse_next(input)?;

    // parse labels table
    eprintln!("debug: parsing labels. Next bytes: {:?}", input.iter().take(10).collect::<Vec<_>>());
    cut_err(literal(LBLS).context(StrContext::Expected(StrContextValue::StringLiteral("LBLs"))))
        .parse_next(input)?;
    let labels = parse_labels_table.parse_next(input).map_err(|e| {
        eprintln!("debug: parse_labels_table failed: {:?}", e);
        e
    })?;

    // parse checksum
    let _null_separator = literal([0x00]).parse_next(input)?;
    cut_err(literal(CHCK).context(StrContext::Expected(StrContextValue::StringLiteral("ChCk"))))
        .parse_next(input)?;
    let _checksum_bytes = take(4usize).parse_next(input)?;


    //TODO check checksum
    //eof.parse_next(input)?;

    Ok(Program { chunks, labels })
}

pub fn parse_labels_table(input: &mut Input) -> OrgamsParseResult<StringTable> {
    use winnow::combinator::{peek, repeat_till};

    eprintln!("debug: parsing labels. Next bytes: {:?}", &input[..10]);
    // Parse version
    let _version = any.context(StrContext::Expected(StrContextValue::Description("labels version"))).verify(|&b| b==2).parse_next(input)?;
    
    // collect each of them
    let (strings, _): (Vec<_>, _) = repeat_till(0.., |input: &mut Input| {
             let res = parse_bit7on_text.context(StrContext::Expected(StrContextValue::Description("label text"))).parse_next(input);
             if res.is_err() {
                 eprintln!("debug: parse_bit7on_text failed at offset unknown");
             }
             res
        }, peek(literal([0x00]).context(StrContext::Expected(StrContextValue::Description("null terminator")))))
        .context(StrContext::Expected(StrContextValue::Description("labels until null")))
        .parse_next(input)?;

    Ok(StringTable::from_vec_bit7on_texts(strings))
}

/// Parse all items
fn parse_items(input: &mut Input) -> OrgamsParseResult<Vec<Item>> {
    repeat(.., parse_inner_item).parse_next(input)
}

/// Parse all items until LBLs
fn parse_items_untils_lbls(input: &mut Input) -> OrgamsParseResult<Vec<Item>> {
    const LBLS: &[u8] = b"LBLs";

    terminated(parse_items, LBLS).parse_next(input)
}

fn parse_star_repeat(input: &mut Input) -> OrgamsParseResult<Item> {
    consume_marker(CMD_REPEAT)(input)?;
    let expr = parse_sized_expression.parse_next(input)?;
    let item = cut_err(parse_inner_item.context(StrContext::Label("Failed to parse item after repeat expression"))).parse_next(input)?;

    // XXX need to be done here ?
    literal([MARKER_ESCAPE, CMD_END_BIS]).void().parse_next(input)?;

    Ok(Item::Statement(Statement::StarRepeat(Box::new(expr), Box::new(item))))
}

/// Parse single item EXCEPT newline, comments,  indenations, label, macrodref.
/// assign seems to be prefexid by 7f
fn parse_inner_item(input: &mut Input) -> OrgamsParseResult<Item> {
    dbg!("parse_inner_item input.len={}", input.len());
    debug_slice(input);
    // if let Some(b) = input.as_bytes().get(0) {
    //    eprintln!("DBG: parse_item input[0]={:02X}", b);
    //}
    alt((
        parse_escaped_7f_item,
        parse_byte,
        parse_word,
        parse_star_repeat,
        parse_assign.map(Item::Assign),
      //  parse_indent.map(Item::Indent),
        parse_import.map(|s| Item::Statement(Statement::Import(s, false))),
        parse_instruction.map(|i| Item::Statement(Statement::Instruction(i)))
    ))
    .parse_next(input)
}


fn parse_byte(input: &mut Input) -> OrgamsParseResult<Item> {
    consume_marker(MARKER_BYTE)(input)?;

    let length = any.parse_next(input)? as usize;
    assert!(length == 2, "Need to handle other cases !!");

    let size = any.parse_next(input)?; // size byte
    expect_end_marker.parse_next(input)?;

    let mut exprs = Vec::new();

    Ok(Item::Statement(Statement::Byte(exprs)))
}

fn parse_word(input: &mut Input) -> OrgamsParseResult<Item> {
    consume_marker(MARKER_WORD)(input)?;

    let length = any.parse_next(input)? as usize;
    assert!(length == 2, "Need to handle other cases !!");

    let size = any.parse_next(input)?; // size byte
    expect_end_marker.parse_next(input)?;

    let mut exprs = Vec::new();

    Ok(Item::Statement(Statement::Word(exprs)))
}




/// Parse explicit import directive (0x17)
fn parse_import(input: &mut Input) -> OrgamsParseResult<String> {
    consume_marker(CMD_IMPORT)(input)?;
    parse_sized_text.parse_next(input).map(|s| s.0)
}

fn parse_item_with_endline(input: &mut Input) -> OrgamsParseResult<Item> {
    alt((parse_endline, parse_inner_item)).parse_next(input)
}

/// Parse an indent marker (0x49) followed by space count
fn parse_indent(input: &mut Input) -> OrgamsParseResult<Indent> {
    consume_marker(MARKER_INDENT)(input)?;
    // Read the space count byte
    any.parse_next(input).map(|i| Indent(i))
}

/// Parse a comment: 0x43 followed by text until newline (newline not consumed)
fn parse_comment(input: &mut Input) -> OrgamsParseResult<Comment> {
    // Expect comment marker
    consume_marker(MARKER_COMMENT)(input)?;
    parse_sized_text.parse_next(input).map(Comment)
}

/// Parse a newline marker
fn parse_endline(input: &mut Input) -> OrgamsParseResult<Item> {
    consume_marker(MARKER_NEWLINE)(input)?;
    Ok(Item::NewLine)
}

fn parse_label(input: &mut Input) -> OrgamsParseResult<LabelRef> {
    // For now, just read one byte as index
    let index_byte: u8 = any.verify(|&b| b>=SHORT_LABEL_START).parse_next(input)?;
    if index_byte >= LONG_LABEL_START {
        // Long label
        let second_byte: u8 = any.parse_next(input)?;
        Ok(LabelRef::new_long_from_stream(index_byte, second_byte))
    } else {
        Ok(LabelRef::new_short_from_stream(index_byte))
    }
}

/// Parse expression: size_byte + expression_members
/// Expression members can be:
/// - 0x00-0x1F: Short decimal (0-31)
/// - 0x20: Space
/// - 0x2b: Plus, 0x2d: Minus, 0x2a: Multiply, 0x2f: Divide, 0x25: Modulo
/// - 0x28: ParenOpen, 0x29: ParenClose
/// - 0x30: Decimal8, 0x31: Decimal16, 0x32-0x33: DecimalLong/Custom
/// - 0x34: Hexa8, 0x35: Hexa16, 0x36-0x37: HexaLong/Custom
/// - 0x38: Binary8, 0x39: Binary16, 0x3a-0x3b: BinaryLong/Custom
/// - 0x42: Begin (multi-term), 0x45: End
/// - 0x60-0xFF: Label reference
fn parse_sized_expression(input: &mut Input) -> OrgamsParseResult<SizedExpression> {
    let size = any.parse_next(input)? as usize;
    parse_unsized_expression.map(SizedExpression).parse_next(input)
    // TODO check size
}


fn parse_unsized_expression(input: &mut Input) -> OrgamsParseResult<Expression> {
    let first = peek(any).parse_next(input)?;
    if first != EXP_MULTI_TERM_BEGIN {
        return parse_expression_member.map(Expression::SingleTerm).parse_next(input);
    }

    let _ = any.parse_next(input)?; // Consume BEGIN

    let mut members = Vec::new();

    loop {
        let check = peek(any).parse_next(input)?;
        if check == EXP_MULTI_TERM_END {
            let _ = any.parse_next(input)?;
            break;
        }
        members.push(parse_expression_member(input)?);
    }

    Ok(Expression::MultiTerm(members))
}

fn parse_expression_member(input: &mut Input) -> OrgamsParseResult<ExpressionMember> {
    let b = any.parse_next(input)?;

    if b <= EXP_SHORT_DECIMAL_MAX_VALUE {
        Ok(ExpressionMember::ShortDecimal(b))
    }
    else if b >= SHORT_LABEL_START && b <= SHORT_LABEL_END {
        Ok(ExpressionMember::LabelRef(LabelRef::new_short_from_stream(b)))
    }
    else if b >= LONG_LABEL_START {
         // Long label
         let second_byte = any.parse_next(input)?;
         Ok(ExpressionMember::LabelRef(LabelRef::new_long_from_stream(b, second_byte)))
    }
    else {
        match b {
            EXP_OP_PLUS => Ok(ExpressionMember::Operator(Operator::Plus)),
            EXP_OP_MINUS => Ok(ExpressionMember::Operator(Operator::Minus)),
            EXP_OP_MULT => {
                if let Ok(next_byte) =  peek(any::<_, ContextError>).parse_next(input) && next_byte == EXP_OP_MULT {
                    // Star repeat operator detected
                    let mut err = ContextError::new();
                    err.push(StrContext::Expected(StrContextValue::Description("Use of '**' operator is not allowed in expressions. It is only valid in star repeat statements.")));
                    return Err(ErrMode::Backtrack(err));
                }
                // else normal multiply
                Ok(ExpressionMember::Operator(Operator::Multiply))
            },
            EXP_OP_DIV => Ok(ExpressionMember::Operator(Operator::Divide)),
            EXP_OP_MOD => Ok(ExpressionMember::Operator(Operator::Modulo)),
            EXP_OP_PAREN_OPEN => Ok(ExpressionMember::Operator(Operator::ParenOpen)),
            EXP_OP_PAREN_CLOSE => Ok(ExpressionMember::Operator(Operator::ParenClose)),
            EXP_SPACE => Ok(ExpressionMember::Space),
            EXP_AND => Ok(ExpressionMember::Operator(Operator::And)),
            0x24 => Ok(ExpressionMember::Dollar), // TO BE CHECKED
            0x44 => Ok(ExpressionMember::DoubleDollar), // TO BE CHECKED
            
            // Value types
            _ => {
                 let (basis, content) = match b {
                    // decimal
                    EXP_DECIMAL_8 => {
                        let val = any.parse_next(input)?;
                        (ValueBasis::Decimal, ValueContent::EightBits(val))
                    },
                    EXP_DECIMAL_16 => {
                        let low = any.parse_next(input)? as u16;
                        let high = any.parse_next(input)? as u16;
                        let val = low | (high << 8);
                        (ValueBasis::Decimal, ValueContent::SixteenBits(val))
                    },
                    EXP_DECIMAL_CUSTOM | EXP_DECIMAL_CUSTOM_LONG => {
                        let len = any.parse_next(input)? as usize;
                        let bytes: Vec<u8> = take(len).parse_next(input)?.to_vec();
                        (ValueBasis::Decimal, ValueContent::Custom(bytes))
                    },

                    // hexadecimal
                    EXP_HEXDECIMAL_8 => {
                        let val = any.parse_next(input)?;
                        (ValueBasis::Hexadecimal, ValueContent::EightBits(val))
                    },
                    EXP_HEXDECIMAL_16 => {
                        let low = any.parse_next(input)? as u16;
                        let high = any.parse_next(input)? as u16;
                        let val = low | (high << 8);
                        (ValueBasis::Hexadecimal, ValueContent::SixteenBits(val))
                    },
                    EXP_HEXDECIMAL_CUSTOM | EXP_HEXDECIMAL_CUSTOM_LONG => {
                        let len = any.parse_next(input)? as usize;
                        let bytes: Vec<u8> = take(len).parse_next(input)?.to_vec();
                        (ValueBasis::Hexadecimal, ValueContent::Custom(bytes))
                    },

                    // binary
                    EXP_BINARY_8 => {
                        let val = any.parse_next(input)?;
                        (ValueBasis::Binary, ValueContent::EightBits(val))
                    },
                    EXP_BINARY_16 => {
                        let low = any.parse_next(input)? as u16;
                        let high = any.parse_next(input)? as u16;
                        let val = low | (high << 8);
                        (ValueBasis::Binary, ValueContent::SixteenBits(val))
                    },
                    EXP_BINARY_CUSTOM | EXP_BINARY_CUSTOM_LONG => {
                        let len = any.parse_next(input)? as usize;
                        let bytes: Vec<u8> = take(len).parse_next(input)?.to_vec();
                        (ValueBasis::Binary, ValueContent::Custom(bytes))
                    },
                    
                    _ => {
                        let mut err = ContextError::new();
                        err.push(StrContext::Expected(StrContextValue::Description("valid expression member")));
                        return Err(ErrMode::Backtrack(err))
                    }
                };

                Ok(ExpressionMember::Value(Value { basis, content }))
            }
        }
    }
}

/// Parse an assignment: 0x64 <label_ref> <expression>
fn parse_assign(input: &mut Input) -> OrgamsParseResult<Assign> {
    // Check marker and valid label ahead to differentiate from text containing 'd'
    let (_, next_byte) = peek((literal([MARKER_ASSIGN]), any)).parse_next(input)?;
    if next_byte < SHORT_LABEL_START {
        let mut err = ContextError::new();
        err.push(StrContext::Expected(StrContextValue::Description("A label is expected after assignment marker")));
        return Err(ErrMode::Cut(err));
    }

    // Expect assignment marker
    literal([MARKER_ASSIGN]).void().parse_next(input)?;

    // Read label reference (for now, just read bytes until we understand the encoding)
    // Looking at hex: 64 81 02 30 40 - after marker comes encoded data
    let label = cut_err(
        parse_label.context(StrContext::Expected(StrContextValue::Description(
            "A label is expected"
        )))
    )
    .parse_next(input)?;

    let expression = cut_err(parse_sized_expression.context(StrContext::Expected(
        StrContextValue::Description("An expression is expected")
    )))
    .parse_next(input)?;

    Ok(Assign { label, expression,})
}



/// Parse a standalone label reference (0x60-0xDF)
/// These appear at item boundaries and are standalone items
fn parse_label_ref_item(input: &mut Input) -> OrgamsParseResult<Item> {
    let label = parse_label.parse_next(input)?;
    Ok(Item::Label(label))
}

/// Parse a line according to orgams t_line grammar
/// Returns Line for the line (may include indent, content, newline)
pub fn parse_line(input: &mut Input) -> OrgamsParseResult<Line> {

    debug_slice(&input);

    // order is quite important here
    alt((
        parse_line_starting_with_comment.context(StrContext::Label("line starting with comment")),
        parse_line_starting_with_label.context(StrContext::Label("line starting with label")),
        parse_line_assign.context(StrContext::Label("line starting with assignment")),
        parse_line_empty.context(StrContext::Label("empty line")),
        parse_line_starting_with_macro.context(StrContext::Label("line starting with macro")),
        parse_line_starting_with_item.context(StrContext::Label("line starting with item")),
    ))
    .parse_next(input)
}

fn parse_nl_or_comment(input: &mut Input) -> OrgamsParseResult<Vec<Item>> {
    dbg!(&input[..10]);
    alt((
        parse_endline.map(|_| vec![Item::NewLine]),
        parse_space_and_comment
    ))
    .context(StrContext::Label("New line or comment"))
    .parse_next(input)
}

// The label MUST be followed by another token (not newline or comment)
fn parse_line_starting_with_label(input: &mut Input) -> OrgamsParseResult<Line> {
    consume_marker(MARKER_LABEL_ADDR)(input)?;
    let label = cut_err(parse_label.map(Item::Label).context(StrContext::Label("label"))).parse_next(input)?;
    
    let nl_or_comment = opt(parse_nl_or_comment).parse_next(input)?;
    let (item, nl_or_comment) = if let Some(nl_or_comment) = nl_or_comment {
        (None, nl_or_comment)
    } else {
        let item = dbg!(opt(parse_inner_item).parse_next(input))?; // TODO implement when there are several items on the same line
        let nl_or_comment = cut_err(parse_nl_or_comment).parse_next(input)?;
        (item, nl_or_comment)
    };

    let mut items = if let Some(item) = item {
        vec![label, item]
    } else {
        vec![label]
    };
    items.extend(nl_or_comment);
    Ok(Line { items })

}

fn parse_line_empty(input: &mut Input) -> OrgamsParseResult<Line> {
    let nl = parse_endline.parse_next(input)?;
    Ok(Line { items: vec![nl] })
}
fn parse_line_assign(input: &mut Input) -> OrgamsParseResult<Line> {
    let assign = parse_assign.map(Item::Assign).parse_next(input)?;
    let nl_or_comment = cut_err(parse_nl_or_comment).parse_next(input)?;

    let mut items = Vec::with_capacity(1 + nl_or_comment.len());
    items.push(assign);
    items.extend(nl_or_comment);

    Ok(Line { items })
}

fn parse_line_starting_with_macro(input: &mut Input) -> OrgamsParseResult<Line> {
    let macro_item = parse_macro_def_item.parse_next(input)?;
    let nl_or_comment = cut_err(parse_nl_or_comment).parse_next(input)?;
    let mut items = Vec::with_capacity(1 + nl_or_comment.len());
    items.push(macro_item);
    items.extend(nl_or_comment);
    Ok(Line { items })
}

fn parse_space_and_comment(input: &mut Input) -> OrgamsParseResult<Vec<Item>> {
    let space = opt(parse_indent).parse_next(input)?;
    let comment = parse_comment.parse_next(input)?;

    let items = if let Some(space) = space {
        vec![Item::Indent(space), Item::Comment(comment)]
    } else {
        vec![Item::Comment(comment)]
    };

    Ok(items)
}

fn parse_line_starting_with_comment(input: &mut Input) -> OrgamsParseResult<Line> {
    parse_space_and_comment.map(|items|Line{items}).parse_next(input)
}

fn parse_line_starting_with_item(input: &mut Input) -> OrgamsParseResult<Line> {


    let mut items: Vec<Item> = Vec::new();

    let mut first_loop = true;
    loop {
        dbg!("in loop and already collected", &items);
        debug_slice(input);
        let item = if first_loop {
            parse_inner_item.parse_next(input)?
        } else  {
            cut_err(parse_inner_item).parse_next(input)?
        };
        first_loop = false;
        items.push(item);

        // If we reached EOF for the chunk, emit a newline and stop
        if input.is_empty() {
            items.push(Item::NewLine);
            break;
        }

        // Peek next byte to decide whether to stop or continue parsing items on the same line
        // TODO handle the indent marker
        let next = peek(any).parse_next(input)?;
        if next == MARKER_NEWLINE {
            let nl = parse_endline.parse_next(input)?;
            items.push(nl);
            break;
        } else if next == MARKER_COMMENT {
            let c = parse_comment.parse_next(input)?;
            items.push(Item::Comment(c));
            break;
        } else {
            // There is another item on the same line; loop to parse it
            continue;
        }
    }

    Ok(Line { items })
}

fn parse_all_code(input: &mut Input) -> OrgamsParseResult<Vec<Chunk>> {
    let mut all_chunks = Vec::new();

    loop {
        // Peek to check for the terminator (chunk size 0)
        let b = peek(any).parse_next(input)?;
        if b == 0 {
             break;
        }

        let chunk = parse_chunk.parse_next(input)?;
        all_chunks.push(chunk);
    }

    Ok(all_chunks)
}


/// A chunk is composed of several lines, prefixed by its size in bytesa
/// TODO retreive the logic of parse_chunk_debug it may not be exactly the same now
pub fn parse_chunk(input: &mut Input) -> OrgamsParseResult<Chunk> {
    eprintln!("debug: parse_chunk start");
    let chunk_size =  any.verify(|&s| s > 0 && s<=CHUNK_MAX_SIZE).parse_next(input)?;
    let mut lines = Vec::new();

    let chunk_size = chunk_size as usize;
    eprintln!("debug: chunk size: {}", chunk_size);

    let input_start = input.checkpoint();
    while input.offset_from(&input_start) < chunk_size {
        eprintln!("debug: parsing line at offset {}", input.offset_from(&input_start));
        let line = parse_line.parse_next(input)?;
        lines.push(line);
    }

    if input.offset_from(&input_start) != chunk_size {
        eprintln!("Length mismatch! Expected: {}, Generated: {}", chunk_size, input.offset_from(&input_start));
        todo!("Handle chunk size mismatch and genrete the appropriate error");
    }

    Ok(Chunk { lines })
}



fn parse_escaped_7f_item(input: &mut Input) -> OrgamsParseResult<Item> {
    eprintln!("debug: parse_escaped_7f_item");
    // Expect escape marker
    consume_marker(MARKER_ESCAPE)(input)?;

    // get the command byte
    let cmd = cut_err(any).parse_next(input)?;
    eprintln!("debug: parse_escaped_7f_item cmd=0x{:02x}", cmd);

    match cmd {
        CMD_IF => {
             let condition = cut_err(parse_sized_expression).parse_next(input)?;
             Ok(Item::Statement(Statement::If(condition)))
        },
        CMD_ELSE => {
             Ok(Item::Statement(Statement::Else))
        },
        CMD_END => {
             Ok(Item::Statement(Statement::End))
        },
        CMD_ENDM => {
             Ok(Item::Statement(Statement::EndMacro))
        },
        CMD_IMPORT => {
             // Replaced by top-level parse_import in most cases.
             // If seen here, treat as TODO or same logic?
             // The error trace showed raw 0x17. 
             // If we must support escaped 0x17, assume string?
             let s = cut_err(parse_sized_text).parse_next(input)?;
             Ok(Item::Statement(Statement::Import(s.0, true)))
        },
        CMD_ENT => {
            let expr = cut_err(parse_sized_expression).parse_next(input)?;
            Ok(Item::Statement(Statement::Ent(expr)))
        }
        CMD_ORG => {
             let expr = cut_err(parse_sized_expression).parse_next(input)?;
             Ok(Item::Statement(Statement::Org(expr)))
        },
        CMD_ORG2 => {
             let expr1 = cut_err(parse_sized_expression).parse_next(input)?;
             let expr2 = cut_err(parse_sized_expression).parse_next(input)?;
             Ok(Item::Statement(Statement::Org2(expr1, expr2)))
        },

        CMD_SKIP => {
                let expr = cut_err(parse_sized_expression).parse_next(input)?;
                Ok(Item::Statement(Statement::Skip(expr)))
        }
        CMD_STORE_PC_INSTR => {
             Ok(Item::Statement(Statement::StorePcInstr))
        },
        CMD_END_BIS => {
            Ok(Item::Statement(Statement::EndBis))
        },
        CMD_ASIS => {
             // Raw string: Length + Bytes
             let len = cut_err(any).parse_next(input)? as usize;
             let content = cut_err(take(len)).parse_next(input)?;
             Ok(Item::Statement(Statement::RawString(
                 String::from_utf8_lossy(content).to_string()
             )))
        },
        CMD_REPEAT => {
            cut_err(parse_star_repeat).parse_next(input)
        }, 
        CMD_MACRO_USE => { 
            cut_err(parse_macro_use).parse_next(input).map(Item::Statement)
        },

        _ => {
            let mut err = ContextError::new();
            err.push(StrContext::Expected(StrContextValue::Description("Unknown escaped command")));
            Err(ErrMode::Cut(err))
        }
    }
}


fn parse_instruction(input: &mut Input) -> OrgamsParseResult<Instruction> {
    let b = dbg!(any.parse_next(input)?);
    if b == IX_CODE || b == IY_CODE {
        cut_err(parse_indexed_instruction(b).context(StrContext::Expected(StrContextValue::Description("Indexed instruction")))).parse_next(input)
    } else {
        let mut err = ContextError::new();
        err.push(StrContext::Expected(StrContextValue::Description("Non-indexed instruction parsing not implemented yet")));
        Err(ErrMode::Backtrack(err))
    }
}





/// Parse the arguments of a macro definition.
/// They are contained in the remaining bytes of the definition block.
/// TODO rework macro parsing because it is not coherent
fn parse_macro_args(input: &mut Input) -> OrgamsParseResult<Vec<LabelRef>> {
    let mut params = Vec::new();
    while !input.is_empty() {
         let b = peek(any).parse_next(input)?;
         
         // TODO check whats taht
         if b == END_MARKER {
             cut_err(any).parse_next(input)?;
             continue;
         }

         if b >= SHORT_LABEL {
             let p = cut_err(parse_label).parse_next(input)?;
             params.push(p);
         } else {
             // Unexpected byte in macro definition block?
             // As we are length-bounded, we just consume it.
             cut_err(any).parse_next(input)?;
         }
    }
    Ok(params)
}

/// Parse macro header: 0x60 <len> <name> <params...>
/// Returns Item::Macro. The body follows in the stream.
fn parse_macro_def_item(input: &mut Input) -> OrgamsParseResult<Item> {
    eprintln!("debug: parse_macro_def");
    // 1. Marker
    literal([MARKER_MACRO_DEF]).parse_next(input)?;
    // 2. Length (def_block_len)
    let def_block_len = cut_err(any).parse_next(input)?;
    
    // 3. Take definition content bytes
    let content_bytes = cut_err(take(def_block_len)).parse_next(input)?;

    // 4. Parse Name and Params from content
    let mut sub_input = LocatingSlice::new(content_bytes);
    
    let name = cut_err(parse_label).parse_next(&mut sub_input)?;
    let params = cut_err(parse_macro_args).parse_next(&mut sub_input)?;

    // Do NOT consume newline here as requested.

    Ok(Item::Macro(MacroDef {
        def_block_len,
        name,
        params,
    }))
}

fn expect_end_marker(input: &mut Input) -> OrgamsParseResult<()> {
    cut_err(consume_marker(END_MARKER).context(StrContext::Expected(StrContextValue::Description("Missing end marker")))).parse_next(input)
}

/// Parse macro call: length + name + args + endmarker
fn parse_macro_use(input: &mut Input) -> OrgamsParseResult<Statement> {
    let input_start = input.checkpoint();
    let length = cut_err(any).parse_next(input)? as usize;

    
    //let name = dbg!(parse_expression.parse_next(input))?;
    let mut args: Vec<Expression> = cut_err(repeat(1.., parse_unsized_expression)).parse_next(input)?;
    

    let bytes_consumed = input.offset_from(&input_start);
    if bytes_consumed != length {
        let mut err = ContextError::new();
        err.push(StrContext::Expected(StrContextValue::Description("Wrong macro length consummed")));
        return Err(ErrMode::Cut(err));
    }

    expect_end_marker(input)?;

    let name = args.remove(0);

    Ok(Statement::MacroUse(name, args))
}

/// Parse a text prefixed by its size in bytes
fn parse_sized_text(input: &mut Input) -> OrgamsParseResult<SizedString> {
    // Read size byte
    let size = any.parse_next(input)? as usize;
    let text_bytes: Vec<u8> = cut_err(take(size)).parse_next(input)?.to_vec();
    Ok(SizedString(
        String::from_utf8_lossy(&text_bytes).to_string()
    ))
}

fn parse_bit7on_text(input: &mut Input) -> OrgamsParseResult<Bit7OnString> {
    let mut bytes = Vec::new();
    loop {
        let byte: u8 = cut_err(any.context(StrContext::Expected(StrContextValue::Description("bit7on text byte")))).parse_next(input)?;
        bytes.push(byte);
        if byte & (1 << 7) != 0 {
            break;
        }
    }
    Ok(Bit7OnString::new(&bytes))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn test_parse_line_with_if_else() {
        // [7F, 09, 01, 61, 7F, 0A, 4A]
        // IF (label 61) ELSE \n
        let data = vec![0x7F, 0x09, 0x01, 0x61, 0x7F, 0x0A, 0x4A];
        let mut input = LocatingSlice::new(data.as_slice());
        
        let result = parse_line.parse_next(&mut input);
        assert!(result.is_ok());
        let items = result.unwrap();
        
        // We want it to parse [If, Else, NewLine] and consume everything.
        assert_eq!(items.len(), 3); 
        assert!(input.is_empty());
    }

    #[test]
    fn test_parse_comment_simple() {
        // Simple comment: 0x43 + size byte + text
        let data = b"\x43\x0dHello, World!";
        let mut input = LocatingSlice::new(&data[..]);

        let result = parse_comment(&mut input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "Hello, World!");
    }

    #[test]
    fn test_parse_comment_with_newline() {
        // Comment followed by newline: 0x43 + size + text + newline
        let data = b"\x43\x11This is a comment\x4aNext line";
        let mut input = LocatingSlice::new(&data[..]);

        let result = parse_comment(&mut input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "This is a comment");

        // Newline should be next
        assert_eq!(input[0], MARKER_NEWLINE);
    }

    #[test]
    fn test_parse_comment_from_const_i() {
        // Real comment from CONST.I: C + sized text " <<<< Constants shared across modules >>>>"
        // Size byte 0x2a (42) followed by 42 bytes of text starting with space
        let data = b"\x43\x2a <<<< Constants shared across modules >>>>";
        let mut input = LocatingSlice::new(&data[..]);

        let result = parse_comment(&mut input);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().as_str(),
            " <<<< Constants shared across modules >>>>"
        );
    }

    #[test]
    fn test_parse_newline() {
        let data = b"\x4aNext content";
        let mut input = LocatingSlice::new(&data[..]);

        let result = parse_endline(&mut input);
        assert!(result.is_ok());

        // Should have consumed the newline
        assert_eq!(input[0], b'N');
    }

    #[test]
    fn test_parse_multiterm_expression() {
        // Test parsing: max_symbols + spc - 1 / spc
        // which encodes as multi-term with labels, operators, spaces
        let expr_content = vec![
            0x42, // EXP_MULTI_TERM_BEGIN
            0x8a, // label (138 - 96 = 42)
            0x20, // space
            0x2b, // + operator
            0x20, // space
            0x87, // label (135 - 96 = 39)
            0x20, // space
            0x2d, // - operator
            0x20, // space
            0x01, // short decimal 1
            0x20, // space
            0x2f, // / operator
            0x20, // space
            0x87, // label (135 - 96 = 39)
            0x45, // EXP_MULTI_TERM_END
        ];
        
        // Create full expression bytes with size prefix
        let mut expr_bytes = vec![expr_content.len() as u8];
        expr_bytes.extend_from_slice(&expr_content);
        
        let mut input = LocatingSlice::new(&expr_bytes[..]);
        let expr = parse_sized_expression(&mut input).expect("Failed to parse multi-term expression");
        
        // Create empty string table for serialization
        let table = StringTable::default();
        
        // Verify it round-trips correctly (bytes() includes size prefix)
        let serialized = expr.bytes(&table);
        assert_eq!(serialized, expr_bytes, "Expression should round-trip correctly including size byte");
        
        // Verify structure
        match expr {
            SizedExpression(Expression::MultiTerm(members)) => {
                assert_eq!(members.len(), 13, "Multi-term should have 13 members");
            },
            _ => panic!("Expected MultiTerm expression")
        }
    }

    fn verify_parsing_and_reconstruction(path: &str) {
        let data = fs::read(path).unwrap();
        let input = LocatingSlice::new(data.as_slice());

        let result = parse_orgams_file.parse(input);

        assert!(
            result.is_ok(),
            "Failed to parse {}: {:?}",
            path,
            result.err()
        );

        let program = result.unwrap();

        let generated_bytes = program.bytes();

        // 1. Offset where items start: 0x67 (ORGA block) + 4 (SRCc) + 1 (version) = 0x6C
        let start_items = 0x6C;
        
        // 3. Find ChCk checksum to verify "up to ChCk"
        let expected_limit = data
            .windows(4)
            .position(|w| w == b"ChCk")
            .unwrap_or(data.len());
        
        let generated_limit = generated_bytes
             .windows(4)
             .position(|w| w == b"ChCk")
             .unwrap_or(generated_bytes.len());

        // Compare up to Checksum (excluding Header)
        let generated_payload = &generated_bytes[start_items..generated_limit];
        let expected_payload = &data[start_items..expected_limit];
        
        if generated_payload.len() != expected_payload.len() {
             println!("Length mismatch! Expected: {}, Generated: {}", 
                expected_payload.len(), 
                generated_payload.len());
        }

        // Better error message: find first diff
        if let Some(pos) = generated_payload.iter().zip(expected_payload.iter()).position(|(a, b)| a != b) {
            let abs_pos = start_items + pos;
            panic!(
                "Byte mismatch at offset {}: expected 0x{:02X}, got 0x{:02X}\nOriginal: {:?}\nGenerated: {:?}", 
                abs_pos, expected_payload[pos], generated_payload[pos],
                &expected_payload[pos.saturating_sub(10)..std::cmp::min(pos+10, expected_payload.len())],
                &generated_payload[pos.saturating_sub(10)..std::cmp::min(pos+10, generated_payload.len())]
            );
        }
        
        assert_eq!(generated_payload.len(), expected_payload.len(), "Payload length mismatch");
    }

    #[test]
    #[ignore] // ignored while we test only with debug_load_orgams
    fn test_parse_const_i() {
        verify_parsing_and_reconstruction("tests/orgams-main/CONST.I");
    }


    #[test]
    #[ignore] // ignored while we test only with debug_load_orgams
    fn test_parse_memmap_i() {
        verify_parsing_and_reconstruction("tests/orgams-main/MEMMAP.I");
    }


    #[test]
    fn test_issue_multiple_comments_one_line() {
         // This sequence comes from a memory dump where multiple comments appear consecutively
         // ending with a single newline. The parser should treat each comment as a separate line.
         let data: &[u8] = &[
            0x43, 0x2A, 0x20, 0x3C, 0x3C, 0x3C, 0x3C, 0x20, 0x43, 0x6F, 0x6E, 0x73, 0x74, 0x61, 0x6E, 0x74, 0x73, 0x20, 0x73, 0x68, 0x61, 0x72, 0x65, 0x64, 0x20, 0x61, 0x63, 0x72, 0x6F, 0x73, 0x73, 0x20, 0x6D, 0x6F, 0x64, 0x75, 0x6C, 0x65, 0x73, 0x20, 0x3E, 0x3E, 0x3E, 0x3E, 
            0x43, 0x2B, 0x20, 0x32, 0x30, 0x32, 0x35, 0x20, 0x4A, 0x75, 0x6C, 0x20, 0x32, 0x37, 0x3A, 0x20, 0x4D, 0x6F, 0x76, 0x65, 0x20, 0x73, 0x74, 0x6F, 0x72, 0x65, 0x20, 0x6C, 0x65, 0x6E, 0x67, 0x74, 0x68, 0x73, 0x20, 0x74, 0x6F, 0x20, 0x73, 0x77, 0x61, 0x70, 0x69, 0x2E, 0x69, 
            0x49, 0x05, 0x43, 0x2C, 0x20, 0x4A, 0x75, 0x6E, 0x20, 0x31, 0x31, 0x3A, 0x20, 0x55, 0x70, 0x64, 0x61, 0x74, 0x65, 0x20, 0x61, 0x73, 0x73, 0x5F, 0x73, 0x74, 0x6F, 0x72, 0x65, 0x5F, 0x6C, 0x65, 0x6E, 0x20, 0x73, 0x79, 0x6D, 0x62, 0x5F, 0x73, 0x74, 0x6F, 0x72, 0x65, 0x5F, 0x6C, 0x65, 0x6E, 
            0x49, 0x0D, 0x43, 0x17, 0x20, 0x52, 0x65, 0x6D, 0x6F, 0x76, 0x65, 0x20, 0x6F, 0x62, 0x73, 0x6F, 0x6C, 0x65, 0x74, 0x65, 0x20, 0x63, 0x68, 0x65, 0x63, 0x6B, 0x73, 
            0x4A
        ];

        let mut input = LocatingSlice::new(data);
        let mut lines = Vec::new();
        loop {
             if input.is_empty() { break; }
             match parse_line(&mut input) {
                 Ok(line) => {
                    //println!("Parsed line: {:?}", line);
                    lines.push(line);
                 },
                 Err(_) => break
             }
        }
        
        assert_eq!(lines.len(), 5, "Should parse 5 lines. Got: {:?}", lines);
    }

    #[test]
    fn test_comment_then_newline_split() {
        // [49, 0D, 43, 17, ... text ..., 4A]
        // Should be Line(Indent, Comment) then Line(NewLine)
        let data: &[u8] = &[
            0x49, 0x0D, 
            0x43, 0x17, 0x20, 0x52, 0x65, 0x6D, 0x6F, 0x76, 0x65, 0x20, 0x6F, 0x62, 0x73, 0x6F, 0x6C, 0x65, 0x74, 0x65, 0x20, 0x63, 0x68, 0x65, 0x63, 0x6B, 0x73, 
            0x4A
        ];

        let mut input = LocatingSlice::new(data);
        
        // Line 1
        let line1 = parse_line(&mut input).expect("Failed to parse line 1");
        assert_eq!(line1.items.len(), 2);
        assert!(matches!(line1.items[1], Item::Comment(_)));

        // Line 2
        let line2 = parse_line(&mut input).expect("Failed to parse line 2");
        assert_eq!(line2.items.len(), 1);
        assert!(matches!(line2.items[0], Item::NewLine));
    }
    #[test]
    #[ignore]
    fn test_parse_macro_i() {
        verify_parsing_and_reconstruction("tests/orgams-main/MACRO.I");
    }
}

