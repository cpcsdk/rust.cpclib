/// Located (position-aware) Basic token types for use by the LSP.
///
/// This module provides a winnow-based lexer that scans Locomotive BASIC source
/// text line by line and records the source position (line, column, length) of
/// every token.  The input stream type is `LocatingSlice<&str>`, consistent
/// with how `cpclib-asm` tracks spans.
use cpclib_common::winnow::ascii::Caseless;
use cpclib_common::winnow::combinator::{alt, opt, peek};
use cpclib_common::winnow::error::{ContextError, ErrMode};
use cpclib_common::winnow::stream::{LocatingSlice, Offset};
use cpclib_common::winnow::token::{any, one_of, take_while};
use cpclib_common::winnow::{ModalResult, Parser};

use crate::tokens::{BasicTokenNoPrefix, BasicTokenPrefixed};
use crate::BasicError;

// ─── Stream type alias ────────────────────────────────────────────────────────

/// The winnow stream used throughout this module.
/// `LocatingSlice<&str>` is `Copy`, so we can snapshot it freely for backtracking.
type Input<'a> = LocatingSlice<&'a str>;

// ─── Position ─────────────────────────────────────────────────────────────────

/// A half-open source span `[col, col+len)` on a 0-based source line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub line: u32,
    pub col:  u32,
    pub len:  u32,
}

// ─── Token kinds ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum LocatedTokenKind {
    Keyword(BasicTokenNoPrefix),
    Function(BasicTokenPrefixed),
    /// Variable name, preserving source case.
    Variable(String),
    Number(String),
    /// String literal content (without the surrounding quotes).
    StringLit(String),
    /// Comment text (without the leading `REM` / `'`).
    Comment(String),
    Operator(BasicTokenNoPrefix),
    Space,
    Separator,
    /// The BASIC line number that opens a source line (`10`, `20`, …).
    LineNumber(u16),
    Other(char),
}

// ─── Token ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LocatedBasicToken {
    pub kind: LocatedTokenKind,
    pub span: SourceSpan,
}

// ─── Line ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LocatedBasicLine {
    pub line_number: u16,
    /// 0-based source-text line index.
    pub source_line: u32,
    pub tokens: Vec<LocatedBasicToken>,
}

impl LocatedBasicLine {
    /// Iterate over `(for_keyword_token, variable_token)` pairs on this line.
    pub fn for_vars(&self) -> impl Iterator<Item = (&LocatedBasicToken, &LocatedBasicToken)> {
        let mut pairs: Vec<(&LocatedBasicToken, &LocatedBasicToken)> = Vec::new();
        let mut i = 0;
        while i < self.tokens.len() {
            if matches!(self.tokens[i].kind, LocatedTokenKind::Keyword(BasicTokenNoPrefix::For)) {
                let mut j = i + 1;
                while j < self.tokens.len()
                    && matches!(self.tokens[j].kind, LocatedTokenKind::Space)
                {
                    j += 1;
                }
                if j < self.tokens.len() {
                    if let LocatedTokenKind::Variable(_) = &self.tokens[j].kind {
                        pairs.push((&self.tokens[i], &self.tokens[j]));
                    }
                }
            }
            i += 1;
        }
        pairs.into_iter()
    }
}

// ─── Program ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LocatedBasicProgram {
    pub lines: Vec<LocatedBasicLine>,
}

impl LocatedBasicProgram {
    pub fn find_line(&self, line_number: u16) -> Option<&LocatedBasicLine> {
        self.lines.iter().find(|l| l.line_number == line_number)
    }

    /// Parse plain-text BASIC source.  Lines not starting with a decimal
    /// line number are silently skipped.
    pub fn parse(source: &str) -> Result<Self, BasicError> {
        let mut lines = Vec::new();

        for (line_idx, raw_line) in source.lines().enumerate() {
            let line = raw_line.trim_end_matches('\r');
            if line.trim().is_empty() {
                continue;
            }

            let line_start: Input<'_> = LocatingSlice::new(line);
            let mut input = line_start;

            // Skip leading spaces.
            let _ = parse_spaces(&mut input);

            // Must start with a decimal line number.
            let before_num = input;
            let num_str: &str =
                match take_while::<_, Input<'_>, ContextError>(1.., |c: char| c.is_ascii_digit())
                    .parse_next(&mut input)
                {
                    Ok(s) => s,
                    Err(_) => continue,
                };
            let basic_line_number: u16 = match num_str.parse() {
                Ok(n) => n,
                Err(_) => continue,
            };

            let source_line = line_idx as u32;

            let ln_token = LocatedBasicToken {
                kind: LocatedTokenKind::LineNumber(basic_line_number),
                span: SourceSpan {
                    line: source_line,
                    col:  before_num.offset_from(&line_start) as u32,
                    len:  input.offset_from(&before_num) as u32,
                },
            };

            // Body after the line number.
            let body_start_byte = input.offset_from(&line_start);
            let body_str = &line[body_start_byte..];
            let body_col_offset = body_start_byte as u32;

            let mut tokens = vec![ln_token];
            tokens.extend(lex_body(body_str, source_line, body_col_offset));
            lines.push(LocatedBasicLine { line_number: basic_line_number, source_line, tokens });
        }

        Ok(LocatedBasicProgram { lines })
    }
}

// ─── Trait ────────────────────────────────────────────────────────────────────

pub trait BasicLineT {
    fn basic_line_number(&self) -> u16;
}

impl BasicLineT for crate::BasicLine {
    fn basic_line_number(&self) -> u16 { self.line_number() }
}

impl BasicLineT for LocatedBasicLine {
    fn basic_line_number(&self) -> u16 { self.line_number }
}

// ─── Keyword table ────────────────────────────────────────────────────────────

#[derive(Clone)]
enum KwKind {
    Keyword(BasicTokenNoPrefix),
    Function(BasicTokenPrefixed),
}

struct KwEntry {
    text: &'static str,
    kind: KwKind,
}

/// Returns `true` if `c` can immediately follow a keyword and still be part
/// of an identifier (i.e. there is no word boundary after the keyword text).
#[inline]
fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '$' || c == '%' || c == '_'
}

static KEYWORD_TABLE: std::sync::LazyLock<Vec<KwEntry>> = std::sync::LazyLock::new(|| {
    use BasicTokenNoPrefix as K;
    use BasicTokenPrefixed as F;

    let mut v: Vec<KwEntry> = vec![
        // Multi-word (longest first after sort).
        KwEntry { text: "ON ERROR GOTO", kind: KwKind::Keyword(K::OnErrorGoto) },
        KwEntry { text: "ON BREAK",      kind: KwKind::Keyword(K::OnBreak) },
        // 9-char
        KwEntry { text: "RANDOMIZE",     kind: KwKind::Keyword(K::Randomize) },
        KwEntry { text: "COPYCHR$",      kind: KwKind::Function(F::CopycharDollar) },
        // 8-char
        KwEntry { text: "GRAPHICS",      kind: KwKind::Keyword(K::Graphics) },
        KwEntry { text: "CLOSEOUT",      kind: KwKind::Keyword(K::Closeout) },
        KwEntry { text: "STRING$",       kind: KwKind::Function(F::StringDollar) },
        // 7-char
        KwEntry { text: "TESTSTR",       kind: KwKind::Function(F::Teststr) },
        KwEntry { text: "INKEY$",        kind: KwKind::Function(F::InkeyDollar) },
        KwEntry { text: "SPACE$",        kind: KwKind::Function(F::SpaceDollar) },
        KwEntry { text: "RIGHT$",        kind: KwKind::Function(F::RightDollar) },
        KwEntry { text: "LOWER$",        kind: KwKind::Function(F::LowerDollar) },
        KwEntry { text: "UPPER$",        kind: KwKind::Function(F::UpperDollar) },
        KwEntry { text: "CLOSEIN",       kind: KwKind::Keyword(K::Closein) },
        KwEntry { text: "DEFREAL",       kind: KwKind::Keyword(K::Defreal) },
        KwEntry { text: "OPENOUT",       kind: KwKind::Keyword(K::Openout) },
        KwEntry { text: "RESTORE",       kind: KwKind::Keyword(K::Restore) },
        KwEntry { text: "RELEASE",       kind: KwKind::Keyword(K::Release) },
        KwEntry { text: "RETURN",        kind: KwKind::Keyword(K::Return) },
        KwEntry { text: "REMAIN",        kind: KwKind::Function(F::Remain) },
        KwEntry { text: "WINDOW",        kind: KwKind::Keyword(K::Window) },
        KwEntry { text: "MEMORY",        kind: KwKind::Keyword(K::Memory) },
        KwEntry { text: "LOCATE",        kind: KwKind::Keyword(K::Locate) },
        KwEntry { text: "ORIGIN",        kind: KwKind::Keyword(K::Origin) },
        KwEntry { text: "OPENIN",        kind: KwKind::Keyword(K::Openin) },
        KwEntry { text: "RESUME",        kind: KwKind::Keyword(K::Resume) },
        KwEntry { text: "SYMBOL",        kind: KwKind::Keyword(K::Symbol) },
        KwEntry { text: "CURSOR",        kind: KwKind::Keyword(K::Cursor) },
        KwEntry { text: "DEFSTR",        kind: KwKind::Keyword(K::Defstr) },
        KwEntry { text: "DEFINT",        kind: KwKind::Keyword(K::Defint) },
        KwEntry { text: "DELETE",        kind: KwKind::Keyword(K::Delete) },
        KwEntry { text: "TAGOFF",        kind: KwKind::Keyword(K::Tagoff) },
        KwEntry { text: "TROFF",         kind: KwKind::Keyword(K::Troff) },
        KwEntry { text: "EVERY",         kind: KwKind::Keyword(K::Every) },
        KwEntry { text: "ERASE",         kind: KwKind::Keyword(K::Erase) },
        KwEntry { text: "INSTR",         kind: KwKind::Function(F::Instr) },
        KwEntry { text: "LEFT$",         kind: KwKind::Function(F::LeftDollar) },
        KwEntry { text: "LOG10",         kind: KwKind::Function(F::Log10) },
        KwEntry { text: "CREAL",         kind: KwKind::Function(F::Creal) },
        // 6-char
        KwEntry { text: "BORDER",        kind: KwKind::Keyword(K::Border) },
        KwEntry { text: "USING",         kind: KwKind::Keyword(K::Using) },
        KwEntry { text: "WHILE",         kind: KwKind::Keyword(K::While) },
        KwEntry { text: "WIDTH",         kind: KwKind::Keyword(K::Width) },
        KwEntry { text: "WRITE",         kind: KwKind::Keyword(K::Write) },
        KwEntry { text: "SOUND",         kind: KwKind::Keyword(K::Sound) },
        KwEntry { text: "SPEED",         kind: KwKind::Keyword(K::Speed) },
        KwEntry { text: "PRINT",         kind: KwKind::Keyword(K::Print) },
        KwEntry { text: "INPUT",         kind: KwKind::Keyword(K::Input) },
        KwEntry { text: "MERGE",         kind: KwKind::Keyword(K::Merge) },
        KwEntry { text: "AFTER",         kind: KwKind::Keyword(K::After) },
        KwEntry { text: "GOSUB",         kind: KwKind::Keyword(K::Gosub) },
        KwEntry { text: "PLOTR",         kind: KwKind::Keyword(K::Plotr) },
        KwEntry { text: "MOVER",         kind: KwKind::Keyword(K::Mover) },
        KwEntry { text: "DRAWR",         kind: KwKind::Keyword(K::Drawr) },
        KwEntry { text: "FRAME",         kind: KwKind::Keyword(K::Frame) },
        KwEntry { text: "CHAIN",         kind: KwKind::Keyword(K::Chain) },
        KwEntry { text: "CLEAR",         kind: KwKind::Keyword(K::Clear) },
        KwEntry { text: "HEX$",          kind: KwKind::Function(F::HexDollar) },
        KwEntry { text: "BIN$",          kind: KwKind::Function(F::BinDollar) },
        KwEntry { text: "DEC$",          kind: KwKind::Function(F::DecDollar) },
        KwEntry { text: "CHR$",          kind: KwKind::Function(F::ChrDollar) },
        KwEntry { text: "STR$",          kind: KwKind::Function(F::StrDollar) },
        KwEntry { text: "INKEY",         kind: KwKind::Function(F::Inkey) },
        KwEntry { text: "HIMEM",         kind: KwKind::Function(F::Himem) },
        KwEntry { text: "ROUND",         kind: KwKind::Function(F::Round) },
        KwEntry { text: "RENUM",         kind: KwKind::Keyword(K::Renum) },
        // 5-char
        KwEntry { text: "MID$",          kind: KwKind::Keyword(K::MidDollar) },
        KwEntry { text: "ERROR",         kind: KwKind::Keyword(K::Error) },
        KwEntry { text: "WEND",          kind: KwKind::Keyword(K::Wend) },
        KwEntry { text: "SWAP",          kind: KwKind::Keyword(K::Swap) },
        KwEntry { text: "SAVE",          kind: KwKind::Keyword(K::Save) },
        KwEntry { text: "LOAD",          kind: KwKind::Keyword(K::Load) },
        KwEntry { text: "POKE",          kind: KwKind::Keyword(K::Poke) },
        KwEntry { text: "PLOT",          kind: KwKind::Keyword(K::Plot) },
        KwEntry { text: "MOVE",          kind: KwKind::Keyword(K::Move) },
        KwEntry { text: "DRAW",          kind: KwKind::Keyword(K::Draw) },
        KwEntry { text: "MASK",          kind: KwKind::Keyword(K::Mask) },
        KwEntry { text: "LINE",          kind: KwKind::Keyword(K::Line) },
        KwEntry { text: "LIST",          kind: KwKind::Keyword(K::List) },
        KwEntry { text: "FILL",          kind: KwKind::Keyword(K::Fill) },
        KwEntry { text: "EDIT",          kind: KwKind::Keyword(K::Edit) },
        KwEntry { text: "DATA",          kind: KwKind::Keyword(K::Data) },
        KwEntry { text: "CALL",          kind: KwKind::Keyword(K::Call) },
        KwEntry { text: "AUTO",          kind: KwKind::Keyword(K::Auto) },
        KwEntry { text: "WAIT",          kind: KwKind::Keyword(K::Wait) },
        KwEntry { text: "ZONE",          kind: KwKind::Keyword(K::Zone) },
        KwEntry { text: "TRON",          kind: KwKind::Keyword(K::Tron) },
        KwEntry { text: "THEN",          kind: KwKind::Keyword(K::Then) },
        KwEntry { text: "STOP",          kind: KwKind::Keyword(K::Stop) },
        KwEntry { text: "STEP",          kind: KwKind::Keyword(K::Step) },
        KwEntry { text: "READ",          kind: KwKind::Keyword(K::Read) },
        KwEntry { text: "PEEK",          kind: KwKind::Function(F::Peek) },
        KwEntry { text: "PAPER",         kind: KwKind::Keyword(K::Paper) },
        KwEntry { text: "NEXT",          kind: KwKind::Keyword(K::Next) },
        KwEntry { text: "MODE",          kind: KwKind::Keyword(K::Mode) },
        KwEntry { text: "GOTO",          kind: KwKind::Keyword(K::Goto) },
        KwEntry { text: "ELSE",          kind: KwKind::Keyword(K::Else) },
        KwEntry { text: "CINT",          kind: KwKind::Function(F::Cint) },
        KwEntry { text: "XPOS",          kind: KwKind::Function(F::Xpos) },
        KwEntry { text: "YPOS",          kind: KwKind::Function(F::Ypos) },
        KwEntry { text: "VPOS",          kind: KwKind::Function(F::Vpos) },
        KwEntry { text: "TIME",          kind: KwKind::Function(F::Time) },
        KwEntry { text: "SIGN",          kind: KwKind::Function(F::Sign) },
        KwEntry { text: "DERR",          kind: KwKind::Function(F::Derr) },
        KwEntry { text: "TEST",          kind: KwKind::Function(F::Test) },
        // 4-char
        KwEntry { text: "OPEN",          kind: KwKind::Keyword(K::Openin) },
        KwEntry { text: "NEW",           kind: KwKind::Keyword(K::New) },
        KwEntry { text: "MAX",           kind: KwKind::Function(F::Max) },
        KwEntry { text: "MIN",           kind: KwKind::Function(F::Min) },
        KwEntry { text: "POS",           kind: KwKind::Function(F::Pos) },
        KwEntry { text: "SQR",           kind: KwKind::Function(F::Sqr) },
        KwEntry { text: "ABS",           kind: KwKind::Function(F::Abs) },
        KwEntry { text: "ASC",           kind: KwKind::Function(F::Asc) },
        KwEntry { text: "ATN",           kind: KwKind::Function(F::Atn) },
        KwEntry { text: "COS",           kind: KwKind::Function(F::Cos) },
        KwEntry { text: "EXP",           kind: KwKind::Function(F::Exp) },
        KwEntry { text: "FIX",           kind: KwKind::Function(F::Fix) },
        KwEntry { text: "FRE",           kind: KwKind::Function(F::Fre) },
        KwEntry { text: "INP",           kind: KwKind::Function(F::Inp) },
        KwEntry { text: "INT",           kind: KwKind::Function(F::Int) },
        KwEntry { text: "JOY",           kind: KwKind::Function(F::Joy) },
        KwEntry { text: "LEN",           kind: KwKind::Function(F::Len) },
        KwEntry { text: "LOG",           kind: KwKind::Function(F::Log) },
        KwEntry { text: "SIN",           kind: KwKind::Function(F::Sin) },
        KwEntry { text: "TAN",           kind: KwKind::Function(F::Tan) },
        KwEntry { text: "UNT",           kind: KwKind::Function(F::Unt) },
        KwEntry { text: "VAL",           kind: KwKind::Function(F::Val) },
        KwEntry { text: "EOF",           kind: KwKind::Function(F::Eof) },
        KwEntry { text: "ERR",           kind: KwKind::Function(F::Err) },
        KwEntry { text: "RND",           kind: KwKind::Function(F::Rnd) },
        KwEntry { text: "SQ",            kind: KwKind::Function(F::Sq) },
        KwEntry { text: "PI",            kind: KwKind::Function(F::Pi) },
        // 3-char
        KwEntry { text: "FOR",           kind: KwKind::Keyword(K::For) },
        KwEntry { text: "REM",           kind: KwKind::Keyword(K::Rem) },
        KwEntry { text: "LET",           kind: KwKind::Keyword(K::Let) },
        KwEntry { text: "DIM",           kind: KwKind::Keyword(K::Dim) },
        KwEntry { text: "RUN",           kind: KwKind::Keyword(K::Run) },
        KwEntry { text: "OUT",           kind: KwKind::Keyword(K::Out) },
        KwEntry { text: "PEN",           kind: KwKind::Keyword(K::Pen) },
        KwEntry { text: "CLG",           kind: KwKind::Keyword(K::Clg) },
        KwEntry { text: "CLS",           kind: KwKind::Keyword(K::Cls) },
        KwEntry { text: "INK",           kind: KwKind::Keyword(K::Ink) },
        KwEntry { text: "KEY",           kind: KwKind::Keyword(K::Key) },
        KwEntry { text: "ENT",           kind: KwKind::Keyword(K::Ent) },
        KwEntry { text: "ENV",           kind: KwKind::Keyword(K::Env) },
        KwEntry { text: "TAB",           kind: KwKind::Keyword(K::Tab) },
        KwEntry { text: "SPC",           kind: KwKind::Keyword(K::Spc) },
        KwEntry { text: "TAG",           kind: KwKind::Keyword(K::Tag) },
        KwEntry { text: "DEG",           kind: KwKind::Keyword(K::Deg) },
        KwEntry { text: "RAD",           kind: KwKind::Keyword(K::Rad) },
        KwEntry { text: "END",           kind: KwKind::Keyword(K::End) },
        KwEntry { text: "DEF",           kind: KwKind::Keyword(K::Def) },
        KwEntry { text: "CAT",           kind: KwKind::Keyword(K::Cat) },
        KwEntry { text: "ERL",           kind: KwKind::Keyword(K::Erl) },
        KwEntry { text: "FN",            kind: KwKind::Keyword(K::Fn) },
        // 2-char
        KwEntry { text: "IF",            kind: KwKind::Keyword(K::If) },
        KwEntry { text: "ON",            kind: KwKind::Keyword(K::On) },
        KwEntry { text: "TO",            kind: KwKind::Keyword(K::To) },
        KwEntry { text: "DI",            kind: KwKind::Keyword(K::Di) },
        KwEntry { text: "EI",            kind: KwKind::Keyword(K::Ei) },
        // Word-bounded logical/arithmetic operators.
        KwEntry { text: "AND",           kind: KwKind::Keyword(K::And) },
        KwEntry { text: "NOT",           kind: KwKind::Keyword(K::Not) },
        KwEntry { text: "MOD",           kind: KwKind::Keyword(K::Mod) },
        KwEntry { text: "XOR",           kind: KwKind::Keyword(K::Xor) },
        KwEntry { text: "OR",            kind: KwKind::Keyword(K::Or) },
    ];

    // Longest match: sort by descending length, deduplicate by text.
    v.sort_by(|a, b| b.text.len().cmp(&a.text.len()));
    let mut seen = std::collections::HashSet::new();
    v.retain(|e| seen.insert(e.text));
    v
});

// ─── Individual token parsers ─────────────────────────────────────────────────

fn parse_spaces(input: &mut Input<'_>) -> ModalResult<(), ContextError> {
    take_while(0.., |c: char| c == ' ' || c == '\t').parse_next(input)?;
    Ok(())
}

/// Double-quoted string literal.  The `StringLit` value holds content without quotes.
fn parse_string_lit(input: &mut Input<'_>) -> ModalResult<LocatedTokenKind, ContextError> {
    '"'.parse_next(input)?;
    let content = take_while(0.., |c: char| c != '"').parse_next(input)?;
    let _ = opt('"').parse_next(input)?;
    Ok(LocatedTokenKind::StringLit(content.to_owned()))
}

/// Single-quote comment (`'…` to end of line).
fn parse_sq_comment(input: &mut Input<'_>) -> ModalResult<LocatedTokenKind, ContextError> {
    '\''.parse_next(input)?;
    let rest = take_while(0.., |_: char| true).parse_next(input)?;
    Ok(LocatedTokenKind::Comment(rest.to_owned()))
}

/// RSX call `|NAME` — emitted as `Other('|')`.
fn parse_rsx(input: &mut Input<'_>) -> ModalResult<LocatedTokenKind, ContextError> {
    '|'.parse_next(input)?;
    let _ = take_while(0.., |c: char| c.is_ascii_alphanumeric() || c == '_').parse_next(input)?;
    Ok(LocatedTokenKind::Other('|'))
}

/// Try every keyword from the table (longest first, case-insensitive, word-bounded).
/// `LocatingSlice<&str>` is `Copy`, so backtracking is a simple copy-back.
fn parse_keyword(input: &mut Input<'_>) -> ModalResult<KwKind, ContextError> {
    for entry in KEYWORD_TABLE.iter() {
        let saved = *input; // Copy — free snapshot
        let kw_r: ModalResult<_, ContextError> = Caseless(entry.text).parse_next(input);
        if kw_r.is_ok() {
            // Word-boundary check: next char must not continue an identifier.
            let at_boundary = opt(peek(one_of(|c: char| is_ident_continue(c))))
                .parse_next(input)?
                .is_none();

            if at_boundary {
                return Ok(entry.kind.clone());
            }
        }
        *input = saved; // backtrack
    }
    Err(ErrMode::Backtrack(ContextError::new()))
}

/// Keyword, function, or plain variable/identifier.
fn parse_word(input: &mut Input<'_>) -> ModalResult<LocatedTokenKind, ContextError> {
    // Must start with an ASCII letter.
    peek(one_of(|c: char| c.is_ascii_alphabetic())).parse_next(input)?;

    if let Ok(kw) = parse_keyword(input) {
        return Ok(match kw {
            KwKind::Keyword(k) => LocatedTokenKind::Keyword(k),
            KwKind::Function(f) => LocatedTokenKind::Function(f),
        });
    }

    // No keyword matched — consume as identifier.
    let name =
        take_while(1.., |c: char| c.is_ascii_alphanumeric() || c == '_').parse_next(input)?;
    let suffix: Option<char> = opt(one_of(['$', '%'])).parse_next(input)?;
    let mut var = name.to_owned();
    if let Some(s) = suffix {
        var.push(s);
    }
    Ok(LocatedTokenKind::Variable(var))
}

/// Numeric literal: `&X…` binary, `&[H]…` hex, decimal, or float.
/// The actual text is filled in by the caller from the offset difference.
fn parse_number_body(input: &mut Input<'_>) -> ModalResult<(), ContextError> {
    if peek(one_of::<Input<'_>, _, ContextError>('&')).parse_next(input).is_ok() {
        '&'.parse_next(input)?;
        let kind = opt(one_of(['X', 'x', 'H', 'h'])).parse_next(input)?;
        match kind {
            Some('X') | Some('x') => {
                take_while(0.., |c: char| c == '0' || c == '1').parse_next(input)?;
            }
            _ => {
                take_while(0.., |c: char| c.is_ascii_hexdigit()).parse_next(input)?;
            }
        }
        return Ok(());
    }

    // Decimal / float.
    let has_digit = peek(one_of::<Input<'_>, _, ContextError>(|c: char| c.is_ascii_digit()))
        .parse_next(input)
        .is_ok();
    let has_dot_digit = {
        let saved = *input;
        let ok = (
            one_of::<Input<'_>, _, ContextError>('.'),
            one_of::<Input<'_>, _, ContextError>(|c: char| c.is_ascii_digit()),
        )
            .parse_next(input)
            .is_ok();
        *input = saved;
        ok
    };
    if !has_digit && !has_dot_digit {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }

    take_while(0.., |c: char| c.is_ascii_digit()).parse_next(input)?;
    if peek(one_of::<Input<'_>, _, ContextError>('.')).parse_next(input).is_ok() {
        '.'.parse_next(input)?;
        take_while(0.., |c: char| c.is_ascii_digit()).parse_next(input)?;
    }
    let exp_r: ModalResult<char, ContextError> = peek(one_of(['E', 'e'])).parse_next(input);
    if exp_r.is_ok() {
        one_of(['E', 'e']).parse_next(input).map(|_: char| ())?;
        opt(one_of(['+', '-'])).parse_next(input)?;
        take_while(0.., |c: char| c.is_ascii_digit()).parse_next(input)?;
    }
    Ok(())
}

/// Two-character operators: `>=`, `<=`, `<>`.
fn parse_op2(input: &mut Input<'_>) -> ModalResult<LocatedTokenKind, ContextError> {
    use BasicTokenNoPrefix as K;
    alt((
        ">=".map(|_| LocatedTokenKind::Operator(K::GreaterOrEqual)),
        "<=".map(|_| LocatedTokenKind::Operator(K::LessThanOrEqual)),
        "<>".map(|_| LocatedTokenKind::Operator(K::NotEqual)),
    ))
    .parse_next(input)
}

/// Single-character operators.
fn parse_op1(input: &mut Input<'_>) -> ModalResult<LocatedTokenKind, ContextError> {
    use BasicTokenNoPrefix as K;
    one_of(['>', '<', '=', '+', '-', '*', '/', '^', '\\'])
        .map(|c| {
            LocatedTokenKind::Operator(match c {
                '>' => K::GreaterThan,
                '<' => K::LessThan,
                '=' => K::Equal,
                '+' => K::Addition,
                '-' => K::SubstractionOrUnaryMinus,
                '*' => K::Multiplication,
                '/' => K::Division,
                '^' => K::Power,
                '\\' => K::IntegerDivision,
                _ => unreachable!(),
            })
        })
        .parse_next(input)
}

// ─── Lexer ────────────────────────────────────────────────────────────────────

/// Lex everything after the line number.
///
/// `col_offset` is the byte column where `body` starts on the source line
/// (number of bytes occupied by the leading line-number + whitespace).
fn lex_body(body: &str, source_line: u32, col_offset: u32) -> Vec<LocatedBasicToken> {
    let line_start: Input<'_> = LocatingSlice::new(body);
    let mut input = line_start;
    let mut out = Vec::new();

    // Emit a token from a (before, kind) pair using the current `input` position.
    macro_rules! push {
        ($before:expr, $kind:expr) => {{
            let col = col_offset + $before.offset_from(&line_start) as u32;
            let len = input.offset_from(&$before) as u32;
            if len > 0 {
                out.push(LocatedBasicToken {
                    kind: $kind,
                    span: SourceSpan { line: source_line, col, len },
                });
            }
        }};
    }

    while !input.is_empty() {
        let before = input;

        // Numbers need special handling: `parse_number_body` consumes the bytes
        // but doesn't capture the text; we recover it from the offset difference.
        if let Ok(()) = parse_number_body(&mut input) {
            let start = before.offset_from(&line_start);
            let end   = input.offset_from(&line_start);
            let text  = body[start..end].to_owned();
            let col   = col_offset + start as u32;
            let len   = (end - start) as u32;
            if len > 0 {
                out.push(LocatedBasicToken {
                    kind: LocatedTokenKind::Number(text),
                    span: SourceSpan { line: source_line, col, len },
                });
            }
            continue;
        }
        input = before; // restore (number failed)

        // String literal.
        if let Ok(kind) = parse_string_lit(&mut input) {
            push!(before, kind);
            continue;
        }
        input = before;

        // Single-quote comment.
        if let Ok(kind) = parse_sq_comment(&mut input) {
            push!(before, kind);
            continue;
        }
        input = before;

        // Keyword, function, or variable.
        if let Ok(kind) = parse_word(&mut input) {
            // REM: emit the keyword then consume the rest of the line as Comment.
            if matches!(kind, LocatedTokenKind::Keyword(BasicTokenNoPrefix::Rem)) {
                push!(before, kind);
                let comment_col_start = input.offset_from(&line_start);
                let rest = &body[comment_col_start..];
                if !rest.is_empty() {
                    out.push(LocatedBasicToken {
                        kind: LocatedTokenKind::Comment(rest.to_owned()),
                        span: SourceSpan {
                            line: source_line,
                            col:  col_offset + comment_col_start as u32,
                            len:  rest.len() as u32,
                        },
                    });
                    let _ = take_while::<_, Input<'_>, ContextError>(0.., |_: char| true)
                        .parse_next(&mut input);
                }
                continue;
            }
            push!(before, kind);
            continue;
        }
        input = before;

        // Separator `:`.
        let sep_r: ModalResult<char, ContextError> = ':'.parse_next(&mut input);
        if sep_r.is_ok() {
            push!(before, LocatedTokenKind::Separator);
            continue;
        }
        input = before;

        // Whitespace.
        if let Ok(_) = take_while::<_, Input<'_>, ContextError>(1.., |c: char| c == ' ' || c == '\t')
            .parse_next(&mut input)
        {
            push!(before, LocatedTokenKind::Space);
            continue;
        }
        input = before;

        // RSX `|name`.
        if let Ok(kind) = parse_rsx(&mut input) {
            push!(before, kind);
            continue;
        }
        input = before;

        // Two-char operator.
        if let Ok(kind) = parse_op2(&mut input) {
            push!(before, kind);
            continue;
        }
        input = before;

        // One-char operator.
        if let Ok(kind) = parse_op1(&mut input) {
            push!(before, kind);
            continue;
        }
        input = before;

        // Fallback: consume one character.
        if let Ok(c) = any::<Input<'_>, ContextError>.parse_next(&mut input) {
            push!(before, LocatedTokenKind::Other(c));
        } else {
            break; // truly stuck — should not happen
        }
    }

    out
}
