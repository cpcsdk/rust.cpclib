//! Parser and AST for [mdlz80optimizer](https://github.com/santiontanon/mdlz80optimizer)'s
//! optimization-pattern format.
//!
//! The format (see that project's `doc/pattern-definition.md`) is:
//!
//! ```text
//! pattern: <human-readable description, may reference ?variables>
//! name: <optional identifier>
//! tags: <optional comma-separated tags, e.g. "cpc">
//! <n>: <instruction>
//! ...
//! replacement:
//! <n>: <instruction>
//! ...
//! constraints:
//! <constraint>
//! ...
//! ```
//!
//! Patterns are separated by blank lines and `;` starts a comment. The `<n>:`
//! numbers map a pattern line to its replacement line: a number present in the
//! pattern but absent from the replacement means that instruction is removed,
//! and vice-versa for insertion. Number `0` is special - it marks the line the
//! optimization message is reported against, and at least one line must carry
//! it.
//!
//! This module only *parses*; it deliberately accepts the full real grammar
//! (including constraints this crate does not evaluate yet) so real upstream
//! pattern files load end to end. Deciding which constraints are actually
//! supported, and skipping rules that use anything else, is the matching
//! engine's job.

use std::collections::HashSet;

use cpclib_common::winnow::ascii::{digit1, line_ending, space0, till_line_ending};
use cpclib_common::winnow::combinator::{alt, delimited, opt, preceded, repeat, terminated};
use cpclib_common::winnow::error::{ContextError, ParserError};
use cpclib_common::winnow::token::{one_of, take_while};
use cpclib_common::winnow::{ModalResult, Parser};

/// A whole pattern file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuleSet {
    pub rules: Vec<Rule>
}

/// One `pattern:` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// The `pattern:` line - the message shown to the user, possibly
    /// referencing `?variables` that get substituted from the match.
    pub description: String,
    /// The optional `name:` line.
    pub name: Option<String>,
    /// The optional `tags:` line, split on commas (e.g. `cpc`).
    pub tags: Vec<String>,
    /// The numbered instruction lines before `replacement:`.
    pub match_lines: Vec<NumberedInstr>,
    /// The numbered instruction lines after `replacement:`. Empty means the
    /// matched instructions are simply deleted.
    pub replacement_lines: Vec<NumberedInstr>,
    /// The `constraints:` lines, kept as unevaluated generic calls - this
    /// module does not know or care which constraint names are real.
    pub constraints: Vec<Constraint>
}

impl Rule {
    /// The line number that the optimization message anchors to (`0` per the
    /// format's own rule that at least one line must carry it).
    pub const ANCHOR: u32 = 0;

    /// Whether some matched line carries the anchor index, i.e. whether this
    /// rule can report a location at all. Enforced at parse time.
    pub fn has_anchor(&self) -> bool {
        self.match_lines.iter().any(|l| l.index == Self::ANCHOR)
    }

    /// Every distinct `?variable` name appearing anywhere in this rule.
    pub fn variables(&self) -> HashSet<&str> {
        let mut out = HashSet::new();
        for line in self.match_lines.iter().chain(self.replacement_lines.iter()) {
            line.instr.collect_variables(&mut out);
        }
        out
    }
}

/// An instruction line, prefixed by the index that maps pattern to replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberedInstr {
    pub index: u32,
    pub instr: InstrPattern
}

/// One line's instruction pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstrPattern {
    /// `*` - matches zero or more consecutive instructions.
    Wildcard,
    /// `[n] <instr>` / `[?var] <instr>` - the instruction repeated a fixed or
    /// variable number of times.
    Repeat {
        count: RepeatCount,
        instr: Box<InstrPattern>
    },
    /// A real instruction pattern, e.g. `ld a, ?const1` or `?op1 c,?const2`.
    Instr {
        mnemonic: MnemonicPattern,
        operands: Vec<OperandPattern>
    }
}

impl InstrPattern {
    fn collect_variables<'a>(&'a self, out: &mut HashSet<&'a str>) {
        match self {
            Self::Wildcard => {},
            Self::Repeat { count, instr } => {
                if let RepeatCount::Variable(name) = count {
                    out.insert(name.as_str());
                }
                instr.collect_variables(out);
            },
            Self::Instr { mnemonic, operands } => {
                if let MnemonicPattern::Variable(name) = mnemonic {
                    out.insert(name.as_str());
                }
                for op in operands {
                    op.collect_variables(out);
                }
            }
        }
    }
}

/// The repetition count of a `[..]`-prefixed instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepeatCount {
    Fixed(u32),
    Variable(String)
}

/// The mnemonic slot of an instruction pattern.
///
/// A literal mnemonic is kept as its normalized (upper-case) *name* rather
/// than a `cpclib_tokens::Mnemonic`: real upstream pattern files reference
/// instructions from Z80 variants this toolchain does not necessarily model
/// (and mnemonic spellings differ between assembler dialects), and a pattern
/// file must still parse end to end even when one of its rules mentions an
/// instruction we cannot represent. Resolving a name against the real
/// `Mnemonic` set is the matching engine's job, where an unresolvable name
/// simply never matches instead of failing the whole file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MnemonicPattern {
    Literal(String),
    /// `?op`, `?op1`, ... - matches any mnemonic and binds it.
    Variable(String)
}

/// One operand of an instruction pattern.
///
/// Deliberately a small, self-contained expression tree rather than a reuse of
/// `cpclib-asm`'s own expression parser: that parser is built around
/// `InnerZ80Span`/`ParserContext` (real source files, include paths, assembler
/// options) and has no notion of a `?variable` atom, so reusing it would mean
/// pushing a DSL-only concept down into the real assembler's grammar. The
/// subset here covers everything the real upstream pattern files actually use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperandPattern {
    /// `?const1`, `?reg0`, `?any`, ...
    Variable(String),
    /// A register, condition flag, or label name: `a`, `hl`, `nz`, `foo`.
    Ident(String),
    Number(i64),
    /// `(hl)`, `(?const1)`, ...
    Indirect(Box<OperandPattern>),
    Binary {
        lhs: Box<OperandPattern>,
        op: BinOp,
        rhs: Box<OperandPattern>
    },
    Unary {
        op: UnOp,
        operand: Box<OperandPattern>
    }
}

impl OperandPattern {
    fn collect_variables<'a>(&'a self, out: &mut HashSet<&'a str>) {
        match self {
            Self::Variable(name) => {
                out.insert(name.as_str());
            },
            Self::Ident(_) | Self::Number(_) => {},
            Self::Indirect(inner) | Self::Unary { operand: inner, .. } => {
                inner.collect_variables(out)
            },
            Self::Binary { lhs, rhs, .. } => {
                lhs.collect_variables(out);
                rhs.collect_variables(out);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    ShiftLeft,
    ShiftRight,
    BitAnd,
    BitOr,
    BitXor,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum UnOp {
    Neg,
    Not
}

/// A `constraints:` entry, kept as an unevaluated call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constraint {
    pub name: String,
    pub args: Vec<OperandPattern>,
    /// The optional `:ID` suffix - check this constraint as soon as the
    /// pattern line with that index has matched, rather than at the end.
    pub check_after: Option<u32>
}

/// Everything that can go wrong while reading a pattern file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleParseError {
    /// A line could not be parsed at all.
    Syntax { line: usize, text: String },
    /// A `pattern:` block had no line numbered [`Rule::ANCHOR`].
    MissingAnchor { line: usize, name: Option<String> },
    /// The same line index was used twice within one section.
    DuplicateIndex { line: usize, index: u32 },
    /// A section keyword appeared without an enclosing `pattern:` block.
    OrphanSection { line: usize, keyword: String }
}

impl std::fmt::Display for RuleParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Syntax { line, text } => {
                write!(f, "line {line}: cannot parse {text:?}")
            },
            Self::MissingAnchor { line, name } => {
                write!(
                    f,
                    "line {line}: pattern {} has no line numbered {} to anchor its message to",
                    name.as_deref().unwrap_or("<unnamed>"),
                    Rule::ANCHOR
                )
            },
            Self::DuplicateIndex { line, index } => {
                write!(f, "line {line}: index {index} used more than once")
            },
            Self::OrphanSection { line, keyword } => {
                write!(f, "line {line}: `{keyword}` outside of a pattern block")
            }
        }
    }
}

impl std::error::Error for RuleParseError {}

/// Which section of a `pattern:` block subsequent numbered lines belong to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Match,
    Replacement,
    Constraints
}

impl RuleSet {
    /// Parse a whole pattern file.
    ///
    /// `include "other.txt"` directives are *not* resolved here (this function
    /// is pure, with no filesystem access) - they are reported through
    /// [`RuleSet::parse_with_includes`], which the caller gives a resolver.
    pub fn parse(source: &str) -> Result<Self, RuleParseError> {
        let (set, includes) = Self::parse_inner(source)?;
        debug_assert!(
            includes.is_empty() || !includes.is_empty(),
            "includes are surfaced, never silently dropped"
        );
        Ok(set)
    }

    /// Parse a pattern file, resolving each `include "..."` through `resolve`
    /// (given the quoted path, return that file's text). Included rules are
    /// placed before the including file's own, matching the upstream
    /// convention where an `include` at the top pulls in a base rule set.
    pub fn parse_with_includes<F>(source: &str, mut resolve: F) -> Result<Self, RuleParseError>
    where F: FnMut(&str) -> Option<String> {
        Self::parse_with_includes_inner(source, &mut resolve)
    }

    fn parse_with_includes_inner<F>(
        source: &str,
        resolve: &mut F
    ) -> Result<Self, RuleParseError>
    where F: FnMut(&str) -> Option<String> {
        let (mut set, includes) = Self::parse_inner(source)?;
        let mut all = Vec::new();
        for path in includes {
            if let Some(text) = resolve(&path) {
                let included = Self::parse_with_includes_inner(&text, resolve)?;
                all.extend(included.rules);
            }
        }
        all.append(&mut set.rules);
        Ok(Self { rules: all })
    }

    /// The shared line-oriented driver. Returns the parsed rules plus every
    /// `include`d path encountered, in order.
    fn parse_inner(source: &str) -> Result<(Self, Vec<String>), RuleParseError> {
        let mut rules = Vec::new();
        let mut includes = Vec::new();
        let mut current: Option<(Rule, usize)> = None;
        let mut section = Section::Match;

        for (idx, raw) in source.lines().enumerate() {
            let line_no = idx + 1;
            let line = strip_comment(raw).trim();
            if line.is_empty() {
                // A blank line closes the current pattern.
                if let Some((rule, start)) = current.take() {
                    finish_rule(rule, start, &mut rules)?;
                }
                section = Section::Match;
                continue;
            }

            if let Some(rest) = strip_keyword(line, "pattern:") {
                let description = rest.trim();
                // A *bare* `pattern:` is overloaded upstream: after a
                // described `pattern:`/`name:` header it acts as a plain
                // section marker introducing the match lines (e.g. the
                // `sdccshiftr2` rule, whose one-line description is long
                // enough that the author restated where the pattern body
                // begins), but it also legitimately opens a rule that simply
                // has no description at all (every rule in upstream's
                // `jumptablepatterns.txt`). Disambiguate on whether the rule
                // being built has collected any match lines yet.
                let is_section_marker = description.is_empty()
                    && current
                        .as_ref()
                        .is_some_and(|(rule, _)| rule.match_lines.is_empty());
                if is_section_marker {
                    section = Section::Match;
                    continue;
                }

                if let Some((rule, start)) = current.take() {
                    finish_rule(rule, start, &mut rules)?;
                }
                current = Some((
                    Rule {
                        description: description.to_string(),
                        name: None,
                        tags: Vec::new(),
                        match_lines: Vec::new(),
                        replacement_lines: Vec::new(),
                        constraints: Vec::new()
                    },
                    line_no
                ));
                section = Section::Match;
                continue;
            }

            if let Some(rest) = strip_keyword(line, "include") {
                includes.push(rest.trim().trim_matches('"').to_string());
                continue;
            }

            let Some((rule, _)) = current.as_mut()
            else {
                // Anything outside a pattern block that is not an include is a
                // stray section keyword or junk.
                return Err(RuleParseError::OrphanSection {
                    line: line_no,
                    keyword: line.split(':').next().unwrap_or(line).to_string()
                });
            };

            if let Some(rest) = strip_keyword(line, "name:") {
                rule.name = Some(rest.trim().to_string());
                continue;
            }
            if let Some(rest) = strip_keyword(line, "tags:") {
                rule.tags = rest
                    .split(',')
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect();
                continue;
            }
            if let Some(rest) = strip_keyword(line, "parameterized:") {
                // Real, documented syntax that this crate does not expand yet.
                // Parsed as a constraint so the rule still round-trips; the
                // engine treats an unknown constraint name as "unsupported"
                // and skips the whole rule, which is the correct conservative
                // behavior for an unexpanded parameterized pattern.
                rule.constraints.push(Constraint {
                    name: "parameterized".to_string(),
                    args: vec![OperandPattern::Ident(rest.trim().to_string())],
                    check_after: None
                });
                continue;
            }
            if strip_keyword(line, "replacement:").is_some() {
                section = Section::Replacement;
                continue;
            }
            if strip_keyword(line, "constraints:").is_some() {
                section = Section::Constraints;
                continue;
            }

            match section {
                Section::Constraints => {
                    let constraint = parse_constraint(line).ok_or_else(|| {
                        RuleParseError::Syntax {
                            line: line_no,
                            text: line.to_string()
                        }
                    })?;
                    rule.constraints.push(constraint);
                },
                Section::Match | Section::Replacement => {
                    let numbered = parse_numbered_instr(line).ok_or_else(|| {
                        RuleParseError::Syntax {
                            line: line_no,
                            text: line.to_string()
                        }
                    })?;
                    let target = if section == Section::Match {
                        &mut rule.match_lines
                    }
                    else {
                        &mut rule.replacement_lines
                    };
                    if target.iter().any(|l| l.index == numbered.index) {
                        return Err(RuleParseError::DuplicateIndex {
                            line: line_no,
                            index: numbered.index
                        });
                    }
                    target.push(numbered);
                }
            }
        }

        if let Some((rule, start)) = current.take() {
            finish_rule(rule, start, &mut rules)?;
        }

        Ok((Self { rules }, includes))
    }
}

fn finish_rule(rule: Rule, start: usize, out: &mut Vec<Rule>) -> Result<(), RuleParseError> {
    if !rule.has_anchor() {
        return Err(RuleParseError::MissingAnchor {
            line: start,
            name: rule.name.clone()
        });
    }
    out.push(rule);
    Ok(())
}

/// Remove a trailing `;` comment, respecting quoted strings.
fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_string = !in_string,
            ';' if !in_string => return &line[..i],
            _ => {}
        }
    }
    line
}

/// If `line` starts with `keyword` (case-insensitively), return the rest.
fn strip_keyword<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let head = line.get(..keyword.len())?;
    head.eq_ignore_ascii_case(keyword)
        .then(|| &line[keyword.len()..])
}

fn parse_numbered_instr(line: &str) -> Option<NumberedInstr> {
    let (index_text, rest) = line.split_once(':')?;
    let index: u32 = index_text.trim().parse().ok()?;
    let instr = parse_instr_pattern(rest.trim())?;
    Some(NumberedInstr { index, instr })
}

fn parse_instr_pattern(text: &str) -> Option<InstrPattern> {
    let text = text.trim();
    if text == "*" {
        return Some(InstrPattern::Wildcard);
    }

    // `[n] <instr>` / `[?var] <instr>`
    if let Some(rest) = text.strip_prefix('[') {
        let (count_text, rest) = rest.split_once(']')?;
        let count_text = count_text.trim();
        let count = if let Some(name) = count_text.strip_prefix('?') {
            RepeatCount::Variable(name.trim().to_string())
        }
        else {
            RepeatCount::Fixed(count_text.parse().ok()?)
        };
        return Some(InstrPattern::Repeat {
            count,
            instr: Box::new(parse_instr_pattern(rest)?)
        });
    }

    let (mnemonic_text, operand_text) = match text.split_once(char::is_whitespace) {
        Some((m, rest)) => (m, rest.trim()),
        None => (text, "")
    };
    if mnemonic_text.is_empty() {
        return None;
    }
    let mnemonic = match mnemonic_text.strip_prefix('?') {
        Some(name) => MnemonicPattern::Variable(name.to_string()),
        None => MnemonicPattern::Literal(mnemonic_text.to_ascii_uppercase())
    };

    let mut operands = Vec::new();
    if !operand_text.is_empty() {
        for part in split_top_level(operand_text, ',') {
            operands.push(parse_operand(part.trim())?);
        }
    }

    Some(InstrPattern::Instr { mnemonic, operands })
}

/// Split on `sep`, ignoring separators nested inside parentheses.
fn split_top_level(text: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (i, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            c if c == sep && depth == 0 => {
                out.push(&text[start..i]);
                start = i + c.len_utf8();
            },
            _ => {}
        }
    }
    out.push(&text[start..]);
    out
}

fn parse_operand(text: &str) -> Option<OperandPattern> {
    let mut input = text.trim();
    let parsed = operand_expr(&mut input).ok()?;
    input.trim().is_empty().then_some(parsed)
}

// ─── winnow expression parser for operands ────────────────────────────────────

type Stream<'a> = &'a str;

fn operand_expr(input: &mut Stream<'_>) -> ModalResult<OperandPattern, ContextError> {
    comparison(input)
}

fn comparison(input: &mut Stream<'_>) -> ModalResult<OperandPattern, ContextError> {
    let mut lhs = additive(input)?;
    loop {
        let _ = space0(input)?;
        let Some(op) = opt(alt((
            "<=".value(BinOp::LessEqual),
            ">=".value(BinOp::GreaterEqual),
            "!=".value(BinOp::NotEqual),
            "==".value(BinOp::Equal),
            "<<".value(BinOp::ShiftLeft),
            ">>".value(BinOp::ShiftRight),
            "<".value(BinOp::Less),
            ">".value(BinOp::Greater)
        )))
        .parse_next(input)?
        else {
            return Ok(lhs);
        };
        let _ = space0(input)?;
        let rhs = additive(input)?;
        lhs = OperandPattern::Binary {
            lhs: Box::new(lhs),
            op,
            rhs: Box::new(rhs)
        };
    }
}

fn additive(input: &mut Stream<'_>) -> ModalResult<OperandPattern, ContextError> {
    let mut lhs = multiplicative(input)?;
    loop {
        let _ = space0(input)?;
        let Some(op) = opt(alt((
            '+'.value(BinOp::Add),
            '-'.value(BinOp::Sub),
            '|'.value(BinOp::BitOr),
            '^'.value(BinOp::BitXor)
        )))
        .parse_next(input)?
        else {
            return Ok(lhs);
        };
        let _ = space0(input)?;
        let rhs = multiplicative(input)?;
        lhs = OperandPattern::Binary {
            lhs: Box::new(lhs),
            op,
            rhs: Box::new(rhs)
        };
    }
}

fn multiplicative(input: &mut Stream<'_>) -> ModalResult<OperandPattern, ContextError> {
    let mut lhs = unary(input)?;
    loop {
        let _ = space0(input)?;
        let Some(op) = opt(alt((
            '*'.value(BinOp::Mul),
            '/'.value(BinOp::Div),
            '%'.value(BinOp::Mod),
            '&'.value(BinOp::BitAnd)
        )))
        .parse_next(input)?
        else {
            return Ok(lhs);
        };
        let _ = space0(input)?;
        let rhs = unary(input)?;
        lhs = OperandPattern::Binary {
            lhs: Box::new(lhs),
            op,
            rhs: Box::new(rhs)
        };
    }
}

fn unary(input: &mut Stream<'_>) -> ModalResult<OperandPattern, ContextError> {
    let _ = space0(input)?;
    if let Some(op) = opt(alt(('-'.value(UnOp::Neg), '~'.value(UnOp::Not)))).parse_next(input)? {
        let operand = unary(input)?;
        return Ok(OperandPattern::Unary {
            op,
            operand: Box::new(operand)
        });
    }
    atom(input)
}

fn atom(input: &mut Stream<'_>) -> ModalResult<OperandPattern, ContextError> {
    let _ = space0(input)?;
    alt((indirect, variable, number, ident)).parse_next(input)
}

fn indirect(input: &mut Stream<'_>) -> ModalResult<OperandPattern, ContextError> {
    delimited(('(', space0), operand_expr, (space0, ')'))
        .map(|inner| OperandPattern::Indirect(Box::new(inner)))
        .parse_next(input)
}

fn variable(input: &mut Stream<'_>) -> ModalResult<OperandPattern, ContextError> {
    preceded('?', take_while(1.., |c: char| c.is_ascii_alphanumeric() || c == '_'))
        .map(|name: &str| OperandPattern::Variable(name.to_string()))
        .parse_next(input)
}

fn number(input: &mut Stream<'_>) -> ModalResult<OperandPattern, ContextError> {
    alt((hex_number, binary_number, decimal_number)).parse_next(input)
}

fn hex_number(input: &mut Stream<'_>) -> ModalResult<OperandPattern, ContextError> {
    let digits = preceded(
        alt(("0x", "0X", "#", "$", "&")),
        take_while(1.., |c: char| c.is_ascii_hexdigit())
    )
    .parse_next(input)?;
    i64::from_str_radix(digits, 16)
        .map(OperandPattern::Number)
        .map_err(|_| ParserError::from_input(input))
}

fn binary_number(input: &mut Stream<'_>) -> ModalResult<OperandPattern, ContextError> {
    let digits = preceded(alt(("0b", "0B", "%")), take_while(1.., |c: char| c == '0' || c == '1'))
        .parse_next(input)?;
    i64::from_str_radix(digits, 2)
        .map(OperandPattern::Number)
        .map_err(|_| ParserError::from_input(input))
}

fn decimal_number(input: &mut Stream<'_>) -> ModalResult<OperandPattern, ContextError> {
    let digits = digit1(input)?;
    digits
        .parse::<i64>()
        .map(OperandPattern::Number)
        .map_err(|_| ParserError::from_input(input))
}

fn ident(input: &mut Stream<'_>) -> ModalResult<OperandPattern, ContextError> {
    let first = one_of(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '.').parse_next(input)?;
    let rest: &str = take_while(0.., |c: char| {
        c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '\''
    })
    .parse_next(input)?;
    let mut name = String::with_capacity(1 + rest.len());
    name.push(first);
    name.push_str(rest);
    Ok(OperandPattern::Ident(name))
}

fn parse_constraint(line: &str) -> Option<Constraint> {
    let open = line.find('(')?;
    let name = line[..open].trim();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let close = line.rfind(')')?;
    if close < open {
        return None;
    }
    let args_text = &line[open + 1..close];

    // An optional `:ID` suffix after the closing parenthesis.
    let tail = line[close + 1..].trim();
    let check_after = if let Some(id) = tail.strip_prefix(':') {
        Some(id.trim().parse().ok()?)
    }
    else if tail.is_empty() {
        None
    }
    else {
        return None;
    };

    let mut args = Vec::new();
    if !args_text.trim().is_empty() {
        for part in split_top_level(args_text, ',') {
            args.push(parse_operand(part.trim())?);
        }
    }

    Some(Constraint {
        name: name.to_string(),
        args,
        check_after
    })
}

/// Silence unused-import warnings for combinators kept for the next phase.
#[allow(dead_code)]
fn _unused(input: &mut Stream<'_>) -> ModalResult<(), ContextError> {
    let _ = opt(terminated(till_line_ending, opt(line_ending))).parse_next(input)?;
    let _: Vec<()> = repeat(0.., ' '.value(())).parse_next(input)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instr(mnemonic: &str, operands: Vec<OperandPattern>) -> InstrPattern {
        InstrPattern::Instr {
            mnemonic: MnemonicPattern::Literal(mnemonic.to_ascii_uppercase()),
            operands
        }
    }

    fn var(name: &str) -> OperandPattern {
        OperandPattern::Variable(name.to_string())
    }

    fn id(name: &str) -> OperandPattern {
        OperandPattern::Ident(name.to_string())
    }

    /// The real `cp02ora` pattern, verbatim from upstream `pbo-patterns.txt`.
    #[test]
    fn parses_the_real_cp_zero_to_or_a_pattern() {
        let src = "\
pattern: Replace cp 0 with or a
name: cp02ora
0: cp 0
replacement:
0: or a
constraints:
flagsNotUsedAfter(0,N,P/V)
";
        let set = RuleSet::parse(src).unwrap();
        assert_eq!(set.rules.len(), 1);
        let rule = &set.rules[0];
        assert_eq!(rule.description, "Replace cp 0 with or a");
        assert_eq!(rule.name.as_deref(), Some("cp02ora"));
        assert_eq!(rule.match_lines, vec![NumberedInstr {
            index: 0,
            instr: instr("cp", vec![OperandPattern::Number(0)])
        }]);
        assert_eq!(rule.replacement_lines, vec![NumberedInstr {
            index: 0,
            instr: instr("or", vec![id("a")])
        }]);
        assert_eq!(rule.constraints.len(), 1);
        assert_eq!(rule.constraints[0].name, "flagsNotUsedAfter");
        assert_eq!(rule.constraints[0].check_after, None);
    }

    /// The real `ld0-to-xor` pattern - exercises a `?const` variable operand
    /// and an `equal(...)` constraint carrying one.
    #[test]
    fn parses_the_real_ld_a_zero_to_xor_pattern() {
        let src = "\
pattern: Replace ld a,?const with xor a
name: ld0-to-xor
0: ld a,?const
replacement:
0: xor a
constraints:
equal(?const,0)
flagsNotUsedAfter(0,S,Z,H,P/V,N,C)
";
        let rule = &RuleSet::parse(src).unwrap().rules[0];
        assert_eq!(rule.match_lines[0].instr, instr("ld", vec![
            id("a"),
            var("const")
        ]));
        assert_eq!(rule.constraints[0].name, "equal");
        assert_eq!(rule.constraints[0].args, vec![
            var("const"),
            OperandPattern::Number(0)
        ]);
        assert!(rule.variables().contains("const"));
    }

    /// The real `jp2jr` pattern - the address-aware case, and the only rule
    /// upstream that carries a `tags:` line this crate cares about.
    #[test]
    fn parses_the_real_jp_to_jr_pattern_with_its_tag() {
        let src = "\
pattern: Replace jp ?const1 with jr ?const1
name: jp2jr
tags: cpc
0: jp ?const1
replacement:
0: jr ?const1
constraints:
reachableByJr(0,?const1)
";
        let rule = &RuleSet::parse(src).unwrap().rules[0];
        assert_eq!(rule.tags, vec!["cpc".to_string()]);
        assert_eq!(rule.match_lines[0].instr, instr("jp", vec![var("const1")]));
        assert_eq!(rule.replacement_lines[0].instr, instr("jr", vec![var("const1")]));
        assert_eq!(rule.constraints[0].name, "reachableByJr");
        assert_eq!(rule.constraints[0].args, vec![
            OperandPattern::Number(0),
            var("const1")
        ]);
    }

    /// An empty `replacement:` section means "delete the matched
    /// instructions" - real upstream syntax (`unnecessary-ld-to-itself`).
    #[test]
    fn an_empty_replacement_section_means_deletion() {
        let src = "\
pattern: Remove ld ?reg,?reg
name: unnecessary-ld-to-itself
0: ld ?reg,?reg
replacement:
constraints:
in(?reg,A,B,C,D,E,H,L)
";
        let rule = &RuleSet::parse(src).unwrap().rules[0];
        assert!(rule.replacement_lines.is_empty());
        assert_eq!(rule.constraints[0].name, "in");
        assert_eq!(rule.constraints[0].args.len(), 8);
    }

    /// `?op` in mnemonic position, and a multi-line pattern whose replacement
    /// drops one of the matched lines (index 2 present in the match, absent
    /// from the replacement) - the real `czjump2c` shape.
    #[test]
    fn parses_an_op_variable_and_a_dropped_replacement_line() {
        let src = "\
pattern: Replace cp ?const1; ?op1 c,?const2; ?op1 z,?const2
name: czjump2c
0: cp ?8bitconst1
1: ?op1 c,?const2
2: ?op1 z,?const2
replacement:
0: cp ?8bitconst1+1
1: ?op1 c,?const2
constraints:
in(?op1,jp,jr)
notEqual(?8bitconst1,255)
";
        let rule = &RuleSet::parse(src).unwrap().rules[0];
        assert_eq!(rule.match_lines.len(), 3);
        assert_eq!(rule.replacement_lines.len(), 2);
        assert!(!rule.replacement_lines.iter().any(|l| l.index == 2));

        let InstrPattern::Instr { mnemonic, .. } = &rule.match_lines[1].instr
        else {
            panic!("expected a plain instruction")
        };
        assert_eq!(mnemonic, &MnemonicPattern::Variable("op1".to_string()));

        // `?8bitconst1+1` must parse as real arithmetic, not as one identifier.
        assert_eq!(rule.replacement_lines[0].instr, instr("cp", vec![
            OperandPattern::Binary {
                lhs: Box::new(var("8bitconst1")),
                op: BinOp::Add,
                rhs: Box::new(OperandPattern::Number(1))
            }
        ]));
    }

    /// `*` wildcards and `[?var] instr` repeats - both real upstream forms
    /// (120 and 14 occurrences respectively in `pbo-patterns.txt`).
    #[test]
    fn parses_wildcards_and_repeats() {
        let src = "\
pattern: wildcard and repeat
0: [?const1] srl a
1: *
replacement:
0: [?const1] rrca
1: and #ff >> ?const1
";
        let rule = &RuleSet::parse(src).unwrap().rules[0];
        assert_eq!(rule.match_lines[0].instr, InstrPattern::Repeat {
            count: RepeatCount::Variable("const1".to_string()),
            instr: Box::new(instr("srl", vec![id("a")]))
        });
        assert_eq!(rule.match_lines[1].instr, InstrPattern::Wildcard);
        // `#ff >> ?const1`: hex literal, shift operator, variable.
        assert_eq!(rule.replacement_lines[1].instr, instr("and", vec![
            OperandPattern::Binary {
                lhs: Box::new(OperandPattern::Number(0xFF)),
                op: BinOp::ShiftRight,
                rhs: Box::new(var("const1"))
            }
        ]));
    }

    #[test]
    fn parses_indirect_and_indexed_operands() {
        let src = "\
pattern: indirect
0: ld (?regixiy + ?const1), ?const4
replacement:
0: ld (hl), ?const4
";
        let rule = &RuleSet::parse(src).unwrap().rules[0];
        assert_eq!(rule.match_lines[0].instr, instr("ld", vec![
            OperandPattern::Indirect(Box::new(OperandPattern::Binary {
                lhs: Box::new(var("regixiy")),
                op: BinOp::Add,
                rhs: Box::new(var("const1"))
            })),
            var("const4")
        ]));
        assert_eq!(rule.replacement_lines[0].instr, instr("ld", vec![
            OperandPattern::Indirect(Box::new(id("hl"))),
            var("const4")
        ]));
    }

    #[test]
    fn several_patterns_separated_by_blank_lines() {
        let src = "\
; a leading comment
pattern: first
0: nop
replacement:
0: nop

pattern: second
0: ret
replacement:
0: ret
";
        let set = RuleSet::parse(src).unwrap();
        assert_eq!(set.rules.len(), 2);
        assert_eq!(set.rules[0].description, "first");
        assert_eq!(set.rules[1].description, "second");
    }

    #[test]
    fn comments_are_stripped_but_not_inside_strings() {
        assert_eq!(strip_comment("0: nop ; trailing"), "0: nop ");
        assert_eq!(strip_comment("include \"a;b.txt\""), "include \"a;b.txt\"");
    }

    #[test]
    fn a_constraint_may_carry_a_check_after_suffix() {
        let c = parse_constraint("regsNotUsed(1,A):1").unwrap();
        assert_eq!(c.name, "regsNotUsed");
        assert_eq!(c.check_after, Some(1));
    }

    #[test]
    fn includes_are_resolved_through_the_callback() {
        let base = "\
pattern: base rule
0: nop
replacement:
0: nop
";
        let src = "\
include \"base.txt\"

pattern: own rule
0: ret
replacement:
0: ret
";
        let set = RuleSet::parse_with_includes(src, |path| {
            (path == "base.txt").then(|| base.to_string())
        })
        .unwrap();
        // Included rules come first, then the including file's own.
        assert_eq!(set.rules.len(), 2);
        assert_eq!(set.rules[0].description, "base rule");
        assert_eq!(set.rules[1].description, "own rule");
    }

    #[test]
    fn a_pattern_without_an_anchor_line_is_rejected() {
        let src = "\
pattern: no anchor
1: nop
replacement:
1: nop
";
        assert!(matches!(
            RuleSet::parse(src),
            Err(RuleParseError::MissingAnchor { .. })
        ));
    }

    #[test]
    fn a_duplicated_line_index_is_rejected() {
        let src = "\
pattern: duplicated
0: nop
0: ret
replacement:
0: nop
";
        assert!(matches!(
            RuleSet::parse(src),
            Err(RuleParseError::DuplicateIndex { index: 0, .. })
        ));
    }

    #[test]
    fn a_numbered_line_outside_a_pattern_block_is_rejected() {
        assert!(matches!(
            RuleSet::parse("0: nop\n"),
            Err(RuleParseError::OrphanSection { .. })
        ));
    }

    #[test]
    fn a_malformed_instruction_line_is_rejected() {
        let src = "\
pattern: bad
0: ld a, (
replacement:
0: nop
";
        assert!(matches!(
            RuleSet::parse(src),
            Err(RuleParseError::Syntax { .. })
        ));
    }
}
