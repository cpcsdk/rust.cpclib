/// Located (position-aware) Basic token types for use by the LSP.
///
/// This module provides a simple hand-written lexer that scans Locomotive BASIC
/// source text line by line and records the source position (line, column, length)
/// of every token.  It does NOT require the existing binary-encoded representation
/// and works purely on plain-text `.bas` files.
use crate::tokens::{BasicTokenNoPrefix, BasicTokenPrefixed};
use crate::BasicError;

// ─── Position ─────────────────────────────────────────────────────────────────

/// A half-open source span: [col, col+len) on a given 0-based source line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    /// 0-based source-text line index.
    pub line: u32,
    /// 0-based byte column on that line.
    pub col: u32,
    /// Length in bytes (== characters for ASCII BASIC source).
    pub len: u32,
}

// ─── Token kinds ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum LocatedTokenKind {
    /// A BASIC statement keyword (FOR, GOTO, PRINT, …).
    Keyword(BasicTokenNoPrefix),
    /// A built-in function (ABS, SIN, CHR$, …).
    Function(BasicTokenPrefixed),
    /// A variable name, preserving source case.
    Variable(String),
    /// A numeric literal (decimal, hex `&Hnn`, binary `&Xnn`, float).
    Number(String),
    /// String literal content (without the surrounding quotes).
    StringLit(String),
    /// Comment text (without the leading `REM ` / `'`).
    Comment(String),
    /// An operator (=, +, -, *, /, ^, \, >=, <=, <>, AND, OR, …).
    Operator(BasicTokenNoPrefix),
    /// Whitespace (space / tab).
    Space,
    /// Statement separator `:`.
    Separator,
    /// The BASIC line number that opens a source line (`10`, `20`, …).
    LineNumber(u16),
    /// Anything else that does not fit the above categories.
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
    /// The BASIC line number (e.g. 10, 20, 100, …).
    pub line_number: u16,
    /// 0-based index of this source-text line.
    pub source_line: u32,
    /// All tokens on this BASIC line (including the leading `LineNumber` token).
    pub tokens: Vec<LocatedBasicToken>,
}

impl LocatedBasicLine {
    /// Iterate over `(for_keyword_token, variable_token)` pairs found on this line.
    pub fn for_vars(&self) -> impl Iterator<Item = (&LocatedBasicToken, &LocatedBasicToken)> {
        let mut pairs: Vec<(&LocatedBasicToken, &LocatedBasicToken)> = Vec::new();
        let mut i = 0;
        while i < self.tokens.len() {
            if matches!(
                self.tokens[i].kind,
                LocatedTokenKind::Keyword(BasicTokenNoPrefix::For)
            ) {
                // Skip spaces to find the variable.
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
    /// Find a BASIC line by its BASIC line number.
    pub fn find_line(&self, line_number: u16) -> Option<&LocatedBasicLine> {
        self.lines.iter().find(|l| l.line_number == line_number)
    }

    /// Parse plain-text BASIC source into a `LocatedBasicProgram`.
    ///
    /// Each non-empty source line is expected to start with a decimal line
    /// number.  Lines that do not start with a digit are silently skipped.
    pub fn parse(source: &str) -> Result<Self, BasicError> {
        let mut lines = Vec::new();

        for (line_idx, raw_line) in source.lines().enumerate() {
            // Strip CR that Windows may have left behind.
            let line = raw_line.trim_end_matches('\r');
            if line.trim().is_empty() {
                continue;
            }

            let bytes = line.as_bytes();
            let mut col = 0usize;

            // Skip leading spaces (some sources indent the line number).
            while col < bytes.len() && bytes[col] == b' ' {
                col += 1;
            }

            // A BASIC line MUST start with a decimal line number.
            if col >= bytes.len() || !bytes[col].is_ascii_digit() {
                continue;
            }

            let num_start = col;
            while col < bytes.len() && bytes[col].is_ascii_digit() {
                col += 1;
            }
            let num_str = &line[num_start..col];
            let basic_line_number: u16 = match num_str.parse() {
                Ok(n) => n,
                Err(_) => continue, // line number overflow — skip
            };

            let source_line_u32 = line_idx as u32;

            // The leading LineNumber token.
            let ln_token = LocatedBasicToken {
                kind: LocatedTokenKind::LineNumber(basic_line_number),
                span: SourceSpan {
                    line: source_line_u32,
                    col: num_start as u32,
                    len: (col - num_start) as u32,
                },
            };

            // Body of the line (after the line number).
            let body = &line[col..];
            let body_col_offset = col as u32;

            let mut tokens = vec![ln_token];
            tokens.extend(lex_line_body(body, source_line_u32, body_col_offset));

            lines.push(LocatedBasicLine {
                line_number: basic_line_number,
                source_line: source_line_u32,
                tokens,
            });
        }

        Ok(LocatedBasicProgram { lines })
    }
}

// ─── Trait abstractions ───────────────────────────────────────────────────────

pub trait BasicLineT {
    fn basic_line_number(&self) -> u16;
}

impl BasicLineT for crate::BasicLine {
    fn basic_line_number(&self) -> u16 {
        self.line_number()
    }
}

impl BasicLineT for LocatedBasicLine {
    fn basic_line_number(&self) -> u16 {
        self.line_number
    }
}

// ─── Lexer ────────────────────────────────────────────────────────────────────

/// Keyword table entry: (upper-case keyword text, token kind).
///
/// IMPORTANT: entries must appear sorted by **descending** string length so that
/// the longest match wins (e.g. "ON ERROR GOTO" before "ON BREAK" before "ON",
/// "CLOSE" before something shorter, etc.).
struct KwEntry {
    text: &'static str,
    kind: KwKind,
}

#[derive(Clone)]
enum KwKind {
    Keyword(BasicTokenNoPrefix),
    Function(BasicTokenPrefixed),
}

static KEYWORD_TABLE: std::sync::LazyLock<Vec<KwEntry>> = std::sync::LazyLock::new(|| {
    use BasicTokenNoPrefix as K;
    use BasicTokenPrefixed as F;

    let mut v: Vec<KwEntry> = vec![
        // ── Multi-word / long keywords first ────────────────────────────────
        KwEntry { text: "ON ERROR GOTO",  kind: KwKind::Keyword(K::OnErrorGoto) },
        KwEntry { text: "ON BREAK",       kind: KwKind::Keyword(K::OnBreak) },
        // ── 9-char ──────────────────────────────────────────────────────────
        KwEntry { text: "RANDOMIZE",      kind: KwKind::Keyword(K::Randomize) },
        KwEntry { text: "COPYCHR$",       kind: KwKind::Function(F::CopycharDollar) },
        // ── 8-char ──────────────────────────────────────────────────────────
        KwEntry { text: "GRAPHICS",       kind: KwKind::Keyword(K::Graphics) },
        KwEntry { text: "CLOSEOUT",       kind: KwKind::Keyword(K::Closeout) },
        KwEntry { text: "TESTSTR",        kind: KwKind::Function(F::Teststr) },
        KwEntry { text: "STRING$",        kind: KwKind::Function(F::StringDollar) },
        KwEntry { text: "INKEY$",         kind: KwKind::Function(F::InkeyDollar) },
        KwEntry { text: "SPACE$",         kind: KwKind::Function(F::SpaceDollar) },
        KwEntry { text: "RIGHT$",         kind: KwKind::Function(F::RightDollar) },
        KwEntry { text: "LOWER$",         kind: KwKind::Function(F::LowerDollar) },
        KwEntry { text: "UPPER$",         kind: KwKind::Function(F::UpperDollar) },
        KwEntry { text: "REMAIN",         kind: KwKind::Function(F::Remain) },
        // ── 7-char ──────────────────────────────────────────────────────────
        KwEntry { text: "CLOSEIN",        kind: KwKind::Keyword(K::Closein) },
        KwEntry { text: "DEFREAL",        kind: KwKind::Keyword(K::Defreal) },
        KwEntry { text: "OPENOUT",        kind: KwKind::Keyword(K::Openout) },
        KwEntry { text: "RESTORE",        kind: KwKind::Keyword(K::Restore) },
        KwEntry { text: "RELEASE",        kind: KwKind::Keyword(K::Release) },
        KwEntry { text: "RETURN",         kind: KwKind::Keyword(K::Return) },
        KwEntry { text: "MID$",           kind: KwKind::Keyword(K::MidDollar) },  // stmt form
        KwEntry { text: "WINDOW",         kind: KwKind::Keyword(K::Window) },
        KwEntry { text: "MEMORY",         kind: KwKind::Keyword(K::Memory) },
        KwEntry { text: "LOCATE",         kind: KwKind::Keyword(K::Locate) },
        KwEntry { text: "RENUM",          kind: KwKind::Keyword(K::Renum) },
        KwEntry { text: "ORIGIN",         kind: KwKind::Keyword(K::Origin) },
        KwEntry { text: "OPENIN",         kind: KwKind::Keyword(K::Openin) },
        KwEntry { text: "RESUME",         kind: KwKind::Keyword(K::Resume) },
        KwEntry { text: "SYMBOL",         kind: KwKind::Keyword(K::Symbol) },
        KwEntry { text: "CURSOR",         kind: KwKind::Keyword(K::Cursor) },
        KwEntry { text: "DEFSTR",         kind: KwKind::Keyword(K::Defstr) },
        KwEntry { text: "DEFINT",         kind: KwKind::Keyword(K::Defint) },
        KwEntry { text: "DELETE",         kind: KwKind::Keyword(K::Delete) },
        KwEntry { text: "TAGOFF",         kind: KwKind::Keyword(K::Tagoff) },
        KwEntry { text: "TROFF",          kind: KwKind::Keyword(K::Troff) },
        KwEntry { text: "ERASE",          kind: KwKind::Keyword(K::Erase) },
        KwEntry { text: "EVERY",          kind: KwKind::Keyword(K::Every) },
        KwEntry { text: "INSTR",          kind: KwKind::Function(F::Instr) },
        KwEntry { text: "LEFT$",          kind: KwKind::Function(F::LeftDollar) },
        KwEntry { text: "LOG10",          kind: KwKind::Function(F::Log10) },
        KwEntry { text: "CREAL",          kind: KwKind::Function(F::Creal) },
        KwEntry { text: "USING",          kind: KwKind::Keyword(K::Using) },
        KwEntry { text: "WHILE",          kind: KwKind::Keyword(K::While) },
        KwEntry { text: "WIDTH",          kind: KwKind::Keyword(K::Width) },
        KwEntry { text: "WRITE",          kind: KwKind::Keyword(K::Write) },
        KwEntry { text: "SOUND",          kind: KwKind::Keyword(K::Sound) },
        KwEntry { text: "SPEED",          kind: KwKind::Keyword(K::Speed) },
        KwEntry { text: "PRINT",          kind: KwKind::Keyword(K::Print) },
        KwEntry { text: "INPUT",          kind: KwKind::Keyword(K::Input) },
        KwEntry { text: "MERGE",          kind: KwKind::Keyword(K::Merge) },
        // ── 6-char ──────────────────────────────────────────────────────────
        KwEntry { text: "BORDER",         kind: KwKind::Keyword(K::Border) },
        KwEntry { text: "AFTER",          kind: KwKind::Keyword(K::After) },
        KwEntry { text: "GOSUB",          kind: KwKind::Keyword(K::Gosub) },
        KwEntry { text: "PLOTR",          kind: KwKind::Keyword(K::Plotr) },
        KwEntry { text: "MOVER",          kind: KwKind::Keyword(K::Mover) },
        KwEntry { text: "DRAWR",          kind: KwKind::Keyword(K::Drawr) },
        KwEntry { text: "FRAME",          kind: KwKind::Keyword(K::Frame) },
        KwEntry { text: "CHAIN",          kind: KwKind::Keyword(K::Chain) },
        KwEntry { text: "CLEAR",          kind: KwKind::Keyword(K::Clear) },
        KwEntry { text: "CLOSE",          kind: KwKind::Keyword(K::Closein) }, // alias
        KwEntry { text: "HEX$",           kind: KwKind::Function(F::HexDollar) },
        KwEntry { text: "BIN$",           kind: KwKind::Function(F::BinDollar) },
        KwEntry { text: "DEC$",           kind: KwKind::Function(F::DecDollar) },
        KwEntry { text: "CHR$",           kind: KwKind::Function(F::ChrDollar) },
        KwEntry { text: "STR$",           kind: KwKind::Function(F::StrDollar) },
        KwEntry { text: "INKEY",          kind: KwKind::Function(F::Inkey) },
        KwEntry { text: "HIMEM",          kind: KwKind::Function(F::Himem) },
        KwEntry { text: "ROUND",          kind: KwKind::Function(F::Round) },
        KwEntry { text: "CINT",           kind: KwKind::Function(F::Cint) },
        KwEntry { text: "RANDOMIZE",      kind: KwKind::Keyword(K::Randomize) },
        KwEntry { text: "LOCATE",         kind: KwKind::Keyword(K::Locate) },
        KwEntry { text: "WEND",           kind: KwKind::Keyword(K::Wend) },
        KwEntry { text: "SWAP",           kind: KwKind::Keyword(K::Swap) },
        KwEntry { text: "SAVE",           kind: KwKind::Keyword(K::Save) },
        KwEntry { text: "LOAD",           kind: KwKind::Keyword(K::Load) },
        KwEntry { text: "POKE",           kind: KwKind::Keyword(K::Poke) },
        KwEntry { text: "PLOT",           kind: KwKind::Keyword(K::Plot) },
        KwEntry { text: "MOVE",           kind: KwKind::Keyword(K::Move) },
        KwEntry { text: "DRAW",           kind: KwKind::Keyword(K::Draw) },
        KwEntry { text: "MASK",           kind: KwKind::Keyword(K::Mask) },
        KwEntry { text: "LINE",           kind: KwKind::Keyword(K::Line) },
        KwEntry { text: "LIST",           kind: KwKind::Keyword(K::List) },
        KwEntry { text: "FILL",           kind: KwKind::Keyword(K::Fill) },
        KwEntry { text: "EDIT",           kind: KwKind::Keyword(K::Edit) },
        KwEntry { text: "DATA",           kind: KwKind::Keyword(K::Data) },
        KwEntry { text: "CALL",           kind: KwKind::Keyword(K::Call) },
        KwEntry { text: "AUTO",           kind: KwKind::Keyword(K::Auto) },
        KwEntry { text: "WAIT",           kind: KwKind::Keyword(K::Wait) },
        KwEntry { text: "ZONE",           kind: KwKind::Keyword(K::Zone) },
        KwEntry { text: "TRON",           kind: KwKind::Keyword(K::Tron) },
        KwEntry { text: "THEN",           kind: KwKind::Keyword(K::Then) },
        KwEntry { text: "STOP",           kind: KwKind::Keyword(K::Stop) },
        KwEntry { text: "STEP",           kind: KwKind::Keyword(K::Step) },
        KwEntry { text: "READ",           kind: KwKind::Keyword(K::Read) },
        KwEntry { text: "PEEK",           kind: KwKind::Function(F::Peek) },
        KwEntry { text: "PAPER",          kind: KwKind::Keyword(K::Paper) },
        KwEntry { text: "NEXT",           kind: KwKind::Keyword(K::Next) },
        KwEntry { text: "NEW",            kind: KwKind::Keyword(K::New) },
        KwEntry { text: "MODE",           kind: KwKind::Keyword(K::Mode) },
        KwEntry { text: "GOTO",           kind: KwKind::Keyword(K::Goto) },
        KwEntry { text: "ELSE",           kind: KwKind::Keyword(K::Else) },
        KwEntry { text: "ERASE",          kind: KwKind::Keyword(K::Erase) },
        KwEntry { text: "XPOS",           kind: KwKind::Function(F::Xpos) },
        KwEntry { text: "YPOS",           kind: KwKind::Function(F::Ypos) },
        KwEntry { text: "VPOS",           kind: KwKind::Function(F::Vpos) },
        KwEntry { text: "TIME",           kind: KwKind::Function(F::Time) },
        KwEntry { text: "SIGN",           kind: KwKind::Function(F::Sign) },
        KwEntry { text: "DERR",           kind: KwKind::Function(F::Derr) },
        KwEntry { text: "TEST",           kind: KwKind::Function(F::Test) },
        KwEntry { text: "MAX",            kind: KwKind::Function(F::Max) },
        KwEntry { text: "MIN",            kind: KwKind::Function(F::Min) },
        KwEntry { text: "POS",            kind: KwKind::Function(F::Pos) },
        KwEntry { text: "SQR",            kind: KwKind::Function(F::Sqr) },
        KwEntry { text: "ABS",            kind: KwKind::Function(F::Abs) },
        KwEntry { text: "ASC",            kind: KwKind::Function(F::Asc) },
        KwEntry { text: "ATN",            kind: KwKind::Function(F::Atn) },
        KwEntry { text: "COS",            kind: KwKind::Function(F::Cos) },
        KwEntry { text: "EXP",            kind: KwKind::Function(F::Exp) },
        KwEntry { text: "FIX",            kind: KwKind::Function(F::Fix) },
        KwEntry { text: "FRE",            kind: KwKind::Function(F::Fre) },
        KwEntry { text: "INP",            kind: KwKind::Function(F::Inp) },
        KwEntry { text: "INT",            kind: KwKind::Function(F::Int) },
        KwEntry { text: "JOY",            kind: KwKind::Function(F::Joy) },
        KwEntry { text: "LEN",            kind: KwKind::Function(F::Len) },
        KwEntry { text: "LOG",            kind: KwKind::Function(F::Log) },
        KwEntry { text: "SIN",            kind: KwKind::Function(F::Sin) },
        KwEntry { text: "TAN",            kind: KwKind::Function(F::Tan) },
        KwEntry { text: "UNT",            kind: KwKind::Function(F::Unt) },
        KwEntry { text: "VAL",            kind: KwKind::Function(F::Val) },
        KwEntry { text: "EOF",            kind: KwKind::Function(F::Eof) },
        KwEntry { text: "ERR",            kind: KwKind::Function(F::Err) },
        KwEntry { text: "RND",            kind: KwKind::Function(F::Rnd) },
        KwEntry { text: "SQ",             kind: KwKind::Function(F::Sq) },
        KwEntry { text: "PI",             kind: KwKind::Function(F::Pi) },
        // ── 5-char ──────────────────────────────────────────────────────────
        KwEntry { text: "PRINT",          kind: KwKind::Keyword(K::Print) },
        KwEntry { text: "GOSUB",          kind: KwKind::Keyword(K::Gosub) },
        KwEntry { text: "ERROR",          kind: KwKind::Keyword(K::Error) },
        KwEntry { text: "EVERY",          kind: KwKind::Keyword(K::Every) },
        KwEntry { text: "ERASE",          kind: KwKind::Keyword(K::Erase) },
        KwEntry { text: "CLEAR",          kind: KwKind::Keyword(K::Clear) },
        KwEntry { text: "CHAIN",          kind: KwKind::Keyword(K::Chain) },
        KwEntry { text: "WHILE",          kind: KwKind::Keyword(K::While) },
        KwEntry { text: "WIDTH",          kind: KwKind::Keyword(K::Width) },
        KwEntry { text: "WRITE",          kind: KwKind::Keyword(K::Write) },
        KwEntry { text: "SOUND",          kind: KwKind::Keyword(K::Sound) },
        KwEntry { text: "SPEED",          kind: KwKind::Keyword(K::Speed) },
        KwEntry { text: "INPUT",          kind: KwKind::Keyword(K::Input) },
        KwEntry { text: "MERGE",          kind: KwKind::Keyword(K::Merge) },
        KwEntry { text: "WEND",           kind: KwKind::Keyword(K::Wend) },
        KwEntry { text: "SWAP",           kind: KwKind::Keyword(K::Swap) },
        KwEntry { text: "POKE",           kind: KwKind::Keyword(K::Poke) },
        KwEntry { text: "MOVE",           kind: KwKind::Keyword(K::Move) },
        KwEntry { text: "DRAW",           kind: KwKind::Keyword(K::Draw) },
        KwEntry { text: "MASK",           kind: KwKind::Keyword(K::Mask) },
        KwEntry { text: "LIST",           kind: KwKind::Keyword(K::List) },
        KwEntry { text: "FILL",           kind: KwKind::Keyword(K::Fill) },
        KwEntry { text: "CALL",           kind: KwKind::Keyword(K::Call) },
        KwEntry { text: "AUTO",           kind: KwKind::Keyword(K::Auto) },
        KwEntry { text: "WAIT",           kind: KwKind::Keyword(K::Wait) },
        KwEntry { text: "ZONE",           kind: KwKind::Keyword(K::Zone) },
        KwEntry { text: "TRON",           kind: KwKind::Keyword(K::Tron) },
        KwEntry { text: "TROFF",          kind: KwKind::Keyword(K::Troff) },
        KwEntry { text: "THEN",           kind: KwKind::Keyword(K::Then) },
        KwEntry { text: "STOP",           kind: KwKind::Keyword(K::Stop) },
        KwEntry { text: "STEP",           kind: KwKind::Keyword(K::Step) },
        KwEntry { text: "READ",           kind: KwKind::Keyword(K::Read) },
        KwEntry { text: "NEXT",           kind: KwKind::Keyword(K::Next) },
        KwEntry { text: "MODE",           kind: KwKind::Keyword(K::Mode) },
        KwEntry { text: "GOTO",           kind: KwKind::Keyword(K::Goto) },
        KwEntry { text: "ELSE",           kind: KwKind::Keyword(K::Else) },
        KwEntry { text: "RAND",           kind: KwKind::Keyword(K::Randomize) },
        // ── 4-char ──────────────────────────────────────────────────────────
        KwEntry { text: "SAVE",           kind: KwKind::Keyword(K::Save) },
        KwEntry { text: "LOAD",           kind: KwKind::Keyword(K::Load) },
        KwEntry { text: "PLOT",           kind: KwKind::Keyword(K::Plot) },
        KwEntry { text: "EDIT",           kind: KwKind::Keyword(K::Edit) },
        KwEntry { text: "DATA",           kind: KwKind::Keyword(K::Data) },
        KwEntry { text: "LINE",           kind: KwKind::Keyword(K::Line) },
        KwEntry { text: "OPEN",           kind: KwKind::Keyword(K::Openin) },
        KwEntry { text: "RENUM",          kind: KwKind::Keyword(K::Renum) },
        KwEntry { text: "NEW",            kind: KwKind::Keyword(K::New) },
        KwEntry { text: "PAPER",          kind: KwKind::Keyword(K::Paper) },
        // ── 3-char ──────────────────────────────────────────────────────────
        KwEntry { text: "FOR",            kind: KwKind::Keyword(K::For) },
        KwEntry { text: "REM",            kind: KwKind::Keyword(K::Rem) },
        KwEntry { text: "LET",            kind: KwKind::Keyword(K::Let) },
        KwEntry { text: "DIM",            kind: KwKind::Keyword(K::Dim) },
        KwEntry { text: "RUN",            kind: KwKind::Keyword(K::Run) },
        KwEntry { text: "OUT",            kind: KwKind::Keyword(K::Out) },
        KwEntry { text: "PEN",            kind: KwKind::Keyword(K::Pen) },
        KwEntry { text: "CLG",            kind: KwKind::Keyword(K::Clg) },
        KwEntry { text: "CLS",            kind: KwKind::Keyword(K::Cls) },
        KwEntry { text: "INK",            kind: KwKind::Keyword(K::Ink) },
        KwEntry { text: "KEY",            kind: KwKind::Keyword(K::Key) },
        KwEntry { text: "ENT",            kind: KwKind::Keyword(K::Ent) },
        KwEntry { text: "ENV",            kind: KwKind::Keyword(K::Env) },
        KwEntry { text: "TAB",            kind: KwKind::Keyword(K::Tab) },
        KwEntry { text: "SPC",            kind: KwKind::Keyword(K::Spc) },
        KwEntry { text: "TAG",            kind: KwKind::Keyword(K::Tag) },
        KwEntry { text: "DEG",            kind: KwKind::Keyword(K::Deg) },
        KwEntry { text: "RAD",            kind: KwKind::Keyword(K::Rad) },
        KwEntry { text: "END",            kind: KwKind::Keyword(K::End) },
        KwEntry { text: "DEF",            kind: KwKind::Keyword(K::Def) },
        KwEntry { text: "CAT",            kind: KwKind::Keyword(K::Cat) },
        KwEntry { text: "ERL",            kind: KwKind::Keyword(K::Erl) },
        KwEntry { text: "FN",             kind: KwKind::Keyword(K::Fn) },
        // ── 2-char ──────────────────────────────────────────────────────────
        KwEntry { text: "IF",             kind: KwKind::Keyword(K::If) },
        KwEntry { text: "ON",             kind: KwKind::Keyword(K::On) },
        KwEntry { text: "TO",             kind: KwKind::Keyword(K::To) },
        KwEntry { text: "DI",             kind: KwKind::Keyword(K::Di) },
        KwEntry { text: "EI",             kind: KwKind::Keyword(K::Ei) },
        // Keyword operators (word-bounded: must be surrounded by non-alnum)
        KwEntry { text: "AND",            kind: KwKind::Keyword(K::And) },
        KwEntry { text: "NOT",            kind: KwKind::Keyword(K::Not) },
        KwEntry { text: "MOD",            kind: KwKind::Keyword(K::Mod) },
        KwEntry { text: "OR",             kind: KwKind::Keyword(K::Or) },
        KwEntry { text: "XOR",            kind: KwKind::Keyword(K::Xor) },
    ];

    // Sort by descending keyword length so longer matches win.
    v.sort_by(|a, b| b.text.len().cmp(&a.text.len()));
    // Deduplicate: keep first occurrence (longest wins in case of equal length).
    let mut seen = std::collections::HashSet::new();
    v.retain(|e| seen.insert(e.text));
    v
});

/// Returns true if `c` is a character that can immediately follow a keyword
/// without acting as a word boundary (i.e. the keyword is NOT matched here).
#[inline]
fn is_ident_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'$' || c == b'%' || c == b'_'
}

/// Try to match a keyword at the start of `text` (case-insensitive).
/// Returns `Some((kind, len))` on success, where `len` is the number of bytes consumed.
fn try_keyword(text: &[u8]) -> Option<(KwKind, usize)> {
    for entry in KEYWORD_TABLE.iter() {
        let kw = entry.text.as_bytes();
        if text.len() < kw.len() {
            continue;
        }
        // Case-insensitive compare.
        if !text[..kw.len()].eq_ignore_ascii_case(kw) {
            continue;
        }
        // Word-boundary check: the character after the keyword must not be
        // an alphanumeric, '$', or '%' (but spaces and operators are fine).
        let after = text.get(kw.len()).copied();
        let is_boundary = match after {
            None => true,
            Some(c) => {
                // Allow keyword-operator like AND/OR/NOT/MOD/XOR to end
                // before any non-alphanumeric char.
                !is_ident_char(c)
            }
        };
        if is_boundary {
            return Some((entry.kind.clone(), kw.len()));
        }
    }
    None
}

/// Lex the body of a BASIC line (everything after the line number).
///
/// `col_offset` is the byte column where `text` starts on the source line
/// (i.e. the number of bytes occupied by the line-number prefix).
fn lex_line_body(text: &str, source_line: u32, col_offset: u32) -> Vec<LocatedBasicToken> {
    use BasicTokenNoPrefix as K;

    let bytes = text.as_bytes();
    let mut col = 0usize; // index into `bytes`
    let mut tokens = Vec::new();

    // Helper to push a token.
    macro_rules! push {
        ($kind:expr, $start:expr) => {
            tokens.push(LocatedBasicToken {
                kind: $kind,
                span: SourceSpan {
                    line: source_line,
                    col: col_offset + $start as u32,
                    len: (col - $start) as u32,
                },
            });
        };
    }

    while col < bytes.len() {
        let start = col;
        let ch = bytes[col];

        // ── String literal ───────────────────────────────────────────────────
        if ch == b'"' {
            col += 1; // opening quote
            let content_start = col;
            while col < bytes.len() && bytes[col] != b'"' {
                col += 1;
            }
            let content = std::str::from_utf8(&bytes[content_start..col]).unwrap_or("").to_owned();
            if col < bytes.len() {
                col += 1; // closing quote
            }
            push!(LocatedTokenKind::StringLit(content), start);
            continue;
        }

        // ── Single-quote comment (') ────────────────────────────────────────
        if ch == b'\'' {
            col += 1;
            let comment = std::str::from_utf8(&bytes[col..]).unwrap_or("").to_owned();
            col = bytes.len();
            push!(LocatedTokenKind::Comment(comment), start);
            continue;
        }

        // ── Keyword / function match ─────────────────────────────────────────
        if ch.is_ascii_alphabetic() {
            if let Some((kind, len)) = try_keyword(&bytes[col..]) {
                col += len;
                // Special case: REM → rest of line is comment.
                if matches!(kind, KwKind::Keyword(BasicTokenNoPrefix::Rem)) {
                    let kw_kind = LocatedTokenKind::Keyword(K::Rem);
                    push!(kw_kind, start);
                    // Consume everything else as a Comment.
                    if col < bytes.len() {
                        let cstart = col;
                        let comment = std::str::from_utf8(&bytes[col..]).unwrap_or("").to_owned();
                        col = bytes.len();
                        tokens.push(LocatedBasicToken {
                            kind: LocatedTokenKind::Comment(comment),
                            span: SourceSpan {
                                line: source_line,
                                col: col_offset + cstart as u32,
                                len: (col - cstart) as u32,
                            },
                        });
                    }
                    continue;
                }
                let tok_kind = match kind {
                    KwKind::Keyword(k) => LocatedTokenKind::Keyword(k),
                    KwKind::Function(f) => LocatedTokenKind::Function(f),
                };
                push!(tok_kind, start);
                continue;
            }

            // No keyword matched → identifier / variable.
            while col < bytes.len() && (bytes[col].is_ascii_alphanumeric() || bytes[col] == b'_') {
                col += 1;
            }
            // Optional type suffix.
            if col < bytes.len() && (bytes[col] == b'$' || bytes[col] == b'%') {
                col += 1;
            }
            let name = std::str::from_utf8(&bytes[start..col]).unwrap_or("").to_owned();
            push!(LocatedTokenKind::Variable(name), start);
            continue;
        }

        // ── Numeric literal ──────────────────────────────────────────────────
        if ch == b'&' {
            col += 1;
            if col < bytes.len() && (bytes[col] == b'X' || bytes[col] == b'x') {
                col += 1;
                while col < bytes.len() && (bytes[col] == b'0' || bytes[col] == b'1') {
                    col += 1;
                }
            } else {
                // Hex: &[H]digits
                if col < bytes.len() && (bytes[col] == b'H' || bytes[col] == b'h') {
                    col += 1;
                }
                while col < bytes.len() && bytes[col].is_ascii_hexdigit() {
                    col += 1;
                }
            }
            let num = std::str::from_utf8(&bytes[start..col]).unwrap_or("").to_owned();
            push!(LocatedTokenKind::Number(num), start);
            continue;
        }

        if ch.is_ascii_digit() || (ch == b'.' && col + 1 < bytes.len() && bytes[col + 1].is_ascii_digit()) {
            while col < bytes.len() && bytes[col].is_ascii_digit() {
                col += 1;
            }
            if col < bytes.len() && bytes[col] == b'.' {
                col += 1;
                while col < bytes.len() && bytes[col].is_ascii_digit() {
                    col += 1;
                }
            }
            // Optional exponent.
            if col < bytes.len() && (bytes[col] == b'E' || bytes[col] == b'e') {
                col += 1;
                if col < bytes.len() && (bytes[col] == b'+' || bytes[col] == b'-') {
                    col += 1;
                }
                while col < bytes.len() && bytes[col].is_ascii_digit() {
                    col += 1;
                }
            }
            let num = std::str::from_utf8(&bytes[start..col]).unwrap_or("").to_owned();
            push!(LocatedTokenKind::Number(num), start);
            continue;
        }

        // ── Separator `:` ────────────────────────────────────────────────────
        if ch == b':' {
            col += 1;
            push!(LocatedTokenKind::Separator, start);
            continue;
        }

        // ── Whitespace ───────────────────────────────────────────────────────
        if ch == b' ' || ch == b'\t' {
            while col < bytes.len() && (bytes[col] == b' ' || bytes[col] == b'\t') {
                col += 1;
            }
            push!(LocatedTokenKind::Space, start);
            continue;
        }

        // ── RSX call `|name` ─────────────────────────────────────────────────
        if ch == b'|' {
            col += 1;
            while col < bytes.len() && (bytes[col].is_ascii_alphanumeric() || bytes[col] == b'_') {
                col += 1;
            }
            let rsx = std::str::from_utf8(&bytes[start..col]).unwrap_or("|").to_owned();
            // Treat as Other (there is no LocatedTokenKind for RSX).
            push!(LocatedTokenKind::Other('|'), start);
            let _ = rsx; // suppress warning
            continue;
        }

        // ── Multi-char operators ─────────────────────────────────────────────
        if col + 1 < bytes.len() {
            let two = &bytes[col..col + 2];
            let op = match two {
                b">=" => Some((K::GreaterOrEqual, 2)),
                b"<=" => Some((K::LessThanOrEqual, 2)),
                b"<>" => Some((K::NotEqual, 2)),
                _ => None,
            };
            if let Some((k, len)) = op {
                col += len;
                push!(LocatedTokenKind::Operator(k), start);
                continue;
            }
        }

        // ── Single-char operators ─────────────────────────────────────────────
        let op_kind: Option<K> = match ch {
            b'>' => Some(K::GreaterThan),
            b'<' => Some(K::LessThan),
            b'=' => Some(K::Equal),
            b'+' => Some(K::Addition),
            b'-' => Some(K::SubstractionOrUnaryMinus),
            b'*' => Some(K::Multiplication),
            b'/' => Some(K::Division),
            b'^' => Some(K::Power),
            b'\\' => Some(K::IntegerDivision),
            _ => None,
        };
        if let Some(k) = op_kind {
            col += 1;
            push!(LocatedTokenKind::Operator(k), start);
            continue;
        }

        // ── Anything else ────────────────────────────────────────────────────
        col += 1;
        push!(LocatedTokenKind::Other(ch as char), start);
    }

    tokens
}
