//! Lexical data and small text-extraction helpers shared by the bndbuild
//! feature modules.

use std::collections::HashSet;
use std::sync::LazyLock;

use super::BuildFileAnalyzer;
use crate::common::document::Document;

// Reuse the same token type indices as the basm module (same SemanticTokensLegend)
pub(super) const TT_KEYWORD: u32 = 0;
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

/// Every alias of the `targets:` key group (`targets`/`tgt`/`target`/
/// `build`, per `cpclib_bndbuild::lsp::RULE_KEYS`) — hoisted here since
/// nearly every feature module (`definition.rs`, `call_hierarchy.rs`,
/// `autocomplete.rs`, `command.rs`, `symbols.rs`) previously recomputed this
/// exact `.iter().find(...).map(...)` scan independently, on every call.
pub(super) static TGT_KEY_NAMES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    cpclib_bndbuild::lsp::RULE_KEYS
        .iter()
        .find(|k| k.names.contains(&"targets"))
        .map(|k| k.names.to_vec())
        .unwrap_or_default()
});

/// Every alias of the `dependencies:` key group (`dependencies`/`dep`/
/// `dependency`/`requires`). See `TGT_KEY_NAMES`.
pub(super) static DEP_KEY_NAMES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    cpclib_bndbuild::lsp::RULE_KEYS
        .iter()
        .find(|k| k.names.contains(&"dependencies"))
        .map(|k| k.names.to_vec())
        .unwrap_or_default()
});

/// Every alias of the `tasks:` key group (`tasks`/`cmd`/`task`/`run`/...).
/// See `TGT_KEY_NAMES`.
pub(super) static TASK_KEY_NAMES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    cpclib_bndbuild::lsp::RULE_KEYS
        .iter()
        .find(|k| k.names.contains(&"tasks"))
        .map(|k| k.names.to_vec())
        .unwrap_or_default()
});

/// `TGT_KEY_NAMES` and `DEP_KEY_NAMES` combined — every key whose value is a
/// filename (as opposed to a command line or a boolean/help string), used
/// wherever a field is navigated/completed as a file regardless of whether
/// it's a target or a dependency.
pub(super) static FILE_KEY_NAMES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    cpclib_bndbuild::lsp::RULE_KEYS
        .iter()
        .filter(|k| k.names.contains(&"targets") || k.names.contains(&"dependencies"))
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

#[cfg(test)]
mod rule_key_alias_tests {
    use super::*;

    #[test]
    fn tgt_key_names_contains_every_targets_alias() {
        assert!(TGT_KEY_NAMES.contains(&"targets"));
        assert!(TGT_KEY_NAMES.contains(&"tgt"));
        assert!(!TGT_KEY_NAMES.contains(&"dep"));
    }

    #[test]
    fn dep_key_names_contains_every_dependencies_alias() {
        assert!(DEP_KEY_NAMES.contains(&"dependencies"));
        assert!(DEP_KEY_NAMES.contains(&"dep"));
        assert!(!DEP_KEY_NAMES.contains(&"tgt"));
    }

    #[test]
    fn task_key_names_contains_cmd() {
        assert!(TASK_KEY_NAMES.contains(&"tasks"));
        assert!(TASK_KEY_NAMES.contains(&"cmd"));
    }

    #[test]
    fn file_key_names_is_the_union_of_targets_and_dependencies() {
        for &k in TGT_KEY_NAMES.iter() {
            assert!(FILE_KEY_NAMES.contains(&k), "missing tgt alias {k}");
        }
        for &k in DEP_KEY_NAMES.iter() {
            assert!(FILE_KEY_NAMES.contains(&k), "missing dep alias {k}");
        }
        assert!(!FILE_KEY_NAMES.contains(&"cmd"));
    }
}
