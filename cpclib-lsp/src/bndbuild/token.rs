//! Lexical data and small text-extraction helpers shared by the bndbuild
//! feature modules.

use std::collections::HashSet;
use std::sync::LazyLock;

use super::BuildFileAnalyzer;

// Reuse the same token type indices as the basm module (same SemanticTokensLegend)
pub(super) const TT_KEYWORD: u32 = 0;
pub(super) const TT_VARIABLE: u32 = 4;
pub(super) const TT_NUMBER: u32 = 5;
pub(super) const TT_STRING: u32 = 6;
pub(super) const TT_COMMENT: u32 = 7;
pub(super) const TT_OPERATOR: u32 = 8;
pub(super) const TT_ENUM_MEMBER: u32 = 9;
pub(super) const MOD_DECLARATION: u32 = 1 << 0;

/// All accepted rule-level key names (canonical + aliases).
pub(super) static RULE_KEYS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    cpclib_bndbuild::lsp::RULE_KEYS
        .iter()
        .flat_map(|k| k.names.iter().copied())
        .collect()
});

/// What multi-line block value is currently being accumulated while scanning.
#[derive(PartialEq)]
pub(super) enum Collecting {
    Nothing,
    Target(u32), // original line of the tgt key
    Help
}

impl BuildFileAnalyzer {
    pub(super) fn extract_word_at_position(&self, line: &str, column: usize) -> Option<String> {
        let chars: Vec<char> = line.chars().collect();
        if column >= chars.len() {
            return None;
        }

        let mut start = column;
        let mut end = column;

        while start > 0
            && (chars[start - 1].is_alphanumeric()
                || chars[start - 1] == '_'
                || chars[start - 1] == '-')
        {
            start -= 1;
        }

        while end < chars.len()
            && (chars[end].is_alphanumeric() || chars[end] == '_' || chars[end] == '-')
        {
            end += 1;
        }

        if start < end {
            Some(chars[start..end].iter().collect())
        }
        else {
            None
        }
    }
}
