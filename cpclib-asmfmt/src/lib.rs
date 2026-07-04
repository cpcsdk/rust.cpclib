use std::path::{Path, PathBuf};

use cpclib_asm::{AssemblerError, ListingElement, LocatedListing, LocatedToken, MayHaveSpan, parse_z80_str};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CaseStyle {
    UpperCase,  // Set in uppercase
    LowerCase,  // Set in lowercase
    Untouched,  // Preserve the original case of the source text
}

/// Controls how spaces are written around `:` instruction separators when
/// `one_instruction_per_line = false` and a source line has multiple instructions.
/// Has no effect when `one_instruction_per_line = true` (separators become newlines).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SpaceAroundColumn {
    None,      // a:b
    Before,    // a :b
    After,     // a: b
    Both,      // a : b
    Untouched, // preserve original spacing
}

fn default_indent_size() -> usize { 4 }
fn default_comment_column() -> usize { 30 }
fn default_mnemonic_case() -> CaseStyle { CaseStyle::UpperCase }
fn default_directive_case() -> CaseStyle { CaseStyle::UpperCase }
fn default_register_case() -> CaseStyle { CaseStyle::UpperCase }
fn default_one_instruction_per_line() -> bool { true }
fn default_space_around_column() -> SpaceAroundColumn { SpaceAroundColumn::Untouched }

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, bon::Builder)]
pub struct AsmFormatOptions {
    /// Number of spaces per indentation level (default: 4).
    #[serde(default = "default_indent_size")]
    #[builder(default = default_indent_size())]
    pub indent_size: usize,
    /// Minimum column (0-indexed) at which trailing comments start (default: 30).
    #[serde(default = "default_comment_column")]
    #[builder(default = default_comment_column())]
    pub comment_column: usize,
    /// Case transformation applied to Z80 mnemonic keywords (LD, PUSH, …) (default: UpperCase).
    #[serde(default = "default_mnemonic_case")]
    #[builder(default = default_mnemonic_case())]
    pub mnemonic_case: CaseStyle,
    /// Case transformation applied to directive keywords
    /// (ORG, EQU, REPEAT, ENDREPEAT, …) (default: UpperCase).
    #[serde(default = "default_directive_case")]
    #[builder(default = default_directive_case())]
    pub directive_case: CaseStyle,
    /// Case transformation applied to Z80 register names in operands (default: UpperCase).
    #[serde(default = "default_register_case")]
    #[builder(default = default_register_case())]
    pub register_case: CaseStyle,
    /// When true, multiple instructions on the same source line (separated by `:`) are
    /// placed on individual lines. When false the original multi-instruction line is kept
    /// verbatim (default: true).
    #[serde(default = "default_one_instruction_per_line")]
    #[builder(default = default_one_instruction_per_line())]
    pub one_instruction_per_line: bool,
    /// How spaces are written around `:` instruction separators when
    /// `one_instruction_per_line = false` (default: Untouched).
    #[serde(default = "default_space_around_column")]
    #[builder(default = default_space_around_column())]
    pub space_around_column: SpaceAroundColumn,
}

impl Default for AsmFormatOptions {
    fn default() -> Self {
        Self {
            indent_size: default_indent_size(),
            comment_column: default_comment_column(),
            mnemonic_case: default_mnemonic_case(),
            directive_case: default_directive_case(),
            register_case: default_register_case(),
            one_instruction_per_line: default_one_instruction_per_line(),
            space_around_column: default_space_around_column(),
        }
    }
}

// ── Config file loading ──────────────────────────────────────────────────────

pub const CONFIG_FILE_NAME: &str = "basm-fmt.toml";

pub fn find_config_file() -> Option<PathBuf> {
    if let Ok(mut dir) = std::env::current_dir() {
        loop {
            let path = dir.join(CONFIG_FILE_NAME);
            if path.is_file() { return Some(path); }
            if !dir.pop() { break; }
        }
    }
    let config_base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
        .or_else(|_| std::env::var("APPDATA").map(PathBuf::from))
        .ok()?;
    let path = config_base.join("basm-fmt").join(CONFIG_FILE_NAME);
    if path.is_file() { Some(path) } else { None }
}

pub fn load_config_from(path: &Path) -> Result<AsmFormatOptions, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    toml::from_str(&content)
        .map_err(|e| format!("invalid config in {}: {e}", path.display()))
}

pub fn load_config() -> AsmFormatOptions {
    find_config_file()
        .and_then(|p| load_config_from(&p).ok())
        .unwrap_or_default()
}

// ── Formatter ────────────────────────────────────────────────────────────────

struct Formatter<'src> {
    source_lines: Vec<&'src str>,
    indent_size: usize,
    comment_column: usize,
    mnemonic_case: CaseStyle,
    directive_case: CaseStyle,
    register_case: CaseStyle,
    one_instruction_per_line: bool,
    space_around_column: SpaceAroundColumn,
    current_line: usize,
    output: String,
    // Per-source-line segment cache (`:` splitting for one_instruction_per_line)
    seg_line: usize,             // which source line is currently cached (usize::MAX = none)
    seg_idx: usize,              // next segment to consume
    seg_items: Vec<String>,      // content segments (before trailing `;` comment)
    seg_trailing: Option<String>, // the trailing `;` comment of the whole source line
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
            one_instruction_per_line: opt.one_instruction_per_line,
            space_around_column: opt.space_around_column,
            current_line: 0,
            output: String::new(),
            seg_line: usize::MAX,
            seg_idx: 0,
            seg_items: Vec::new(),
            seg_trailing: None,
        }
    }

    fn indent(&self, depth: usize) -> String {
        " ".repeat(depth * self.indent_size)
    }

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

    // Apply case to the first whitespace-delimited word only; rest is preserved verbatim.
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

    // Apply case to the second whitespace-delimited word (e.g., the EQU keyword in
    // "symbol EQU value"), leaving the first word (user symbol name) unchanged.
    fn apply_case_to_second_word(content: &str, case: CaseStyle) -> String {
        if matches!(case, CaseStyle::Untouched) {
            return content.to_string();
        }
        let bytes = content.as_bytes();
        // Find end of first word
        let first_end = bytes.iter().position(|b| b.is_ascii_whitespace()).unwrap_or(bytes.len());
        // Find start of second word (skip whitespace)
        let second_start = bytes[first_end..]
            .iter()
            .position(|b| !b.is_ascii_whitespace())
            .map(|p| first_end + p)
            .unwrap_or(bytes.len());
        // Find end of second word
        let second_end = bytes[second_start..]
            .iter()
            .position(|b| b.is_ascii_whitespace())
            .map(|p| second_start + p)
            .unwrap_or(bytes.len());
        let mut result = content[..first_end].to_string();
        result.push_str(&content[first_end..second_start]);
        result.push_str(&Self::apply_case(&content[second_start..second_end], case));
        result.push_str(&content[second_end..]);
        result
    }

    // Apply case to a mnemonic line: transforms the mnemonic keyword and register names
    // in operands but leaves numeric literals / labels / expressions unchanged.
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

    fn apply_register_case(operands: &str, case: CaseStyle) -> String {
        const REGISTERS: &[&str] = &[
            "AF'", "IXH", "IXL", "IYH", "IYL",
            "AF", "BC", "DE", "HL", "IX", "IY", "SP", "PC",
            "A", "B", "C", "D", "E", "H", "L", "F", "I", "R",
        ];
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
                if !matched { result.push(b as char); i += 1; }
            } else {
                result.push(b as char); i += 1;
            }
        }
        result
    }

    // Split "content ; comment" → (content.trim_end(), Option<"; comment">)
    fn split_comment(line: &str) -> (&str, Option<&str>) {
        match line.find(';') {
            Some(pos) => (line[..pos].trim_end(), Some(line[pos..].trim_end())),
            None => (line, None),
        }
    }

    // Split `content` (already stripped of trailing comment) into `:` separated instruction
    // segments. Colons inside parentheses or double-quoted strings are not split points.
    // A `:` is only an instruction separator when BOTH the preceding and following bytes are
    // ASCII whitespace (or boundary): this avoids splitting label prefixes (`other: equ 5`),
    // global-scope paths (`jp ::label1`), and bare label colons (`myloop: ld a,0`).
    // Empty segments (e.g. from a trailing `:` on a label) are discarded.
    fn split_instructions(content: &str) -> Vec<&str> {
        let mut result = Vec::new();
        let mut depth = 0i32;
        let mut in_string = false;
        // Track unmatched `?` so we can suppress splitting at the `:` of a ternary
        // expression (`cond ? then : else`).
        let mut ternary_depth = 0u32;
        let mut start = 0;
        let bytes = content.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if in_string {
                if b == b'"' { in_string = false; }
            } else {
                match b {
                    b'"' => in_string = true,
                    b'(' | b'[' => depth += 1,
                    b')' | b']' => { depth -= 1; if depth < 0 { depth = 0; } }
                    b'?' if depth == 0 => ternary_depth += 1,
                    b':' if depth == 0 => {
                        if ternary_depth > 0 {
                            // This `:` closes a ternary — not an instruction separator.
                            ternary_depth -= 1;
                        } else {
                            // Only split when prev byte is whitespace (or at start) AND
                            // next byte is whitespace or end-of-content.
                            let prev_ws = i == 0 || bytes[i - 1].is_ascii_whitespace();
                            let next_ws = i + 1 >= bytes.len() || bytes[i + 1].is_ascii_whitespace();
                            if prev_ws && next_ws {
                                let seg = content[start..i].trim();
                                if !seg.is_empty() { result.push(seg); }
                                start = i + 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
            i += 1;
        }
        let last = content[start..].trim();
        if !last.is_empty() { result.push(last); }
        result
    }

    // Reformat `:` instruction separators in `content` according to `spacing`.
    // Only separators that are already surrounded by whitespace (` : `) are
    // recognised — label colons and other `:` uses are left untouched.
    // When `spacing` is `Untouched` the string is returned as-is.
    fn normalize_colon_spacing(content: &str, spacing: SpaceAroundColumn) -> String {
        if matches!(spacing, SpaceAroundColumn::Untouched) {
            return content.to_string();
        }
        let segs = Self::split_instructions(content);
        if segs.len() <= 1 {
            return content.to_string();
        }
        let sep = match spacing {
            SpaceAroundColumn::None => ":",
            SpaceAroundColumn::Before => " :",
            SpaceAroundColumn::After => ": ",
            SpaceAroundColumn::Both => " : ",
            SpaceAroundColumn::Untouched => unreachable!(),
        };
        segs.join(sep)
    }

    // Initialise the per-line segment cache when we move to a new source line.
    // Must be called at the top of format_token (after warning/comment guards).
    fn init_segments_for_line(&mut self, line_0: usize) {
        if self.seg_line == line_0 { return; }
        let src = self.source_lines.get(line_0).copied().unwrap_or("");
        let (content, trailing) = Self::split_comment(src.trim());
        self.seg_items = Self::split_instructions(content)
            .into_iter().map(str::to_string).collect();
        self.seg_trailing = trailing.map(str::to_string);
        self.seg_idx = 0;
        self.seg_line = line_0;
    }

    fn emit_line(&mut self, depth: usize, content: &str, comment: Option<&str>) {
        let indent = self.indent(depth);
        self.output.push_str(&indent);
        self.output.push_str(content);
        if let Some(c) = comment {
            let current_col = indent.len() + content.len();
            let padding = self.comment_column.saturating_sub(current_col).max(2);
            self.output.push_str(&" ".repeat(padding));
            self.output.push_str(c);
        }
        self.output.push('\n');
    }

    // Emit a source line with reformatted indentation and directive_case on the first word.
    // Used for block headers and closers (REPEAT … ENDREPEAT, IF … ENDIF, etc.)
    fn emit_source_line_indented(&mut self, depth: usize, line_0: usize) {
        let src = self.source_lines.get(line_0).copied().unwrap_or("");
        let (content, comment) = Self::split_comment(src.trim());
        let formatted = Self::apply_case_to_first_word(content, self.directive_case);
        self.emit_line(depth, &formatted, comment);
    }

    fn find_closer_start(&self, keywords: &[&str]) -> usize {
        let kws: Vec<String> = keywords.iter().map(|k| k.to_ascii_uppercase()).collect();
        for i in self.current_line..self.source_lines.len() {
            let t = self.source_lines[i].trim().to_ascii_uppercase();
            for kw in &kws {
                if t == kw.as_str()
                    || t.starts_with(&format!("{kw} "))
                    || t.starts_with(&format!("{kw}\t"))
                    || t.starts_with(&format!("{kw};"))
                    || t.starts_with(&format!("{kw}//"))
                {
                    return i;
                }
            }
        }
        self.current_line
    }

    fn emit_closer(&mut self, depth: usize, keywords: &[&str]) {
        let line = self.find_closer_start(keywords);
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
        if token.is_warning() {
            self.format_token(token.warning_token(), depth, line_0);
            return;
        }

        // Standalone comment line → emit verbatim; trailing comment (same line as an
        // instruction already emitted) → skip to avoid duplication.
        if token.is_comment() {
            if line_0 < self.current_line { return; }
            let src = self.source_lines.get(line_0).copied().unwrap_or("");
            self.output.push_str(src);
            self.output.push('\n');
            self.current_line = line_0 + 1;
            return;
        }

        // Pre-split the source line into `:` segments so label and instruction branches
        // can consume them in order.
        if self.one_instruction_per_line {
            self.init_segments_for_line(line_0);
        }

        if token.is_label() {
            self.format_label(token, depth, line_0);
        } else if token.is_if() {
            self.format_if(token, depth, line_0);
        } else if token.is_repeat() {
            self.format_block(
                token.repeat_listing(), depth, line_0,
                &["ENDREPEAT", "ENDREPT", "ENDREP", "ENDR", "REND"],
            );
        } else if token.is_while() {
            self.format_block(
                token.while_listing(), depth, line_0,
                &["ENDWHILE", "ENDW", "WEND"],
            );
        } else if token.is_for() {
            self.format_block(
                token.for_listing(), depth, line_0,
                &["ENDFOR", "FEND", "ENDF"],
            );
        } else if token.is_module() {
            self.format_block(token.module_listing(), depth, line_0, &["ENDMODULE"]);
        } else if token.is_confined() {
            self.format_block(
                token.confined_listing(), depth, line_0,
                &["ENDCONFINED", "CEND", "ENDC"],
            );
        } else if token.is_repeat_until() {
            self.format_repeat_until(token, depth, line_0);
        } else if token.is_iterate() {
            self.format_block(
                token.iterate_listing(), depth, line_0,
                &["ENDITERATE", "ENDITER", "ENDI", "IEND"],
            );
        } else if token.is_rorg() {
            self.format_block(token.rorg_listing(), depth, line_0, &["DEPHASE", "REND", "ENDR"]);
        } else if token.is_crunched_section() {
            self.format_block(
                token.crunched_section_listing(), depth, line_0,
                &["LZCLOSE"],
            );
        } else if token.is_function_definition() {
            self.format_block(
                token.function_definition_inner(), depth, line_0,
                &["ENDFUNCTION", "ENDF"],
            );
        } else if token.is_switch() {
            self.format_switch(token, depth, line_0);
        } else if token.is_macro_definition() {
            self.format_macro_def(token, depth, line_0);
        } else {
            // Simple instruction, directive, or macro call.
            self.format_simple(token, depth, line_0);
        }
    }

    fn format_label(&mut self, token: &LocatedToken, depth: usize, line_0: usize) {
        let name = token.label_symbol();
        if self.one_instruction_per_line {
            // The segment at seg_idx may contain "label_name [trailing_instruction]"
            // (when a label and an instruction are on the same line without a `:` between them).
            // Consume the segment but re-inject any trailing instruction content.
            let seg_text = self.seg_items.get(self.seg_idx).cloned().unwrap_or_default();
            let trimmed = seg_text.trim_start();
            let after_label = trimmed
                .strip_prefix(name)
                .map(|rest| rest.trim_start_matches(':').trim())
                .unwrap_or("");

            self.seg_idx += 1;

            if !after_label.is_empty() {
                // Re-inject the trailing instruction as the next segment to consume.
                self.seg_items.insert(self.seg_idx, after_label.to_string());
            }

            // Emit trailing comment only if nothing more follows on this line.
            let comment = if self.seg_idx >= self.seg_items.len() {
                self.seg_trailing.clone()
            } else {
                None
            };
            self.emit_line(0, &format!("{name}:"), comment.as_deref());
        } else {
            let src = self.source_lines.get(line_0).copied().unwrap_or("");
            let (content_no_comment, comment) = Self::split_comment(src.trim());
            // Extract any instruction content that follows the label name on the same source line.
            let after_label = content_no_comment.trim_start()
                .strip_prefix(name)
                .map(|rest| rest.trim_start_matches(':').trim())
                .unwrap_or("");
            if after_label.is_empty() {
                self.emit_line(0, &format!("{name}:"), comment);
            } else {
                // Label and instruction share a line in the source; split them out.
                // Emit the instruction content verbatim to avoid misidentifying
                // struct/macro names as mnemonics and applying the wrong case.
                self.emit_line(0, &format!("{name}:"), None);
                let after = Self::normalize_colon_spacing(after_label, self.space_around_column);
                self.emit_line(depth, &after, comment);
            }
        }
        self.current_line = line_0 + 1;
    }

    // Format a non-block, non-label token.
    fn format_simple(&mut self, token: &LocatedToken, depth: usize, line_0: usize) {
        let (content, comment) = if self.one_instruction_per_line {
            let idx = self.seg_idx;
            self.seg_idx += 1;
            let is_last = idx + 1 >= self.seg_items.len();
            let seg = self.seg_items.get(idx).map(|s| s.as_str()).unwrap_or("");
            let (c, inline_cmt) = Self::split_comment(seg);
            let trailing_cmt = if is_last { self.seg_trailing.as_deref() } else { None };
            // Inline comment on this segment takes priority; fall back to line-level trailing comment.
            let comment = inline_cmt.or(trailing_cmt).map(str::to_string);
            (c.to_string(), comment)
        } else {
            // Without splitting: skip tokens that land on an already-emitted source line.
            if line_0 < self.current_line { return; }
            let src = self.source_lines.get(line_0).copied().unwrap_or("");
            let (c, cmt) = Self::split_comment(src.trim());
            // Reformat instruction-separator spacing if requested.
            let c = Self::normalize_colon_spacing(c, self.space_around_column);
            (c, cmt.map(str::to_string))
        };

        if token.mnemonic().is_some() {
            self.emit_line(
                depth,
                &Self::apply_mnemonic_case(&content, self.mnemonic_case, self.register_case),
                comment.as_deref(),
            );
        } else if token.is_call_macro_or_build_struct() {
            // Macro names are user-defined: preserve casing.
            self.emit_line(depth, &content, comment.as_deref());
        } else if token.is_assign() {
            // Symbol assignment (label = value, label += value, etc.):
            // first word is a user-defined symbol name — always at column 0.
            self.emit_line(0, &content, comment.as_deref());
        } else if token.is_equ() {
            // "symbol EQU value": label (first word) always at column 0;
            // apply directive_case only to the keyword (second word).
            self.emit_line(
                0,
                &Self::apply_case_to_second_word(&content, self.directive_case),
                comment.as_deref(),
            );
        } else {
            // Directives where a user-defined symbol precedes the keyword (like SETN/NEXT)
            // must not have that symbol name case-converted.
            // All directives where a user-defined symbol precedes the keyword:
            // SETN/NEXT (set-next symbol), FIELD/# (MAP entry allocation).
            const LABEL_FIRST_KWS: &[&str] = &["SETN", "SETNX", "NEXT", "FIELD", "#"];
            let second_word_upper = content
                .split_ascii_whitespace()
                .nth(1)
                .map(|w| w.trim_start_matches(':').to_ascii_uppercase())
                .unwrap_or_default();
            if LABEL_FIRST_KWS.contains(&second_word_upper.as_str())
                || second_word_upper.starts_with('#') {
                // Label-first directives: label is a top-level symbol name → column 0.
                self.emit_line(
                    0,
                    &Self::apply_case_to_second_word(&content, self.directive_case),
                    comment.as_deref(),
                );
            } else {
                // All other directives: keyword is the first word.
                self.emit_line(
                    depth,
                    &Self::apply_case_to_first_word(&content, self.directive_case),
                    comment.as_deref(),
                );
            }
        }

        // If the token spans multiple source lines (e.g. a multi-line expression),
        // emit the continuation lines verbatim so the assembler sees the complete syntax.
        let span_lines: usize = {
            let s: &str = token.span().as_ref();
            s.lines().count().max(1)
        };
        if span_lines > 1 {
            for i in 1..span_lines {
                if let Some(src) = self.source_lines.get(line_0 + i).copied() {
                    self.output.push_str(src);
                    self.output.push('\n');
                }
            }
            self.current_line = self.current_line.max(line_0 + span_lines);
        } else if line_0 >= self.current_line {
            self.current_line = line_0 + 1;
        }
    }

    fn format_block(
        &mut self,
        inner: &[LocatedToken],
        depth: usize,
        line_0: usize,
        closers: &[&str],
    ) {
        if line_0 >= self.current_line {
            self.emit_source_line_indented(depth, line_0);
            self.current_line = line_0 + 1;
        }
        if self.is_block_inline(inner, line_0) {
            return;
        }
        self.format_tokens(inner, depth + 1);
        self.emit_closer(depth, closers);
    }

    // Returns true if a block whose header is at `line_0` has its first inner token on the
    // same source line (i.e. the whole block is inline: `if x : body : endif`).
    fn is_block_inline(&self, inner: &[LocatedToken], line_0: usize) -> bool {
        inner.first()
            .map(|t| t.span().relative_line_and_column().0.saturating_sub(1) == line_0)
            // Empty body but closer might still be on the same line (e.g. `if x : endif`).
            .unwrap_or_else(|| {
                self.source_lines.get(line_0)
                    .map(|l| {
                        let u = l.to_ascii_uppercase();
                        u.contains("ENDIF") || u.contains("ENDM") || u.contains("ENDREPEAT")
                    })
                    .unwrap_or(false)
            })
    }

    fn format_if(&mut self, token: &LocatedToken, depth: usize, line_0: usize) {
        let nb_tests = token.if_nb_tests();
        let (_, inner_0) = token.if_test(0);

        if line_0 >= self.current_line {
            self.emit_source_line_indented(depth, line_0);
            self.current_line = line_0 + 1;
        }

        // Inline IF (all on one source line): header already emitted; skip body/closer.
        if self.is_block_inline(inner_0, line_0) {
            return;
        }

        self.format_tokens(inner_0, depth + 1);

        for i in 1..nb_tests {
            // Determine the ELSEIF* header line from the first body token's source position
            // (the header is always the line immediately before the body).
            // This handles ELSEIFDEF/ELSEIFNDEF/ELSEIF etc. without keyword-specific searches.
            let (_, inner_i) = token.if_test(i);
            let header = inner_i.first()
                .map(|t| t.span().relative_line_and_column().0.saturating_sub(2))
                .unwrap_or_else(|| self.find_closer_start(&["ELSEIF", "ELSE"]));
            self.emit_interstitial(header);
            self.emit_source_line_indented(depth, header);
            self.current_line = header + 1;
            self.format_tokens(inner_i, depth + 1);
        }

        if let Some(else_inner) = token.if_else() {
            let else_line = self.find_closer_start(&["ELSE"]);
            self.emit_interstitial(else_line);
            self.emit_source_line_indented(depth, else_line);
            self.current_line = else_line + 1;
            self.format_tokens(else_inner, depth + 1);
        }

        self.emit_closer(depth, &["ENDIF"]);
    }

    fn format_repeat_until(&mut self, token: &LocatedToken, depth: usize, line_0: usize) {
        if line_0 >= self.current_line {
            self.emit_source_line_indented(depth, line_0);
            self.current_line = line_0 + 1;
        }
        self.format_tokens(token.repeat_until_listing(), depth + 1);
        self.emit_closer(depth, &["UNTIL"]);
    }

    fn format_macro_def(&mut self, token: &LocatedToken, depth: usize, line_0: usize) {
        // Emit the macro header applying directive_case only to the MACRO keyword, not the
        // user-defined name. For name-first syntax (`name MACRO`) the first word is the name
        // and the second word is the keyword. For keyword-first (`MACRO name`) it is reversed.
        let src = self.source_lines.get(line_0).copied().unwrap_or("");
        let (content, comment) = Self::split_comment(src.trim());
        let macro_name = token.macro_definition_name();
        let first_word = content.split_ascii_whitespace().next().unwrap_or("");
        let first_upper = first_word.to_ascii_uppercase();
        let name_upper = macro_name.to_ascii_uppercase();
        let is_name_first = first_upper == name_upper
            || first_upper == format!("{}:", name_upper);
        let formatted = if is_name_first {
            // name MACRO[(params)]: case only the leading alpha chars of the keyword word,
            // leaving the macro name AND any parameter list unchanged.
            let name_end = content.find(|c: char| c.is_ascii_whitespace()).unwrap_or(content.len());
            let ws_and_rest = &content[name_end..];
            let rest_start = ws_and_rest.find(|c: char| !c.is_ascii_whitespace()).unwrap_or(ws_and_rest.len());
            let kw_and_tail = &ws_and_rest[rest_start..];
            let kw_end = kw_and_tail.find(|c: char| !c.is_ascii_alphabetic() && c != '_').unwrap_or(kw_and_tail.len());
            format!(
                "{}{}{}{}",
                &content[..name_end],
                &ws_and_rest[..rest_start],
                Self::apply_case(&kw_and_tail[..kw_end], self.directive_case),
                &kw_and_tail[kw_end..],
            )
        } else {
            // MACRO name [...]: case the first word (the keyword) as usual.
            Self::apply_case_to_first_word(content, self.directive_case)
        };
        self.emit_line(depth, &formatted, comment);
        self.current_line = line_0 + 1;
        let body = token.macro_definition_code();
        // The body content captured by the parser ends just before the ENDM keyword.
        // For name-first macros (`name MACRO`) the newline of the header line is
        // included at the start of the body. For all macros the indentation prefix
        // of the ENDM line appears as a trailing whitespace-only fragment.
        // Strip both so current_line stays aligned with the ENDM source line.
        let mut lines: Vec<&str> = body.lines().collect();
        let first_content = lines.iter().position(|l| !l.trim().is_empty()).unwrap_or(lines.len());
        lines.drain(..first_content);
        if lines.last().map_or(false, |l| l.trim().is_empty()) {
            lines.pop();
        }
        for line in lines {
            self.output.push_str(line);
            self.output.push('\n');
            self.current_line += 1;
        }
        self.emit_closer(depth, &["ENDM", "ENDMACRO", "MEND"]);
    }

    fn format_switch(&mut self, token: &LocatedToken, depth: usize, line_0: usize) {
        if line_0 >= self.current_line {
            self.emit_source_line_indented(depth, line_0);
            self.current_line = line_0 + 1;
        }

        // Collect cases so we can inspect them without consuming the iterator twice.
        let switch_cases: Vec<_> = token.switch_cases().collect();

        for (_, case_inner, has_break) in &switch_cases {
            let case_line = self.find_closer_start(&["CASE"]);
            // If no CASE line exists past current position (e.g. entirely inline switch), stop.
            if case_line >= self.source_lines.len() { break; }
            self.emit_interstitial(case_line);
            self.emit_source_line_indented(depth, case_line);
            self.current_line = case_line + 1;

            // If the first body token is on the same source line as the CASE header, the
            // entire case clause is inline (e.g. `case 1: db 1 : break`) and was already
            // emitted by emit_source_line_indented above.
            let body_inline = case_inner.first()
                .map(|t| t.span().relative_line_and_column().0.saturating_sub(1) <= case_line)
                .unwrap_or(false);
            if !body_inline {
                self.format_tokens(case_inner, depth + 1);
                if *has_break {
                    self.emit_closer(depth + 1, &["BREAK"]);
                }
            }
        }

        if let Some(default_inner) = token.switch_default() {
            let default_line = self.find_closer_start(&["DEFAULT"]);
            if default_line < self.source_lines.len() {
                self.emit_interstitial(default_line);
                self.emit_source_line_indented(depth, default_line);
                self.current_line = default_line + 1;
                let body_inline = default_inner.first()
                    .map(|t| t.span().relative_line_and_column().0.saturating_sub(1) <= default_line)
                    .unwrap_or(false);
                if !body_inline {
                    self.format_tokens(default_inner, depth + 1);
                }
            }
        }

        let endswitch_line = self.find_closer_start(&["ENDSWITCH", "ENDS"]);
        if endswitch_line < self.source_lines.len() {
            self.emit_closer(depth, &["ENDSWITCH", "ENDS"]);
        }
    }
}

pub fn format_listing(listing: &LocatedListing, source: &str, depth: usize, opt: &AsmFormatOptions) -> String {
    let mut fmt = Formatter::new(source, opt);
    if let Some(first) = listing.iter().next() {
        let (line_1, _) = first.span().relative_line_and_column();
        fmt.current_line = line_1.saturating_sub(1);
    }
    fmt.format_tokens(listing, depth);
    fmt.output
}

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
        assert!(out.contains("REPEAT 10"), "got: {out:?}");
        assert!(out.contains("ENDREPEAT"), "got: {out:?}");
    }

    #[test]
    fn test_block_header_directive_case() {
        let out = fmt("repeat 5, i, 3\n  ld a, i\nendr");
        assert!(out.contains("REPEAT 5, i, 3"), "REPEAT not uppercased: {out:?}");
        assert!(out.contains("ENDR") || out.contains("ENDREPEAT"), "closer not uppercased: {out:?}");
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
        let out = fmt("push af ; save af");
        let line = out.lines().next().unwrap();
        let col = line.find(';').expect("no comment found");
        assert_eq!(col, 30, "comment not at column 30: {line:?}");
    }

    #[test]
    fn test_comment_column_long_content() {
        let long = "ld hl, (some_very_long_symbol_name_that_is_long)";
        let src = format!("{long} ; cmnt");
        let out = fmt(&src);
        let line = out.lines().next().unwrap();
        let col = line.find(';').expect("no comment");
        let content_end = 4 + long.len();
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
        let out = fmt("ld a, 1\nld hl, 100\nld de, 0x40\nadd a, %00001111");
        assert!(out.contains("LD A, 1\n"),           "literal 1 re-encoded: {out:?}");
        assert!(out.contains("LD HL, 100\n"),        "literal 100 re-encoded: {out:?}");
        assert!(out.contains("LD DE, 0x40\n"),       "literal 0x40 re-encoded: {out:?}");
        assert!(out.contains("ADD A, %00001111\n"),  "literal %… re-encoded: {out:?}");
    }

    #[test]
    fn test_registers_uppercased() {
        let out = fmt("ld hl, (ix+2)\npush af\nex af, af'");
        assert!(out.contains("LD HL, (IX+2)\n"),  "registers not uppercased: {out:?}");
        assert!(out.contains("PUSH AF\n"),          "AF not uppercased: {out:?}");
        assert!(out.contains("EX AF, AF'\n"),       "AF' not uppercased: {out:?}");
    }

    #[test]
    fn test_label_not_treated_as_register() {
        let r1 = Formatter::apply_register_case("hl, bc_label", CaseStyle::UpperCase);
        assert_eq!(r1, "HL, bc_label", "bc_label was altered: {r1:?}");
        let r2 = Formatter::apply_register_case("a, hlabel", CaseStyle::UpperCase);
        assert_eq!(r2, "A, hlabel", "hlabel was altered: {r2:?}");
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
    fn test_colon_separator_no_duplicate() {
        // Multiple instructions on one line must not be duplicated.
        let src = "    pop hl : push af : pop af\n";
        let out = fmt(src);
        assert_eq!(out.matches("POP HL").count(),  1, "POP HL duplicated: {out:?}");
        assert_eq!(out.matches("PUSH AF").count(), 1, "PUSH AF duplicated: {out:?}");
        assert_eq!(out.matches("POP AF").count(),  1, "POP AF duplicated: {out:?}");
    }

    #[test]
    fn test_one_instruction_per_line_splits() {
        let src = "pop hl : push af : pop af\n";
        let out = fmt(src);
        let lines: Vec<&str> = out.lines().collect();
        // Each instruction on its own line
        assert!(lines.iter().any(|l| l.trim() == "POP HL"),  "POP HL not on own line: {out:?}");
        assert!(lines.iter().any(|l| l.trim() == "PUSH AF"), "PUSH AF not on own line: {out:?}");
        assert!(lines.iter().any(|l| l.trim() == "POP AF"),  "POP AF not on own line: {out:?}");
    }

    #[test]
    fn test_one_instruction_per_line_false_keeps_line() {
        let opt = AsmFormatOptions {
            one_instruction_per_line: false,
            ..AsmFormatOptions::default()
        };
        let src = "pop hl : push af\n";
        let out = format(src, &opt).unwrap();
        // Both instructions must be on a single line (no splitting).
        // The second mnemonic keyword is not in first-word position so its case is not
        // transformed; only registers like HL/AF are uppercased within the line.
        assert!(
            out.lines().any(|l| l.contains("POP HL") && l.contains("push AF")),
            "line was split when one_instruction_per_line=false: {out:?}"
        );
    }

    #[test]
    fn test_colon_comment_on_last_instruction() {
        // Trailing comment must appear once, on the last instruction.
        let src = "pop hl : push af ; my comment\n";
        let out = fmt(src);
        assert_eq!(out.matches("my comment").count(), 1, "comment duplicated: {out:?}");
        // The comment should be on the PUSH AF line, not the POP HL line.
        let push_line = out.lines().find(|l| l.contains("PUSH AF")).expect("no PUSH AF line");
        assert!(push_line.contains("my comment"), "comment not on last instruction: {push_line:?}");
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

    #[test]
    fn test_assign_symbol_case_not_changed() {
        // Symbol names in assignment directives must not be case-transformed.
        let out = fmt("my_label = 42\nassert my_label == 42");
        assert!(out.contains("my_label = 42"), "symbol name changed: {out:?}");
        assert!(out.contains("ASSERT my_label"), "symbol in expr changed: {out:?}");
    }

    #[test]
    fn test_equ_keyword_case_changed() {
        // EQU keyword should be case-transformed, symbol name should not.
        let out = fmt("my_sym equ 10\ndb my_sym");
        assert!(out.contains("my_sym EQU 10"), "EQU not uppercased or symbol changed: {out:?}");
    }

    #[test]
    fn test_label_with_instruction_on_same_line() {
        // Label followed by instruction on the same line (no colon separator).
        let out = fmt("myloop\tpush af");
        assert!(out.contains("myloop:"), "label missing colon: {out:?}");
        assert!(out.contains("PUSH AF"), "instruction after inline label lost: {out:?}");
    }

    #[test]
    fn test_equ_at_column_zero() {
        // EQU labels must start at column 0 regardless of any surrounding block depth.
        let out = fmt("FOO EQU 42");
        let line = out.lines().next().unwrap();
        assert!(!line.starts_with(' '), "EQU line is indented: {line:?}");
        assert!(line.starts_with("FOO"), "EQU label not at column 0: {line:?}");
    }

    #[test]
    fn test_assign_at_column_zero() {
        // Symbol assignments (=) must start at column 0.
        let out = fmt("my_var = 10");
        let line = out.lines().next().unwrap();
        assert!(!line.starts_with(' '), "assignment line is indented: {line:?}");
        assert!(line.starts_with("my_var"), "assignment not at column 0: {line:?}");
    }

    #[test]
    fn test_comment_column_custom() {
        // comment_column should be honoured for non-default values.
        let opt = AsmFormatOptions::builder().comment_column(50).build();
        let out = format("nop ; hi", &opt).unwrap();
        let line = out.lines().next().unwrap();
        let col = line.find(';').expect("no comment found");
        assert_eq!(col, 50, "comment not at column 50: {line:?}");
    }

    #[test]
    fn test_space_around_column_both() {
        // SpaceAroundColumn::Both forces ` : ` between instructions.
        let opt = AsmFormatOptions::builder()
            .one_instruction_per_line(false)
            .space_around_column(SpaceAroundColumn::Both)
            .build();
        let out = format("nop : ld a, 5", &opt).unwrap();
        let line = out.lines().next().unwrap();
        assert!(line.contains(" : "), "separator not ' : ': {line:?}");
    }

    #[test]
    fn test_space_around_column_none() {
        // SpaceAroundColumn::None removes all spaces around `:`.
        let opt = AsmFormatOptions::builder()
            .one_instruction_per_line(false)
            .space_around_column(SpaceAroundColumn::None)
            .build();
        let out = format("nop : ld a, 5", &opt).unwrap();
        let line = out.lines().next().unwrap();
        assert!(line.contains(':') && !line.contains(" :") && !line.contains(": "),
            "unexpected spacing around ':': {line:?}");
    }

    #[test]
    fn test_space_around_column_untouched_preserves() {
        // SpaceAroundColumn::Untouched (default) must not alter existing spacing.
        let opt = AsmFormatOptions::builder()
            .one_instruction_per_line(false)
            .space_around_column(SpaceAroundColumn::Untouched)
            .build();
        let src = "nop : ld a, 5";
        let out = format(src, &opt).unwrap();
        // The ` : ` from source should be preserved.
        assert!(out.contains(" : "), "spacing was altered: {out:?}");
    }
}
