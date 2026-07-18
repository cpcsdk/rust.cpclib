//! Lexical data and small text-extraction helpers shared by the bndbuild
//! feature modules.

use std::collections::HashSet;
use std::sync::LazyLock;

use super::BuildFileAnalyzer;
use crate::common::document::Document;

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

    /// A bare `- value` list item never repeats its governing key on its own
    /// line — that key is on an earlier, less-indented `key:` line, e.g.:
    /// ```yaml
    /// dep:
    ///   - a.bin
    ///   - b.bin
    /// ```
    /// Scan upward from `line_idx` for the nearest strictly-less-indented
    /// line and, if it's a `key:` with an empty value (i.e. it introduces the
    /// following block list rather than being itself a scalar or a rule-start
    /// item), return that key's trimmed text. Returns `None` for a root-level
    /// `- ` item (which starts a *new rule*, not a list value) or when no
    /// such enclosing key is found.
    pub(super) fn enclosing_key_for_list_item(
        document: &Document,
        line_idx: usize
    ) -> Option<String> {
        let line = document.line(line_idx)?;
        let indent = line.chars().take_while(|c| c.is_whitespace()).count();

        for prev_idx in (0..line_idx).rev() {
            let prev = document.line(prev_idx)?;
            if prev.trim().is_empty() {
                continue;
            }
            let prev_indent = prev.chars().take_while(|c| c.is_whitespace()).count();
            if prev_indent >= indent {
                continue;
            }
            let content = prev.trim_start();
            let content = content.strip_prefix("- ").unwrap_or(content);
            let (key, value) = content.split_once(':')?;
            return if value.trim().is_empty() {
                Some(key.trim().to_string())
            }
            else {
                None
            };
        }
        None
    }
}
