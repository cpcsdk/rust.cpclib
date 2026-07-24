//! Semantic tokens (syntax highlighting) for assembly files, including
//! embedded Locomotive BASIC blocks.

use std::collections::HashSet;

use cpclib_tokens::ListingElement;
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use super::embedded_basic::{extract_locomotive_blocks, push_locomotive_basic_tokens};
use super::token::*;
use crate::common::document::Document;

impl AssemblyAnalyzer {
    /// Produce semantic tokens for the full document.
    pub fn semantic_tokens(&self, document: &Document) -> Vec<SemanticToken> {
        // Static lookup sets (built once on first call)
        let instructions = &*INSTRUCTION_SET;
        let directives = &*DIRECTIVE_SET;
        let registers = &*REGISTER_SET;

        // Best-effort AST parse to identify EQU / assign / macro / module definition names
        let mut equ_names: HashSet<String> = HashSet::new();
        let mut assign_names: HashSet<String> = HashSet::new();
        let mut macro_names: HashSet<String> = HashSet::new();
        let mut module_names: HashSet<String> = HashSet::new();
        if let Ok(listing) = self.parse_document(document) {
            for token in listing.iter() {
                if token.is_equ() {
                    equ_names.insert(token.equ_symbol().to_uppercase());
                }
                else if token.is_assign() {
                    assign_names.insert(token.assign_symbol().to_uppercase());
                }
                else if token.is_macro_definition() {
                    macro_names.insert(token.macro_definition_name().to_uppercase());
                }
                else if token.is_module() {
                    module_names.insert(token.module_name().to_uppercase());
                }
            }
        }

        // Raw tokens collected in document order: (line, col, len, type, modifiers)
        let mut raw: Vec<(u32, u32, u32, u32, u32)> = Vec::new();
        let text = document.text();

        // Detect LOCOMOTIVE blocks — their lines receive BASIC tokens, not ASM tokens.
        let loco_blocks = extract_locomotive_blocks(&text);
        let mut loco_lines: HashSet<usize> = HashSet::new();
        for block in &loco_blocks {
            loco_lines.insert(block.directive_line);
            if let Some(hl) = block.hide_lines_line {
                loco_lines.insert(hl);
            }
            for i in block.basic_range.clone() {
                loco_lines.insert(i);
            }
            loco_lines.insert(block.end_line);
        }

        'line: for (line_idx, line) in text.lines().enumerate() {
            // LOCOMOTIVE block lines are tokenised as BASIC below.
            if loco_lines.contains(&line_idx) {
                continue;
            }
            let line_u = line_idx as u32;
            let bytes = line.as_bytes();
            let mut col: usize = 0;

            while col < bytes.len() {
                let c = bytes[col];

                // Whitespace — skip
                if c == b' ' || c == b'\t' {
                    col += 1;
                    continue;
                }

                // Comment: `;` through end of line
                if c == b';' {
                    raw.push((
                        line_u,
                        col as u32,
                        (bytes.len() - col) as u32,
                        TT_COMMENT,
                        0
                    ));
                    continue 'line;
                }

                // Double-quoted string
                if c == b'"' {
                    let start = col;
                    col += 1;
                    while col < bytes.len() {
                        if bytes[col] == b'\\' && col + 1 < bytes.len() {
                            col += 2;
                            continue;
                        }
                        if bytes[col] == b'"' {
                            col += 1;
                            break;
                        }
                        col += 1;
                    }
                    raw.push((line_u, start as u32, (col - start) as u32, TT_STRING, 0));
                    continue;
                }

                // Single-quoted string — only when NOT preceded by a word char (avoids AF')
                if c == b'\'' {
                    let prev_word = col > 0 && {
                        let p = bytes[col - 1];
                        p.is_ascii_alphanumeric() || p == b'_'
                    };
                    if !prev_word {
                        let start = col;
                        col += 1;
                        while col < bytes.len() {
                            if bytes[col] == b'\\' && col + 1 < bytes.len() {
                                col += 2;
                                continue;
                            }
                            if bytes[col] == b'\'' {
                                col += 1;
                                break;
                            }
                            col += 1;
                        }
                        raw.push((line_u, start as u32, (col - start) as u32, TT_STRING, 0));
                    }
                    else {
                        col += 1; // stray ' (e.g. AF' already consumed by register scan)
                    }
                    continue;
                }

                // Hex literal: $hexdigits
                if c == b'$' && col + 1 < bytes.len() && bytes[col + 1].is_ascii_hexdigit() {
                    let start = col;
                    col += 1;
                    while col < bytes.len() && bytes[col].is_ascii_hexdigit() {
                        col += 1;
                    }
                    raw.push((line_u, start as u32, (col - start) as u32, TT_NUMBER, 0));
                    continue;
                }

                // Binary literal: %0101… (else treat % as operator)
                if c == b'%' {
                    let start = col;
                    col += 1;
                    if col < bytes.len() && (bytes[col] == b'0' || bytes[col] == b'1') {
                        while col < bytes.len() && (bytes[col] == b'0' || bytes[col] == b'1') {
                            col += 1;
                        }
                        raw.push((line_u, start as u32, (col - start) as u32, TT_NUMBER, 0));
                    }
                    else {
                        raw.push((line_u, start as u32, 1, TT_OPERATOR, 0));
                    }
                    continue;
                }

                // Numeric literal starting with a digit
                if c.is_ascii_digit() {
                    let start = col;
                    if c == b'0'
                        && col + 1 < bytes.len()
                        && (bytes[col + 1] == b'x' || bytes[col + 1] == b'X')
                    {
                        col += 2;
                        while col < bytes.len() && bytes[col].is_ascii_hexdigit() {
                            col += 1;
                        }
                    }
                    else if c == b'0'
                        && col + 1 < bytes.len()
                        && (bytes[col + 1] == b'b' || bytes[col + 1] == b'B')
                    {
                        col += 2;
                        while col < bytes.len() && (bytes[col] == b'0' || bytes[col] == b'1') {
                            col += 1;
                        }
                    }
                    else {
                        while col < bytes.len() && bytes[col].is_ascii_hexdigit() {
                            col += 1;
                        }
                        if col < bytes.len() && (bytes[col] == b'H' || bytes[col] == b'h') {
                            col += 1;
                        }
                    }
                    raw.push((line_u, start as u32, (col - start) as u32, TT_NUMBER, 0));
                    continue;
                }

                // Macro parameter: {identifier}
                if c == b'{' {
                    let start = col;
                    col += 1;
                    while col < bytes.len() && bytes[col] != b'}' {
                        col += 1;
                    }
                    if col < bytes.len() {
                        col += 1;
                    } // consume '}'
                    raw.push((line_u, start as u32, (col - start) as u32, TT_PARAMETER, 0));
                    continue;
                }

                // Identifier: letter / _ / @ / .
                if c.is_ascii_alphabetic() || c == b'_' || c == b'@' || c == b'.' {
                    let start = col;
                    while col < bytes.len() {
                        let ch = bytes[col];
                        if ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'@' || ch == b'.' {
                            col += 1;
                        }
                        else {
                            break;
                        }
                    }

                    // AF' special case: include trailing '
                    let word_no_prime = &line[start..col];
                    let word_upper_base = word_no_prime.to_uppercase();
                    let has_prime = col < bytes.len() && bytes[col] == b'\'';
                    let is_af_prime = has_prime && word_upper_base == "AF";
                    if is_af_prime {
                        col += 1;
                    }

                    let word_upper: String = if is_af_prime {
                        "AF'".to_string()
                    }
                    else {
                        word_upper_base
                    };
                    let word_len = col - start;

                    // Detect label definition sites:
                    //   - identifier at column 0 (no leading whitespace on this line)
                    //     AND not a known keyword/directive
                    //   - OR identifier immediately followed by ':'
                    let followed_by_colon = col < bytes.len() && bytes[col] == b':';
                    let at_col_zero = start == 0;
                    let is_label_def = followed_by_colon
                        || (at_col_zero
                            && !instructions.contains(word_upper.as_str())
                            && !directives.contains(word_upper.as_str()));

                    let (tok_type, modifiers) = if equ_names.contains(word_upper.as_str())
                        || assign_names.contains(word_upper.as_str())
                    {
                        (TT_ENUM_MEMBER, MOD_READONLY)
                    }
                    else if macro_names.contains(word_upper.as_str()) {
                        (TT_FUNCTION, if is_label_def { MOD_DECLARATION } else { 0 })
                    }
                    else if module_names.contains(word_upper.as_str()) {
                        (TT_NAMESPACE, if is_label_def { MOD_DECLARATION } else { 0 })
                    }
                    else if instructions.contains(word_upper.as_str()) {
                        (TT_KEYWORD, 0)
                    }
                    else if directives.contains(word_upper.as_str()) {
                        (TT_MACRO, 0)
                    }
                    else if registers.contains(word_upper.as_str()) {
                        (TT_VARIABLE, 0)
                    }
                    else if is_label_def {
                        (TT_LABEL, MOD_DECLARATION)
                    }
                    else {
                        (TT_LABEL, 0) // label reference
                    };

                    raw.push((line_u, start as u32, word_len as u32, tok_type, modifiers));

                    // Emit the ':' as an operator token
                    if followed_by_colon {
                        raw.push((line_u, col as u32, 1, TT_OPERATOR, 0));
                        col += 1;
                    }
                    continue;
                }

                // Single-character operators
                match c {
                    b'+' | b'-' | b'*' | b'/' | b'<' | b'>' | b'=' | b'!' | b'&' | b'|' | b'^'
                    | b'~' | b'#' | b'(' | b')' | b'[' | b']' | b',' | b':' => {
                        raw.push((line_u, col as u32, 1, TT_OPERATOR, 0));
                    },
                    _ => {}
                }
                col += 1;
            }
        }

        // Emit BASIC semantic tokens for LOCOMOTIVE blocks - `all_lines` is
        // only ever needed here, so it's only built when there's at least
        // one block to use it (the common case for `.asm` files has none).
        if !loco_blocks.is_empty() {
            let all_lines: Vec<&str> = text.lines().collect();
            for block in &loco_blocks {
                push_locomotive_basic_tokens(block, &all_lines, &mut raw);
            }
        }

        // Sort by (line, col) — LOCOMOTIVE tokens were appended out of document order.
        raw.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

        // Convert raw (line, col, len, type, mods) to LSP delta-encoded SemanticToken
        let mut result = Vec::with_capacity(raw.len());
        let mut prev_line = 0u32;
        let mut prev_start = 0u32;
        for (line, start, len, tok_type, modifiers) in raw {
            let delta_line = line - prev_line;
            let delta_start = if delta_line == 0 {
                start - prev_start
            }
            else {
                start
            };
            result.push(SemanticToken {
                delta_line,
                delta_start,
                length: len,
                token_type: tok_type,
                token_modifiers_bitset: modifiers
            });
            prev_line = line;
            prev_start = start;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///t.asm").unwrap(), text.to_string(), 1)
    }

    #[test]
    fn af_prime_is_tokenized_as_a_single_register_token() {
        let d = doc("ex af, af'\n");
        let tokens = AssemblyAnalyzer::new().semantic_tokens(&d);
        // "af'" spans 3 characters (a, f, ') as one token, not split apart.
        assert!(
            tokens
                .iter()
                .any(|t| t.token_type == TT_VARIABLE && t.length == 3),
            "{tokens:?}"
        );
    }

    #[test]
    fn locomotive_block_still_gets_basic_tokens_when_all_lines_is_lazily_built() {
        let text = "ORG 0x8000\nLOCOMOTIVE\n10 PRINT \"A\"\nENDLOCOMOTIVE\n";
        let d = doc(text);
        let tokens = AssemblyAnalyzer::new().semantic_tokens(&d);
        // The BASIC line number token (TT_NUMBER) on line 2 proves
        // `push_locomotive_basic_tokens` still ran correctly after
        // `all_lines` became conditionally built.
        let mut line = 0u32;
        let mut col = 0u32;
        let mut found_on_line_2 = false;
        for t in &tokens {
            if t.delta_line == 0 {
                col += t.delta_start;
            }
            else {
                line += t.delta_line;
                col = t.delta_start;
            }
            if line == 2 && t.token_type == TT_NUMBER {
                found_on_line_2 = true;
            }
        }
        assert!(found_on_line_2, "{tokens:?}");
    }

    #[test]
    fn a_document_with_no_locomotive_block_still_tokenizes_normally() {
        let d = doc("start:\n  ld a, 1\n  ret\n");
        let tokens = AssemblyAnalyzer::new().semantic_tokens(&d);
        assert!(!tokens.is_empty());
    }
}
