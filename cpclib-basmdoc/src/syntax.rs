//! Syntax highlighting module for Z80/BASM assembly code.
//!
//! This module provides syntax highlighting functionality with support for:
//! - Z80 instructions and directives
//! - Registers and macro parameters
//! - Numbers, strings, and comments
//! - Symbol linking for cross-references

use lazy_static::lazy_static;
use regex;

/// Apply syntax highlighting to Z80/BASM code using highlight.js compatible CSS classes
pub(crate) fn highlight_z80_syntax(source: &str) -> String {
    lazy_static! {
        // Build regex patterns from cpclib_asm constants
        static ref INSTRUCTIONS_REGEX: regex::Regex = {
            // SAFETY: INSTRUCTIONS constants are guaranteed to be valid UTF-8 by cpclib_asm
            let instructions = cpclib_asm::parser::instructions::INSTRUCTIONS
                .iter()
                .map(|s| unsafe { std::str::from_utf8_unchecked(s) });
            let pattern = format!(r"(?i)\b({})\b", instructions.collect::<Vec<_>>().join("|"));
            regex::Regex::new(&pattern).unwrap()
        };

        static ref DIRECTIVES_REGEX: regex::Regex = {
            use cpclib_asm::parser::parser::*;
            // SAFETY: Directive constants are guaranteed to be valid UTF-8 by cpclib_asm
            let directives = STAND_ALONE_DIRECTIVE.iter()
                .chain(START_DIRECTIVE.iter())
                .chain(END_DIRECTIVE.iter())
                .map(|s| unsafe { std::str::from_utf8_unchecked(s) });

            let pattern = format!(r"(?i)\b({})\b", directives.collect::<Vec<_>>().join("|"));
            regex::Regex::new(&pattern).unwrap()
        };

        static ref REGISTERS_REGEX: regex::Regex = {
            regex::Regex::new(
                r"(?i)\b(af|bc|de|hl|ix|iy|ixh|ixl|iyh|iyl|a|b|c|d|e|h|l|i|r|sp|pc|f)\b"
            ).unwrap()
        };

        static ref MACRO_PARAMS_REGEX: regex::Regex = {
            regex::Regex::new(r"\{[a-zA-Z_][a-zA-Z0-9_]*\}").unwrap()
        };

        static ref NUMBERS_REGEX: regex::Regex = {
            regex::Regex::new(
                r"\b0x[0-9a-fA-F]+|\$[0-9a-fA-F]+|#[0-9a-fA-F]+|&[0-9a-fA-F]+|0b[01]+|%[01]+|\b[0-9]+\b"
            ).unwrap()
        };

        static ref STRINGS_REGEX: regex::Regex = {
            regex::Regex::new(r#""[^"]*"|'[^']*'"#).unwrap()
        };

        static ref COMMENTS_REGEX: regex::Regex = {
            // Match single-line comments per-line (multiline mode)
            regex::Regex::new(r"(?m);.*$").unwrap()
        };

        static ref MULTILINE_COMMENT_REGEX: regex::Regex = {
            // Use (?s) flag to make . match newlines, or use [\s\S] to match any character
            regex::Regex::new(r"(?s)/\*.*?\*/").unwrap()
        };
    }

    // Collect all matches with their positions, types, and matched text
    let mut matches: Vec<(usize, usize, &str)> = Vec::new();

    // Find multiline comments first (highest priority - they can span lines)
    for cap in MULTILINE_COMMENT_REGEX.find_iter(source) {
        matches.push((cap.start(), cap.end(), "comment"));
    }

    // Find single-line comments
    for cap in COMMENTS_REGEX.find_iter(source) {
        matches.push((cap.start(), cap.end(), "comment"));
    }

    // Sort matches by start position
    matches.sort_by_key(|m| m.0);

    // Merge overlapping comment ranges to find non-comment regions
    let mut comment_ranges: Vec<(usize, usize)> = Vec::new();
    for (start, end, _) in &matches {
        if let Some(last) = comment_ranges.last_mut() {
            if *start <= last.1 {
                // Overlapping or adjacent - merge
                last.1 = (*end).max(last.1);
                continue;
            }
        }
        comment_ranges.push((*start, *end));
    }

    // Find non-comment ranges (code regions)
    let mut code_ranges: Vec<(usize, usize)> = Vec::new();
    let mut last_end = 0;
    for (start, end) in &comment_ranges {
        if *start > last_end {
            code_ranges.push((last_end, *start));
        }
        last_end = *end;
    }
    if last_end < source.len() {
        code_ranges.push((last_end, source.len()));
    }

    // Find other syntax elements in code regions only
    let mut code_matches: Vec<(usize, usize, &str)> = Vec::new();

    for (code_start, code_end) in code_ranges {
        let code_part = &source[code_start..code_end];

        for cap in STRINGS_REGEX.find_iter(code_part) {
            code_matches.push((cap.start() + code_start, cap.end() + code_start, "string"));
        }
        for cap in NUMBERS_REGEX.find_iter(code_part) {
            code_matches.push((cap.start() + code_start, cap.end() + code_start, "number"));
        }
        for cap in MACRO_PARAMS_REGEX.find_iter(code_part) {
            code_matches.push((cap.start() + code_start, cap.end() + code_start, "meta"));
        }
        for cap in DIRECTIVES_REGEX.find_iter(code_part) {
            code_matches.push((cap.start() + code_start, cap.end() + code_start, "built_in"));
        }
        for cap in INSTRUCTIONS_REGEX.find_iter(code_part) {
            code_matches.push((cap.start() + code_start, cap.end() + code_start, "keyword"));
        }
        for cap in REGISTERS_REGEX.find_iter(code_part) {
            code_matches.push((cap.start() + code_start, cap.end() + code_start, "variable"));
        }
    }

    // Merge all matches and sort by position
    matches.extend(code_matches);
    matches.sort_by_key(|m| m.0);

    // Build the result, applying highlighting and avoiding overlaps
    let mut result = String::new();
    let mut last_end = 0;

    for (start, end, class) in matches {
        if start >= last_end {
            // Add un-highlighted text before this match
            result.push_str(&source[last_end..start]);
            // Add highlighted match
            result.push_str(&format!(
                "<span class=\"hljs-{}\">{}</span>",
                class,
                &source[start..end]
            ));
            last_end = end;
        }
    }

    // Add any remaining un-highlighted text
    result.push_str(&source[last_end..]);

    result
}

/// Link symbols in source code to their documentation anchors
/// Applies after syntax highlighting to preserve both features
pub(crate) fn link_symbols_in_source(source: &str, symbols: &[(String, String)]) -> String {
    // First apply syntax highlighting
    let highlighted = highlight_z80_syntax(source);

    // Pre-compile all regexes once (HUGE performance improvement!)
    let mut symbol_regexes: Vec<(regex::Regex, String, String)> = Vec::new();
    for (name, key) in symbols {
        let pattern = format!(r"\b{}\b", regex::escape(name));
        if let Ok(re) = regex::Regex::new(&pattern) {
            symbol_regexes.push((re, name.clone(), key.clone()));
        }
    }

    // Sort by length (longest first) to avoid partial matches
    symbol_regexes.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    // Then add symbol links - process text segments only
    let mut result = String::new();
    let mut chars = highlighted.chars().peekable();
    let mut in_tag = false;
    let mut in_anchor = false;
    let mut current_text = String::new();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            // Process accumulated text before tag
            if !in_tag && !in_anchor && !current_text.is_empty() {
                // Apply ALL symbol replacements to this text segment
                let mut text = current_text.clone();
                for (re, name, key) in &symbol_regexes {
                    let replacement =
                        format!("<a href=\"#{}\" class=\"symbol-link\">{}</a>", key, name);
                    text = re.replace_all(&text, replacement.as_str()).to_string();
                }
                result.push_str(&text);
            }
            else {
                result.push_str(&current_text);
            }
            current_text.clear();

            in_tag = true;
            result.push(ch);

            // Check if this is an anchor tag
            let ahead: String = chars.clone().take(2).collect();
            if ahead == "a " || ahead == "a>" {
                in_anchor = true;
            }
            else if ahead == "/a" {
                in_anchor = false;
            }
        }
        else if ch == '>' && in_tag {
            in_tag = false;
            result.push(ch);
        }
        else if in_tag {
            result.push(ch);
        }
        else {
            current_text.push(ch);
        }
    }

    // Process remaining text
    if !in_tag && !in_anchor && !current_text.is_empty() {
        let mut text = current_text;
        for (re, name, key) in &symbol_regexes {
            let replacement = format!("<a href=\"#{}\" class=\"symbol-link\">{}</a>", key, name);
            text = re.replace_all(&text, replacement.as_str()).to_string();
        }
        result.push_str(&text);
    }
    else {
        result.push_str(&current_text);
    }

    result
}

/// Replace a symbol with a link in plain text (not inside HTML tags)
pub(crate) fn replace_symbol_in_text(text: &str, symbol: &str, key: &str) -> String {
    let pattern = format!(r"\b{}\b", regex::escape(symbol));
    if let Ok(re) = regex::Regex::new(&pattern) {
        let replacement = format!("<a href=\"#{}\" class=\"symbol-link\">{}</a>", key, symbol);
        re.replace_all(text, replacement.as_str()).to_string()
    }
    else {
        text.to_string()
    }
}
