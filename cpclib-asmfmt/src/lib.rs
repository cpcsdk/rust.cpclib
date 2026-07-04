use std::path::{Path, PathBuf};

use cpclib_asm::{AssemblerError, ListingElement, LocatedListing, LocatedToken, MayHaveSpan, parse_z80_str};
use cpclib_common::parse::{EncodingKind, scan_numeric_literals};

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

/// Controls the prefix or suffix used when reformatting hexadecimal literals.
/// TOML / JSON values: `"0x"`, `"0X"`, `"#"`, `"$"`, `"&"`, `"h"`, `"H"`, `"Untouched"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HexEncoding {
    #[serde(rename = "0x")]  Prefix0x,      // 0x1A
    #[serde(rename = "0X")]  Prefix0X,      // 0X1A
    #[serde(rename = "#")]   PrefixHash,    // #1A
    #[serde(rename = "$")]   PrefixDollar,  // $1A
    #[serde(rename = "&")]   PrefixAmp,     // &1A
    #[serde(rename = "h")]   SuffixLower,   // 1ah
    #[serde(rename = "H")]   SuffixUpper,   // 1AH
    Untouched,                              // preserve original encoding
}

/// Controls the prefix used when reformatting octal literals.
/// TOML / JSON values: `"0o"`, `"0O"`, `"@"`, `"Untouched"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OctalEncoding {
    #[serde(rename = "0o")]  Prefix0o,   // 0o17
    #[serde(rename = "0O")]  Prefix0O,   // 0O17
    #[serde(rename = "@")]   PrefixAt,   // @17
    Untouched,                           // preserve original encoding
}

/// Controls the prefix used when reformatting binary literals.
/// TOML / JSON values: `"0b"`, `"0B"`, `"%"`, `"Untouched"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BinaryEncoding {
    #[serde(rename = "0b")]  Prefix0b,       // 0b00110101
    #[serde(rename = "0B")]  Prefix0B,       // 0B00110101
    #[serde(rename = "%")]   PrefixPercent,  // %00110101
    Untouched,                               // preserve original encoding
}

/// Controls whether label definitions are emitted with or without a trailing `:`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LabelPostfix {
    NoColumn,   // emit without trailing ':'
    WithColumn, // emit with trailing ':'
    Untouched,  // preserve original source (colon if source had one)
}

fn default_indent_size() -> usize { 4 }
fn default_comment_column() -> usize { 30 }
fn default_mnemonic_case() -> CaseStyle { CaseStyle::UpperCase }
fn default_directive_case() -> CaseStyle { CaseStyle::UpperCase }
fn default_register_case() -> CaseStyle { CaseStyle::UpperCase }
fn default_one_instruction_per_line() -> bool { true }
fn default_space_around_column() -> SpaceAroundColumn { SpaceAroundColumn::Untouched }
fn default_space_around_assignment() -> SpaceAroundColumn { SpaceAroundColumn::Untouched }
fn default_hexadecimal_case() -> CaseStyle { CaseStyle::Untouched }
fn default_hexadecimal_encoding() -> HexEncoding { HexEncoding::Untouched }
fn default_octal_encoding() -> OctalEncoding { OctalEncoding::Untouched }
fn default_binary_encoding() -> BinaryEncoding { BinaryEncoding::Untouched }
fn default_label_postfix() -> LabelPostfix { LabelPostfix::WithColumn }

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
    /// How spaces are written around the assignment operator (`=`, `+=`, `>>=`, …)
    /// in symbol-assignment statements (default: Untouched).
    #[serde(default = "default_space_around_assignment")]
    #[builder(default = default_space_around_assignment())]
    pub space_around_assignment: SpaceAroundColumn,
    /// Case applied to the A-F letters inside hex literals (default: Untouched).
    #[serde(default = "default_hexadecimal_case")]
    #[builder(default = default_hexadecimal_case())]
    pub hexadecimal_case: CaseStyle,
    /// Prefix/suffix form used when reformatting hex literals (default: Untouched).
    #[serde(default = "default_hexadecimal_encoding")]
    #[builder(default = default_hexadecimal_encoding())]
    pub hexadecimal_encoding: HexEncoding,
    /// Prefix form used when reformatting octal literals (default: Untouched).
    #[serde(default = "default_octal_encoding")]
    #[builder(default = default_octal_encoding())]
    pub octal_encoding: OctalEncoding,
    /// Prefix form used when reformatting binary literals (default: Untouched).
    #[serde(default = "default_binary_encoding")]
    #[builder(default = default_binary_encoding())]
    pub binary_encoding: BinaryEncoding,
    /// Whether label definitions are emitted with or without a trailing `:` (default: Untouched).
    #[serde(default = "default_label_postfix")]
    #[builder(default = default_label_postfix())]
    pub label_definition_postfix_with_column: LabelPostfix,
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
            space_around_assignment: default_space_around_assignment(),
            hexadecimal_case: default_hexadecimal_case(),
            hexadecimal_encoding: default_hexadecimal_encoding(),
            octal_encoding: default_octal_encoding(),
            binary_encoding: default_binary_encoding(),
            label_definition_postfix_with_column: default_label_postfix(),
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
    space_around_assignment: SpaceAroundColumn,
    hexadecimal_case: CaseStyle,
    hexadecimal_encoding: HexEncoding,
    octal_encoding: OctalEncoding,
    binary_encoding: BinaryEncoding,
    label_definition_postfix_with_column: LabelPostfix,
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
            space_around_assignment: opt.space_around_assignment,
            hexadecimal_case: opt.hexadecimal_case,
            hexadecimal_encoding: opt.hexadecimal_encoding,
            octal_encoding: opt.octal_encoding,
            binary_encoding: opt.binary_encoding,
            label_definition_postfix_with_column: opt.label_definition_postfix_with_column,
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
    // Also normalises any run of whitespace between the keyword and its arguments to a single
    // space so that "ORG  40" → "ORG 40".
    fn apply_case_to_first_word(content: &str, case: CaseStyle) -> String {
        let word_end = content.find(|c: char| c.is_ascii_whitespace()).unwrap_or(content.len());
        let keyword = Self::apply_case(&content[..word_end], case);
        let rest = content[word_end..].trim_start();
        if rest.is_empty() {
            keyword
        } else {
            format!("{keyword} {rest}")
        }
    }

    // Apply case to the second whitespace-delimited word (e.g., the EQU keyword in
    // "symbol EQU value"), leaving the first word (user symbol name) unchanged.
    // Also normalises inter-word whitespace to a single space.
    fn apply_case_to_second_word(content: &str, case: CaseStyle) -> String {
        let bytes = content.as_bytes();
        let first_end = bytes.iter().position(|b| b.is_ascii_whitespace()).unwrap_or(bytes.len());
        let second_start = bytes[first_end..]
            .iter()
            .position(|b| !b.is_ascii_whitespace())
            .map(|p| first_end + p)
            .unwrap_or(bytes.len());
        let second_end = bytes[second_start..]
            .iter()
            .position(|b| b.is_ascii_whitespace())
            .map(|p| second_start + p)
            .unwrap_or(bytes.len());
        let symbol  = &content[..first_end];
        let keyword = Self::apply_case(&content[second_start..second_end], case);
        let rest    = content[second_end..].trim_start();
        if rest.is_empty() {
            format!("{symbol} {keyword}")
        } else {
            format!("{symbol} {keyword} {rest}")
        }
    }

    // Apply case to a mnemonic line: transforms the mnemonic keyword and register names
    // in operands but leaves numeric literals / labels / expressions unchanged.
    // Also normalises the whitespace between mnemonic and operands to a single space.
    fn apply_mnemonic_case(content: &str, mnemonic_case: CaseStyle, register_case: CaseStyle) -> String {
        let word_end = content.find(|c: char| c.is_ascii_whitespace()).unwrap_or(content.len());
        let mnemonic = Self::apply_case(&content[..word_end], mnemonic_case);
        let rest = content[word_end..].trim_start();
        if rest.is_empty() {
            mnemonic
        } else {
            let operands = if matches!(register_case, CaseStyle::Untouched) {
                rest.to_string()
            } else {
                Self::apply_register_case(rest, register_case)
            };
            format!("{mnemonic} {operands}")
        }
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

    // Reformat the assignment operator spacing in a `label [op]= value` statement.
    // Locates the first `=` and scans back over compound-operator prefix characters
    // (`+`, `-`, `*`, `/`, `%`, `&`, `|`, `^`, `<`, `>`) to find the full operator.
    // Whitespace on both sides of the operator is then replaced according to `spacing`.
    fn normalize_assignment_spacing(content: &str, spacing: SpaceAroundColumn) -> String {
        if matches!(spacing, SpaceAroundColumn::Untouched) {
            return content.to_string();
        }
        let bytes = content.as_bytes();
        let is_op_prefix = |b: u8| matches!(b, b'+' | b'-' | b'*' | b'/' | b'%' | b'&' | b'|' | b'^' | b'<' | b'>');
        let Some(eq_pos) = bytes.iter().position(|&b| b == b'=') else {
            return content.to_string();
        };
        // Find where the operator starts (scan back over prefix chars only, no whitespace).
        let mut op_start = eq_pos;
        while op_start > 0 && is_op_prefix(bytes[op_start - 1]) {
            op_start -= 1;
        }
        let label = content[..op_start].trim_end();
        let op    = &content[op_start..=eq_pos];
        let value = content[eq_pos + 1..].trim_start();
        let (sp_before, sp_after) = match spacing {
            SpaceAroundColumn::None   => ("", ""),
            SpaceAroundColumn::Before => (" ", ""),
            SpaceAroundColumn::After  => ("", " "),
            SpaceAroundColumn::Both   => (" ", " "),
            SpaceAroundColumn::Untouched => unreachable!(),
        };
        format!("{}{}{}{}{}", label, sp_before, op, sp_after, value)
    }

    // Reformat all numeric literals in `content` according to the hex/oct/bin encoding and
    // hex case settings.  When all four settings are Untouched the string is returned as-is.
    fn reformat_numeric_literals(&self, content: &str) -> String {
        if matches!(self.hexadecimal_case, CaseStyle::Untouched)
            && matches!(self.hexadecimal_encoding, HexEncoding::Untouched)
            && matches!(self.octal_encoding, OctalEncoding::Untouched)
            && matches!(self.binary_encoding, BinaryEncoding::Untouched)
        {
            return content.to_string();
        }

        let spans = scan_numeric_literals(content);
        if spans.is_empty() {
            return content.to_string();
        }

        let mut result = String::with_capacity(content.len());
        let mut cursor = 0usize;
        for (start, end, value, kind) in spans {
            result.push_str(&content[cursor..start]);
            let original = &content[start..end];
            result.push_str(&self.reformat_number(value, kind, original));
            cursor = end;
        }
        result.push_str(&content[cursor..]);
        result
    }

    fn reformat_number(&self, value: u32, kind: EncodingKind, original: &str) -> String {
        match kind {
            EncodingKind::Hex => self.reformat_hex(value, original),
            EncodingKind::Oct => self.reformat_oct(value, original),
            EncodingKind::Bin => self.reformat_bin(value, original),
            _ => original.to_string(), // Dec and internal states: unchanged
        }
    }

    fn reformat_hex(&self, value: u32, original: &str) -> String {
        let enc = self.hexadecimal_encoding;
        let case = self.hexadecimal_case;

        if matches!(enc, HexEncoding::Untouched) && matches!(case, CaseStyle::Untouched) {
            return original.to_string();
        }

        if matches!(enc, HexEncoding::Untouched) {
            // Only change letter case; preserve prefix/suffix verbatim.
            return original.chars().map(|c| match c {
                'a'..='f' | 'A'..='F' => match case {
                    CaseStyle::UpperCase => c.to_ascii_uppercase(),
                    CaseStyle::LowerCase => c.to_ascii_lowercase(),
                    CaseStyle::Untouched => c,
                },
                _ => c,
            }).collect();
        }

        // Re-encode: format value as hex digits with the requested case.
        let raw = format!("{:X}", value); // always uppercase first
        let digits: String = raw.chars().map(|c| match case {
            CaseStyle::LowerCase => c.to_ascii_lowercase(),
            _ => c, // UpperCase or Untouched → uppercase
        }).collect();

        let is_suffix = matches!(enc, HexEncoding::SuffixLower | HexEncoding::SuffixUpper);
        // Suffix form must start with a digit to avoid being parsed as an identifier.
        let digits = if is_suffix && digits.chars().next().map_or(false, |c| c.is_ascii_alphabetic()) {
            format!("0{digits}")
        } else {
            digits
        };

        match enc {
            HexEncoding::Prefix0x     => format!("0x{digits}"),
            HexEncoding::Prefix0X     => format!("0X{digits}"),
            HexEncoding::PrefixHash   => format!("#{digits}"),
            HexEncoding::PrefixDollar => format!("${digits}"),
            HexEncoding::PrefixAmp    => format!("&{digits}"),
            HexEncoding::SuffixLower  => format!("{digits}h"),
            HexEncoding::SuffixUpper  => format!("{digits}H"),
            HexEncoding::Untouched    => unreachable!(),
        }
    }

    fn reformat_oct(&self, value: u32, original: &str) -> String {
        match self.octal_encoding {
            OctalEncoding::Untouched => original.to_string(),
            OctalEncoding::Prefix0o  => format!("0o{:o}", value),
            OctalEncoding::Prefix0O  => format!("0O{:o}", value),
            OctalEncoding::PrefixAt  => format!("@{:o}", value),
        }
    }

    fn reformat_bin(&self, value: u32, original: &str) -> String {
        match self.binary_encoding {
            BinaryEncoding::Untouched      => original.to_string(),
            BinaryEncoding::Prefix0b       => format!("0b{:b}", value),
            BinaryEncoding::Prefix0B       => format!("0B{:b}", value),
            BinaryEncoding::PrefixPercent  => format!("%{:b}", value),
        }
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
        let formatted = self.reformat_numeric_literals(&formatted);
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

        // Determine whether to emit the trailing ':' based on the postfix option.
        let src_line = self.source_lines.get(line_0).copied().unwrap_or("");
        let original_had_colon = src_line.trim_start()
            .strip_prefix(name)
            .map_or(false, |rest| rest.trim_start().starts_with(':'));
        let emit_colon = match self.label_definition_postfix_with_column {
            LabelPostfix::WithColumn => true,
            LabelPostfix::NoColumn   => false,
            LabelPostfix::Untouched  => original_had_colon,
        };
        let label_str = if emit_colon { format!("{name}:") } else { name.to_string() };

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
            self.emit_line(0, &label_str, comment.as_deref());
        } else {
            let src = self.source_lines.get(line_0).copied().unwrap_or("");
            let (content_no_comment, comment) = Self::split_comment(src.trim());
            // Extract any instruction content that follows the label name on the same source line.
            let after_label = content_no_comment.trim_start()
                .strip_prefix(name)
                .map(|rest| rest.trim_start_matches(':').trim())
                .unwrap_or("");
            if after_label.is_empty() {
                self.emit_line(0, &label_str, comment);
            } else {
                // Label and instruction share a line in the source; split them out.
                // Emit the instruction content verbatim to avoid misidentifying
                // struct/macro names as mnemonics and applying the wrong case.
                self.emit_line(0, &label_str, None);
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
            let out = Self::apply_mnemonic_case(&content, self.mnemonic_case, self.register_case);
            let out = self.reformat_numeric_literals(&out);
            self.emit_line(depth, &out, comment.as_deref());
        } else if token.is_call_macro_or_build_struct() {
            // Macro names are user-defined: preserve casing; only reformat numeric literals.
            let out = self.reformat_numeric_literals(&content);
            self.emit_line(depth, &out, comment.as_deref());
        } else if token.is_assign() {
            // Symbol assignment (label = value, label += value, etc.):
            // first word is a user-defined symbol name — always at column 0.
            let out = Self::normalize_assignment_spacing(&content, self.space_around_assignment);
            let out = self.reformat_numeric_literals(&out);
            self.emit_line(0, &out, comment.as_deref());
        } else if token.is_equ() {
            // "symbol EQU value": label (first word) always at column 0;
            // apply directive_case only to the keyword (second word).
            let out = Self::apply_case_to_second_word(&content, self.directive_case);
            let out = self.reformat_numeric_literals(&out);
            self.emit_line(0, &out, comment.as_deref());
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
                let out = Self::apply_case_to_second_word(&content, self.directive_case);
                let out = self.reformat_numeric_literals(&out);
                self.emit_line(0, &out, comment.as_deref());
            } else {
                // All other directives: keyword is the first word.
                let out = Self::apply_case_to_first_word(&content, self.directive_case);
                let out = self.reformat_numeric_literals(&out);
                self.emit_line(depth, &out, comment.as_deref());
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

    // ── space_around_assignment ──────────────────────────────────────────────

    fn fmt_assign(src: &str, spacing: SpaceAroundColumn) -> String {
        let opt = AsmFormatOptions::builder()
            .space_around_assignment(spacing)
            .build();
        format(src, &opt).unwrap()
    }

    #[test]
    fn test_assign_spacing_both() {
        let out = fmt_assign("my_var=5", SpaceAroundColumn::Both);
        assert!(out.contains("my_var = 5"), "Both: {out:?}");
    }

    #[test]
    fn test_assign_spacing_none() {
        let out = fmt_assign("my_var = 5", SpaceAroundColumn::None);
        assert!(out.contains("my_var=5"), "None: {out:?}");
    }

    #[test]
    fn test_assign_spacing_before() {
        let out = fmt_assign("my_var=5", SpaceAroundColumn::Before);
        assert!(out.contains("my_var =5"), "Before: {out:?}");
    }

    #[test]
    fn test_assign_spacing_after() {
        let out = fmt_assign("my_var=5", SpaceAroundColumn::After);
        assert!(out.contains("my_var= 5"), "After: {out:?}");
    }

    #[test]
    fn test_assign_spacing_untouched() {
        // Untouched (default) must preserve original spacing exactly.
        let out = fmt_assign("my_var=5", SpaceAroundColumn::Untouched);
        assert!(out.contains("my_var=5"), "Untouched: {out:?}");
        let out2 = fmt_assign("my_var = 5", SpaceAroundColumn::Untouched);
        assert!(out2.contains("my_var = 5"), "Untouched spaces: {out2:?}");
    }

    #[test]
    fn test_assign_compound_operator_both() {
        let out = fmt_assign("my_var+=10", SpaceAroundColumn::Both);
        assert!(out.contains("my_var += 10"), "compound Both: {out:?}");
    }

    #[test]
    fn test_assign_compound_operator_none() {
        let out = fmt_assign("my_var += 10", SpaceAroundColumn::None);
        assert!(out.contains("my_var+=10"), "compound None: {out:?}");
    }

    #[test]
    fn test_assign_shift_operator_both() {
        let out = fmt_assign("my_var>>=2", SpaceAroundColumn::Both);
        assert!(out.contains("my_var >>= 2"), "shift Both: {out:?}");
    }

    // ── TOML config roundtrip ─────────────────────────────────────────────────

    #[test]
    fn test_toml_config_roundtrip() {
        let toml = r#"
indent_size = 4
comment_column = 13
mnemonic_case = "LowerCase"
directive_case = "UpperCase"
register_case = "LowerCase"
one_instruction_per_line = false
space_around_column = "Both"
space_around_assignment = "Both"
hexadecimal_case = "UpperCase"
hexadecimal_encoding = "0x"
octal_encoding = "0o"
binary_encoding = "0b"
label_definition_postfix_with_column = "NoColumn"
"#;
        let cfg: AsmFormatOptions = toml::from_str(toml).expect("TOML parse failed");
        assert!(matches!(cfg.mnemonic_case, CaseStyle::LowerCase), "mnemonic_case: {cfg:?}");
        assert!(matches!(cfg.register_case, CaseStyle::LowerCase), "register_case");
        assert!(matches!(cfg.hexadecimal_encoding, HexEncoding::Prefix0x), "hex_enc");
        assert!(matches!(cfg.octal_encoding, OctalEncoding::Prefix0o), "oct_enc");
        assert!(matches!(cfg.binary_encoding, BinaryEncoding::Prefix0b), "bin_enc");
        assert!(matches!(cfg.label_definition_postfix_with_column, LabelPostfix::NoColumn), "label_postfix");
    }

    // ── hexadecimal_case ─────────────────────────────────────────────────────

    #[test]
    fn test_hex_case_upper() {
        let opt = AsmFormatOptions::builder().hexadecimal_case(CaseStyle::UpperCase).build();
        let out = format("ld a, 0xff\nld b, $ab", &opt).unwrap();
        assert!(out.contains("0xFF") || out.contains("0XFF"), "hex not uppercased: {out:?}");
        assert!(out.contains("$AB"), "dollar hex not uppercased: {out:?}");
    }

    #[test]
    fn test_hex_case_lower() {
        let opt = AsmFormatOptions::builder().hexadecimal_case(CaseStyle::LowerCase).build();
        let out = format("ld a, 0xFF\nld b, $AB", &opt).unwrap();
        assert!(out.contains("0xff") || out.contains("ff"), "hex not lowercased: {out:?}");
        assert!(out.contains("$ab"), "dollar hex not lowercased: {out:?}");
    }

    // ── hexadecimal_encoding ─────────────────────────────────────────────────

    #[test]
    fn test_hex_encoding_prefix_dollar() {
        let opt = AsmFormatOptions::builder()
            .hexadecimal_encoding(HexEncoding::PrefixDollar)
            .build();
        let out = format("ld a, 0xff", &opt).unwrap();
        assert!(out.contains("$FF") || out.contains("$ff"), "not $ prefix: {out:?}");
        assert!(!out.contains("0xff") && !out.contains("0xFF"), "old prefix still present: {out:?}");
    }

    #[test]
    fn test_hex_encoding_suffix_h() {
        let opt = AsmFormatOptions::builder()
            .hexadecimal_encoding(HexEncoding::SuffixLower)
            .build();
        let out = format("ld a, 0x1A", &opt).unwrap();
        assert!(out.contains("1ah") || out.contains("1Ah"), "not h suffix: {out:?}");
    }

    #[test]
    fn test_hex_encoding_suffix_h_leading_zero() {
        // When the first hex digit is alphabetic, a leading 0 must be added.
        let opt = AsmFormatOptions::builder()
            .hexadecimal_encoding(HexEncoding::SuffixUpper)
            .build();
        let out = format("ld a, 0xFF", &opt).unwrap();
        assert!(out.contains("0FFH") || out.contains("0ffH"), "leading 0 missing: {out:?}");
    }

    // ── octal_encoding ────────────────────────────────────────────────────────

    #[test]
    fn test_octal_encoding_prefix_at() {
        let opt = AsmFormatOptions::builder()
            .octal_encoding(OctalEncoding::PrefixAt)
            .build();
        let out = format("ld a, 0o17", &opt).unwrap();
        assert!(out.contains("@17"), "not @ prefix: {out:?}");
    }

    #[test]
    fn test_octal_encoding_prefix_0o() {
        let opt = AsmFormatOptions::builder()
            .octal_encoding(OctalEncoding::Prefix0o)
            .build();
        let out = format("ld a, @17", &opt).unwrap();
        assert!(out.contains("0o17"), "not 0o prefix: {out:?}");
    }

    // ── binary_encoding ───────────────────────────────────────────────────────

    #[test]
    fn test_binary_encoding_percent() {
        let opt = AsmFormatOptions::builder()
            .binary_encoding(BinaryEncoding::PrefixPercent)
            .build();
        let out = format("ld a, 0b00001111", &opt).unwrap();
        assert!(out.contains("%1111") || out.contains("%00001111"), "not % prefix: {out:?}");
    }

    #[test]
    fn test_binary_encoding_0b() {
        let opt = AsmFormatOptions::builder()
            .binary_encoding(BinaryEncoding::Prefix0b)
            .build();
        let out = format("ld a, %00001111", &opt).unwrap();
        assert!(out.contains("0b"), "not 0b prefix: {out:?}");
    }

    // ── label_definition_postfix_with_column ──────────────────────────────────

    #[test]
    fn test_label_postfix_no_column() {
        let opt = AsmFormatOptions::builder()
            .label_definition_postfix_with_column(LabelPostfix::NoColumn)
            .build();
        let out = format("myloop:\n  push af", &opt).unwrap();
        let label_line = out.lines().next().unwrap();
        assert!(!label_line.contains(':'), "colon present with NoColumn: {out:?}");
        assert!(label_line.trim() == "myloop", "wrong label line: {out:?}");
    }

    #[test]
    fn test_label_postfix_with_column() {
        let opt = AsmFormatOptions::builder()
            .label_definition_postfix_with_column(LabelPostfix::WithColumn)
            .build();
        let out = format("myloop:\n  push af", &opt).unwrap();
        let label_line = out.lines().next().unwrap();
        assert!(label_line.contains(':'), "colon missing with WithColumn: {out:?}");
    }

    // ── single space after directive ──────────────────────────────────────────

    #[test]
    fn test_single_space_after_directive() {
        let out = fmt("ORG  0x40\nDB   1, 2, 3");
        assert!(out.contains("ORG 0x40"), "double space after ORG not collapsed: {out:?}");
        assert!(out.contains("DB 1, 2, 3"), "double space after DB not collapsed: {out:?}");
    }
}
