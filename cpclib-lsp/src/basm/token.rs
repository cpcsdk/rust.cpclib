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

// ─── Generated directive documentation ────────────────────────────────────────

// `DIRECTIVE_DOCS` + `DIRECTIVE_FILE_ARGS`, generated from docs/basm/directives.md.
include!(concat!(env!("OUT_DIR"), "/directive_docs_generated.rs"));
