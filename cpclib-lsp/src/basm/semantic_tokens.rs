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

        // Best-effort AST parse to identify EQU / assign / macro / module
        // definition names. Still needed even though the AST walker below
        // also claims these names' own *definition* sites directly: the old
        // scanner (still the fallback for everything the walker doesn't
        // claim - e.g. a label/EQU/MACRO/MODULE name *referenced* inside a
        // DB/DW/ORG/DEFS-style data expression, which the walker
        // deliberately doesn't walk) still needs these sets to classify
        // such a reference correctly instead of falling through to a plain
        // TT_LABEL guess.
        let mut equ_names: HashSet<String> = HashSet::new();
        let mut assign_names: HashSet<String> = HashSet::new();
        let mut macro_names: HashSet<String> = HashSet::new();
        let mut module_names: HashSet<String> = HashSet::new();
        // AST-derived tokens (mnemonics, registers, labels, EQU/ASSIGN/
        // MACRO/MODULE names, numeric literals, ...) - real spans from the
        // parsed listing, wherever the AST has reliable span data. `Err`
        // here means the parse degraded enough not to trust (matches this
        // function's own pre-existing `if let Ok(...)` convention) -
        // `LocatedListing`'s `Deref`/`.iter()` genuinely panics ("No
        // listing available.") for the failure `ParseResult` variants, so
        // this gate is required, not just conservative. A broken document
        // falls back to 100% raw-text scanning below, exactly as before.
        let mut raw: Vec<RawSemanticToken> = Vec::new();
        // Lines inside a statically-known-inactive IF/ELSEIF/ELSE branch -
        // computed here (only a real AST gives us `Token::If` structure at
        // all), but applied to *every* token on those lines only at the very
        // end (see the final `dim_inactive_lines` call below), since plenty
        // of token kinds (e.g. `PRINT` statements - not walked by
        // `ast_semantic_tokens`, see its own doc comment) are only ever
        // claimed later, by the byte-level scanner further down this
        // function or the LOCOMOTIVE/bndbuild embedded-block pushes after
        // it - applying the dim modifier here would silently miss all of
        // those.
        let mut inactive_lines: HashSet<u32> = HashSet::new();
        if let Ok(listing) = self.parse_document(document) {
            for token in super::token::flatten_listing(listing.iter()) {
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
            raw = super::semantic_tokens_ast::ast_semantic_tokens(&listing);

            // Reuses the same dry-run `Env` hover already uses for value
            // substitution. Skipped entirely when the document has no `IF`
            // at all (the overwhelmingly common case) - a full dry-run
            // assembly pass is real work, not worth paying on every
            // semantic-tokens request (which fires on nearly every
            // keystroke) for a file that could never need it.
            if super::token::flatten_listing(listing.iter()).any(|t| t.is_if()) {
                let mut env = self.dry_run_env_cached(document, &listing);
                inactive_lines =
                    super::semantic_tokens_ast::inactive_if_branch_lines(&listing, &mut env);
            }
        }
        let claimed_by_line = super::semantic_tokens_ast::claimed_ranges_by_line(&raw);

        let text = document.text();

        // Carries an unterminated `/* ...` across the per-line loop below,
        // so a block comment spanning several physical lines greys out
        // every line it covers, not just the one it opens on.
        let mut in_block_comment = false;

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

        // Same idea for `#!bndbuild`-embedded rule blocks — their content
        // lines receive bndbuild tokens, not ASM tokens. The marker line
        // itself is deliberately *not* included: it stays visible to the
        // ordinary full-line-comment handling below and renders as plain
        // grey comment, which is correct (unlike LOCOMOTIVE's own directive
        // lines, a `#!bndbuild` marker isn't a basm-meaningful keyword).
        let bndbuild_blocks = self.embedded_bndbuild_blocks(document);
        let mut bndbuild_lines: HashSet<usize> = HashSet::new();
        for block in &bndbuild_blocks {
            let end = block.yaml_start_line + block.yaml_text.lines().count();
            bndbuild_lines.extend(block.yaml_start_line..end);
        }

        'line: for (line_idx, line) in text.lines().enumerate() {
            // LOCOMOTIVE/bndbuild block lines are tokenised separately below.
            if loco_lines.contains(&line_idx) || bndbuild_lines.contains(&line_idx) {
                continue;
            }
            let line_u = line_idx as u32;
            let bytes = line.as_bytes();
            let mut col: usize = 0;
            let line_claims = claimed_by_line.get(&line_u);

            if in_block_comment {
                match line.find("*/") {
                    Some(rel_end) => {
                        col = rel_end + 2;
                        raw.push(RawSemanticToken {
                            line: line_u,
                            col: 0,
                            len: col as u32,
                            token_type: TT_COMMENT,
                            modifiers: 0
                        });
                        in_block_comment = false;
                        // Falls through — code can follow `*/` on this line.
                    },
                    None => {
                        if !bytes.is_empty() {
                            raw.push(RawSemanticToken {
                                line: line_u,
                                col: 0,
                                len: bytes.len() as u32,
                                token_type: TT_COMMENT,
                                modifiers: 0
                            });
                        }
                        continue 'line;
                    }
                }
            }

            while col < bytes.len() {
                // A claimed range means the AST walker already pushed a
                // token covering these columns - skip straight past it
                // rather than re-classifying from raw bytes.
                if let Some(ranges) = line_claims
                    && let Some(&(_, end)) = ranges
                        .iter()
                        .find(|&&(s, e)| s <= col as u32 && (col as u32) < e)
                {
                    col = end as usize;
                    continue;
                }

                let c = bytes[col];

                // Whitespace — skip
                if c == b' ' || c == b'\t' {
                    col += 1;
                    continue;
                }

                // Comment: `;` through end of line
                if c == b';' {
                    raw.push(RawSemanticToken {
                        line: line_u,
                        col: col as u32,
                        len: (bytes.len() - col) as u32,
                        token_type: TT_COMMENT,
                        modifiers: 0
                    });
                    continue 'line;
                }

                // Comment: `//` through end of line
                if c == b'/' && col + 1 < bytes.len() && bytes[col + 1] == b'/' {
                    raw.push(RawSemanticToken {
                        line: line_u,
                        col: col as u32,
                        len: (bytes.len() - col) as u32,
                        token_type: TT_COMMENT,
                        modifiers: 0
                    });
                    continue 'line;
                }

                // Comment: `/* ... */`, possibly spanning several physical
                // lines (an unterminated one is picked back up at the top
                // of the next iterations via `in_block_comment`).
                if c == b'/' && col + 1 < bytes.len() && bytes[col + 1] == b'*' {
                    let start = col;
                    match line[col..].find("*/") {
                        Some(rel_end) => {
                            col += rel_end + 2;
                            raw.push(RawSemanticToken {
                                line: line_u,
                                col: start as u32,
                                len: (col - start) as u32,
                                token_type: TT_COMMENT,
                                modifiers: 0
                            });
                        },
                        None => {
                            raw.push(RawSemanticToken {
                                line: line_u,
                                col: start as u32,
                                len: (bytes.len() - start) as u32,
                                token_type: TT_COMMENT,
                                modifiers: 0
                            });
                            in_block_comment = true;
                            continue 'line;
                        }
                    }
                    continue;
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
                    raw.push(RawSemanticToken {
                        line: line_u,
                        col: start as u32,
                        len: (col - start) as u32,
                        token_type: TT_STRING,
                        modifiers: 0
                    });
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
                        raw.push(RawSemanticToken {
                            line: line_u,
                            col: start as u32,
                            len: (col - start) as u32,
                            token_type: TT_STRING,
                            modifiers: 0
                        });
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
                    raw.push(RawSemanticToken {
                        line: line_u,
                        col: start as u32,
                        len: (col - start) as u32,
                        token_type: TT_NUMBER,
                        modifiers: 0
                    });
                    continue;
                }

                // Hex literal: #hexdigits or &hexdigits — the same
                // operator-vs-literal ambiguity as `$` above (and resolved
                // the same way, by requiring a hex digit immediately after);
                // `cpclib_common::parse::scan_numeric_literals` (the real
                // lexer, used by `color.rs`'s own numeral scan) already
                // treats both as hex prefixes, e.g. every firmware `equ`
                // constant's `#BBC0`-style value.
                if (c == b'#' || c == b'&')
                    && col + 1 < bytes.len()
                    && bytes[col + 1].is_ascii_hexdigit()
                {
                    let start = col;
                    col += 1;
                    while col < bytes.len() && bytes[col].is_ascii_hexdigit() {
                        col += 1;
                    }
                    raw.push(RawSemanticToken {
                        line: line_u,
                        col: start as u32,
                        len: (col - start) as u32,
                        token_type: TT_NUMBER,
                        modifiers: 0
                    });
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
                        raw.push(RawSemanticToken {
                            line: line_u,
                            col: start as u32,
                            len: (col - start) as u32,
                            token_type: TT_NUMBER,
                            modifiers: 0
                        });
                    }
                    else {
                        raw.push(RawSemanticToken {
                            line: line_u,
                            col: start as u32,
                            len: 1,
                            token_type: TT_OPERATOR,
                            modifiers: 0
                        });
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
                    raw.push(RawSemanticToken {
                        line: line_u,
                        col: start as u32,
                        len: (col - start) as u32,
                        token_type: TT_NUMBER,
                        modifiers: 0
                    });
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
                    raw.push(RawSemanticToken {
                        line: line_u,
                        col: start as u32,
                        len: (col - start) as u32,
                        token_type: TT_PARAMETER,
                        modifiers: 0
                    });
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

                    raw.push(RawSemanticToken {
                        line: line_u,
                        col: start as u32,
                        len: word_len as u32,
                        token_type: tok_type,
                        modifiers
                    });

                    // Emit the ':' as an operator token
                    if followed_by_colon {
                        raw.push(RawSemanticToken {
                            line: line_u,
                            col: col as u32,
                            len: 1,
                            token_type: TT_OPERATOR,
                            modifiers: 0
                        });
                        col += 1;
                    }
                    continue;
                }

                // Single-character operators
                match c {
                    b'+' | b'-' | b'*' | b'/' | b'<' | b'>' | b'=' | b'!' | b'&' | b'|' | b'^'
                    | b'~' | b'#' | b'(' | b')' | b'[' | b']' | b',' | b':' => {
                        raw.push(RawSemanticToken {
                            line: line_u,
                            col: col as u32,
                            len: 1,
                            token_type: TT_OPERATOR,
                            modifiers: 0
                        });
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

        // Same idea for `#!bndbuild`-embedded rule blocks: their own lines
        // were skipped above (`bndbuild_lines`) and get bndbuild-style
        // tokens instead, appended here at absolute document coordinates.
        for block in &bndbuild_blocks {
            super::embedded_bndbuild::push_embedded_bndbuild_tokens(document, block, &mut raw);
        }

        if !inactive_lines.is_empty() {
            super::semantic_tokens_ast::dim_inactive_lines(&mut raw, &inactive_lines);
        }

        // Sort by (line, col) — LOCOMOTIVE/bndbuild tokens were appended out
        // of document order.
        raw.sort_unstable_by(|a, b| a.line.cmp(&b.line).then(a.col.cmp(&b.col)));

        // Convert raw absolute tokens to LSP delta-encoded SemanticToken
        let mut result = Vec::with_capacity(raw.len());
        let mut prev_line = 0u32;
        let mut prev_start = 0u32;
        for t in raw {
            let delta_line = t.line - prev_line;
            let delta_start = if delta_line == 0 {
                t.col - prev_start
            }
            else {
                t.col
            };
            result.push(SemanticToken {
                delta_line,
                delta_start,
                length: t.len,
                token_type: t.token_type,
                token_modifiers_bitset: t.modifiers
            });
            prev_line = t.line;
            prev_start = t.col;
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

    /// Decode delta-encoded `SemanticToken`s back to absolute
    /// `(line, col, length, token_type)`, for assertions that don't want to
    /// reason about deltas directly.
    fn decode(tokens: &[SemanticToken]) -> Vec<(u32, u32, u32, u32)> {
        let mut line = 0u32;
        let mut col = 0u32;
        let mut out = Vec::with_capacity(tokens.len());
        for t in tokens {
            if t.delta_line == 0 {
                col += t.delta_start;
            }
            else {
                line += t.delta_line;
                col = t.delta_start;
            }
            out.push((line, col, t.length, t.token_type));
        }
        out
    }

    #[test]
    fn slash_slash_comment_is_recognized() {
        let text = "ld a, 1 // BUFFER\n";
        let d = doc(text);
        let decoded = decode(&AssemblyAnalyzer::new().semantic_tokens(&d));
        let comment_start = text.find("//").unwrap() as u32;
        assert!(
            decoded.contains(&(0, comment_start, "// BUFFER".len() as u32, TT_COMMENT)),
            "{decoded:?}"
        );
        // "BUFFER" inside the comment must not also show up as its own
        // (mis-highlighted) label-reference token.
        assert!(
            !decoded
                .iter()
                .any(|&(l, c, _, ty)| l == 0 && c > comment_start && ty != TT_COMMENT),
            "{decoded:?}"
        );
    }

    #[test]
    fn inline_block_comment_closing_on_the_same_line_is_recognized() {
        let text = "ld a, /* x */ 1\n";
        let d = doc(text);
        let decoded = decode(&AssemblyAnalyzer::new().semantic_tokens(&d));
        let comment_start = text.find("/*").unwrap() as u32;
        assert!(
            decoded.contains(&(0, comment_start, "/* x */".len() as u32, TT_COMMENT)),
            "{decoded:?}"
        );
        // Code after the closed block comment, on the same line, is still
        // tokenized normally.
        let number_start = text.find('1').unwrap() as u32;
        assert!(
            decoded.contains(&(0, number_start, 1, TT_NUMBER)),
            "{decoded:?}"
        );
    }

    #[test]
    fn unterminated_block_comment_spans_multiple_lines() {
        // "start"/"done" (not "a"/"b"!) - single-letter label names would
        // collide with the Z80 A/B registers and be tokenized as TT_VARIABLE
        // instead, regardless of the `:` - irrelevant to what this test is
        // actually checking.
        let line0 = "start: /* note";
        let line1 = "still comment BUFFER";
        let line2 = "end */ done:";
        let text = format!("{line0}\n{line1}\n{line2}\n");
        let d = doc(&text);
        let decoded = decode(&AssemblyAnalyzer::new().semantic_tokens(&d));

        // Line 0: the label + colon before the comment opens are still real
        // tokens, then the comment covers the rest of the line.
        assert!(
            decoded.contains(&(0, 0, "start".len() as u32, TT_LABEL)),
            "{decoded:?}"
        );
        let comment_start0 = line0.find("/*").unwrap() as u32;
        assert!(
            decoded.contains(&(
                0,
                comment_start0,
                line0.len() as u32 - comment_start0,
                TT_COMMENT
            )),
            "{decoded:?}"
        );

        // Line 1 is entirely inside the still-open comment - nothing on it
        // (in particular "BUFFER") is tokenized as anything else.
        assert!(
            decoded
                .iter()
                .all(|&(l, _, _, ty)| l != 1 || ty == TT_COMMENT),
            "{decoded:?}"
        );
        assert!(
            decoded.contains(&(1, 0, line1.len() as u32, TT_COMMENT)),
            "{decoded:?}"
        );

        // Line 2: the comment closes mid-line ("end " is still comment
        // content), and the label after `*/` is tokenized normally again.
        let close_end = line2.find("*/").unwrap() as u32 + 2;
        assert!(
            decoded.contains(&(2, 0, close_end, TT_COMMENT)),
            "{decoded:?}"
        );
        let done_col = line2.find("done").unwrap() as u32;
        assert!(
            decoded.contains(&(2, done_col, "done".len() as u32, TT_LABEL)),
            "{decoded:?}"
        );
    }

    #[test]
    fn hash_and_ampersand_prefixed_hex_literals_are_recognized_as_numbers() {
        // Every firmware `equ` constant is written this way, e.g.
        // `GRA_MOVE_ABSOLUTE equ #BBC0` — before this fix, `#`/`&` only ever
        // matched the plain-operator fallback, and the hex digits after them
        // were left to be misread as a bare label reference.
        let text = "GRA_MOVE_ABSOLUTE equ #BBC0\nGRA_MOVE_ABSOLUTE2 equ &BBC0\n";
        let d = doc(text);
        let decoded = decode(&AssemblyAnalyzer::new().semantic_tokens(&d));

        let hash_col = text.find("#BBC0").unwrap() as u32;
        assert!(
            decoded.contains(&(0, hash_col, "#BBC0".len() as u32, TT_NUMBER)),
            "{decoded:?}"
        );
        assert!(
            !decoded
                .iter()
                .any(|&(l, c, _, ty)| l == 0 && c == hash_col && ty == TT_OPERATOR)
        );

        let amp_col = "GRA_MOVE_ABSOLUTE2 equ &BBC0".find('&').unwrap() as u32;
        assert!(
            decoded.contains(&(1, amp_col, "&BBC0".len() as u32, TT_NUMBER)),
            "{decoded:?}"
        );
    }

    #[test]
    fn a_bare_hash_or_ampersand_not_followed_by_a_hex_digit_is_still_an_operator() {
        let d = doc("ld a, hl & 5\n");
        let decoded = decode(&AssemblyAnalyzer::new().semantic_tokens(&d));
        let amp_col = "ld a, hl & 5".find('&').unwrap() as u32;
        assert!(
            decoded.contains(&(0, amp_col, 1, TT_OPERATOR)),
            "{decoded:?}"
        );
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

    /// Regression test for a real bug caught by the corpus differential
    /// pass (not by any hand-written test): `CP A, C`'s optional `A,`
    /// prefix is silently discarded by the parser before
    /// `mnemonic_arg1()`'s span even starts (`preceded(opt((parse_register_a,
    /// parse_comma)), ...)` in `cpclib-asm`), so bounding the mnemonic's
    /// claimed width by "distance to the first operand's span" swallowed
    /// "CP A, " into one bogus keyword token. `opcode_tokens` no longer
    /// does that at all - the mnemonic is always just its own leading word.
    #[test]
    fn cp_with_the_two_operand_implicit_accumulator_form_does_not_swallow_the_prefix() {
        let d = doc("\tcp a, c\n");
        let decoded = decode(&AssemblyAnalyzer::new().semantic_tokens(&d));
        assert!(
            decoded.contains(&(0, 1, 2, TT_KEYWORD)),
            "expected a 2-char \"cp\" keyword token, not one swallowing the operands: {decoded:?}"
        );
        assert!(
            decoded
                .iter()
                .any(|&(_, c, l, ty)| ty == TT_VARIABLE && c == 4 && l == 1),
            "expected the implicit \"a\" to still be tokenized as its own register: {decoded:?}"
        );
        assert!(
            decoded
                .iter()
                .any(|&(_, c, l, ty)| ty == TT_VARIABLE && c == 7 && l == 1),
            "expected \"c\" (the real compare target) as its own register: {decoded:?}"
        );
    }

    /// Regression test: `RET Z`'s `FlagTest` span must cover only the flag
    /// letter, not trailing whitespace after it - `parse_word` (used inside
    /// `parse_flag_test`) deliberately consumes trailing whitespace as part
    /// of its own contract, so an outer `.with_taken()` around it (the
    /// original implementation) produced a span like `"Z\t\t"` instead of
    /// `"Z"`. Fixed at the source in `cpclib-asm` (`parse_flag_test_located`
    /// captures `parse_word`'s own already-correctly-scoped span directly,
    /// rather than re-deriving a wider one) - caught by the corpus
    /// differential pass against a real cruncher asset, not a hand-written
    /// test.
    #[test]
    fn ret_z_flag_test_does_not_include_trailing_whitespace() {
        let d = doc("\tret\tz\t\t\n");
        let decoded = decode(&AssemblyAnalyzer::new().semantic_tokens(&d));
        assert!(
            decoded
                .iter()
                .any(|&(_, _, l, ty)| ty == TT_VARIABLE && l == 1),
            "expected a single-character \"z\" flag-test token, not padded with trailing \
             whitespace: {decoded:?}"
        );
    }

    /// Regression test: `(C)`/`(HL)`-shaped memory/port addressing modes
    /// (`LocatedDataAccess::PortC`/`MemoryRegister16`) deliberately have a
    /// span covering the *whole* parenthesized form in the real AST (unlike
    /// a plain register reference) - this is intentional parser design, not
    /// a bug, so the walker must NOT claim them as a single token (that
    /// would incorrectly recolor the parens themselves), leaving them
    /// entirely to the old scanner instead. Caught by the corpus
    /// differential pass (`OUT (C),C`), not a hand-written test.
    #[test]
    fn out_c_c_does_not_swallow_the_parens_into_the_register_token() {
        let d = doc("\tout (c), c\n");
        let decoded = decode(&AssemblyAnalyzer::new().semantic_tokens(&d));
        assert!(
            !decoded
                .iter()
                .any(|&(_, _, l, ty)| ty == TT_VARIABLE && l == 3),
            "no register token should span 3 chars here (that would mean the parens got \
             swallowed into it): {decoded:?}"
        );
    }

    /// Regression test: a symbol reference (`LocatedExpr::Label`) inside an
    /// EQU/ASSIGN value expression must stay unclaimed by the AST walker,
    /// not blanket-colored as TT_LABEL - it could just as easily be a
    /// reference to *another* EQU/ASSIGN/MACRO/MODULE name, which only the
    /// old scanner's still-alive `equ_names`/`assign_names`/etc. lookup
    /// sets can currently disambiguate correctly. Caught by the corpus
    /// differential pass (`CPT=CPT-1`, where the right-hand `CPT` reference
    /// was wrongly reclassified from the correct TT_ENUM_MEMBER down to a
    /// blanket TT_LABEL), not a hand-written test.
    #[test]
    fn a_label_expression_referencing_an_assign_name_is_still_classified_by_the_old_scanner() {
        let d = doc("CPT=3\nCPT=CPT-1\n");
        let decoded = decode(&AssemblyAnalyzer::new().semantic_tokens(&d));
        // The right-hand "CPT" on line 1 (0-indexed) starts at column 4.
        assert!(
            decoded.contains(&(1, 4, 3, TT_ENUM_MEMBER)),
            "expected the CPT reference to be classified as an ENUM_MEMBER (an ASSIGN name), \
             not a blanket LABEL: {decoded:?}"
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

    #[test]
    fn embedded_bndbuild_block_gets_bndbuild_tokens_not_asm_tokens() {
        let text = "; #!bndbuild\n; - tgt: test\n;   cmd: echo hi\nld a, 1\n";
        let d = doc(text);
        let decoded = decode(&AssemblyAnalyzer::new().semantic_tokens(&d));

        // Line 1 ("- tgt: test", content starting at column 2) must NOT get
        // a single whole-line TT_COMMENT the way an ordinary `;` comment
        // would - it should have real, separate bndbuild-scanner tokens.
        assert!(
            !decoded.iter().any(|&(l, c, len, ty)| {
                l == 1 && c == 0 && len == "; - tgt: test".len() as u32 && ty == TT_COMMENT
            }),
            "expected no whole-line asm comment token on the block's content line, got {decoded:?}"
        );
        assert!(!decoded.is_empty());

        // The marker line itself (line 0) still renders as a plain comment.
        assert!(
            decoded
                .iter()
                .any(|&(l, c, _, ty)| l == 0 && c == 0 && ty == TT_COMMENT),
            "{decoded:?}"
        );

        // An asm instruction outside the block still tokenizes normally.
        let ld_col = text.lines().nth(3).unwrap().find("ld").unwrap() as u32;
        assert!(
            decoded
                .iter()
                .any(|&(l, c, _, ty)| l == 3 && c == ld_col && ty == TT_KEYWORD),
            "{decoded:?}"
        );
    }

    #[test]
    fn an_ordinary_comment_line_outside_any_embedded_block_still_gets_one_token() {
        let text = "; just a comment\nld a, 1\n";
        let d = doc(text);
        let decoded = decode(&AssemblyAnalyzer::new().semantic_tokens(&d));
        assert!(
            decoded.contains(&(0, 0, "; just a comment".len() as u32, TT_COMMENT)),
            "{decoded:?}"
        );
    }

    /// Corpus sanity guard for the AST walker (`semantic_tokens_ast.rs`):
    /// runs `semantic_tokens()` over every real `.asm`/`.rasm` file findable
    /// in this workspace and asserts no token is absurd - specifically, no
    /// token's `length` exceeds the line it starts on (which would mean a
    /// span leaked across a newline, violating the LSP semantic-tokens
    /// protocol's inherently single-line encoding). This is exactly the
    /// shape of bug a hand-written unit test is unlikely to ever stumble
    /// into by chance: a real firmware asset (`deshrink.asm`) has a `$`
    /// (current-address) expression nested inside an `IFNDEF` block whose
    /// `LocatedExpr::Value` span - for reasons not chased down in
    /// `cpclib-asm` itself, since `$` is inherently unresolvable at parse
    /// time - covered almost the entire rest of the file; only running
    /// against real, large, varied source caught it (`span_token`'s
    /// newline/empty-span guard is what actually prevents it from being
    /// emitted). Originally written as a differential test against a frozen
    /// copy of the pre-rewrite scanner while migrating; kept on in this
    /// lighter form rather than maintaining a second, permanently-frozen
    /// implementation of the same feature indefinitely.
    #[test]
    fn semantic_tokens_never_produces_a_token_wider_than_its_own_line_across_the_real_corpus() {
        let roots = [
            "../cpclib-asm/assets",
            "../cpclib-rasm-basm-tests/tests/asm"
        ];
        let mut total_files = 0usize;

        for root in roots {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(root);
            if !root.exists() {
                continue;
            }
            for entry in walkdir::WalkDir::new(&root)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|ext| ext == "asm" || ext == "rasm")
                })
            {
                let Ok(text) = std::fs::read_to_string(entry.path())
                else {
                    continue;
                };
                total_files += 1;
                let d = doc(&text);
                let lines: Vec<&str> = text.lines().collect();
                // A fresh analyzer per file - `parse_document` caches by
                // (uri, version) only, and `doc()` always uses the same
                // synthetic URI/version, so reusing one analyzer across
                // files would silently keep serving a stale cached parse.
                let decoded = decode(&AssemblyAnalyzer::new().semantic_tokens(&d));
                for (line, col, len, token_type) in decoded {
                    let line_len = lines
                        .get(line as usize)
                        .map(|l| l.len() as u32)
                        .unwrap_or(0);
                    assert!(
                        col + len <= line_len,
                        "{}: token type {token_type} at line {line} col {col} len {len} \
                         extends past the line's own length {line_len}",
                        entry.path().display()
                    );
                }
            }
        }
        assert!(
            total_files > 50,
            "expected a real corpus, only found {total_files} files"
        );
    }
}

#[cfg(test)]
mod inactive_if_branch_dimming_tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///t.asm").unwrap(), text.to_string(), 1)
    }

    /// `(line, modifiers_bitset)` for every emitted token, decoded from
    /// delta-encoding.
    fn decode_lines_with_modifiers(tokens: &[SemanticToken]) -> Vec<(u32, u32)> {
        let mut line = 0u32;
        let mut out = Vec::with_capacity(tokens.len());
        for t in tokens {
            if t.delta_line != 0 {
                line += t.delta_line;
            }
            out.push((line, t.token_modifiers_bitset));
        }
        out
    }

    #[test]
    fn false_if_branch_is_dimmed_true_branch_is_not() {
        let text = "if 0\n    ld a, 1\nelse\n    ld a, 2\nendif\n";
        let d = doc(text);
        let decoded = decode_lines_with_modifiers(&AssemblyAnalyzer::new().semantic_tokens(&d));
        // Line 1 ("ld a, 1") is inside the untaken `if 0` branch - dimmed.
        assert!(
            decoded
                .iter()
                .any(|&(l, m)| l == 1 && m & MOD_INACTIVE != 0),
            "{decoded:?}"
        );
        // Line 3 ("ld a, 2") is the taken `else` branch - not dimmed.
        assert!(
            decoded
                .iter()
                .any(|&(l, m)| l == 3 && m & MOD_INACTIVE == 0),
            "{decoded:?}"
        );
    }

    #[test]
    fn true_if_branch_is_not_dimmed_else_branch_is() {
        let text = "if 1\n    ld a, 1\nelse\n    ld a, 2\nendif\n";
        let d = doc(text);
        let decoded = decode_lines_with_modifiers(&AssemblyAnalyzer::new().semantic_tokens(&d));
        assert!(
            decoded
                .iter()
                .any(|&(l, m)| l == 1 && m & MOD_INACTIVE == 0),
            "{decoded:?}"
        );
        assert!(
            decoded
                .iter()
                .any(|&(l, m)| l == 3 && m & MOD_INACTIVE != 0),
            "{decoded:?}"
        );
    }

    #[test]
    fn nested_if_is_evaluated_independently_inside_the_taken_outer_branch() {
        let text = "if 1\n    if 0\n        ld a, 1\n    endif\nendif\n";
        let d = doc(text);
        let decoded = decode_lines_with_modifiers(&AssemblyAnalyzer::new().semantic_tokens(&d));
        // Line 2 is inside the outer (taken) IF but the inner (untaken) IF.
        assert!(
            decoded
                .iter()
                .any(|&(l, m)| l == 2 && m & MOD_INACTIVE != 0),
            "{decoded:?}"
        );
    }

    #[test]
    fn unresolvable_condition_dims_nothing() {
        // A symbol that's never defined anywhere - genuinely unresolvable
        // even after every dry-run pass converges, so neither branch is
        // dimmed rather than guessing.
        let text = "if truly_undefined_symbol\n    ld a, 1\nelse\n    ld a, 2\nendif\n";
        let d = doc(text);
        let decoded = decode_lines_with_modifiers(&AssemblyAnalyzer::new().semantic_tokens(&d));
        assert!(
            decoded
                .iter()
                .all(|&(l, m)| !(l == 1 || l == 3) || m & MOD_INACTIVE == 0),
            "{decoded:?}"
        );
    }

    /// The other half of the user's report: `if false` must dim the *first*
    /// branch, exactly as `if 0` does. `true`/`false` are expression literals
    /// in basm, not symbols, so there is nothing here that needs a symbol
    /// table to decide.
    #[test]
    fn a_false_literal_dims_the_first_branch() {
        let text = "if false\n\tprint \"true\"\nelse\n\tprint \"false\"\nendif\n";
        let d = doc(text);
        let decoded = decode_lines_with_modifiers(&AssemblyAnalyzer::new().semantic_tokens(&d));
        assert!(!decoded.is_empty(), "expected some tokens for {text:?}");
        assert!(
            decoded
                .iter()
                .any(|&(l, m)| l == 1 && m & MOD_INACTIVE != 0),
            "the `if false` branch must be dimmed: {decoded:?}"
        );
        assert!(
            decoded
                .iter()
                .any(|&(l, m)| l == 3 && m & MOD_INACTIVE == 0),
            "the taken `else` branch must not be dimmed: {decoded:?}"
        );
    }

    /// Regression test for a real user report: `PRINT` isn't walked by the
    /// AST-driven scanner (`ast_semantic_tokens`), so it's entirely claimed
    /// by the byte-level fallback scanner further down `semantic_tokens` -
    /// the dimming pass used to run *before* that scanner added its own
    /// tokens, so a `PRINT` line inside an inactive branch never actually
    /// got dimmed. `dim_inactive_lines` must run only after every token
    /// source (AST walker, byte-level scanner, embedded LOCOMOTIVE/bndbuild
    /// blocks) has contributed to `raw`.
    #[test]
    fn print_statement_in_the_inactive_branch_is_dimmed() {
        let text = "if true\n\tprint \"true\"\nelse\n\tprint \"false\"\nendif\n";
        let d = doc(text);
        let decoded = decode_lines_with_modifiers(&AssemblyAnalyzer::new().semantic_tokens(&d));
        assert!(!decoded.is_empty(), "expected some tokens for {text:?}");
        // Line 1 ("true" branch) is taken - not dimmed.
        assert!(
            decoded
                .iter()
                .any(|&(l, m)| l == 1 && m & MOD_INACTIVE == 0),
            "{decoded:?}"
        );
        // Line 3 (the "false"/else branch) is not taken - must be dimmed.
        assert!(
            decoded
                .iter()
                .any(|&(l, m)| l == 3 && m & MOD_INACTIVE != 0),
            "{decoded:?}"
        );
    }
}

#[cfg(test)]
mod crlf_tests {
    use tower_lsp::lsp_types::Url;

    use super::*;
    use crate::common::document::Document;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///main.asm").unwrap(), text.to_string(), 1)
    }

    /// A file saved on Windows must highlight exactly as the same file saved on
    /// Unix. Line endings are not part of the program.
    #[test]
    fn crlf_and_lf_produce_the_same_tokens() {
        const SOURCE: &str = "STORE_SP1\n\tld sp, &1000\n\tei\n\tret\nTAB_COLOR1\tdefb 68,84,87\n";

        let analyzer = AssemblyAnalyzer::new();
        let lf = analyzer.semantic_tokens(&doc(SOURCE));
        let crlf = analyzer.semantic_tokens(&doc(&SOURCE.replace('\n', "\r\n")));

        assert_eq!(
            lf.len(),
            crlf.len(),
            "different token counts:\nlf   {lf:?}\ncrlf {crlf:?}"
        );
        for (index, (a, b)) in lf.iter().zip(crlf.iter()).enumerate() {
            assert_eq!(
                (a.delta_line, a.delta_start, a.length, a.token_type),
                (b.delta_line, b.delta_start, b.length, b.token_type),
                "token {index} moved between LF and CRLF"
            );
        }
    }
}
