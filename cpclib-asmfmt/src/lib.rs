use std::path::{Path, PathBuf};

use cpclib_asm::{AssemblerError, ListingElement, LocatedListing, LocatedToken, MayHaveSpan, parse_z80_str};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CaseStyle {
    UpperCase,
    LowerCase,
    Untouched,
}

// Per-field default functions used by serde so that a partial config file
// (where only some keys are set) inherits the standard defaults for the rest.
fn default_indent_size() -> usize { 4 }
fn default_comment_column() -> usize { 30 }
fn default_mnemonic_case() -> CaseStyle { CaseStyle::UpperCase }
fn default_directive_case() -> CaseStyle { CaseStyle::UpperCase }
fn default_register_case() -> CaseStyle { CaseStyle::UpperCase }

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AsmFormatOptions {
    /// Number of spaces per indentation level (default: 4).
    #[serde(default = "default_indent_size")]
    pub indent_size: usize,
    /// Minimum column (0-indexed) at which trailing comments start.
    /// If the content is already past this column, at least 2 spaces are used (default: 30).
    #[serde(default = "default_comment_column")]
    pub comment_column: usize,
    /// Case transformation applied to Z80 mnemonic keywords (LD, PUSH, …) (default: UpperCase).
    #[serde(default = "default_mnemonic_case")]
    pub mnemonic_case: CaseStyle,
    /// Case transformation applied to the leading keyword of directives
    /// (ORG, EQU, DB, DW, INCLUDE, INCBIN, PRINT, ASSERT, …) (default: UpperCase).
    #[serde(default = "default_directive_case")]
    pub directive_case: CaseStyle,
    /// Case transformation applied to Z80 register names in operands (AF, BC, IX, …) (default: UpperCase).
    #[serde(default = "default_register_case")]
    pub register_case: CaseStyle,
}

impl Default for AsmFormatOptions {
    fn default() -> Self {
        Self {
            indent_size: default_indent_size(),
            comment_column: default_comment_column(),
            mnemonic_case: default_mnemonic_case(),
            directive_case: default_directive_case(),
            register_case: default_register_case(),
        }
    }
}

// ── Config file loading ──────────────────────────────────────────────────────

/// Name of the formatter config file searched in project directories.
pub const CONFIG_FILE_NAME: &str = "basm-fmt.toml";

/// Search for a `basm-fmt.toml` config file.
///
/// The search order is:
/// 1. Walk up from the current working directory toward the root.
/// 2. User-level config: `$XDG_CONFIG_HOME/basm-fmt/basm-fmt.toml`
///    (falls back to `$HOME/.config/…` on Unix and `%APPDATA%\…` on Windows).
///
/// Returns the path of the first file found, or `None`.
pub fn find_config_file() -> Option<PathBuf> {
    // Walk upward from cwd
    if let Ok(mut dir) = std::env::current_dir() {
        loop {
            let path = dir.join(CONFIG_FILE_NAME);
            if path.is_file() {
                return Some(path);
            }
            if !dir.pop() {
                break;
            }
        }
    }

    // User-level config directory
    let config_base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
        .or_else(|_| std::env::var("APPDATA").map(PathBuf::from))
        .ok()?;
    let path = config_base.join("basm-fmt").join(CONFIG_FILE_NAME);
    if path.is_file() { Some(path) } else { None }
}

/// Load `AsmFormatOptions` from a specific TOML file.
///
/// Fields not present in the file keep their default values, so a minimal
/// config like `mnemonic_case = "LowerCase"` is valid.
pub fn load_config_from(path: &Path) -> Result<AsmFormatOptions, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    toml::from_str(&content)
        .map_err(|e| format!("invalid config in {}: {e}", path.display()))
}

/// Load `AsmFormatOptions` by searching for a `basm-fmt.toml` config file.
///
/// Returns `AsmFormatOptions::default()` if no file is found or if parsing fails.
pub fn load_config() -> AsmFormatOptions {
    find_config_file()
        .and_then(|p| load_config_from(&p).ok())
        .unwrap_or_default()
}

struct Formatter<'src> {
    source_lines: Vec<&'src str>,
    indent_size: usize,
    comment_column: usize,
    mnemonic_case: CaseStyle,
    directive_case: CaseStyle,
    register_case: CaseStyle,
    current_line: usize,
    output: String,
}

impl<'src> Formatter<'src> {
    fn new(source: &'src str, opt: &AsmFormatOptions) -> Self {
        Self {
            source_lines: source.lines().collect(),
            indent_size: opt.indent_size,
            comment_column: opt.comment_column,
            mnemonic_case: opt.mnemonic_case,
            directive_case: opt.directive_case,
            register_case: opt.register_case,
            current_line: 0,
            output: String::new(),
        }
    }

    fn indent(&self, depth: usize) -> String {
        " ".repeat(depth * self.indent_size)
    }

    // Emit blank/comment-only source lines up to (not including) target_line
    fn emit_interstitial(&mut self, target_line: usize) {
        while self.current_line < target_line {
            let src = self.source_lines.get(self.current_line).copied().unwrap_or("");
            let trimmed = src.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with("//") {
                self.output.push_str(src);
                self.output.push('\n');
            }
            self.current_line += 1;
        }
    }

    fn apply_case(text: &str, case: CaseStyle) -> String {
        match case {
            CaseStyle::UpperCase => text.to_ascii_uppercase(),
            CaseStyle::LowerCase => text.to_ascii_lowercase(),
            CaseStyle::Untouched => text.to_string(),
        }
    }

    // Apply case to the first whitespace-delimited word, keep the rest unchanged.
    // Used for directives where only the keyword (ORG, EQU, …) is transformed.
    fn apply_case_to_first_word(content: &str, case: CaseStyle) -> String {
        if matches!(case, CaseStyle::Untouched) {
            return content.to_string();
        }
        let word_end = content.find(|c: char| c.is_ascii_whitespace()).unwrap_or(content.len());
        let keyword = Self::apply_case(&content[..word_end], case);
        let mut result = keyword;
        result.push_str(&content[word_end..]);
        result
    }

    // Format a mnemonic line: apply case to the mnemonic keyword AND to any Z80
    // register names found in the operands. Everything else (numeric literals,
    // labels, operators) is kept exactly as in the source, so `ld a, 1` gives
    // `LD A, 1` (not `LD A, 0x1`).
    fn apply_mnemonic_case(content: &str, mnemonic_case: CaseStyle, register_case: CaseStyle) -> String {
        let word_end = content.find(|c: char| c.is_ascii_whitespace()).unwrap_or(content.len());
        let mnemonic = Self::apply_case(&content[..word_end], mnemonic_case);
        let operands = if matches!(register_case, CaseStyle::Untouched) {
            content[word_end..].to_string()
        } else {
            Self::apply_register_case(&content[word_end..], register_case)
        };
        format!("{}{}", mnemonic, operands)
    }

    // Scan `operands` (everything after the mnemonic) and apply `case` to any
    // substring that is a Z80 register name at a word boundary. All other text
    // (numbers, labels, parentheses, operators) is passed through unchanged.
    fn apply_register_case(operands: &str, case: CaseStyle) -> String {
        // Ordered longest-first to ensure correct greedy matching (AF' before AF, IXH before IX).
        const REGISTERS: &[&str] = &[
            "AF'", "IXH", "IXL", "IYH", "IYL",
            "AF", "BC", "DE", "HL", "IX", "IY", "SP", "PC",
            "A", "B", "C", "D", "E", "H", "L", "F", "I", "R",
        ];
        // Valid identifier characters; a register must not be preceded or followed by one.
        let is_ident = |b: u8| b.is_ascii_alphanumeric() || b == b'_';

        let bytes = operands.as_bytes();
        let mut result = String::with_capacity(operands.len());
        let mut i = 0;

        while i < bytes.len() {
            let b = bytes[i];
            let prev_ok = i == 0 || !is_ident(bytes[i - 1]);

            if prev_ok && b.is_ascii_alphabetic() {
                let mut matched = false;
                for &reg in REGISTERS {
                    let reg_bytes = reg.as_bytes();
                    if bytes[i..].len() >= reg_bytes.len()
                        && bytes[i..i + reg_bytes.len()].eq_ignore_ascii_case(reg_bytes)
                    {
                        let after = i + reg_bytes.len();
                        let next_ok = after >= bytes.len() || !is_ident(bytes[after]);
                        if next_ok {
                            result.push_str(&Self::apply_case(&operands[i..after], case));
                            i = after;
                            matched = true;
                            break;
                        }
                    }
                }
                if !matched {
                    result.push(b as char);
                    i += 1;
                }
            } else {
                result.push(b as char);
                i += 1;
            }
        }
        result
    }

    // Split "content ; comment" → (content, Option<"; comment">)
    // Simplified: finds first ';' (does not handle strings)
    fn split_comment(line: &str) -> (&str, Option<&str>) {
        match line.find(';') {
            Some(pos) => (line[..pos].trim_end(), Some(line[pos..].trim_end())),
            None => (line, None),
        }
    }

    fn emit_line(&mut self, depth: usize, content: &str, comment: Option<&str>) {
        let indent = self.indent(depth);
        self.output.push_str(&indent);
        self.output.push_str(content);
        if let Some(c) = comment {
            let current_col = indent.len() + content.len();
            // Pad to comment_column, but always keep at least 2 spaces.
            let padding = self.comment_column.saturating_sub(current_col).max(2);
            self.output.push_str(&" ".repeat(padding));
            self.output.push_str(c);
        }
        self.output.push('\n');
    }

    // Emit source line at source_lines[line_0] with reformatted indentation
    fn emit_source_line_indented(&mut self, depth: usize, line_0: usize) {
        let src = self.source_lines.get(line_0).copied().unwrap_or("");
        let (content, comment) = Self::split_comment(src.trim());
        self.emit_line(depth, content, comment);
    }

    // Find first source line at or after current_line whose trimmed content
    // starts with `keyword` (case-insensitive)
    fn find_keyword_start(&self, keyword: &str) -> usize {
        let kw = keyword.to_ascii_uppercase();
        for i in self.current_line..self.source_lines.len() {
            let t = self.source_lines[i].trim().to_ascii_uppercase();
            if t == kw
                || t.starts_with(&format!("{kw} "))
                || t.starts_with(&format!("{kw}\t"))
                || t.starts_with(&format!("{kw};"))
                || t.starts_with(&format!("{kw}//"))
            {
                return i;
            }
        }
        self.current_line
    }

    // Emit the closer keyword line (found in source) at the given indentation depth
    fn emit_closer(&mut self, depth: usize, keyword: &str) {
        let line = self.find_keyword_start(keyword);
        self.emit_interstitial(line);
        self.emit_source_line_indented(depth, line);
        self.current_line = line + 1;
    }

    pub fn format_tokens(&mut self, tokens: &[LocatedToken], depth: usize) {
        for token in tokens {
            let (line_1, _) = token.span().relative_line_and_column();
            let line_0 = line_1.saturating_sub(1);
            self.emit_interstitial(line_0);
            self.format_token(token, depth, line_0);
        }
    }

    fn format_token(&mut self, token: &LocatedToken, depth: usize, line_0: usize) {
        // Warning tokens wrap another token with a parse warning message.
        // Delegate to the inner token so it gets canonical formatting.
        if token.is_warning() {
            self.format_token(token.warning_token(), depth, line_0);
            return;
        }

        // Comment tokens are either:
        //   - trailing after an instruction on the same source line → the instruction
        //     handler already emitted the comment via split_comment; skip to avoid duplication
        //   - standalone comment line → emit with original source indentation
        if token.is_comment() {
            if line_0 < self.current_line {
                return;
            }
            let src = self.source_lines.get(line_0).copied().unwrap_or("");
            self.output.push_str(src);
            self.output.push('\n');
            self.current_line = line_0 + 1;
            return;
        }

        if token.is_label() {
            let name = token.label_symbol();
            let (_, comment) = Self::split_comment(
                self.source_lines.get(line_0).copied().unwrap_or("").trim()
            );
            self.emit_line(0, &format!("{name}:"), comment);
            self.current_line = line_0 + 1;
        } else if token.is_if() {
            self.format_if(token, depth, line_0);
        } else if token.is_repeat() {
            self.format_block(token.repeat_listing(), depth, line_0, "ENDREPEAT");
        } else if token.is_while() {
            self.format_block(token.while_listing(), depth, line_0, "WEND");
        } else if token.is_for() {
            self.format_block(token.for_listing(), depth, line_0, "ENDFOR");
        } else if token.is_module() {
            self.format_block(token.module_listing(), depth, line_0, "ENDMODULE");
        } else if token.is_confined() {
            self.format_block(token.confined_listing(), depth, line_0, "ENDCONFINED");
        } else if token.is_repeat_until() {
            self.format_repeat_until(token, depth, line_0);
        } else if token.is_iterate() {
            self.format_block(token.iterate_listing(), depth, line_0, "ENDITERATE");
        } else if token.is_rorg() {
            self.format_block(token.rorg_listing(), depth, line_0, "REND");
        } else if token.is_crunched_section() {
            self.format_block(token.crunched_section_listing(), depth, line_0, "LZCLOSE");
        } else if token.is_function_definition() {
            self.format_block(token.function_definition_inner(), depth, line_0, "ENDFUNCTION");
        } else if token.is_switch() {
            self.format_switch(token, depth, line_0);
        } else if token.is_macro_definition() {
            self.format_macro_def(token, depth, line_0);
        } else {
            let src_line = self.source_lines.get(line_0).copied().unwrap_or("");
            let (content, comment) = Self::split_comment(src_line.trim());

            if token.mnemonic().is_some() {
                // Z80 opcode: apply case to the mnemonic keyword and to any register
                // names in the operands; leave numeric literals as source text so
                // `ld a, 1` stays `LD A, 1`, not `LD A, 0x1`.
                self.emit_line(depth, &Self::apply_mnemonic_case(content, self.mnemonic_case, self.register_case), comment);
            } else if token.is_call_macro_or_build_struct() {
                // Macro calls: preserve source line (macro names are user-defined).
                self.emit_line(depth, content, comment);
            } else {
                // All other directives (ORG, EQU, DB, DW, DEFB, DEFW, DEFS, INCLUDE,
                // INCBIN, PRINT, ASSERT, SET, SAVE, RUN, …): apply case to the leading keyword.
                self.emit_line(depth, &Self::apply_case_to_first_word(content, self.directive_case), comment);
            }
            self.current_line = line_0 + 1;
        }
    }

    // Generic block: header from source, inner listing at depth+1, canonical closer
    fn format_block(
        &mut self,
        inner: &[LocatedToken],
        depth: usize,
        line_0: usize,
        closer: &str
    ) {
        self.emit_source_line_indented(depth, line_0);
        self.current_line = line_0 + 1;
        self.format_tokens(inner, depth + 1);
        self.emit_closer(depth, closer);
    }

    fn format_if(&mut self, token: &LocatedToken, depth: usize, line_0: usize) {
        let nb_tests = token.if_nb_tests();

        // First test — header is the IF line in source
        self.emit_source_line_indented(depth, line_0);
        self.current_line = line_0 + 1;
        let (_, inner_0) = token.if_test(0);
        self.format_tokens(inner_0, depth + 1);

        // Remaining tests (ELSE IF / ELSEIF / IFNOT …)
        for i in 1..nb_tests {
            let header = self.find_keyword_start("ELSE");
            self.emit_interstitial(header);
            self.emit_source_line_indented(depth, header);
            self.current_line = header + 1;
            let (_, inner_i) = token.if_test(i);
            self.format_tokens(inner_i, depth + 1);
        }

        // Optional ELSE (default branch)
        if let Some(else_inner) = token.if_else() {
            let else_line = self.find_keyword_start("ELSE");
            self.emit_interstitial(else_line);
            self.emit_source_line_indented(depth, else_line);
            self.current_line = else_line + 1;
            self.format_tokens(else_inner, depth + 1);
        }

        self.emit_closer(depth, "ENDIF");
    }

    fn format_repeat_until(&mut self, token: &LocatedToken, depth: usize, line_0: usize) {
        self.emit_source_line_indented(depth, line_0);
        self.current_line = line_0 + 1;
        self.format_tokens(token.repeat_until_listing(), depth + 1);
        self.emit_closer(depth, "UNTIL");
    }

    fn format_macro_def(&mut self, token: &LocatedToken, depth: usize, line_0: usize) {
        self.emit_source_line_indented(depth, line_0);
        self.current_line = line_0 + 1;
        let body = token.macro_definition_code();
        for body_line in body.lines() {
            self.output.push_str(body_line);
            self.output.push('\n');
            self.current_line += 1;
        }
        self.emit_closer(depth, "ENDM");
    }

    fn format_switch(&mut self, token: &LocatedToken, depth: usize, line_0: usize) {
        self.emit_source_line_indented(depth, line_0);
        self.current_line = line_0 + 1;

        for (_, case_inner, _) in token.switch_cases() {
            let case_line = self.find_keyword_start("CASE");
            self.emit_interstitial(case_line);
            self.emit_source_line_indented(depth, case_line);
            self.current_line = case_line + 1;
            self.format_tokens(case_inner, depth + 1);
        }

        if let Some(default_inner) = token.switch_default() {
            let default_line = self.find_keyword_start("DEFAULT");
            self.emit_interstitial(default_line);
            self.emit_source_line_indented(depth, default_line);
            self.current_line = default_line + 1;
            self.format_tokens(default_inner, depth + 1);
        }

        self.emit_closer(depth, "ENDSWITCH");
    }
}

/// Format a located listing using the original source for comment and blank-line preservation.
///
/// `depth` is the initial indentation depth (0 for top-level code).
pub fn format_listing(listing: &LocatedListing, source: &str, depth: usize, opt: &AsmFormatOptions) -> String {
    let mut fmt = Formatter::new(source, opt);
    if let Some(first) = listing.iter().next() {
        let (line_1, _) = first.span().relative_line_and_column();
        fmt.current_line = line_1.saturating_sub(1);
    }
    fmt.format_tokens(listing, depth);
    fmt.output
}

/// Parse and format Z80 assembly source code.
///
/// Top-level instructions are indented one level. Use `format_listing` with `depth = 0`
/// for no base indentation.
pub fn format(asm: &str, opt: &AsmFormatOptions) -> Result<String, AssemblerError> {
    let listing = parse_z80_str(asm)?;
    Ok(format_listing(&listing, asm, 1, opt))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(src: &str) -> String {
        format(src, &AsmFormatOptions::default()).expect("parse failed")
    }

    #[test]
    fn test_simple_instructions() {
        let out = fmt("push af\n pop bc\n push hl");
        assert!(out.contains("    PUSH AF\n"), "got: {out:?}");
        assert!(out.contains("    POP BC\n"), "got: {out:?}");
    }

    #[test]
    fn test_label_at_col0() {
        let out = fmt("  myloop:\n    push af");
        assert!(out.starts_with("myloop:\n"), "got: {out:?}");
        assert!(out.contains("    PUSH AF"), "got: {out:?}");
    }

    #[test]
    fn test_repeat_block() {
        let out = fmt("repeat 10\n push af\n endrepeat");
        assert!(out.contains("        PUSH AF\n"), "got: {out:?}");
        assert!(out.contains("repeat 10"), "got: {out:?}");
        assert!(out.contains("endrepeat"), "got: {out:?}");
    }

    #[test]
    fn test_blank_lines_preserved() {
        let out = fmt("push af\n\npop bc");
        assert!(out.contains("\n\n"), "blank line not preserved: {out:?}");
    }

    #[test]
    fn test_comment_preserved() {
        let out = fmt("push af ; save af");
        assert!(out.contains("; save af"), "comment not preserved: {out:?}");
    }

    #[test]
    fn test_comment_column() {
        // With default comment_column=30, the ';' should start at column 30.
        // depth=1 (4 spaces) + "PUSH AF" (7 chars) = col 11 → pad to 30
        let out = fmt("push af ; save af");
        let line = out.lines().next().unwrap();
        let col = line.find(';').expect("no comment found");
        assert_eq!(col, 30, "comment not at column 30: {line:?}");
    }

    #[test]
    fn test_comment_column_long_content() {
        // When content is wider than comment_column, keep at least 2 spaces.
        let long = "ld hl, (some_very_long_symbol_name_that_is_long)";
        let src = format!("{long} ; cmnt");
        let out = fmt(&src);
        let line = out.lines().next().unwrap();
        let col = line.find(';').expect("no comment");
        let content_end = 4 + long.len(); // depth=1 indent + opcode
        assert!(col >= content_end + 2, "less than 2 spaces before comment: {line:?}");
    }

    #[test]
    fn test_macro_call_no_panic() {
        let out = fmt("MY_MACRO arg1, arg2\npush af");
        assert!(out.contains("MY_MACRO arg1, arg2"), "macro call lost: {out:?}");
        assert!(out.contains("PUSH AF"), "opcode after macro call lost: {out:?}");
    }

    #[test]
    fn test_equ_preserved() {
        let out = fmt("FOO EQU 42\npush af");
        assert!(out.contains("FOO"), "EQU lost: {out:?}");
        assert!(out.contains("42"), "EQU value lost: {out:?}");
    }

    #[test]
    fn test_case_lowercase() {
        let opt = AsmFormatOptions {
            mnemonic_case: CaseStyle::LowerCase,
            directive_case: CaseStyle::LowerCase,
            register_case: CaseStyle::LowerCase,
            ..AsmFormatOptions::default()
        };
        let out = format("PUSH AF\nORG 0x40\n", &opt).unwrap();
        assert!(out.contains("push af"), "mnemonic not lowercased: {out:?}");
        assert!(out.contains("org 0x40"), "directive not lowercased: {out:?}");
    }

    #[test]
    fn test_case_untouched() {
        let opt = AsmFormatOptions {
            mnemonic_case: CaseStyle::Untouched,
            directive_case: CaseStyle::Untouched,
            register_case: CaseStyle::Untouched,
            ..AsmFormatOptions::default()
        };
        let out = format("Push Af\nOrg 0x40\n", &opt).unwrap();
        assert!(out.contains("Push Af"), "mnemonic case changed: {out:?}");
        assert!(out.contains("Org 0x40"), "directive case changed: {out:?}");
    }

    #[test]
    fn test_register_case_independent() {
        // mnemonic uppercase, registers lowercase
        let opt = AsmFormatOptions {
            mnemonic_case: CaseStyle::UpperCase,
            register_case: CaseStyle::LowerCase,
            ..AsmFormatOptions::default()
        };
        let out = format("PUSH AF\nLD HL, BC\n", &opt).unwrap();
        assert!(out.contains("PUSH af\n"), "register not lowercased: {out:?}");
        assert!(out.contains("LD hl, bc\n"), "registers not lowercased: {out:?}");
    }

    #[test]
    fn test_literal_not_hex_encoded() {
        // Numeric literals must keep their source representation, not be re-encoded.
        let out = fmt("ld a, 1\nld hl, 100\nld de, 0x40\nadd a, %00001111");
        println!("literal test:\n{out}");
        assert!(out.contains("LD A, 1\n"),   "literal 1 re-encoded: {out:?}");
        assert!(out.contains("LD HL, 100\n"), "literal 100 re-encoded: {out:?}");
        assert!(out.contains("LD DE, 0x40\n"), "literal 0x40 re-encoded: {out:?}");
        assert!(out.contains("ADD A, %00001111\n"), "literal %… re-encoded: {out:?}");
    }

    #[test]
    fn test_registers_uppercased() {
        // Registers in operands must follow mnemonic_case.
        let out = fmt("ld hl, (ix+2)\npush af\nex af, af'");
        assert!(out.contains("LD HL, (IX+2)\n"),  "registers not uppercased: {out:?}");
        assert!(out.contains("PUSH AF\n"),          "AF not uppercased: {out:?}");
        assert!(out.contains("EX AF, AF'\n"),       "AF' not uppercased: {out:?}");
    }

    #[test]
    fn test_label_not_treated_as_register() {
        // Test apply_register_case directly: labels whose prefix matches a register
        // name must NOT be uppercased if they are part of a longer identifier.
        // "bc_label": "bc" followed by '_' → not register BC
        let r1 = Formatter::apply_register_case("hl, bc_label", CaseStyle::UpperCase);
        assert_eq!(r1, "HL, bc_label", "bc_label was altered: {r1:?}");
        // "hlabel": "h" followed by 'l' (alphanumeric) → not register H
        let r2 = Formatter::apply_register_case("a, hlabel", CaseStyle::UpperCase);
        assert_eq!(r2, "A, hlabel", "hlabel was altered: {r2:?}");
        // Bare register IS transformed
        let r3 = Formatter::apply_register_case("hl, bc", CaseStyle::UpperCase);
        assert_eq!(r3, "HL, BC", "registers not uppercased: {r3:?}");
    }

    #[test]
    fn test_trailing_comment_no_duplicate() {
        let src = "    org 0x40  ; comment 1\n    push af  ; comment 2\n    pop af\n";
        let out = fmt(src);
        assert_eq!(out.matches("comment 1").count(), 1, "comment 1 duplicated: {out:?}");
        assert_eq!(out.matches("comment 2").count(), 1, "comment 2 duplicated: {out:?}");
    }

    #[test]
    fn test_user_sample() {
        let src = "\tei\n\txor b\n\n\txor b\n\n\tld de, ix\n\n\torg 40\n    pop hl : push af: pop af\n\tpop AF\n";
        let result = format(src, &AsmFormatOptions::default());
        match result {
            Ok(out) => {
                assert!(out.contains("    EI\n"), "EI missing: {out:?}");
                assert!(out.contains("    ORG 40\n"), "ORG missing: {out:?}");
            }
            Err(e) => panic!("format failed: {e}"),
        }
    }
}
