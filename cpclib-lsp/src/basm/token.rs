//! Lexical data for Z80/basm assembly: instruction/directive/register sets,
//! semantic-token indices, the tokens legend, and word-extraction helpers.

use std::collections::HashSet;
use std::sync::LazyLock;

use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;

// Semantic token type indices — must match `semantic_tokens_legend()` order
pub(super) const TT_KEYWORD: u32 = 0; // Z80 instructions
pub(super) const TT_MACRO: u32 = 1; // assembler directives (EQU, DEFB, MACRO…)
pub(super) const TT_FUNCTION: u32 = 2; // macro invocation names
pub(super) const TT_NAMESPACE: u32 = 3; // module names
pub(super) const TT_VARIABLE: u32 = 4; // registers / condition codes
pub(super) const TT_NUMBER: u32 = 5; // numeric literals
pub(super) const TT_STRING: u32 = 6; // string literals
pub(super) const TT_COMMENT: u32 = 7; // line comments
pub(super) const TT_OPERATOR: u32 = 8; // operators
pub(super) const TT_ENUM_MEMBER: u32 = 9; // EQU / assign constants
pub(super) const TT_LABEL: u32 = 10; // jump / procedure labels
pub(super) const TT_PARAMETER: u32 = 11; // macro parameters {param}

pub(super) const MOD_DECLARATION: u32 = 1 << 0;
pub(super) const MOD_READONLY: u32 = 1 << 1;

// Full Z80 register set + condition codes used as operands
pub(super) const REGISTER_LIST: &[&str] = &[
    "AF'", "AF", "BC", "DE", "HL", "IX", "IY", "SP", "PC", "IXH", "IXL", "IYH", "IYL", "A", "B",
    "C", "D", "E", "H", "L", "F", "I", "R", "NZ", "Z", "NC", "PE", "PO", "P", "M"
];

// Static lookup sets — built once, shared across all tokenizer calls
pub(super) static INSTRUCTION_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| cpclib_asm::lsp::Z80_INSTRUCTIONS.iter().copied().collect());

pub(super) static DIRECTIVE_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    for d in cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_STANDALONE {
        s.insert(*d);
    }
    for d in cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_START {
        s.insert(*d);
    }
    for d in cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_END {
        s.insert(*d);
    }
    s
});

pub(super) static REGISTER_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| REGISTER_LIST.iter().copied().collect());

/// Returns the SemanticTokensLegend that must be advertised in `initialize()`.
pub(crate) fn semantic_tokens_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::KEYWORD,     // 0  Z80 instructions
            SemanticTokenType::MACRO,       // 1  assembler directives
            SemanticTokenType::FUNCTION,    // 2  macro invocation names
            SemanticTokenType::NAMESPACE,   // 3  module names
            SemanticTokenType::VARIABLE,    // 4  registers / condition codes
            SemanticTokenType::NUMBER,      // 5  numeric literals
            SemanticTokenType::STRING,      // 6  string literals
            SemanticTokenType::COMMENT,     // 7  comments
            SemanticTokenType::OPERATOR,    // 8  operators
            SemanticTokenType::ENUM_MEMBER, // 9  EQU / assign constants
            SemanticTokenType::TYPE, // 10 jump / procedure labels (teal — avoids theme blue)
            SemanticTokenType::DECORATOR, // 11 macro parameters {param}
        ],
        token_modifiers: vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::READONLY,
        ]
    }
}

impl AssemblyAnalyzer {
    // Helper methods

    pub(super) fn extract_word_at_position(&self, line: &str, column: usize) -> Option<String> {
        let chars: Vec<char> = line.chars().collect();
        if column >= chars.len() {
            return None;
        }

        // Z80/basm identifier characters: alphanumeric, _, ., @
        // The dot allows `.local` labels and qualified names like `module.symbol`
        let is_word = |c: char| c.is_alphanumeric() || c == '_' || c == '.' || c == '@';

        let mut start = column;
        let mut end = column;

        while start > 0 && is_word(chars[start - 1]) {
            start -= 1;
        }
        while end < chars.len() && is_word(chars[end]) {
            end += 1;
        }

        if start < end {
            Some(chars[start..end].iter().collect())
        }
        else {
            None
        }
    }

    // ── Code actions ──────────────────────────────────────────────────────────
}

pub(super) fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'.' || b == b'@'
}

// ─── Listing flattening ────────────────────────────────────────────────────────

/// Recursively flatten a parsed listing, descending into every kind of
/// nested block (`IF`/`IFDEF`/`IFNDEF`, `MODULE`, `REPEAT`/`REPEAT...UNTIL`,
/// `WHILE`, `RORG`, `FOR`, `ITERATE`, `SWITCH`, `CONFINED`, crunched
/// sections, assembler-control blocks).
///
/// `listing.iter()` alone only sees the *top-level* statements — a file
/// entirely wrapped in an `ifndef GUARD ... endif` header guard (extremely
/// common in real-world sources, including several of basm's own
/// `inner://` resources, e.g. `ga.asm`) would otherwise expose exactly one
/// top-level `IF` token and none of the labels/constants/macros inside it.
/// Symbol lookups (completion, hover, goto-definition) should use this
/// instead of a shallow `.iter()` so they see everything, not just what
/// happens to sit outside every conditional/loop/module.
pub(super) fn flatten_listing<'a, T>(tokens: impl IntoIterator<Item = &'a T>) -> Vec<&'a T>
where T: cpclib_tokens::ListingElement + 'a {
    fn walk<'a, T>(tokens: impl IntoIterator<Item = &'a T>, out: &mut Vec<&'a T>)
    where T: cpclib_tokens::ListingElement + 'a {
        for token in tokens {
            out.push(token);
            if token.is_module() {
                walk(token.module_listing(), out);
            }
            if token.is_if() {
                for i in 0..token.if_nb_tests() {
                    walk(token.if_test(i).1, out);
                }
                if let Some(else_listing) = token.if_else() {
                    walk(else_listing, out);
                }
            }
            if token.is_repeat() {
                walk(token.repeat_listing(), out);
            }
            if token.is_repeat_until() {
                walk(token.repeat_until_listing(), out);
            }
            if token.is_while() {
                walk(token.while_listing(), out);
            }
            if token.is_rorg() {
                walk(token.rorg_listing(), out);
            }
            if token.is_for() {
                walk(token.for_listing(), out);
            }
            if token.is_iterate() {
                walk(token.iterate_listing(), out);
            }
            if token.is_switch() {
                for (_, case_listing, _) in token.switch_cases() {
                    walk(case_listing, out);
                }
                if let Some(default_listing) = token.switch_default() {
                    walk(default_listing, out);
                }
            }
            if token.is_confined() {
                walk(token.confined_listing(), out);
            }
            if token.is_crunched_section() {
                walk(token.crunched_section_listing(), out);
            }
            if token.is_assembler_control() {
                walk(token.assembler_control_get_listing(), out);
            }
        }
    }

    let mut out = Vec::new();
    walk(tokens, &mut out);
    out
}

// ─── RANGE / DEFSECTION (section definitions) ─────────────────────────────────

/// `true` when `token`'s own statement starts with the `RANGE` or
/// `DEFSECTION` keyword. `ListingElement` has no dedicated `is_range`/
/// `range_*` accessors, so extracting a section's name goes through the
/// (documented-costly, and only partially implemented) `to_token()`
/// conversion — this text-based pre-check, read from the token's own source
/// rather than any parsed representation, keeps that call off the hot path
/// of ordinary opcode/directive lines and away from `to_token()`'s
/// unimplemented (`todo!()`-panicking) fallback for other directive kinds.
pub(super) fn starts_with_range_keyword<T: cpclib_asm::parser::obtained::MayHaveSpan>(
    token: &T
) -> bool {
    let span_text: &str = token.span().as_ref();
    let first_line = span_text.lines().next().unwrap_or(span_text);
    let first_word: String = first_line
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    let upper = first_word.to_uppercase();
    upper == "RANGE" || upper == "DEFSECTION"
}

/// Best-effort `(line, column)` of `name` within `token`'s own statement —
/// used for `RANGE`/`DEFSECTION`, where the token's span points at the
/// keyword (`RANGE start, stop, name`), not at the name argument itself.
/// Falls back to the keyword's own position if `name` can't be found on the
/// statement's first line (shouldn't happen for a well-formed `Token::Range`
/// just extracted from this same token).
pub(super) fn locate_name_in_statement<T: cpclib_asm::parser::obtained::MayHaveSpan>(
    token: &T,
    name: &str
) -> (u32, u32) {
    let span = token.span();
    let span_text: &str = span.as_ref();
    let first_line = span_text.lines().next().unwrap_or(span_text);
    let (line_1based, col_1based) = span.relative_line_and_column();
    let lsp_line = line_1based.saturating_sub(1) as u32;
    let base_col = col_1based.saturating_sub(1) as u32;
    let col = first_line
        .find(name)
        .map(|byte_off| base_col + byte_off as u32)
        .unwrap_or(base_col);
    (lsp_line, col)
}

/// A numeral literal's radix — used to locate a 16-bit literal's high/low
/// byte sub-spans (2 hex digits or 8 binary digits per byte; decimal is
/// never byte-split, since decimal digit boundaries don't align to bytes).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum NumeralStyle {
    Hex,
    Decimal,
    Binary
}

impl NumeralStyle {
    pub(super) fn digits_per_byte(self) -> usize {
        match self {
            NumeralStyle::Hex => 2,
            NumeralStyle::Binary => 8,
            NumeralStyle::Decimal => 0
        }
    }
}

/// A numeral literal found while lexing, with enough detail to both
/// evaluate it and precisely place/reformat a color swatch: `token_start`/
/// `token_end` bound the whole literal (base prefix and suffix included,
/// e.g. all of `0x54` or `7F40h`); `prefix`/`suffix` are those captured
/// verbatim (whichever the source actually used — `0x`/`#`/`$`/`&`/`0b`/`%`,
/// or a trailing `h`/`b`) so an edit can preserve the exact same notation;
/// `digits_start`/`digits_end` bound the digit run alone, for locating a
/// 16-bit literal's byte midpoint.
pub(super) struct NumeralLiteral {
    pub(super) line: u32,
    pub(super) token_start: u32,
    pub(super) token_end: u32,
    pub(super) prefix: String,
    pub(super) suffix: String,
    pub(super) digits_start: u32,
    pub(super) digits_end: u32,
    pub(super) value: u32,
    pub(super) style: NumeralStyle
}

/// Lex every numeral literal in `text` (skipping `skip_lines` — LOCOMOTIVE
/// block lines, tokenised separately as BASIC), delegating the actual
/// lexing/parsing to `cpclib_common::parse::scan_numeric_literals` — the
/// same numeral grammar the real assembler uses (every prefix/suffix form:
/// `0x`/`0X`/`#`/`$`/`&` hex, `0b`/`0B`/`%` binary, trailing `h`/`b`
/// suffixes — a hand-rolled lexer here would drift from it). That function
/// already refuses to start a numeral right after an identifier character,
/// so a label like `STATE84` is never misread as ending in a literal `84`.
/// Only its own line-scoped double-quote-string skipping is built in, so
/// `;` comments are stripped first via `format::strip_asm_comment`.
pub(super) fn scan_numeral_literals(
    text: &str,
    skip_lines: &std::collections::HashSet<usize>
) -> Vec<NumeralLiteral> {
    let mut out = Vec::new();

    for (line_idx, line) in text.lines().enumerate() {
        if skip_lines.contains(&line_idx) {
            continue;
        }
        let code = super::format::strip_asm_comment(line);
        for (start, end, value, kind) in cpclib_common::parse::scan_numeric_literals(code) {
            let style = match kind {
                cpclib_common::parse::EncodingKind::Hex => NumeralStyle::Hex,
                cpclib_common::parse::EncodingKind::Bin => NumeralStyle::Binary,
                cpclib_common::parse::EncodingKind::Dec => NumeralStyle::Decimal,
                // Octal has no GA-byte meaning; the ambiguous/unknown
                // states are internal to the parser and never returned.
                _ => continue
            };
            let token_text = &code[start..end];
            let (prefix_len, suffix_len) = prefix_and_suffix_len(token_text, style);
            out.push(NumeralLiteral {
                line: line_idx as u32,
                token_start: start as u32,
                token_end: end as u32,
                prefix: token_text[..prefix_len].to_string(),
                suffix: token_text[token_text.len() - suffix_len..].to_string(),
                digits_start: (start + prefix_len) as u32,
                digits_end: (end - suffix_len) as u32,
                value,
                style
            });
        }
    }

    out
}

/// How many of a numeral token's leading/trailing characters are its base
/// prefix/suffix (as opposed to digits) — e.g. `("0x", 2, 0)`, `("$", 1,
/// 0)`, `("7F40h", 0, 1)`, `("84", 0, 0)` for a bare decimal.
fn prefix_and_suffix_len(text: &str, style: NumeralStyle) -> (usize, usize) {
    match style {
        NumeralStyle::Hex => {
            if text.len() >= 2 && matches!(&text[..2], "0x" | "0X") {
                (2, 0)
            }
            else if text.starts_with(['#', '$', '&']) {
                (1, 0)
            }
            else if text.ends_with(['h', 'H']) {
                (0, 1)
            }
            else {
                (0, 0)
            }
        },
        NumeralStyle::Binary => {
            if text.len() >= 2 && matches!(&text[..2], "0b" | "0B") {
                (2, 0)
            }
            else if text.starts_with('%') {
                (1, 0)
            }
            else if text.ends_with(['b', 'B']) {
                (0, 1)
            }
            else {
                (0, 0)
            }
        },
        NumeralStyle::Decimal => (0, 0)
    }
}

// ─── Generated directive documentation ────────────────────────────────────────

// `DIRECTIVE_DOCS` + `DIRECTIVE_FILE_ARGS`, generated from docs/basm/directives.md.
include!(concat!(env!("OUT_DIR"), "/directive_docs_generated.rs"));

/// Look up a directive by name (case-insensitive) and return a one-line
/// summary of its documentation, for use as a completion item's `.detail`.
pub(super) fn directive_first_doc_line(word_upper: &str) -> Option<String> {
    DIRECTIVE_DOCS
        .iter()
        .find(|(names, _)| names.iter().any(|n| n.to_uppercase() == word_upper))
        .map(|(_, doc)| crate::common::render::first_doc_line(doc))
}
