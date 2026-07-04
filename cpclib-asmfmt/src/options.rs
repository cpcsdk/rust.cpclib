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
