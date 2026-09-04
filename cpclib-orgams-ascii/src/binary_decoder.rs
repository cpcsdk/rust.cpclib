//! Winnow-based parser for Orgams binary format
//!
//! This parser uses winnow combinators and models semantic structures.

use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::ops::Deref;

use cpclib_common::itertools::Itertools;
use cpclib_common::smallvec::SmallVec;
use cpclib_common::winnow::combinator::{cut_err, repeat};
use cpclib_common::winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use cpclib_common::winnow::stream::Offset;
use cpclib_common::winnow::{self, LocatingSlice};
use cpclib_tokens::opcode_table::{
    TABINSTR, TABINSTRCB, TABINSTRDD, TABINSTRDDCB, TABINSTRED, TABINSTRFD, TABINSTRFDCB
};
use winnow::combinator::{alt, opt, peek};
use winnow::prelude::*;
use winnow::token::{any, literal, take};

/// Byte-offset-tracking input stream consumed by every parser in this module.
pub type Input<'a> = LocatingSlice<&'a [u8]>;
/// Result type returned by every winnow parser in this module.
pub type OrgamsParseResult<T> = ModalResult<T>;

const CHUNK_MAX_SIZE: u8 = 222;

// Marker bytes
const MARKER_NEWLINE: u8 = 0x4A; // 'J'
const MARKER_INDENT: u8 = 0x49; // 'I'
const MARKER_ESCAPE: u8 = 0x7F; // escape code for commands
const MARKER_COMMENT: u8 = 0x43; // 'C' - introduces a comment
const MARKER_ASSIGN: u8 = 0x64; // 'd' - introduces an assignment
const MARKER_WORD: u8 = 0xD7;
const MARKER_BYTE: u8 = 0xCF;
const MARKER_LOCAL_LABEL: u8 = 0x51;
const MARKER_LABEL_ADDR: u8 = 0x40;
const MARKER_MACRO_DEF: u8 = 0x6D;
const MARKER_IX_IND: u8 = 0xDF;
const MARKER_IY_IND: u8 = 0xFF;

const fn is_escaped_byte(b: u8) -> bool {
    // Return false for known command bytes and markers (these are not instructions
    // and should be treated as commands when following an escape), true otherwise.
    match b {
        // Command opcodes (escaped commands)
        MARKER_ASSIGN | MARKER_BYTE | MARKER_COMMENT | MARKER_ESCAPE | MARKER_INDENT
        | MARKER_LABEL_ADDR | MARKER_LOCAL_LABEL | MARKER_MACRO_DEF | MARKER_NEWLINE
        | MARKER_WORD | 0x52 | 0x58 | 0x5B => true,
        _ => false
    }
}

/// Byte sequence that precedes a Z80 opcode byte and selects which disassembly/encoding table
/// applies to it (standard, `IX`/`IY`-indexed, extended, or bit/rotate instructions).
#[derive(Debug, Clone, PartialEq)]
pub enum InstructionPrefix {
    /// No prefix: a plain, unprefixed opcode.
    None,
    /// `0xDD` prefix: `IX`-indexed instruction (e.g. `LD A,(IX+n)`).
    DD,
    /// `0xFD` prefix: `IY`-indexed instruction (e.g. `LD A,(IY+n)`).
    FD,
    /// `0xDD 0xCB` prefix: `IX`-indexed bit/rotate/shift instruction.
    DDCB,
    /// `0xFD 0xCB` prefix: `IY`-indexed bit/rotate/shift instruction.
    FDCB,
    /// Orgams-specific `0xDF` prefix: an `IX`-indirect instruction encoded with an implicit
    /// zero offset, displayed as `(ix)` rather than `(ix+0)`.
    DF,
    /// Orgams-specific `0xFF` prefix: the `IY`-indirect equivalent of [`InstructionPrefix::DF`],
    /// displayed as `(iy)` rather than `(iy+0)`.
    FF,
    /// Orgams-specific `0xDF 0xCB` prefix: the bit/rotate/shift counterpart of
    /// [`InstructionPrefix::DF`].
    DFCB,
    /// Orgams-specific `0xFF 0xCB` prefix: the bit/rotate/shift counterpart of
    /// [`InstructionPrefix::FF`].
    FFCB,
    /// `0xCB` prefix: bit/rotate/shift instruction.
    CB,
    /// `0xED` prefix: extended instruction.
    ED
}

impl InstructionPrefix {
    /// Maps a single prefix byte (`0xDD`, `0xFD`, `0xDF`, `0xFF`, `0xCB` or `0xED`) to its
    /// [`InstructionPrefix`] variant.
    ///
    /// # Panics
    /// Panics if `b` is not one of the recognized prefix bytes.
    pub fn from_byte(b: u8) -> Self {
        match b {
            IX_CODE => InstructionPrefix::DD,
            IY_CODE => InstructionPrefix::FD,
            0xDF => InstructionPrefix::DF,
            0xFF => InstructionPrefix::FF,
            0xCB => InstructionPrefix::CB,
            0xED => InstructionPrefix::ED,
            _ => unreachable!("Invalid instruction prefix byte: 0x{:02X}", b)
        }
    }

    /// Maps a two-byte prefix (`0xDD 0xCB`, `0xFD 0xCB`, `0xDF 0xCB` or `0xFF 0xCB`) to its
    /// [`InstructionPrefix`] variant.
    ///
    /// # Panics
    /// Panics if `(first, second)` is not one of the recognized two-byte prefixes.
    pub fn from_bytes(first: u8, second: u8) -> Self {
        match (first, second) {
            (IX_CODE, 0xCB) => InstructionPrefix::DDCB,
            (IY_CODE, 0xCB) => InstructionPrefix::FDCB,
            (0xDF, 0xCB) => InstructionPrefix::DFCB,
            (0xFF, 0xCB) => InstructionPrefix::FFCB,
            _ => {
                unreachable!(
                    "Invalid instruction prefix bytes: 0x{:02X} 0x{:02X}",
                    first, second
                )
            }
        }
    }

    /// Returns the raw prefix byte(s) that must be emitted before the opcode byte, or an empty
    /// slice for [`InstructionPrefix::None`].
    pub fn bytes(&self) -> &[u8] {
        match self {
            InstructionPrefix::None => &[],
            InstructionPrefix::DD => &[IX_CODE],
            InstructionPrefix::DF => &[0xDF],
            InstructionPrefix::FD => &[IY_CODE],
            InstructionPrefix::FF => &[0xFF],
            InstructionPrefix::DDCB => &[IX_CODE, 0xCB],
            InstructionPrefix::DFCB => &[0xDF, 0xCB],
            InstructionPrefix::FDCB => &[IY_CODE, 0xCB],
            InstructionPrefix::FFCB => &[0xFF, 0xCB],
            InstructionPrefix::CB => &[0xCB],
            InstructionPrefix::ED => &[0xED]
        }
    }

    /// Returns `true` unless this is [`InstructionPrefix::None`].
    pub fn is_prefix(&self) -> bool {
        !matches!(self, InstructionPrefix::None)
    }

    /// Returns `true` if `b` is a byte that introduces an instruction prefix (`IX`/`IY` index
    /// bytes, the Orgams `IX`/`IY`-indirect markers, `0xED` or `0xCB`).
    pub const fn is_valid_prefix(b: u8) -> bool {
        matches!(
            b,
            IX_CODE | IY_CODE | 0xED | 0xCB | MARKER_IX_IND | MARKER_IY_IND
        )
    }

    /// Returns the 256-entry Z80 disassembly table (indexed by opcode byte) that applies for
    /// this prefix.
    pub const fn disassembler_table(&self) -> &'static [&'static str; 256] {
        match self {
            InstructionPrefix::None => &TABINSTR,
            InstructionPrefix::DD | InstructionPrefix::DF => &TABINSTRDD,
            InstructionPrefix::FD | InstructionPrefix::FF => &TABINSTRFD,
            InstructionPrefix::DDCB | InstructionPrefix::DFCB => &TABINSTRDDCB,
            InstructionPrefix::FDCB | InstructionPrefix::FFCB => &TABINSTRFDCB,
            InstructionPrefix::CB => &TABINSTRCB,
            InstructionPrefix::ED => &TABINSTRED
        }
    }

    /// Returns `true` if, under this prefix, `opcode` is encoded with a trailing operand
    /// expression beyond the usual `nn`/`nnnn` placeholders (e.g. the bit index of a `CB`
    /// `BIT`/`RES`/`SET` instruction, or an `RST` restart vector).
    pub const fn requires_extra_expression(&self, opcode: u8) -> bool {
        match self {
            InstructionPrefix::CB | InstructionPrefix::DF | InstructionPrefix::FF => {
                is_cb_opcode_with_extra_expression(opcode)
            },
            InstructionPrefix::None => is_standard_opcode_with_extra_expression(opcode),
            _ => false
        }
    }

    /// Returns `true` for the Orgams-specific `DF`/`FF`/`DFCB`/`FFCB` prefixes, which encode
    /// `IX`/`IY`-indirect addressing with an implicit zero offset (`(ix)`/`(iy)`) rather than
    /// the standard `(ix+n)`/`(iy+n)` form.
    pub const fn is_orgams_ix_iy_indirect(&self) -> bool {
        matches!(
            self,
            InstructionPrefix::DF
                | InstructionPrefix::FF
                | InstructionPrefix::DFCB
                | InstructionPrefix::FFCB
        )
    }
}

const fn is_cb_opcode_with_extra_expression(b: u8) -> bool {
    matches!(b,
        0x40..=0x47 | // BIT 0, r
        0x80..=0x87 | // RES 0, r
        0xC0..=0xC7   // SET 0, r
    )
}

const fn is_standard_opcode_with_extra_expression(b: u8) -> bool {
    matches!(
        b, 0xC7 // RST
    )
}

const TAB_INSTR: u8 = 10;
const TAB_COMMAND: u8 = 6;
const TAB_COMMENT: u8 = 24;

// Label reference ranges (from ORGAMS format docs)
const SHORT_LABEL_START: u8 = 0x60; // Short labels: 0x60-0xDF (128 labels)
const SHORT_LABEL_END: u8 = 0xDF;
const LONG_LABEL_START: u8 = 0xE0; // Long labels: 0xE0-0xFF (256 labels)

const EXP_MULTI_TERM_BEGIN: u8 = 0x42; // 'B'
const EXP_MULTI_TERM_END: u8 = 0x45; // 'E'
const EXP_SHORT_DECIMAL_MAX_VALUE: u8 = 0x1F; // 0x00-0x1F

const EXP_ITER1: u8 = b'I';
const EXP_ITER2: u8 = b'J';
const EXP_ITER3: u8 = b'K';

const EXP_SPACE: u8 = 0x20;
const EXP_UNARY_MINUS: u8 = b'#';
const EXP_LOCAL_LABEL: u8 = b'.';
const EXP_OP_PLUS: u8 = 0x2B;
const EXP_OP_MINUS: u8 = 0x2D;
const EXP_OP_TIMES: u8 = 0x2A;
const EXP_OP_DIV: u8 = 0x2F;
const EXP_OP_MOD: u8 = 0x25;
const EXP_OP_PAREN_OPEN: u8 = b'(';
const EXP_OP_PAREN_CLOSE: u8 = b')';

const EXP_OP_LT: u8 = b'<';
const EXP_OP_EQ: u8 = b'=';
const EXP_OP_GT: u8 = b'>';

const EXP_OP_LE: u8 = b'L';
const EXP_OP_GE: u8 = b'M';
const EXP_OP_NEQ: u8 = b'N';

// Reverse-engineered expression-member marker for "no data" (used by a bare .byte/.word
// directive with no operand). No sample file in the test corpus exercises this byte, so the
// parser does not yet special-case it; kept here as a record of the known format encoding.
#[allow(dead_code)]
const EXP_NONE: u8 = b'?'; // no data (.byte ou .word seul)

const EXP_OP_OR: u8 = b'@';
const EXP_OP_XOR: u8 = b'!';
const EXP_OP_AND: u8 = 0x26; // '&';

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

const EXP_STRING: u8 = 0x22; // '"'

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
// The following escaped command opcodes were identified while reverse-engineering the format
// (their positions in the 0x7F command numbering are consistent with the ones around them) but
// no sample file in the test corpus contains them, so `parse_escaped_7f_item` does not yet
// implement a `Statement` variant for them. Kept here as a record of the known format encoding.
#[allow(dead_code)]
const CMD_BRK_SET: u8 = 17;
const CMD_RESTORE: u8 = 18;
#[allow(dead_code)]
const CMD_BANK: u8 = 19;
const CMD_ENDM: u8 = 0x14; // Renamed from CMD_MACRO
const CMD_MACRO_USE: u8 = 0x15; // 0x15 used to be called Endm in decoder2, but decoder.rs calls it MacroUse or If. Assuming unused for now.
#[allow(dead_code)]
const CMD_LOAD: u8 = 22;
const CMD_IMPORT: u8 = 0x17; // This seems to be the escaped version?
#[allow(dead_code)]
const CMD_STR: u8 = 24;
#[allow(dead_code)]
const CMD_SAVE: u8 = 25;
#[allow(dead_code)]
const CMD_SAVEA: u8 = 26;
const CMD_REPEAT: u8 = 0x5B;

const IX_CODE: u8 = 0xDD;
const IY_CODE: u8 = 0xFD;

const END_MARKER: u8 = 0x41;

#[inline]
fn consume_marker(marker: u8) -> impl Fn(&mut Input) -> OrgamsParseResult<()> + 'static {
    move |input: &mut Input| -> OrgamsParseResult<()> { literal(marker).void().parse_next(input) }
}

/// Main program structure
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// Source chunks, in file order. Each chunk was originally stored as a size-prefixed block
    /// in the `SRCc` section of the binary file.
    pub chunks: Vec<Chunk>,
    /// The label string table (`LBLs` section) shared by every label reference in `chunks`.
    pub labels: StringTable
}

/// A single source line: the sequence of [`Item`]s between two newline markers (the trailing
/// [`Item::NewLine`], if any, is included as the last item).
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// The items making up this line, in stream order.
    pub items: Vec<Item>
}

impl Deref for Line {
    type Target = Vec<Item>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl Line {
    /// Re-encodes this line's items back to their original binary representation.
    pub fn bytes(&self, labels: &StringTable) -> Vec<u8> {
        self.items
            .iter()
            .flat_map(|i| i.bytes(labels).into_iter())
            .collect()
    }

    /// Iterates over this line's items in stream order.
    pub fn iter(&self) -> impl Iterator<Item = &Item> {
        self.items.iter()
    }
}

/// A size-prefixed block of source [`Line`]s, as stored in the `SRCc` section of the binary
/// file (each chunk is at most `CHUNK_MAX_SIZE` bytes once re-encoded).
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    /// The lines making up this chunk, in stream order.
    pub lines: Vec<Line>
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
        let mut bytes: Vec<u8> = self
            .lines
            .iter()
            .flat_map(|line| line.items.iter())
            .flat_map(|item| item.bytes(labels).into_iter())
            .collect();
        let computed_size = bytes.len() as u8;
        bytes.insert(0, computed_size); // prepend size byte
        bytes
    }

    /// Iterates over every item in this chunk, across all its lines, in stream order.
    pub fn items(&self) -> impl Iterator<Item = &Item> {
        self.lines.iter().flat_map(|line| line.items.iter())
    }
}

impl Program {
    /// Iterates over every item in the program, across all chunks and lines, in stream order.
    pub fn items(&self) -> impl Iterator<Item = &Item> {
        self.chunks.iter().flat_map(|chunk| chunk.items())
    }

    /// Re-encodes the whole program (header, source chunks, label table and checksum
    /// placeholder) back to its original binary file layout.
    pub fn bytes(&self) -> Vec<u8> {
        self.header_bytes()
            .into_iter()
            .chain(self.src_bytes())
            .chain(self.labels_bytes())
            .chain(self.checksum_bytes())
            .collect()
    }

    /// Get the bytes of the source part
    pub fn src_bytes(&self) -> Vec<u8> {
        self.chunks
            .iter()
            .flat_map(|chunk| chunk.bytes(&self.labels).into_iter())
            .chain(vec![0u8]) // null terminator
            .collect()
    }

    /// Builds the file header: the `ORGA` magic, a zero-filled header body up to offset
    /// `0x67`, and the `SRCc` section magic plus its version byte.
    pub fn header_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"ORGA");
        bytes.extend_from_slice(&[0u8; 0x67 - 4]); // rest of header
        bytes.extend_from_slice(b"SRCc");
        bytes.push(2); // version
        bytes
    }

    /// Builds the `LBLs` label table section: its magic, version byte, every label string
    /// (each already bit7-terminated by [`Bit7OnString::bytes`]), and a null terminator.
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

    /// Builds the trailing `ChCk` checksum section. The 4-byte checksum itself is not computed;
    /// it is left as a zero placeholder.
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
    /// Standalone local label reference (rendered with a leading `.`)
    LocalLabel(LabelRef),
    /// Macro definition
    MacroDef(MacroDef),
    /// Statement
    Statement(Statement)
}

/// Number of spaces of indentation (the byte following an indent marker, `0x49`).
#[derive(Debug, Clone, PartialEq)]
pub struct Indent(pub u8);

/// Comment text, as introduced by the `;` marker in Orgams source (without the leading `;`).
#[derive(Debug, Clone, PartialEq)]
pub struct Comment(pub SizedString);

impl Deref for Comment {
    type Target = SizedString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Represent a string where the bit 7 is on on the last char.
/// The stored string DOES NOT have the bit 7 on.
#[derive(Debug, Clone, PartialEq)]
pub struct Bit7OnString(OrgamsEncodedString);

impl Bit7OnString {
    /// Builds a `Bit7OnString` from raw bytes whose last byte has bit 7 set (as read from the
    /// file). The stored content has that bit cleared; call [`Bit7OnString::bytes`] to get the
    /// bit-7-set form back. Only ASCII strings are expected as input.
    pub fn new(bytes: &[u8]) -> Self {
        let mut content = bytes.to_vec();
        if let Some(last) = content.last_mut() {
            *last &= !(1 << 7); // clear high bit
        }
        Self(OrgamsEncodedString(content))
    }

    /// Returns the raw bytes with bit 7 set on the last byte, as stored in the file.
    pub fn bytes(&self) -> Vec<u8> {
        let mut bytes = self.0.as_bytes().to_vec();
        if let Some(last) = bytes.last_mut() {
            *last |= 1 << 7; // set bit 7 on last char
        }
        bytes
    }
}

impl Deref for Bit7OnString {
    type Target = OrgamsEncodedString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A raw byte string as stored in the Orgams format, encoded with the Windows-1252 codepage
/// (see the [`Display`] impl, which decodes it for rendering).
#[derive(Debug, Clone, PartialEq)]
pub struct OrgamsEncodedString(Vec<u8>);

impl Deref for OrgamsEncodedString {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for OrgamsEncodedString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let res = encoding_rs::WINDOWS_1252.decode(&self.0).0;
        res.fmt(f)
    }
}

impl OrgamsEncodedString {
    /// Returns the raw Windows-1252-encoded bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// A string preceded in the binary stream by a one-byte length prefix (e.g. comment and string
/// literal text).
#[derive(Debug, Clone, PartialEq)]
pub struct SizedString(OrgamsEncodedString);
impl Deref for SizedString {
    type Target = OrgamsEncodedString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq<str> for Bit7OnString {
    fn eq(&self, other: &str) -> bool {
        self.0.to_string() == other
    }
}

impl PartialEq<&str> for Bit7OnString {
    fn eq(&self, other: &&str) -> bool {
        self.0.to_string() == *other
    }
}

impl SizedString {
    /// Re-encodes this string as a length-prefixed byte sequence: one byte holding the string's
    /// length, followed by its raw content bytes.
    pub fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.0.len() + 1);
        bytes.push(self.0.len() as u8);
        bytes.extend_from_slice(&self.0);
        bytes
    }
}

impl Item {
    /// Re-encodes this item back to its original binary representation.
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
            Item::LocalLabel(label) => {
                let mut bytes = Vec::new();
                bytes.push(MARKER_LOCAL_LABEL);
                bytes.extend_from_slice(&label.bytes(table));
                bytes
            },
            Item::MacroDef(m) => m.bytes(table),
            Item::Statement(s) => s.bytes(table)
        }
    }

    /// Renders this item as Orgams source text (as it would appear inside a line), resolving
    /// any label reference against `labels`.
    ///
    /// Returns `Err` if a label reference in this item is out of range for `labels`.
    pub fn display<'i>(&'i self, labels: &StringTable) -> Result<Cow<'i, str>, String> {
        Ok(match self {
            Item::Comment(text) => format!(";{}", ***text).into(),
            Item::NewLine => "\n".into(),
            Item::Indent(count) => " ".repeat(count.0 as usize).into(),
            Item::Assign(assign) => {
                let label = labels.label(&assign.label).ok_or_else(|| {
                    format!(
                        "label reference {:?} is out of range for the label table",
                        assign.label
                    )
                })?;
                let expr_repr = assign.expression.display(labels)?;

                // Heuristic: Left padding for short labels
                let label_len = label.len();
                if label_len < 5 {
                    format!("{}{} = {}", label, " ".repeat(5 - label_len), expr_repr)
                }
                else {
                    format!("{} = {}", label, expr_repr)
                }
                .into()
            },
            Item::Label(label) => label
                .get(labels)
                .ok_or_else(|| {
                    format!("label reference {label:?} is out of range for the label table")
                })?
                .to_string()
                .into(),
            Item::LocalLabel(label) => format!(
                ".{}",
                label.get(labels).ok_or_else(|| format!(
                    "label reference {label:?} is out of range for the label table"
                ))?
            )
            .into(),
            Item::MacroDef(m) => m.display(labels)?.into(),
            Item::Statement(s) => s.display(labels)?
        })
    }
}

/// Label reference (index into string table)
#[derive(Debug, Clone, PartialEq)]
pub enum LabelRef {
    /// A single-byte reference (source bytes `0x60`-`0xDF`), holding the already-resolved
    /// 0-based index (`0`-`127`) into the label string table.
    Short(u8),
    /// A two-byte reference (source bytes `0xE0`-`0xFF` followed by a second byte), holding the
    /// two raw bytes exactly as read from the stream. The index into the label string table is
    /// only computed on lookup, by [`StringTable::label`].
    Long(u8, u8)
}

impl LabelRef {
    const fn new_short_from_stream(byte: u8) -> Self {
        debug_assert!(byte >= SHORT_LABEL_START && byte <= SHORT_LABEL_END);
        LabelRef::Short(byte - SHORT_LABEL_START)
    }

    const fn new_long_from_stream(long: u8, byte: u8) -> Self {
        // `long` is a u8, so it is always <= 0xFF; only the lower bound is meaningful here.
        debug_assert!(long >= LONG_LABEL_START); // XXX I do not know if there is a range of values here
        LabelRef::Long(long, byte)
    }

    /// Resolves this reference to its label text in `table`.
    ///
    /// Returns `None` if the referenced index is out of range for `table`.
    pub fn get<'t>(&self, table: &'t StringTable) -> Option<&'t OrgamsEncodedString> {
        table.label(self)
    }

    /// Re-encodes this reference back to its original one- or two-byte binary form.
    pub fn bytes(&self, _table: &StringTable) -> Vec<u8> {
        match self {
            LabelRef::Short(index) => vec![index + SHORT_LABEL_START],
            LabelRef::Long(long, byte) => vec![*long, *byte]
        }
    }

    /// Renders this reference as its label text (resolved against `table`).
    ///
    /// Returns `Err` if the referenced index is out of range for `table`.
    pub fn display(&self, table: &StringTable) -> Result<String, String> {
        self.get(table)
            .map(|s| s.to_string())
            .ok_or_else(|| format!("label reference {self:?} is out of range for the label table"))
    }
}

/// The `LBLs` section: every label/identifier string used by the source, indexed by
/// [`LabelRef`]. Short references (`0x60`-`0xDF`) index directly into this table; long
/// references (`0xE0`-`0xFF` + byte) are remapped to an index starting right after the last
/// short index (see [`StringTable::label`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StringTable {
    strings: Vec<Bit7OnString>
}

impl StringTable {
    /// Returns the number of label strings in the table.
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Iterates over every label string, in table order (i.e. in short-index order, followed by
    /// long-index order).
    pub fn iter(&self) -> impl Iterator<Item = &Bit7OnString> {
        self.strings.iter()
    }

    /// Builds a table directly from an ordered list of already-decoded label strings.
    pub fn from_vec_bit7on_texts(strings: Vec<Bit7OnString>) -> Self {
        Self { strings }
    }

    fn get(&self, index: usize) -> Option<&Bit7OnString> {
        self.strings.get(index)
    }

    fn label(&self, label_ref: &LabelRef) -> Option<&OrgamsEncodedString> {
        let index = match label_ref {
            LabelRef::Short(idx) => *idx as usize,
            LabelRef::Long(long, byte) => {
                // Compute index from long label encoding
                // Long labels use 2 bytes to encode larger indices
                // 0xE0 corresponds to the start of long labels.
                // 0x60 - 0xDF are short labels (indices 0 - 127)
                // So index starts at 128

                128 + (((*long as usize) << 8) | (*byte as usize)) - 0xE000
            }
        };
        self.get(index).map(|s| s.deref())
    }
}

/// Expression (encoded bytes)
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// A sequence of several members (e.g. `a+b`), delimited in the binary stream by the
    /// multi-term begin/end markers (`'B'`/`'E'`).
    MultiTerm(Vec<ExpressionMember>),
    /// A single expression member, with no multi-term wrapper.
    SingleTerm(ExpressionMember)
}

impl Expression {
    /// serve for byte/word to count the number of elements to convert in bytes or word
    /// globally 1 expect for strings
    pub fn nb_elements(&self) -> usize {
        match self {
            Expression::SingleTerm(ExpressionMember::String(s)) => s.len(),
            Expression::MultiTerm(members) => {
                match members.first() {
                    Some(ExpressionMember::String(s)) => s.len(),
                    Some(_) => 1,
                    _ => 0
                }
            },
            _ => 1
        }
    }
}

/// An expression preceded by its encoded byte length (`0` meaning no expression at all), as
/// used for operands such as `ORG`, `ENT`, `SKIP`, `FILL`, etc.
#[derive(Debug, Clone, PartialEq)]
pub enum SizedExpression {
    /// No expression was present (a zero size byte).
    Empty,
    /// An expression was present, wrapped with its size.
    Sized(Expression)
}

/// A single element making up an [`Expression`].
#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionMember {
    /// A small literal in `0x00`-`0x1F`, stored directly as its value (0-31).
    ShortDecimal(u8),
    /// A decimal/hexadecimal/binary literal, 8/16-bit or custom-width (see [`Value`]).
    Value(Value),
    /// A binary or unary operator.
    Operator(Operator),
    /// A label reference (source bytes `0x60`-`0xFF`).
    LabelRef(LabelRef),
    /// A local label reference, i.e. a [`LabelRef`] preceded by `.` in the source.
    LocalLabelRef(LabelRef),
    /// A literal space character (`0x20`).
    Space,
    /// The current-address symbol `$` (`0x24`).
    Dollar,
    /// The start-of-chunk address symbol `$$` (`0x44`).
    DoubleDollar,
    /// A unary minus applied to the inner member (`#`, e.g. `#5` for `-5`).
    UnaryMinus(Box<ExpressionMember>),
    /// A string literal.
    String(SizedString),
    /// A sub-expression wrapped in `(...)`.
    ParenthesizedExpression(Vec<ExpressionMember>),
    /// A repeat-block iteration variable: `1` for `#`/`I`, `2` for `##`/`J`, `3` for `###`/`K`.
    Iter(u8)
}

/// A binary or unary operator usable inside an [`Expression`].
#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    /// Bitwise/logical OR (`@`), displayed as `OR`.
    Or,
    /// Bitwise/logical XOR (`!`), displayed as `XOR`.
    Xor,

    /// Less-than comparison (`<`).
    LessThan,
    /// Greater-than comparison (`>`).
    GreaterThan,
    /// Equality comparison (`=`), displayed as `==`.
    Equal,

    /// Less-than-or-equal comparison (`'L'`), displayed as `<=`.
    LessEqual,
    /// Greater-than-or-equal comparison (`'M'`), displayed as `>=`.
    GreaterEqual,
    /// Not-equal comparison (`'N'`), displayed as `!=`.
    NotEqual,

    /// Bitwise/logical AND (`0x26`, `&`), displayed as `AND`.
    And,
    /// Addition (`0x2b`, `+`).
    Plus,
    /// Subtraction (`0x2d`, `-`).
    Minus,
    /// Multiplication (`0x2a`, `*`).
    Multiply,
    /// Division (`0x2f`, `/`).
    Divide,
    /// Modulo (`0x25`, `MOD`).
    Modulo,
    /// Opening parenthesis (`0x28`).
    ParenOpen,
    /// Closing parenthesis (`0x29`).
    ParenClose
}

/// The number base a numeric [`Value`] was written in, which controls how [`Value::display`]
/// renders it (plain digits, `&`-prefixed hex, or `%`-prefixed binary).
#[derive(Debug, Clone, PartialEq)]
pub enum ValueBasis {
    /// Decimal (base 10) literal.
    Decimal,
    /// Hexadecimal (base 16) literal, displayed with a leading `&`.
    Hexadecimal,
    /// Binary (base 2) literal, displayed with a leading `%`.
    Binary
}

/// The width/storage form of a numeric [`Value`], independent of its [`ValueBasis`].
#[derive(Debug, Clone, PartialEq)]
pub enum ValueContent {
    /// An 8-bit value.
    EightBits(u8),
    /// A 16-bit value, stored little-endian in the file.
    SixteenBits(u16),
    /// A "custom"-width value: the raw content bytes read after the length byte of an
    /// `EXP_*_CUSTOM`/`EXP_*_CUSTOM_LONG` marker, kept as-is without further interpretation.
    Custom(Vec<u8>)
}

/// A numeric literal, combining its number base and its width/storage form.
#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    /// The number base the literal was written in.
    pub basis: ValueBasis,
    /// The literal's width and raw content.
    pub content: ValueContent
}

impl Value {
    /// Re-encodes this value back to its original marker-byte-prefixed binary form.
    pub fn bytes(&self, _table: &StringTable) -> Vec<u8> {
        // Outputs the appropriate encoding marker followed by the value content
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
    /// Re-encodes this expression member back to its original binary representation.
    pub fn bytes(&self, table: &StringTable) -> Vec<u8> {
        match self {
            ExpressionMember::Iter(n) => {
                let code = match n {
                    1 => EXP_ITER1,
                    2 => EXP_ITER2,
                    3 => EXP_ITER3,
                    _ => unreachable!("Invalid iteration count: {}", n)
                };
                vec![code]
            },
            ExpressionMember::String(s) => {
                let mut result = vec![EXP_STRING];
                result.extend_from_slice(&s.bytes());
                result
            },
            ExpressionMember::UnaryMinus(inner) => {
                let mut result = vec![EXP_UNARY_MINUS];
                result.extend(inner.bytes(table));
                result
            },
            ExpressionMember::ShortDecimal(v) => vec![*v],
            ExpressionMember::Value(val) => val.bytes(table),
            ExpressionMember::Operator(op) => {
                let code = match op {
                    Operator::And => EXP_OP_AND,
                    Operator::Divide => EXP_OP_DIV,
                    Operator::Equal => EXP_OP_EQ,
                    Operator::GreaterEqual => EXP_OP_GE,
                    Operator::GreaterThan => EXP_OP_GT,
                    Operator::LessEqual => EXP_OP_LE,
                    Operator::LessThan => EXP_OP_LT,
                    Operator::Minus => EXP_OP_MINUS,
                    Operator::Modulo => EXP_OP_MOD,
                    Operator::Multiply => EXP_OP_TIMES,
                    Operator::NotEqual => EXP_OP_NEQ,
                    Operator::Or => EXP_OP_OR,
                    Operator::ParenClose => EXP_OP_PAREN_CLOSE,
                    Operator::ParenOpen => EXP_OP_PAREN_OPEN,
                    Operator::Plus => EXP_OP_PLUS,
                    Operator::Xor => EXP_OP_XOR
                };
                vec![code]
            },
            ExpressionMember::LabelRef(label_ref) => label_ref.bytes(table),
            ExpressionMember::LocalLabelRef(label_ref) => {
                let mut bytes = vec![EXP_LOCAL_LABEL];
                bytes.extend_from_slice(&label_ref.bytes(table));
                bytes
            },
            ExpressionMember::Space => vec![0x20],
            ExpressionMember::Dollar => vec![0x24],
            ExpressionMember::DoubleDollar => vec![0x44],
            ExpressionMember::ParenthesizedExpression(expr) => {
                let mut bytes = vec![EXP_OP_PAREN_OPEN];
                let expr = expr.iter().flat_map(|e| e.bytes(table));
                bytes.extend(expr);
                bytes.push(EXP_OP_PAREN_CLOSE);
                bytes
            }
        }
    }
}

impl Expression {
    /// Re-encodes this expression back to its original binary representation (wrapping it with
    /// the multi-term begin/end markers for [`Expression::MultiTerm`]).
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
    /// Re-encodes this expression back to its original size-byte-prefixed binary form (a lone
    /// `0` byte for [`SizedExpression::Empty`]).
    pub fn bytes(&self, table: &StringTable) -> Vec<u8> {
        match self {
            SizedExpression::Empty => vec![0],
            SizedExpression::Sized(expr) => {
                let result = expr.bytes(table);
                // Prepend size byte
                let size = result.len() as u8;
                let mut with_size = vec![size];
                with_size.extend(result);
                with_size
            }
        }
    }

    /// Returns `true` if this is [`SizedExpression::Empty`], i.e. no expression was present.
    pub fn is_empty(&self) -> bool {
        matches!(self, SizedExpression::Empty)
    }

    /// Returns the inner expression, or `None` if this is [`SizedExpression::Empty`].
    pub fn expr(&self) -> Option<&Expression> {
        match self {
            SizedExpression::Empty => None,
            SizedExpression::Sized(expr) => Some(expr)
        }
    }

    /// Renders this expression as Orgams source text (`"0"` for [`SizedExpression::Empty`]).
    ///
    /// Returns `Err` if a label reference inside this expression is out of range for `table`.
    pub fn display(&self, table: &StringTable) -> Result<Cow<'_, str>, String> {
        Ok(match self {
            SizedExpression::Empty => "0".into(),
            SizedExpression::Sized(expr) => expr.display(table)?
        })
    }
}

/// An assignment statement (`label = expression`), introduced in the binary stream by the
/// assignment marker (`0x64`, `'d'`).
#[derive(Debug, Clone, PartialEq)]
pub struct Assign {
    /// The label being assigned to.
    pub label: LabelRef,
    /// The assigned value.
    pub expression: SizedExpression
}

impl Assign {
    /// Re-encodes this assignment back to its original binary representation.
    pub fn bytes(&self, table: &StringTable) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(MARKER_ASSIGN);

        bytes
            .into_iter()
            .chain(
                self.label
                    .bytes(table)
                    .into_iter()
                    .chain(self.expression.bytes(table))
            )
            .collect()
    }
}

impl Expression {
    /// Renders this expression as Orgams source text, resolving any label reference against
    /// `table`.
    ///
    /// Returns `Err` if a label reference in this expression is out of range for `table`.
    pub fn display(&self, table: &StringTable) -> Result<Cow<'_, str>, String> {
        Ok(match self {
            Expression::MultiTerm(members) => {
                members
                    .iter()
                    .map(|m| m.display(table))
                    .collect::<Result<Vec<_>, String>>()?
                    .join("")
                    .into()
            },
            Expression::SingleTerm(member) => member.display(table)?
        })
    }
}

impl ExpressionMember {
    /// Renders this expression member as Orgams source text, resolving any label reference
    /// against `table`.
    ///
    /// Returns `Err` if a label reference in this member is out of range for `table`.
    pub fn display(&self, table: &StringTable) -> Result<Cow<'_, str>, String> {
        Ok(match self {
            ExpressionMember::Iter(n) => {
                match n {
                    1 => "#".into(),
                    2 => "##".into(),
                    3 => "###".into(),
                    _ => unreachable!("Invalid iteration count: {}", n)
                }
            },
            ExpressionMember::String(s) => format!("\"{}\"", **s).into(),
            ExpressionMember::UnaryMinus(inner) => format!("-{}", inner.display(table)?).into(),
            ExpressionMember::ShortDecimal(v) => format!("{}", v).into(),
            ExpressionMember::Value(v) => v.display().into(),
            ExpressionMember::Operator(op) => op.as_str().into(),
            ExpressionMember::LabelRef(l) => l
                .get(table)
                .ok_or_else(|| {
                    format!("label reference {l:?} is out of range for the label table")
                })?
                .to_string()
                .into(),
            ExpressionMember::LocalLabelRef(l) => format!(
                ".{}",
                l.get(table).ok_or_else(|| format!(
                    "label reference {l:?} is out of range for the label table"
                ))?
            )
            .into(),
            ExpressionMember::Space => " ".into(),
            ExpressionMember::Dollar => "$".into(),
            ExpressionMember::DoubleDollar => "$$".into(),
            ExpressionMember::ParenthesizedExpression(expr) => {
                format!(
                    "[{}]",
                    expr.iter()
                        .map(|e| e.display(table))
                        .collect::<Result<Vec<_>, String>>()?
                        .join("")
                )
                .into()
            },
        })
    }
}

impl Operator {
    /// Returns the Orgams source text for this operator (e.g. `"AND"`, `"+"`, `"<="`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Operator::And => "AND",
            Operator::Divide => "/",
            Operator::Equal => "==",
            Operator::GreaterEqual => ">=",
            Operator::GreaterThan => ">",
            Operator::LessEqual => "<=",
            Operator::LessThan => "<",
            Operator::Minus => "-",
            Operator::Modulo => "MOD",
            Operator::Multiply => "*",
            Operator::NotEqual => "!=",
            Operator::Or => "OR",
            Operator::ParenClose => "]",
            Operator::ParenOpen => "[",
            Operator::Plus => "+",
            Operator::Xor => "XOR"
        }
    }
}

impl Value {
    /// Renders this literal as Orgams source text: plain digits for [`ValueBasis::Decimal`],
    /// `&`-prefixed hex for [`ValueBasis::Hexadecimal`], `%`-prefixed binary for
    /// [`ValueBasis::Binary`]. A [`ValueContent::Custom`] value is not currently decoded and
    /// renders as the placeholder `"<custom>"`.
    pub fn display(&self) -> String {
        match &self.content {
            ValueContent::EightBits(val) => {
                match self.basis {
                    ValueBasis::Decimal => format!("{}", val),
                    ValueBasis::Hexadecimal => format!("&{:02X}", val),
                    ValueBasis::Binary => format!("%{:08b}", val)
                }
            },
            ValueContent::SixteenBits(val) => {
                match self.basis {
                    ValueBasis::Decimal => format!("{}", val),
                    ValueBasis::Hexadecimal => format!("&{:04X}", val),
                    ValueBasis::Binary => format!("%{:b}", val)
                }
            },
            ValueContent::Custom(_bytes) => String::from("<custom>")
        }
    }
}

impl Assign {
    /// Renders this assignment as Orgams source text (`"label = expression"`).
    ///
    /// Returns `Err` if a label reference in the assignment is out of range for `table`.
    pub fn display(&self, table: &StringTable) -> Result<String, String> {
        let label = self.label.get(table).ok_or_else(|| {
            format!(
                "label reference {:?} is out of range for the label table",
                self.label
            )
        })?;
        Ok(format!("{} = {}", label, self.expression.display(table)?))
    }
}

/// A `MACRO name param1,param2,...` definition header (the body of the macro is the sequence of
/// [`Item`]s that follows it in the source, up to the matching [`Statement::EndMacro`]).
#[derive(Debug, Clone, PartialEq)]
pub struct MacroDef {
    /// Original block length from file (preserved for exact binary reconstruction)
    pub def_block_len: u8,
    /// The macro's name.
    pub name: LabelRef,
    /// The macro's formal parameter names, in declaration order.
    pub params: Vec<LabelRef>
}

impl MacroDef {
    /// Re-encodes this macro header back to its original binary representation, padding with
    /// the observed `0x41` separator byte up to `def_block_len` if the recomputed content is
    /// shorter (mirrors padding/separator bytes observed in real files).
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
        }
        else {
            bytes.push(computed_len);
            bytes.extend_from_slice(&content);
        }

        bytes
    }

    /// Renders this macro header as Orgams source text (`"MACRO name"` or
    /// `"MACRO name param1,param2,..."`).
    ///
    /// Returns `Err` if the macro name or one of its parameter labels is out of range for
    /// `table`.
    pub fn display(&self, table: &StringTable) -> Result<String, String> {
        let name = self.name.get(table).ok_or_else(|| {
            format!(
                "label reference {:?} is out of range for the label table",
                self.name
            )
        })?;
        Ok(if self.params.is_empty() {
            format!("MACRO {}", name)
        }
        else {
            let params = self
                .params
                .iter()
                .map(|p| {
                    p.get(table)
                        .map(|s| s.to_string())
                        .ok_or_else(|| format!(
                            "label reference {p:?} is out of range for the label table"
                        ))
                })
                .collect::<Result<Vec<_>, String>>()?
                .join(",");
            format!("MACRO {} {}", name, params)
        })
    }
}

/// Statements
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// `BRK` statement, a debugger breakpoint marker.
    Brk,
    /// `BYTE e1,e2,...` directive: a list of 8-bit-sized expressions (or a string, whose
    /// characters each count as one element) to emit as raw bytes.
    Byte(Vec<Expression>),
    /// `ELSE` branch of an enclosing [`Statement::If`].
    Else,
    /// `END` statement, marking the end of the assembled program.
    End,
    /// A second flavor of end marker (distinct byte from [`Statement::End`], observed as a
    /// closing marker in some contexts); it renders as an empty string.
    EndBis,
    /// `ENDM` statement, closing the body of the [`MacroDef`] that started it.
    EndMacro,
    /// `ENT expr` directive: sets the program's entry point address.
    Ent(SizedExpression),
    /// `FILL count,value` directive: fills `count` bytes with `value`.
    Fill(SizedExpression, SizedExpression),
    /// `IF condition` statement, guarding the items up to the matching [`Statement::Else`] or
    /// [`Statement::End`].
    If(SizedExpression),
    /// `IMPORT "path"` directive: includes another source file.
    Import(SizedString),
    /// A single Z80 instruction.
    Instruction(Instruction),
    /// Invocation of a macro by name with argument expressions (`name(arg1,arg2,...)`).
    MacroUse(Expression, Vec<Expression>),
    /// `ORG expr` directive: sets the current assembly address.
    Org(SizedExpression),
    /// `ORG expr1,expr2` directive: the two-argument form of `ORG` (logical and physical
    /// address).
    Org2(SizedExpression, SizedExpression),
    /// Raw text emitted verbatim into the rendered output, with no quoting or marker prefix
    /// (introduced by the `CMD_ASIS` escaped command).
    RawString(OrgamsEncodedString),
    /// `SKIP expr` directive: advances the current assembly address by `expr` bytes without
    /// emitting data.
    Skip(SizedExpression),
    /// Single-item star-repeat (`count ** item`): repeats the wrapped [`Item`] `count` times.
    /// Distinct from the multi-line [`Statement::StartRepeatBloc`]/[`Statement::StopRepeatBloc`]
    /// pair, which brackets a whole block of lines instead of a single item.
    RepeatInstruction(Box<SizedExpression>, Box<Item>),
    /// `RESTORE` statement, restoring a previously stored program-counter/address context (see
    /// [`Statement::StorePcInstr`]/[`Statement::StorePcLine`]).
    Restore,
    /// Opening marker of a multi-line star-repeat block (`count ** [`): repeats every line up to
    /// the matching [`Statement::StopRepeatBloc`] `count` times.
    StartRepeatBloc(SizedExpression),
    /// Closing marker (`]`) of a multi-line star-repeat block opened by
    /// [`Statement::StartRepeatBloc`].
    StopRepeatBloc,
    /// Hidden pseudo-instruction that records the current program counter for internal
    /// per-instruction bookkeeping; it renders as nothing.
    StorePcInstr,
    /// Hidden pseudo-instruction that records the current program counter for internal
    /// per-line bookkeeping; it renders as nothing.
    StorePcLine,
    /// `WORD e1,e2,...` directive: a list of 16-bit-sized expressions to emit as raw
    /// little-endian words.
    Word(Vec<Expression>)
}

fn bytes_for_word_or_byte(exprs: &[Expression], is_word: bool, table: &StringTable) -> Vec<u8> {
    let unit_size: usize = if is_word { 2 } else { 1 };
    let unit_marker: u8 = if is_word { MARKER_WORD } else { MARKER_BYTE };

    let directive_len = exprs.iter().map(|expr| expr.nb_elements()).sum::<usize>() as u8;
    let directive_len = directive_len.max(1) * unit_size as u8;

    let exprs = exprs
        .iter()
        .flat_map(|expr| expr.bytes(table))
        .collect::<Vec<u8>>();
    let exprs_len = exprs.len() as u8;

    let mut bytes = Vec::with_capacity(1 + 1 + 1 + exprs_len as usize + 1);
    bytes.push(unit_marker);
    bytes.push(exprs_len + 2);
    bytes.push(directive_len);
    bytes.extend_from_slice(&exprs);
    bytes.push(END_MARKER);
    bytes
}

impl Statement {
    /// Re-encodes this statement back to its original binary representation.
    pub fn bytes(&self, table: &StringTable) -> Vec<u8> {
        let mut bytes = Vec::new();

        match self {
            Statement::StartRepeatBloc(expr) => {
                let expr_bytes = expr.bytes(table);
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_FACTOR_BLOC);
                bytes.extend_from_slice(&expr_bytes);
            },
            Statement::StopRepeatBloc => {
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_FACTOR_BLOC_END);
            },
            Statement::Brk => {
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_BRK);
            },
            Statement::Restore => {
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_RESTORE);
            },
            Statement::Byte(exprs) => {
                bytes.extend_from_slice(&bytes_for_word_or_byte(exprs, false, table));
            },
            Statement::Word(exprs) => {
                bytes.extend_from_slice(&bytes_for_word_or_byte(exprs, true, table));
            },
            Statement::If(condition) => {
                let condition = condition.bytes(table);
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_IF);
                bytes.extend_from_slice(&condition);
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
            Statement::Fill(count_expr, value_expr) => {
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_FILL);
                bytes.extend_from_slice(&count_expr.bytes(table));
                bytes.extend_from_slice(&value_expr.bytes(table));
            },
            Statement::Import(s) => {
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_IMPORT);
                bytes.push(s.len() as u8);
                bytes.extend_from_slice(s.as_slice());
            },
            Statement::RawString(content) => {
                bytes.push(MARKER_ESCAPE);
                bytes.push(CMD_ASIS); // Raw string command
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

            Statement::RepeatInstruction(exprs, token) => {
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

    /// Renders this statement as Orgams source text, resolving any label reference against
    /// `table`.
    ///
    /// Returns `Err` if a label reference in this statement is out of range for `table`.
    pub fn display<'a>(&'a self, table: &StringTable) -> Result<Cow<'a, str>, String> {
        Ok(match self {
            Statement::StartRepeatBloc(expr) => format!("{} ** [", expr.display(table)?).into(),
            Statement::StopRepeatBloc => "]".into(),
            Statement::Brk => "BRK".into(),
            Statement::Restore => "RESTORE".into(),
            Statement::If(expr) => {
                let cond = expr.display(table)?;
                format!("IF {}", cond).into()
            },
            Statement::Else => "ELSE".into(),
            Statement::End => "END".into(),
            Statement::EndBis => "".into(),
            Statement::EndMacro => "ENDM".into(),
            Statement::Fill(count_expr, value_expr) => {
                format!(
                    "FILL {},{}",
                    count_expr.display(table)?,
                    value_expr.display(table)?
                )
                .into()
            },
            Statement::Import(s) => {
                if s.starts_with(b"\"") && s.len() >= 3 {
                    // Start is " + user number
                    // End is A (maybe access rights/encoding ?)
                    let stripped = &s[2..s.len() - 1];
                    format!(
                        "IMPORT \"{}\"",
                        encoding_rs::WINDOWS_1252.decode(stripped).0
                    )
                    .into()
                }
                else {
                    format!("IMPORT \"{}\"", **s).into()
                }
            },
            Statement::RawString(s) => s.to_string().into(),
            Statement::Ent(e) => format!("ENT {}", e.display(table)?).into(),
            Statement::Org(e) => format!("ORG {}", e.display(table)?).into(),
            Statement::Org2(e1, e2) => {
                format!("ORG {},{}", e1.display(table)?, e2.display(table)?).into()
            },
            Statement::Skip(e) => format!("SKIP {}", e.display(table)?).into(),
            Statement::Byte(exprs) => {
                let exprs = exprs
                    .iter()
                    .map(|e| e.display(table))
                    .collect::<Result<Vec<_>, String>>()?
                    .into_iter()
                    .join(",");
                format!("BYTE {}", exprs).into()
            },
            Statement::Word(exprs) => {
                let exprs = exprs
                    .iter()
                    .map(|e| e.display(table))
                    .collect::<Result<Vec<_>, String>>()?
                    .into_iter()
                    .join(",");
                format!("WORD {}", exprs).into()
            },
            Statement::StorePcInstr | Statement::StorePcLine => {
                // norepresentation is expected
                "".into()
            },
            Statement::RepeatInstruction(expr, item) => {
                format!("{} ** {}", expr.display(table)?, item.display(table)?).into()
            },
            Statement::MacroUse(name, args) => {
                // let name_str = name.get(table).to_string();
                let name_str = name.display(table)?;
                let args_str = args
                    .iter()
                    .map(|e| e.display(table))
                    .collect::<Result<Vec<_>, String>>()?
                    .join(",");

                format!("{}({})", name_str, args_str).into()
            },
            Statement::Instruction(instr) => instr.display(table)?.into()
        })
    }
}

/// A single Z80 instruction, as an optional prefix, an opcode byte, and its coded operand
/// expressions.
#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    /// Prefix for `IX`/`IY`-related (and `ED`/`CB`-extended) instructions.
    pub prefix: InstructionPrefix,
    /// The opcode byte, indexing into `prefix.disassembler_table()`.
    pub opcode: u8,
    /// The instruction's operands, each encoded as a [`SizedExpression`] in the order their
    /// `nn`/`nnnn` placeholders appear in the disassembly mnemonic.
    pub coded_operands: Vec<SizedExpression>
}

impl Instruction {
    /// Re-encodes this instruction back to its original binary representation (prefix bytes,
    /// escape byte if the opcode collides with a marker, opcode byte, then operand bytes).
    pub fn bytes(&self, table: &StringTable) -> Vec<u8> {
        let mut bytes = Vec::new();

        if self.prefix.is_prefix() {
            // prefixed opcode
            let prefix_bytes = self.prefix.bytes();
            bytes.extend_from_slice(prefix_bytes);
        }
        else {
            if is_escaped_byte(self.opcode) {
                // escaped opcode
                bytes.push(MARKER_ESCAPE);
            }
        }

        bytes.push(self.opcode);

        for expr in &self.coded_operands {
            bytes.extend_from_slice(&expr.bytes(table));
        }

        bytes
    }

    /// Renders this instruction as lowercase Z80 mnemonic text, substituting each `nn`/`nnnn`
    /// placeholder (or, for opcodes needing an extra expression such as `RST` or `CB`
    /// `BIT`/`RES`/`SET`, the trailing digit) with its operand's display form, and cleaning up
    /// Orgams-specific `IX`/`IY`-indirect and negative-offset notation.
    ///
    /// Returns `Err` if a label reference in one of this instruction's operands is out of range
    /// for `table`.
    pub fn display(&self, table: &StringTable) -> Result<String, String> {
        let tab = self.prefix.disassembler_table();

        let mut result = tab[self.opcode as usize].to_lowercase();

        let mut i = 0;
        while i < self.coded_operands.len() {
            if let Some(pos) = result.find("nnnn") {
                let expr_str = self.coded_operands[i].display(table)?;
                result.replace_range(pos..pos + 4, &expr_str);
                i += 1;
            }
            else if let Some(pos) = result.find("nn") {
                let expr_str = self.coded_operands[i].display(table)?;
                result.replace_range(pos..pos + 2, &expr_str);
                i += 1;
            }
            else {
                assert_eq!(i, 0);
                if self.prefix.requires_extra_expression(self.opcode) {
                    let expr_str = self.coded_operands[i].display(table)?; //0 may be a rel expression
                    result = result
                        .replace("00", "0") // because of RST
                        .replace("0", &expr_str);
                }
                else {
                    unimplemented!("Too many coded operands for instruction display");
                }
                break;
            }
        }

        if self.prefix.is_orgams_ix_iy_indirect() && self.coded_operands[0].is_empty() {
            result = result.replace("(ix+0)", "(ix)").replace("(iy+0)", "(iy)");
        }
        result = result
            .replace("(ix+-", "(ix-")
            .replace("(iy+-", "(iy-")
            .replace("ex af,af'", "ex af,af")
            .replace("sbc a,", "sbc ");

        // Clean up offset notation for Orgams IX/IY indirect addressing
        if self.prefix.is_orgams_ix_iy_indirect() {
            result = result.replace("+nn", "");
        }

        Ok(result)
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
        }
        else if repr[i..].starts_with("nn") {
            kinds.push(ExpressionKind::EightBits);
            i += 2;
        }
        else {
            i += 1;
        }
    }
    kinds
}

/// Tracks what was last appended to the line currently being rendered by [`DisplayState`], so
/// that the next appended token knows whether it needs leading indentation or a `:` separator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineState {
    /// Nothing has been appended to the current line yet.
    Empty,
    /// A label was just appended; the next token is indented to the instruction/command column.
    AfterLabel,
    /// An assignment was just appended; the next token is indented like [`LineState::AfterLabel`].
    AfterAssign,
    /// A statement was just appended (`true` if it was a Z80 instruction, `false` for a
    /// directive/command); the next token on the same line is separated with `:`.
    AfterStatement(bool),
    /// The opening or closing bracket of a star-repeat block was just appended; the next token
    /// needs neither indentation nor a separator.
    AfterRepeatBloc
}

/// Mutable rendering state threaded through [`DisplayState::render_items`]/
/// [`DisplayState::render_item`] to turn a stream of [`Item`]s into formatted Orgams source
/// text, one line at a time.
pub struct DisplayState<'f, 'g> {
    /// The formatter lines are written to, or `None` to accumulate state without producing
    /// output (e.g. to only track [`DisplayState::last_line`]).
    pub(crate) f: Option<&'f mut std::fmt::Formatter<'g>>,
    /// When set, every flushed line is also appended here (used by [`Program::try_render`] to
    /// collect the full rendered output without going through a [`std::fmt::Formatter`]).
    pub(crate) buffer: Option<String>,
    /// The line currently being built, not yet flushed to `f`.
    pub(crate) current_line: String,
    /// 1-based number of the line currently being built.
    pub(crate) line_number: usize,
    /// What was last appended to `current_line`, driving indentation/separator decisions.
    pub(crate) line_state: LineState,
    /// The last line that was flushed to `f`, if any.
    pub(crate) last_generated_line: Option<String>
}

impl<'f, 'g> DisplayState<'f, 'g> {
    /// Creates a fresh rendering state, optionally writing lines to `f` as they are completed.
    pub fn new(f: Option<&'f mut std::fmt::Formatter<'g>>) -> Self {
        Self {
            f,
            buffer: None,
            current_line: String::new(),
            line_number: 1,
            line_state: LineState::Empty,
            last_generated_line: None
        }
    }

    /// Creates a fresh rendering state that writes to no [`std::fmt::Formatter`] but instead
    /// accumulates every flushed line into an internal buffer, retrievable with
    /// [`DisplayState::take_buffer`].
    pub fn new_buffered() -> Self {
        Self {
            f: None,
            buffer: Some(String::new()),
            current_line: String::new(),
            line_number: 1,
            line_state: LineState::Empty,
            last_generated_line: None
        }
    }

    /// Returns the last line that was flushed, if any.
    pub fn last_line(&self) -> Option<&str> {
        self.last_generated_line.as_deref()
    }

    /// Returns the 1-based number of the line currently being built.
    pub fn line_number(&self) -> usize {
        self.line_number
    }

    /// Takes the buffer accumulated so far (see [`DisplayState::new_buffered`]), leaving an
    /// empty one in its place.
    pub fn take_buffer(&mut self) -> String {
        self.buffer.take().unwrap_or_default()
    }
}

impl<'f, 'g> DisplayState<'f, 'g> {
    fn emit_line(&mut self) -> Result<(), String> {
        // we have rendering bug to fix. In the meanwhile here is a workaround
        let line = self.current_line.clone();
        if let Some(f) = self.f.as_mut() {
            write!(f, "{}\r\n", line).map_err(|e| e.to_string())?;
        }
        if let Some(buffer) = self.buffer.as_mut() {
            buffer.push_str(&line);
            buffer.push_str("\r\n");
        }

        self.last_generated_line = Some(line);
        self.current_line.clear();
        self.line_number += 1;
        self.line_state = LineState::Empty;
        Ok(())
    }

    fn append_instruction<S: AsRef<str>>(&mut self, s: S) {
        self.append_token_or_instruction_representation(true, s);
    }

    fn append_token<S: AsRef<str>>(&mut self, s: S) {
        self.append_token_or_instruction_representation(false, s);
    }

    fn append_start_bloc<S: AsRef<str>>(&mut self, s: S) {
        self.append_token(s);
        self.line_state = LineState::AfterRepeatBloc;
    }

    fn append_stop_bloc<S: AsRef<str>>(&mut self, s: S) {
        if self.line_state != LineState::Empty {
            self.line_state = LineState::AfterRepeatBloc; // we want to avoid :
        }
        self.append_instruction(s);
        self.line_state = LineState::AfterStatement(false);
    }

    fn append_token_or_instruction_representation<S: AsRef<str>>(
        &mut self,
        is_instruction: bool,
        s: S
    ) {
        let indent_size: usize = if is_instruction {
            TAB_INSTR as _
        }
        else {
            TAB_COMMAND as _
        };

        // handle indentation
        match self.line_state {
            LineState::Empty | LineState::AfterLabel | LineState::AfterAssign => {
                let indent_after_label: usize = indent_size;
                if self.col_number() < indent_after_label {
                    self.current_line
                        .push_str(" ".repeat(indent_after_label - self.col_number()).as_str());
                }
                else {
                    self.current_line.push(' ');
                }
            },
            LineState::AfterStatement(_) => {
                self.current_line.push(':');
            },
            LineState::AfterRepeatBloc => {
                // nothing to do
            }
        }

        self.current_line.push_str(s.as_ref());
        self.line_state = LineState::AfterStatement(is_instruction);
    }

    fn append_comment<S: AsRef<str>>(&mut self, c: S) -> Result<(), String> {
        if self.line_state == LineState::Empty || self.has_only_indents() {
            // nothing to do
        }
        else {
            // handle some indentation
            let current_line_len = self.col_number();
            let indent = if current_line_len < TAB_COMMENT as usize {
                " ".repeat(TAB_COMMENT as usize - current_line_len)
                    .to_string()
            }
            else {
                " ".into()
            };
            self.append_string_no_indent(indent);
        }

        self.append_string_no_indent(c);
        self.emit_line()
    }

    fn append_label<S: AsRef<str>>(&mut self, label: S) {
        self.append_string_no_indent(label);
        self.line_state = LineState::AfterLabel;
    }

    /// the string already has its indentation. maybe it has to change
    fn append_assign<S: AsRef<str>>(&mut self, assignment: S) {
        self.append_string_no_indent(assignment);
        self.line_state = LineState::AfterAssign;
    }

    /// XXX does not change the state (so empty stays empty)
    fn append_string_no_indent(&mut self, s: impl AsRef<str>) {
        self.current_line.push_str(s.as_ref());
    }

    fn col_number(&self) -> usize {
        self.current_line.len()
    }

    fn has_only_indents(&self) -> bool {
        self.current_line.chars().all(|c| c == ' ')
    }

    /// Renders every item from `items`, in order, appending to (and periodically flushing) the
    /// line currently being built.
    pub fn render_items<'i>(
        &mut self,
        items: impl IntoIterator<Item = &'i Item>,
        labels: &StringTable
    ) -> Result<(), String> {
        for item in items {
            self.render_item(item, labels)?;
        }
        Ok(())
    }

    /// Renders a single item, appending it to the line currently being built (with appropriate
    /// indentation/`:` separators per [`LineState`]), flushing the line when `item` is an
    /// [`Item::NewLine`] or a comment.
    pub fn render_item(&mut self, item: &Item, labels: &StringTable) -> Result<(), String> {
        let repr = item.display(labels)?;

        match item {
            Item::Statement(Statement::StorePcInstr | Statement::StorePcLine) => {
                // we do stricly nothing
            },
            Item::Statement(Statement::StartRepeatBloc(_)) => {
                self.append_start_bloc(repr);
            },
            Item::Statement(Statement::StopRepeatBloc) => {
                self.append_stop_bloc(repr);
            },
            Item::Statement(Statement::Instruction(..))
            | Item::Statement(Statement::MacroUse(..) | Statement::RepeatInstruction(..)) => {
                self.append_instruction(repr);
            },

            Item::NewLine => {
                self.emit_line()?;
            },
            Item::Comment(..) => {
                self.append_comment(&repr)?;
            },
            Item::Label(..) | Item::LocalLabel(..) => {
                self.append_label(&repr);
            },
            Item::Assign(..) => {
                self.append_assign(&repr);
            },
            Item::Statement(Statement::RawString(..)) => {
                self.append_string_no_indent(repr);
            },
            Item::Indent(..) => {
                self.append_string_no_indent(repr);
            },
            _ => {
                self.append_token(repr);
            }
        }

        Ok(())
    }
}

impl<'f, 'g> Drop for DisplayState<'f, 'g> {
    fn drop(&mut self) {
        if !self.current_line.is_empty() {
            // `drop` cannot propagate a `Result`; a failure to flush the last, still-pending
            // line here would only happen if the underlying `Formatter` write itself fails,
            // which is already unrecoverable at this point. A label-reference-out-of-range
            // failure cannot happen here: by the time `try_render`/`Display::fmt` reach `drop`,
            // every item has already been rendered successfully (any such failure surfaces
            // earlier, via `?`, before this pending line would be dropped).
            let _ = self.emit_line();
        }
    }
}

impl Program {
    /// Renders this program as Orgams source text, resolving every label reference in every
    /// chunk against `self.labels`.
    ///
    /// Returns `Err` if any label reference anywhere in the program is out of range for
    /// `self.labels` (as can happen with a corrupted/malformed input file whose `LBLs` label
    /// table is inconsistent with a label reference elsewhere in the file) instead of panicking.
    pub fn try_render(&self) -> Result<String, String> {
        let mut state = DisplayState::new_buffered();
        for item in self.chunks.iter().flat_map(|c| c.items()) {
            state.render_item(item, &self.labels)?;
        }
        if !state.current_line.is_empty() {
            state.emit_line()?;
        }
        Ok(state.take_buffer())
    }
}

impl std::fmt::Display for Program {
    /// Lossy convenience wrapper around [`Program::try_render`] for contexts (e.g. `{}`
    /// formatting, `.to_string()`) that require an infallible [`Display`] impl: on a label
    /// reference out of range, renders a `<render error: ...>` placeholder instead of panicking
    /// or silently producing empty output. Prefer [`Program::try_render`] directly whenever the
    /// caller can act on the error.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let rendered = self
            .try_render()
            .unwrap_or_else(|e| format!("<render error: {e}>"));
        write!(f, "{rendered}")
    }
}

/// Parses a complete Orgams binary file into a [`Program`]: validates the header and `SRCc`/
/// `LBLs` section magics, locates and parses the label table first (skipping over the source
/// chunks without decoding them), then rewinds and fully parses the source chunks with the
/// resolved label table available.
///
/// `debug` enables verbose `DEBUG:`-prefixed tracing to stdout. `groundtruth_iter`, when
/// `Some`, is forwarded to the source-chunk parser to cross-check each decoded line against an
/// expected `.Z80` listing while debugging (see `examples/debug_load_orgams.rs`).
pub fn parse_orgams_file(
    debug: bool,
    groundtruth_iter: &mut Option<std::slice::Iter<String>>
) -> impl FnMut(&mut Input) -> OrgamsParseResult<Program> {
    move |input: &mut Input| {
        if debug {
            println!("DEBUG: Starting debug_orgams_file");
        }

        // 1. Parse Header
        const ORGA: &[u8] = b"ORGA";
        const SRCC: &[u8] = b"SRCc";
        const LBLS: &[u8] = b"LBLs";

        cut_err(
            literal(ORGA).context(StrContext::Expected(StrContextValue::StringLiteral("ORGA")))
        )
        .parse_next(input)?;

        let _version = cut_err(any.verify(|&b| b == 2).context(StrContext::Expected(
            StrContextValue::Description("version  expected")
        )))
        .parse_next(input)?;

        let header_size = any
            .context(StrContext::Expected(StrContextValue::Description(
                "header size"
            )))
            .parse_next(input)? as usize;

        // Skip rest of header (0x67 bytes total, already read 4)
        let _header = take(header_size + 1).parse_next(input)?;

        cut_err(
            literal(SRCC).context(StrContext::Expected(StrContextValue::StringLiteral("SRCc")))
        )
        .parse_next(input)?;

        let _version = any
            .verify(|&b| b == 2)
            .context(StrContext::Expected(StrContextValue::Description(
                "version 2 expected"
            )))
            .parse_next(input)?;

        // 2. Mark code start
        let code_start_checkpoint = input.checkpoint();

        // 3. Skip chunks to find LBLs
        if debug {
            println!("DEBUG: Skipping chunks to find LBLs...");
        }
        let mut chunk_count = 0;
        loop {
            let b = peek(any).parse_next(input)?;
            if b == 0 {
                // Found terminator
                break;
            }
            let chunk_size = any.verify(|&s| s <= CHUNK_MAX_SIZE).parse_next(input)?;
            let _ = take(chunk_size as usize).parse_next(input)?;
            chunk_count += 1;
        }
        if debug {
            println!("DEBUG: Found terminator 0 after {} chunks", chunk_count);
        }

        // 4. Parse Labels
        let _null_separator = literal([0x00]).parse_next(input)?;
        cut_err(
            literal(LBLS).context(StrContext::Expected(StrContextValue::StringLiteral("LBLs")))
        )
        .parse_next(input)?;
        let labels = parse_labels_table.parse_next(input)?;
        if debug {
            println!("DEBUG: Labels parsed. Count: {}", labels.len());
        }

        // Print labels
        if debug {
            for (idx, label) in labels.iter().enumerate() {
                println!("  Label #{:03}: \"{}\"", idx, **label);
            }
        }

        // 5. Rewind to code start
        input.reset(&code_start_checkpoint);

        // 6. Parse Code with debug
        if debug {
            println!("DEBUG: Parsing code with trace...");
        }
        let chunks = parse_all_code(debug, &labels, groundtruth_iter).parse_next(input)?;
        Ok(Program { chunks, labels })
    }
}

/// Parse complete Orgams file
pub fn parse_labels_table(input: &mut Input) -> OrgamsParseResult<StringTable> {
    use winnow::combinator::{peek, repeat_till};

    // Parse version
    let _version = any
        .context(StrContext::Expected(StrContextValue::Description(
            "labels version"
        )))
        .verify(|&b| b == 2)
        .parse_next(input)?;

    // collect each of them
    let (strings, _): (Vec<_>, _) = repeat_till(
        0..,
        |input: &mut Input| {
            let res = parse_bit7on_text
                .context(StrContext::Expected(StrContextValue::Description(
                    "label text"
                )))
                .parse_next(input);
            if res.is_err() {
                eprintln!("debug: parse_bit7on_text failed at offset unknown");
            }
            res
        },
        peek(
            literal([0x00]).context(StrContext::Expected(StrContextValue::Description(
                "null terminator"
            )))
        )
    )
    .context(StrContext::Expected(StrContextValue::Description(
        "labels until null"
    )))
    .parse_next(input)?;

    // Consume the null terminator that was only peeked
    literal([0x00]).parse_next(input)?;

    Ok(StringTable::from_vec_bit7on_texts(strings))
}

/// Parse all items
fn parse_items(input: &mut Input) -> OrgamsParseResult<Vec<Item>> {
    repeat(.., parse_inner_item).parse_next(input)
}

fn parse_star_repeat_single(input: &mut Input) -> OrgamsParseResult<Item> {
    consume_marker(CMD_REPEAT)(input)?;
    let expr = cut_err(parse_sized_expression.context(StrContext::Label("Repeat counter")))
        .parse_next(input)?;
    let item = cut_err(parse_inner_item.context(StrContext::Label(
        "Failed to parse item after repeat expression"
    )))
    .parse_next(input)?;

    // XXX need to be done here ?
    cut_err(
        literal([MARKER_ESCAPE, CMD_END_BIS])
            .context(StrContext::Label("Expected end of repeat block"))
    )
    .void()
    .parse_next(input)?;

    Ok(Item::Statement(Statement::RepeatInstruction(
        Box::new(expr),
        Box::new(item)
    )))
}

/// Parse single item EXCEPT newline, comments,  indenations, label, macrodref.
/// assign seems to be prefexid by 7f
fn parse_inner_item(input: &mut Input) -> OrgamsParseResult<Item> {
    // stop if end of line
    if peek(alt((
        consume_marker(MARKER_NEWLINE),
        consume_marker(MARKER_INDENT),
        consume_marker(MARKER_COMMENT)
    )))
    .parse_next(input)
    .is_ok()
    {
        let err = ContextError::new();
        return Err(ErrMode::Backtrack(err));
    }

    // if let Some(b) = input.as_bytes().get(0) {
    //    eprintln!("DBG: parse_item input[0]={:02X}", b);
    //}
    alt((
        parse_escaped_7f_item,
        parse_byte,
        parse_word,
        parse_star_repeat_single,
        parse_assign.map(Item::Assign),
        //  parse_indent.map(Item::Indent),
        parse_instruction.map(|i| Item::Statement(Statement::Instruction(i)))
    ))
    .parse_next(input)
}

fn parse_fill_inner(input: &mut Input) -> OrgamsParseResult<Item> {
    let size_expr = cut_err(
        parse_sized_expression
            .context(StrContext::Label("Failed to parse size expression in FILL"))
    )
    .parse_next(input)?;
    let value_expr = cut_err(parse_sized_expression.context(StrContext::Label(
        "Failed to parse value expression in FILL"
    )))
    .parse_next(input)?;

    let item = Item::Statement(Statement::Fill(size_expr, value_expr));
    Ok(item)
}

fn parse_byte(input: &mut Input) -> OrgamsParseResult<Item> {
    parse_word_or_byte(false).parse_next(input)
}

fn parse_word(input: &mut Input) -> OrgamsParseResult<Item> {
    parse_word_or_byte(true).parse_next(input)
}

fn parse_word_or_byte(is_word: bool) -> impl Fn(&mut Input) -> OrgamsParseResult<Item> {
    move |input: &mut Input| {
        let marker = if is_word { MARKER_WORD } else { MARKER_BYTE };

        consume_marker(marker)(input)?;

        let expression_length = cut_err(any).parse_next(input)? as usize;
        let _directive_length = cut_err(any).parse_next(input)?;
        let before_expressions = input.checkpoint();
        let mut exprs = Vec::new();
        while input.offset_from(&before_expressions) < expression_length - 2 {
            let expression = cut_err(
                parse_unsized_expression
                    .context(StrContext::Label("Failed to parse expression in list"))
            )
            .parse_next(input)?;
            exprs.push(expression);
        }

        expect_end_marker.parse_next(input)?;

        let item = if is_word {
            Item::Statement(Statement::Word(exprs))
        }
        else {
            Item::Statement(Statement::Byte(exprs))
        };
        Ok(item)
    }
}

/// Parse an indent marker (0x49) followed by space count
fn parse_indent(input: &mut Input) -> OrgamsParseResult<Indent> {
    consume_marker(MARKER_INDENT)(input)?;
    // Read the space count byte
    any.parse_next(input).map(Indent)
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
    let index_byte: u8 = any.verify(|&b| b >= SHORT_LABEL_START).parse_next(input)?;
    if index_byte >= LONG_LABEL_START {
        // Long label
        let second_byte: u8 = any.parse_next(input)?;
        Ok(LabelRef::new_long_from_stream(index_byte, second_byte))
    }
    else {
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

    if size == 0 {
        Ok(SizedExpression::Empty)
    }
    else {
        let input_checkpoint = input.checkpoint();
        let exp = parse_unsized_expression
            .map(SizedExpression::Sized)
            .parse_next(input)?;
        if input.offset_from(&input_checkpoint) != size {
            let mut err = ContextError::new();
            err.push(StrContext::Expected(StrContextValue::Description(
                "expression of incorrect size"
            )));
            Err(ErrMode::Cut(err))
        }
        else {
            Ok(exp)
        }
    }
}

fn parse_unsized_expression(input: &mut Input) -> OrgamsParseResult<Expression> {
    let first = peek(any).parse_next(input)?;

    if first != EXP_MULTI_TERM_BEGIN {
        return cut_err(
            parse_expression_member.context(StrContext::Label("single expression member"))
        )
        .map(Expression::SingleTerm)
        .parse_next(input);
    }
    else {
        return cut_err(parse_multi_expression.context(StrContext::Label("multi expression")))
            .parse_next(input);
    }
}
fn parse_multi_expression(input: &mut Input) -> OrgamsParseResult<Expression> {
    let _ = cut_err(EXP_MULTI_TERM_BEGIN.context(StrContext::Label("multi expression tag")))
        .parse_next(input)?; // Consume BEGIN
    let members = cut_err(parse_several_expression_member(EXP_MULTI_TERM_END)).parse_next(input)?;
    Ok(Expression::MultiTerm(members))
}

fn parse_several_expression_member(
    closing: u8
) -> impl Fn(&mut Input) -> OrgamsParseResult<Vec<ExpressionMember>> {
    move |input: &mut Input| -> OrgamsParseResult<Vec<ExpressionMember>> {
        let mut members = Vec::new();

        loop {
            let check = peek(any).parse_next(input)?;
            if check == closing {
                let _ = any.parse_next(input)?;
                break;
            }
            let member = cut_err(parse_expression_member.context(StrContext::Expected(
                StrContextValue::Description("Expression member in parenthesized expression")
            )))
            .parse_next(input)?;
            members.push(member);
        }

        Ok(members)
    }
}

fn parse_parenthesized_expression_inner(input: &mut Input) -> OrgamsParseResult<ExpressionMember> {
    let members = parse_several_expression_member(b')').parse_next(input)?;
    Ok(ExpressionMember::ParenthesizedExpression(members))
}

fn parse_expression_member(input: &mut Input) -> OrgamsParseResult<ExpressionMember> {
    let b = cut_err(any).parse_next(input)?;

    let is_local_label = b == EXP_LOCAL_LABEL;
    let b = if is_local_label {
        cut_err(any.context(StrContext::Label("Local label"))).parse_next(input)?
    }
    else {
        b
    };

    match b {
        ..=EXP_SHORT_DECIMAL_MAX_VALUE => Ok(ExpressionMember::ShortDecimal(b)),
        SHORT_LABEL_START..=SHORT_LABEL_END => {
            let label = LabelRef::new_short_from_stream(b);
            if is_local_label {
                Ok(ExpressionMember::LocalLabelRef(label))
            }
            else {
                Ok(ExpressionMember::LabelRef(label))
            }
        },
        LONG_LABEL_START.. => {
            // Long label
            let second_byte = cut_err(any).parse_next(input)?;
            let label = LabelRef::new_long_from_stream(b, second_byte);
            if is_local_label {
                Ok(ExpressionMember::LocalLabelRef(label))
            }
            else {
                Ok(ExpressionMember::LabelRef(label))
            }
        },
        EXP_OP_PAREN_CLOSE => {
            let mut err = ContextError::new();
            err.push(StrContext::Expected(StrContextValue::Description(
                "Use of ')' without matching '(' in expression"
            )));
            Err(ErrMode::Backtrack(err))
        },
        EXP_OP_PAREN_OPEN => {
            cut_err(
                parse_parenthesized_expression_inner.context(StrContext::Expected(
                    StrContextValue::Description("Parenthesized expression")
                ))
            )
            .parse_next(input)
        },
        EXP_STRING => {
            let s = cut_err(parse_sized_text.context(StrContext::Expected(
                StrContextValue::Description("String value in expression")
            )))
            .parse_next(input)?;
            Ok(ExpressionMember::String(s))
        },
        EXP_UNARY_MINUS => {
            let inner = cut_err(parse_expression_member.context(StrContext::Expected(
                StrContextValue::Description("Negated expression")
            )))
            .parse_next(input)?;
            Ok(ExpressionMember::UnaryMinus(Box::new(inner)))
        },
        EXP_OP_PLUS => Ok(ExpressionMember::Operator(Operator::Plus)),
        EXP_OP_MINUS => Ok(ExpressionMember::Operator(Operator::Minus)),
        EXP_OP_TIMES => {
            if let Ok(next_byte) = peek(any::<_, ContextError>).parse_next(input)
                && next_byte == EXP_OP_TIMES
            {
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
        // Note: EXP_OP_PAREN_OPEN (b == '(') and EXP_OP_PAREN_CLOSE (b == ')') are already
        // matched above by the `EXP_OP_PAREN_OPEN` / `EXP_OP_PAREN_CLOSE` arms, which parse a
        // full parenthesized sub-expression / report an unmatched ')' respectively. A bare
        // `Operator::ParenOpen`/`Operator::ParenClose` member is therefore never produced here.
        EXP_SPACE => Ok(ExpressionMember::Space),
        EXP_OP_AND => Ok(ExpressionMember::Operator(Operator::And)),
        EXP_OP_OR => Ok(ExpressionMember::Operator(Operator::Or)),
        EXP_OP_XOR => Ok(ExpressionMember::Operator(Operator::Xor)),
        EXP_OP_NEQ => Ok(ExpressionMember::Operator(Operator::NotEqual)),
        EXP_OP_GE => Ok(ExpressionMember::Operator(Operator::GreaterEqual)),
        EXP_OP_LE => Ok(ExpressionMember::Operator(Operator::LessEqual)),
        EXP_OP_LT => Ok(ExpressionMember::Operator(Operator::LessThan)),
        EXP_OP_GT => Ok(ExpressionMember::Operator(Operator::GreaterThan)),
        EXP_OP_EQ => Ok(ExpressionMember::Operator(Operator::Equal)),
        0x24 => Ok(ExpressionMember::Dollar), // TO BE CHECKED
        0x44 => Ok(ExpressionMember::DoubleDollar), // TO BE CHECKED

        EXP_ITER1 => Ok(ExpressionMember::Iter(1)),
        EXP_ITER2 => Ok(ExpressionMember::Iter(2)),
        EXP_ITER3 => Ok(ExpressionMember::Iter(3)),

        // Value types
        _ => {
            let (basis, content) = match b {
                // decimal
                EXP_DECIMAL_8 => {
                    let val = cut_err(any).parse_next(input)?;
                    (ValueBasis::Decimal, ValueContent::EightBits(val))
                },
                EXP_DECIMAL_16 => {
                    let low = cut_err(any).parse_next(input)? as u16;
                    let high = cut_err(any).parse_next(input)? as u16;
                    let val = low | (high << 8);
                    (ValueBasis::Decimal, ValueContent::SixteenBits(val))
                },
                EXP_DECIMAL_CUSTOM | EXP_DECIMAL_CUSTOM_LONG => {
                    let len = cut_err(any).parse_next(input)? as usize;
                    let bytes: Vec<u8> = take(len).parse_next(input)?.to_vec();
                    (ValueBasis::Decimal, ValueContent::Custom(bytes))
                },

                // hexadecimal
                EXP_HEXDECIMAL_8 => {
                    let val = any.parse_next(input)?;
                    (ValueBasis::Hexadecimal, ValueContent::EightBits(val))
                },
                EXP_HEXDECIMAL_16 => {
                    let low = cut_err(any).parse_next(input)? as u16;
                    let high = cut_err(any).parse_next(input)? as u16;
                    let val = low | (high << 8);
                    (ValueBasis::Hexadecimal, ValueContent::SixteenBits(val))
                },
                EXP_HEXDECIMAL_CUSTOM | EXP_HEXDECIMAL_CUSTOM_LONG => {
                    let len = cut_err(any).parse_next(input)? as usize;
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
                    let len = cut_err(any).parse_next(input)? as usize;
                    let bytes: Vec<u8> = take(len).parse_next(input)?.to_vec();
                    (ValueBasis::Binary, ValueContent::Custom(bytes))
                },

                _ => {
                    let mut err = ContextError::new();
                    err.push(StrContext::Expected(StrContextValue::Description(
                        "Invalid expression member"
                    )));
                    return Err(ErrMode::Backtrack(err));
                }
            };

            Ok(ExpressionMember::Value(Value { basis, content }))
        }
    }
}

/// Parse an assignment: 0x64 <label_ref> <expression>
fn parse_assign(input: &mut Input) -> OrgamsParseResult<Assign> {
    // Check marker and valid label ahead to differentiate from text containing 'd'
    let (_, next_byte) = peek((literal([MARKER_ASSIGN]), any)).parse_next(input)?;
    if next_byte < SHORT_LABEL_START {
        let mut err = ContextError::new();
        err.push(StrContext::Expected(StrContextValue::Description(
            "A label is expected after assignment marker"
        )));
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

    Ok(Assign { label, expression })
}

/// Parse a line according to orgams t_line grammar
/// Returns Line for the line (may include indent, content, newline)
pub fn parse_line(input: &mut Input) -> OrgamsParseResult<Line> {
    // order is quite important here
    alt((
        parse_line_starting_with_comment.context(StrContext::Label("line starting with comment")),
        parse_line_starting_with_label.context(StrContext::Label("line starting with label")),
        parse_line_assign.context(StrContext::Label("line starting with assignment")),
        parse_line_empty.context(StrContext::Label("empty line")),
        parse_line_starting_with_macro.context(StrContext::Label("line starting with macro")),
        parse_line_starting_with_item.context(StrContext::Label("line starting with item"))
    ))
    .parse_next(input)
}

fn parse_nl_or_comment(input: &mut Input) -> OrgamsParseResult<Vec<Item>> {
    alt((
        parse_endline.map(|_| vec![Item::NewLine]),
        parse_space_and_comment
    ))
    .context(StrContext::Label("New line or comment"))
    .parse_next(input)
}

// The label MUST be followed by another token (not newline or comment)
fn parse_line_starting_with_label(input: &mut Input) -> OrgamsParseResult<Line> {
    let label_kind: u8 = any
        .verify(|&b| b == MARKER_LABEL_ADDR || b == MARKER_LOCAL_LABEL)
        .parse_next(input)?;
    let label = cut_err(parse_label.context(StrContext::Label("label"))).parse_next(input)?;
    let label = if label_kind == MARKER_LABEL_ADDR {
        Item::Label(label)
    }
    else {
        Item::LocalLabel(label)
    };

    let nl_or_comment = opt(parse_nl_or_comment).parse_next(input)?;
    let (items, nl_or_comment) = if let Some(nl_or_comment) = nl_or_comment {
        (None, nl_or_comment)
    }
    else {
        let items = parse_items.parse_next(input)?;
        let nl_or_comment = cut_err(parse_nl_or_comment).parse_next(input)?;
        (Some(items), nl_or_comment)
    };

    let mut res = Vec::with_capacity(1 + items.as_ref().map(|items| items.len()).unwrap_or(0) + 1);
    res.push(label);
    if let Some(mut items) = items {
        res.append(&mut items);
    }
    res.extend(nl_or_comment);
    Ok(Line { items: res })
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
    }
    else {
        vec![Item::Comment(comment)]
    };

    Ok(items)
}

fn parse_line_starting_with_comment(input: &mut Input) -> OrgamsParseResult<Line> {
    parse_space_and_comment
        .map(|items| Line { items })
        .parse_next(input)
}

fn parse_line_starting_with_item(input: &mut Input) -> OrgamsParseResult<Line> {
    let mut items: Vec<Item> = Vec::new();

    let mut first_loop = true;
    loop {
        let item = if first_loop {
            parse_inner_item.parse_next(input)?
        }
        else {
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
        }
        else if next == MARKER_COMMENT {
            let c = parse_comment.parse_next(input)?;
            items.push(Item::Comment(c));
            break;
        }
        else {
            // There is another item on the same line; loop to parse it
            continue;
        }
    }

    Ok(Line { items })
}

fn parse_all_code(
    debug: bool,
    labels: &StringTable,
    groundtruth_iter: &mut Option<std::slice::Iter<String>>
) -> impl FnMut(&mut Input) -> OrgamsParseResult<Vec<Chunk>> {
    move |input: &mut Input| {
        let mut chunks = Vec::new();
        let mut chunk_idx = 0;
        let mut start_line = 0;
        loop {
            // Peek to check for the terminator (chunk size 0)
            let b = peek(any).parse_next(input)?;
            if b == 0 {
                if debug {
                    println!("DEBUG: End of code segments (byte 0 found).");
                }
                break;
            }

            if debug {
                println!("DEBUG: Parsing Chunk #{}", chunk_idx);
            }
            let (line_count, chunk) =
                parse_chunk(debug, labels, groundtruth_iter, chunk_idx, start_line)
                    .parse_next(input)?;
            start_line += line_count.unwrap_or_default(); // We don't care in no debug
            chunk_idx += 1;
            chunks.push(chunk)
        }
        Ok(chunks)
    }
}

/// A chunk is composed of several lines, prefixed by its size in bytesa
/// TODO retreive the logic of parse_chunk_debug it may not be exactly the same now
pub fn parse_chunk(
    debug: bool,
    labels: &StringTable,
    groundtruth_iter: &mut Option<std::slice::Iter<String>>,
    chunk_idx: usize,
    start_line: usize
) -> impl FnMut(&mut Input) -> OrgamsParseResult<(Option<usize>, Chunk)> {
    move |input: &mut Input| {
        let chunk_size = any.verify(|&s| s <= CHUNK_MAX_SIZE).parse_next(input)? as usize;

        // Chunk content peek
        let chunk_content = peek(take(chunk_size)).parse_next(input)?;
        if debug {
            println!("DEBUG: Chunk #{} size: {} bytes", chunk_idx, chunk_size);
            println!(
                "DEBUG: Chunk #{} content bytes: {:02X?}",
                chunk_idx, chunk_content
            );
        }

        let chunk_start = input.checkpoint();

        let mut render = if debug {
            Some(DisplayState::new(None))
        }
        else {
            None
        };
        let mut line_number = 0;

        let mut chunk_content = Vec::new();
        while input.offset_from(&chunk_start) < chunk_size {
            let line_offset = input.offset_from(&chunk_start);

            let line_start_check = input.checkpoint();

            // println!("Remaining bytes: [{:02X?}]", &input[.. ]);

            let line = match parse_line.parse_next(input) {
                Ok(line) => {
                    line_number += 1;
                    let consumed_len = input.offset_from(&line_start_check);

                    if let Some(render) = render.as_mut() {
                        // Reconstruct line
                        render.render_items(line.iter(), labels).unwrap();
                        let line_text = render.last_line().unwrap().to_string(); // XXX we assume only one line has been generated

                        // Get bytes
                        input.reset(&line_start_check);
                        let raw_bytes = take(consumed_len).parse_next(input)?;

                        // Printable bytes
                        let printable: String = raw_bytes
                            .iter()
                            .map(|&b| {
                                if (32..=126).contains(&b) {
                                    b as char
                                }
                                else {
                                    '.'
                                }
                            })
                            .collect();
                        let reconstructed_bytes = line.bytes(labels);
                        println!(
                            "\n Chunk: {}       line: {} (total: {})",
                            chunk_idx,
                            line_number,
                            start_line + line_number
                        );
                        println!("          Debug:  {:?}", line);
                        println!("    [{:3}] Bytes:  {:02X?}", line_offset, raw_bytes);
                        println!("  Reconstructed:  {:02X?} ", reconstructed_bytes);
                        println!("      Printable:  {}", printable);
                        println!("           Text:  `{}`", line_text);
                        let groundtruth = if let Some(iter) = groundtruth_iter {
                            if let Some(ground) = iter.next() {
                                println!("         Ground:  `{}`", ground);
                                Some(ground.to_owned())
                            }
                            else {
                                println!("         Ground:  <End of Stream>");
                                Some("".to_owned())
                            }
                        }
                        else {
                            None
                        };

                        assert_eq!(
                            raw_bytes, reconstructed_bytes,
                            "Reconstructed bytes do not match original bytes at chunk offset {}",
                            line_offset
                        );
                        if let Some(groundtruth) = groundtruth {
                            assert_eq!(
                                groundtruth, line_text,
                                "Groundtruth does not match reconstructed text at chunk offset {}",
                                line_offset
                            );
                        }
                        assert_eq!(
                            render.line_number() - 1,
                            line_number,
                            "Line number mismatch at chunk offset {}",
                            line_offset
                        );
                    }

                    line
                },
                Err(e) => {
                    if debug {
                        println!("    ERROR at chunk offset {}: {:?}", line_offset, e);
                        let remainder = &input[..];
                        let context_len = std::cmp::min(100, remainder.len());
                        let context_bytes = &remainder[..context_len];
                        let context_printable: String = context_bytes
                            .iter()
                            .map(|&b| {
                                if (32..=126).contains(&b) {
                                    b as char
                                }
                                else {
                                    '.'
                                }
                            })
                            .collect();

                        println!(
                            "    Context bytes (next {}): {:02X?}",
                            context_len, context_bytes
                        );
                        println!(
                            "    Context chars (next {}): {}",
                            context_len, context_printable
                        );
                    }
                    return Err(e);
                }
            };

            chunk_content.push(line);
        }

        if input.offset_from(&chunk_start) != chunk_size {
            println!(
                "    WARN: Chunk mismatch. Expected {}, got {}",
                chunk_size,
                input.offset_from(&chunk_start)
            );
        }

        let line = render.as_mut().map(|render| render.line_number() - 1);
        Ok((
            line,
            Chunk {
                lines: chunk_content
            }
        ))
    }
}

fn parse_escaped_7f_item(input: &mut Input) -> OrgamsParseResult<Item> {
    // Expect escape marker
    consume_marker(MARKER_ESCAPE)(input)?;

    // check if it is escaped instruction
    let first = peek(any).parse_next(input)?;
    if is_escaped_byte(first) {
        return parse_instruction
            .map(|i| Item::Statement(Statement::Instruction(i)))
            .parse_next(input);
    };

    // otherwhise, parse command

    // get the command byte
    let cmd = cut_err(any).parse_next(input)?;

    match cmd {
        CMD_FACTOR_BLOC => {
            let expr = cut_err(
                parse_sized_expression.context(StrContext::Label("Start repeat bloc expression"))
            )
            .parse_next(input)?;
            Ok(Item::Statement(Statement::StartRepeatBloc(expr)))
        },
        CMD_FACTOR_BLOC_END => Ok(Item::Statement(Statement::StopRepeatBloc)),
        CMD_IF => {
            cut_err(parse_inner_if.context(StrContext::Label("IF")))
                .parse_next(input)
                .map(Item::Statement)
        },
        CMD_ELSE => Ok(Item::Statement(Statement::Else)),
        CMD_END => Ok(Item::Statement(Statement::End)),
        CMD_ENDM => Ok(Item::Statement(Statement::EndMacro)),
        CMD_FILL => cut_err(parse_fill_inner.context(StrContext::Label("FILL"))).parse_next(input),
        CMD_IMPORT => {
            // Escaped IMPORT command with sized text
            let s = cut_err(parse_sized_text).parse_next(input)?;
            Ok(Item::Statement(Statement::Import(s)))
        },
        CMD_ENT => {
            let expr = cut_err(parse_sized_expression).parse_next(input)?;
            Ok(Item::Statement(Statement::Ent(expr)))
        },
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
        },
        CMD_STORE_PC_INSTR => Ok(Item::Statement(Statement::StorePcInstr)),
        CMD_END_BIS => Ok(Item::Statement(Statement::EndBis)),
        CMD_ASIS => {
            // Raw string: Length + Bytes
            let len = cut_err(any).parse_next(input)? as usize;
            let content = cut_err(take(len)).parse_next(input)?;
            Ok(Item::Statement(Statement::RawString(OrgamsEncodedString(
                content.to_vec()
            ))))
        },
        CMD_REPEAT => {
            cut_err(
                parse_star_repeat_single.context(StrContext::Label("REPEAT single instruction"))
            )
            .parse_next(input)
        },
        CMD_MACRO_USE => {
            cut_err(parse_macro_use)
                .parse_next(input)
                .map(Item::Statement)
        },

        CMD_BRK => Ok(Item::Statement(Statement::Brk)),
        CMD_RESTORE => Ok(Item::Statement(Statement::Restore)),

        _ => {
            let mut err = ContextError::new();
            err.push(StrContext::Expected(StrContextValue::Description(
                "Unknown escaped command"
            )));
            Err(ErrMode::Cut(err))
        }
    }
}

fn parse_inner_if(input: &mut Input) -> OrgamsParseResult<Statement> {
    let condition = cut_err(parse_sized_expression).parse_next(input)?;

    Ok(Statement::If(condition))
}

fn parse_instruction_prefix(input: &mut Input) -> OrgamsParseResult<InstructionPrefix> {
    let b = cut_err(peek(any)).parse_next(input)?;
    if !InstructionPrefix::is_valid_prefix(b) {
        Ok(InstructionPrefix::None)
    }
    else {
        let first = cut_err(any).parse_next(input)?;
        if [0xDD, 0xDF, 0xDF, 0xFF].contains(&first) {
            let second = peek(opt(0xCB)).parse_next(input)?;
            if let Some(second) = second {
                let _ = cut_err(any).parse_next(input)?; // consume
                Ok(InstructionPrefix::from_bytes(first, second))
            }
            else {
                Ok(InstructionPrefix::from_byte(first))
            }
        }
        else {
            Ok(InstructionPrefix::from_byte(first))
        }
    }
}

fn parse_instruction(input: &mut Input) -> OrgamsParseResult<Instruction> {
    let prefix = cut_err(parse_instruction_prefix.context(StrContext::Label("Instruction prefix")))
        .parse_next(input)?;
    let opcode = cut_err(any.context(StrContext::Label("Opcode"))).parse_next(input)?;
    let repr = prefix.disassembler_table()[opcode as usize];

    let coded_operands = if prefix.requires_extra_expression(opcode) {
        let param = cut_err(parse_sized_expression.context(StrContext::Expected(
            StrContextValue::Description("Expression member for set/res/bit ?; (hl)")
        )))
        .parse_next(input)?;
        vec![param]
    }
    else {
        let kinds = z80str_to_expressions_list(repr);
        let mut coded_operands = Vec::with_capacity(kinds.len());
        for _kind in kinds {
            let expr = cut_err(parse_sized_expression.context(StrContext::Expected(
                StrContextValue::Description("Expression for instruction")
            )))
            .parse_next(input)?;
            coded_operands.push(expr);
        }
        coded_operands
    };

    Ok(Instruction {
        prefix,
        opcode,
        coded_operands
    })
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
        }
        else {
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

    Ok(Item::MacroDef(MacroDef {
        def_block_len,
        name,
        params
    }))
}

fn expect_end_marker(input: &mut Input) -> OrgamsParseResult<()> {
    cut_err(
        consume_marker(END_MARKER).context(StrContext::Expected(StrContextValue::Description(
            "Missing end marker"
        )))
    )
    .parse_next(input)
}

/// Parse macro call: length + name + args + endmarker
fn parse_macro_use(input: &mut Input) -> OrgamsParseResult<Statement> {
    let length = cut_err(any).parse_next(input)? as usize;

    let input_start = input.checkpoint();
    let name = parse_unsized_expression
        .context(StrContext::Expected(StrContextValue::Description(
            "Macro name"
        )))
        .parse_next(input)?;
    let mut args = Vec::new();
    while input.offset_from(&input_start) < length - 1 {
        let arg = cut_err(parse_unsized_expression.context(StrContext::Expected(
            StrContextValue::Description("Macro argument")
        )))
        .parse_next(input)?;
        args.push(arg);
    }

    expect_end_marker(input)?;
    let bytes_consumed = input.offset_from(&input_start);
    if bytes_consumed != length {
        let mut err = ContextError::new();
        err.push(StrContext::Expected(StrContextValue::Description(
            "Wrong macro length consummed"
        )));
        return Err(ErrMode::Cut(err));
    }

    Ok(Statement::MacroUse(name, args))
}

/// Parse a text prefixed by its size in bytes
fn parse_sized_text(input: &mut Input) -> OrgamsParseResult<SizedString> {
    // Read size byte
    let size = any.parse_next(input)? as usize;
    let text_bytes: Vec<u8> = cut_err(take(size)).parse_next(input)?.to_vec();
    Ok(SizedString(OrgamsEncodedString(text_bytes)))
}

fn parse_bit7on_text(input: &mut Input) -> OrgamsParseResult<Bit7OnString> {
    let mut bytes = Vec::new();
    loop {
        let byte: u8 = cut_err(
            any.context(StrContext::Expected(StrContextValue::Description(
                "bit7on text byte"
            )))
        )
        .parse_next(input)?;
        bytes.push(byte);
        if byte & (1 << 7) != 0 {
            break;
        }
    }
    Ok(Bit7OnString::new(&bytes))
}

#[cfg(test)]
mod tests {
    use fs_err as fs;

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
        assert_eq!(&result.unwrap().to_string(), "Hello, World!");
    }

    #[test]
    fn test_parse_comment_with_newline() {
        // Comment followed by newline: 0x43 + size + text + newline
        let data = b"\x43\x11This is a comment\x4aNext line";
        let mut input = LocatingSlice::new(&data[..]);

        let result = parse_comment(&mut input);
        assert!(result.is_ok());
        assert_eq!(&result.unwrap().to_string(), "This is a comment");

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
            &result.unwrap().to_string(),
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
            0x8A, // label (138 - 96 = 42)
            0x20, // space
            0x2B, // + operator
            0x20, // space
            0x87, // label (135 - 96 = 39)
            0x20, // space
            0x2D, // - operator
            0x20, // space
            0x01, // short decimal 1
            0x20, // space
            0x2F, // / operator
            0x20, // space
            0x87, // label (135 - 96 = 39)
            0x45, // EXP_MULTI_TERM_END
        ];

        // Create full expression bytes with size prefix
        let mut expr_bytes = vec![expr_content.len() as u8];
        expr_bytes.extend_from_slice(&expr_content);

        let mut input = LocatingSlice::new(&expr_bytes[..]);
        let expr =
            parse_sized_expression(&mut input).expect("Failed to parse multi-term expression");

        // Create empty string table for serialization
        let table = StringTable::default();

        // Verify it round-trips correctly (bytes() includes size prefix)
        let serialized = expr.bytes(&table);
        assert_eq!(
            serialized, expr_bytes,
            "Expression should round-trip correctly including size byte"
        );

        // Verify structure
        match expr {
            SizedExpression::Sized(Expression::MultiTerm(members)) => {
                assert_eq!(members.len(), 13, "Multi-term should have 13 members");
            },
            _ => panic!("Expected MultiTerm expression")
        }
    }

    fn verify_parsing_and_reconstruction(path: &str) {
        let data = fs::read(path).unwrap();
        let input = LocatingSlice::new(data.as_slice());

        let result = parse_orgams_file(false, &mut None).parse(input);

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
            println!(
                "Length mismatch! Expected: {}, Generated: {}",
                expected_payload.len(),
                generated_payload.len()
            );
        }

        // Better error message: find first diff
        if let Some(pos) = generated_payload
            .iter()
            .zip(expected_payload.iter())
            .position(|(a, b)| a != b)
        {
            let abs_pos = start_items + pos;
            panic!(
                "Byte mismatch at offset {}: expected 0x{:02X}, got 0x{:02X}\nOriginal: {:?}\nGenerated: {:?}",
                abs_pos,
                expected_payload[pos],
                generated_payload[pos],
                &expected_payload
                    [pos.saturating_sub(10)..std::cmp::min(pos + 10, expected_payload.len())],
                &generated_payload
                    [pos.saturating_sub(10)..std::cmp::min(pos + 10, generated_payload.len())]
            );
        }

        assert_eq!(
            generated_payload.len(),
            expected_payload.len(),
            "Payload length mismatch"
        );
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
            0x43, 0x2A, 0x20, 0x3C, 0x3C, 0x3C, 0x3C, 0x20, 0x43, 0x6F, 0x6E, 0x73, 0x74, 0x61,
            0x6E, 0x74, 0x73, 0x20, 0x73, 0x68, 0x61, 0x72, 0x65, 0x64, 0x20, 0x61, 0x63, 0x72,
            0x6F, 0x73, 0x73, 0x20, 0x6D, 0x6F, 0x64, 0x75, 0x6C, 0x65, 0x73, 0x20, 0x3E, 0x3E,
            0x3E, 0x3E, 0x43, 0x2B, 0x20, 0x32, 0x30, 0x32, 0x35, 0x20, 0x4A, 0x75, 0x6C, 0x20,
            0x32, 0x37, 0x3A, 0x20, 0x4D, 0x6F, 0x76, 0x65, 0x20, 0x73, 0x74, 0x6F, 0x72, 0x65,
            0x20, 0x6C, 0x65, 0x6E, 0x67, 0x74, 0x68, 0x73, 0x20, 0x74, 0x6F, 0x20, 0x73, 0x77,
            0x61, 0x70, 0x69, 0x2E, 0x69, 0x49, 0x05, 0x43, 0x2C, 0x20, 0x4A, 0x75, 0x6E, 0x20,
            0x31, 0x31, 0x3A, 0x20, 0x55, 0x70, 0x64, 0x61, 0x74, 0x65, 0x20, 0x61, 0x73, 0x73,
            0x5F, 0x73, 0x74, 0x6F, 0x72, 0x65, 0x5F, 0x6C, 0x65, 0x6E, 0x20, 0x73, 0x79, 0x6D,
            0x62, 0x5F, 0x73, 0x74, 0x6F, 0x72, 0x65, 0x5F, 0x6C, 0x65, 0x6E, 0x49, 0x0D, 0x43,
            0x17, 0x20, 0x52, 0x65, 0x6D, 0x6F, 0x76, 0x65, 0x20, 0x6F, 0x62, 0x73, 0x6F, 0x6C,
            0x65, 0x74, 0x65, 0x20, 0x63, 0x68, 0x65, 0x63, 0x6B, 0x73, 0x4A
        ];

        let mut input = LocatingSlice::new(data);
        let mut lines = Vec::new();
        loop {
            if input.is_empty() {
                break;
            }
            match parse_line(&mut input) {
                Ok(line) => {
                    // println!("Parsed line: {:?}", line);
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
            0x49, 0x0D, 0x43, 0x17, 0x20, 0x52, 0x65, 0x6D, 0x6F, 0x76, 0x65, 0x20, 0x6F, 0x62,
            0x73, 0x6F, 0x6C, 0x65, 0x74, 0x65, 0x20, 0x63, 0x68, 0x65, 0x63, 0x6B, 0x73, 0x4A
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

    /// Regression test for the panic-on-malformed-input bug: a real, well-formed `.O` file's
    /// `LBLs` section is truncated to zero labels directly in the raw bytes (header and `SRCc`
    /// source bytes are left completely untouched), simulating a corrupted/truncated label
    /// table. The result still parses cleanly (the winnow grammar does not know how many labels
    /// the table is "supposed" to have) since every label reference in the source is re-encoded
    /// as a plain index, independent of the table's actual contents — but every one of those
    /// references is now out of range, and rendering must now fail gracefully with `Err`
    /// instead of panicking.
    #[test]
    fn test_render_fails_on_label_reference_out_of_range() {
        let path = "tests/orgams-main/FARCALL.O";
        let data = fs::read(path).unwrap();

        // Sanity check: the original, uncorrupted file parses and renders fine.
        crate::convert::binary_orgams_to_utf8(&data)
            .unwrap_or_else(|e| panic!("Failed to render unmodified {path}: {e}"));

        let mut input_wrapper = Input::new(data.as_slice());
        let program = parse_orgams_file(false, &mut Option::<std::slice::Iter<String>>::None)
            .parse_next(&mut input_wrapper)
            .unwrap_or_else(|e| panic!("Failed to parse {path}: {e:?}"));

        assert!(
            !program.labels.strings.is_empty(),
            "fixture {path} is expected to have a non-empty label table for this test to be \
             meaningful"
        );

        // Locate the `LBLs` section in the raw file bytes, and the `ChCk` checksum section that
        // follows it, so we can splice in a corrupted `LBLs` section while leaving the header
        // and `SRCc` source bytes (and therefore every label reference's raw encoding) exactly
        // as they were.
        let lbls_pos = data
            .windows(4)
            .position(|w| w == b"LBLs")
            .expect("fixture should contain an LBLs section");
        let chck_pos = data[lbls_pos..]
            .windows(4)
            .position(|w| w == b"ChCk")
            .map(|p| lbls_pos + p)
            .unwrap_or(data.len());

        let mut corrupted = Vec::new();
        corrupted.extend_from_slice(&data[..lbls_pos]);
        corrupted.extend_from_slice(b"LBLs");
        corrupted.push(2); // version
        corrupted.push(0); // null terminator: zero labels, so every reference used by the
        // (untouched) source is now out of range for this table.
        corrupted.extend_from_slice(&data[chck_pos..]);

        let mut corrupted_input = Input::new(corrupted.as_slice());
        let reparsed = parse_orgams_file(false, &mut Option::<std::slice::Iter<String>>::None)
            .parse_next(&mut corrupted_input)
            .unwrap_or_else(|e| panic!("Corrupted file should still parse cleanly: {e:?}"));
        assert_eq!(
            reparsed.labels.len(),
            0,
            "corrupted label table should be empty"
        );

        // The actual assertion: rendering a program with an out-of-range label reference must
        // return `Err`, not panic.
        let render_result = reparsed.try_render();
        assert!(
            render_result.is_err(),
            "rendering a program with an out-of-range label reference should fail gracefully, \
             not succeed"
        );

        // The full public entry point must behave the same way.
        let full_pipeline_result = crate::convert::binary_orgams_to_utf8(&corrupted);
        assert!(
            full_pipeline_result.is_err(),
            "binary_orgams_to_utf8 should return Err on a corrupted label table, not panic"
        );
    }
}
