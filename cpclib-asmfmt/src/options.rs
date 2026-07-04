#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "cmdline", derive(clap::ValueEnum))]
pub enum CaseStyle {
    #[cfg_attr(feature = "cmdline", value(name = "uppercase"))]
    UpperCase,
    #[cfg_attr(feature = "cmdline", value(name = "lowercase"))]
    LowerCase,
    #[cfg_attr(feature = "cmdline", value(name = "untouched"))]
    Untouched,
}

#[cfg(feature = "cmdline")]
impl std::fmt::Display for CaseStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use clap::ValueEnum;
        self.to_possible_value().expect("no values skipped").get_name().fmt(f)
    }
}

/// Controls how spaces are written around `:` instruction separators when
/// `one_instruction_per_line = false` and a source line has multiple instructions.
/// Has no effect when `one_instruction_per_line = true` (separators become newlines).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "cmdline", derive(clap::ValueEnum))]
pub enum SpaceAroundColumn {
    #[cfg_attr(feature = "cmdline", value(name = "none"))]
    None,
    #[cfg_attr(feature = "cmdline", value(name = "before"))]
    Before,
    #[cfg_attr(feature = "cmdline", value(name = "after"))]
    After,
    #[cfg_attr(feature = "cmdline", value(name = "both"))]
    Both,
    #[cfg_attr(feature = "cmdline", value(name = "untouched"))]
    Untouched,
}

#[cfg(feature = "cmdline")]
impl std::fmt::Display for SpaceAroundColumn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use clap::ValueEnum;
        self.to_possible_value().expect("no values skipped").get_name().fmt(f)
    }
}

/// Controls the prefix or suffix used when reformatting hexadecimal literals.
/// TOML / CLI values: `"0x"`, `"0X"`, `"#"`, `"$"`, `"&"`, `"h"`, `"H"`, `"untouched"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "cmdline", derive(clap::ValueEnum))]
pub enum HexEncoding {
    #[serde(rename = "0x")]
    #[cfg_attr(feature = "cmdline", value(name = "0x"))]
    Prefix0x,
    #[serde(rename = "0X")]
    #[cfg_attr(feature = "cmdline", value(name = "0X"))]
    Prefix0X,
    #[serde(rename = "#")]
    #[cfg_attr(feature = "cmdline", value(name = "#"))]
    PrefixHash,
    #[serde(rename = "$")]
    #[cfg_attr(feature = "cmdline", value(name = "$"))]
    PrefixDollar,
    #[serde(rename = "&")]
    #[cfg_attr(feature = "cmdline", value(name = "&"))]
    PrefixAmp,
    #[serde(rename = "h")]
    #[cfg_attr(feature = "cmdline", value(name = "h"))]
    SuffixLower,
    #[serde(rename = "H")]
    #[cfg_attr(feature = "cmdline", value(name = "H"))]
    SuffixUpper,
    #[cfg_attr(feature = "cmdline", value(name = "untouched"))]
    Untouched,
}

#[cfg(feature = "cmdline")]
impl std::fmt::Display for HexEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use clap::ValueEnum;
        self.to_possible_value().expect("no values skipped").get_name().fmt(f)
    }
}

/// Controls the prefix used when reformatting octal literals.
/// TOML / CLI values: `"0o"`, `"0O"`, `"@"`, `"untouched"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "cmdline", derive(clap::ValueEnum))]
pub enum OctalEncoding {
    #[serde(rename = "0o")]
    #[cfg_attr(feature = "cmdline", value(name = "0o"))]
    Prefix0o,
    #[serde(rename = "0O")]
    #[cfg_attr(feature = "cmdline", value(name = "0O"))]
    Prefix0O,
    #[serde(rename = "@")]
    #[cfg_attr(feature = "cmdline", value(name = "@"))]
    PrefixAt,
    #[cfg_attr(feature = "cmdline", value(name = "untouched"))]
    Untouched,
}

#[cfg(feature = "cmdline")]
impl std::fmt::Display for OctalEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use clap::ValueEnum;
        self.to_possible_value().expect("no values skipped").get_name().fmt(f)
    }
}

/// Controls the prefix used when reformatting binary literals.
/// TOML / CLI values: `"0b"`, `"0B"`, `"%"`, `"untouched"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "cmdline", derive(clap::ValueEnum))]
pub enum BinaryEncoding {
    #[serde(rename = "0b")]
    #[cfg_attr(feature = "cmdline", value(name = "0b"))]
    Prefix0b,
    #[serde(rename = "0B")]
    #[cfg_attr(feature = "cmdline", value(name = "0B"))]
    Prefix0B,
    #[serde(rename = "%")]
    #[cfg_attr(feature = "cmdline", value(name = "%"))]
    PrefixPercent,
    #[cfg_attr(feature = "cmdline", value(name = "untouched"))]
    Untouched,
}

#[cfg(feature = "cmdline")]
impl std::fmt::Display for BinaryEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use clap::ValueEnum;
        self.to_possible_value().expect("no values skipped").get_name().fmt(f)
    }
}

/// Controls whether label definitions are emitted with or without a trailing `:`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "cmdline", derive(clap::ValueEnum))]
pub enum LabelPostfix {
    #[cfg_attr(feature = "cmdline", value(name = "no-column"))]
    NoColumn,
    #[cfg_attr(feature = "cmdline", value(name = "with-column"))]
    WithColumn,
    #[cfg_attr(feature = "cmdline", value(name = "untouched"))]
    Untouched,
}

#[cfg(feature = "cmdline")]
impl std::fmt::Display for LabelPostfix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use clap::ValueEnum;
        self.to_possible_value().expect("no values skipped").get_name().fmt(f)
    }
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
#[cfg_attr(feature = "cmdline", derive(clap::Args))]
pub struct AsmFormatOptions {
    /// Number of spaces per indentation level (default: 4).
    #[serde(default = "default_indent_size")]
    #[builder(default = default_indent_size())]
    #[cfg_attr(feature = "cmdline", arg(long, default_value_t = 4))]
    pub indent_size: usize,

    /// Minimum column (0-indexed) at which trailing comments start (default: 30).
    #[serde(default = "default_comment_column")]
    #[builder(default = default_comment_column())]
    #[cfg_attr(feature = "cmdline", arg(long, default_value_t = 30))]
    pub comment_column: usize,

    /// Case transformation applied to Z80 mnemonic keywords (LD, PUSH, …).
    #[serde(default = "default_mnemonic_case")]
    #[builder(default = default_mnemonic_case())]
    #[cfg_attr(feature = "cmdline", arg(long, default_value_t = CaseStyle::UpperCase))]
    pub mnemonic_case: CaseStyle,

    /// Case transformation applied to directive keywords (ORG, EQU, REPEAT, …).
    #[serde(default = "default_directive_case")]
    #[builder(default = default_directive_case())]
    #[cfg_attr(feature = "cmdline", arg(long, default_value_t = CaseStyle::UpperCase))]
    pub directive_case: CaseStyle,

    /// Case transformation applied to Z80 register names in operands.
    #[serde(default = "default_register_case")]
    #[builder(default = default_register_case())]
    #[cfg_attr(feature = "cmdline", arg(long, default_value_t = CaseStyle::UpperCase))]
    pub register_case: CaseStyle,

    /// Split multiple instructions on the same line into separate lines.
    #[serde(default = "default_one_instruction_per_line")]
    #[builder(default = default_one_instruction_per_line())]
    #[cfg_attr(feature = "cmdline", arg(long, action = clap::ArgAction::Set, default_value_t = true))]
    pub one_instruction_per_line: bool,

    /// Spacing around `:` instruction separators (only when one-instruction-per-line=false).
    #[serde(default = "default_space_around_column")]
    #[builder(default = default_space_around_column())]
    #[cfg_attr(feature = "cmdline", arg(long, default_value_t = SpaceAroundColumn::Untouched))]
    pub space_around_column: SpaceAroundColumn,

    /// Spacing around assignment operators (`=`, `+=`, `>>=`, …).
    #[serde(default = "default_space_around_assignment")]
    #[builder(default = default_space_around_assignment())]
    #[cfg_attr(feature = "cmdline", arg(long, default_value_t = SpaceAroundColumn::Untouched))]
    pub space_around_assignment: SpaceAroundColumn,

    /// Case applied to A-F letters inside hex literals.
    #[serde(default = "default_hexadecimal_case")]
    #[builder(default = default_hexadecimal_case())]
    #[cfg_attr(feature = "cmdline", arg(long, default_value_t = CaseStyle::Untouched))]
    pub hexadecimal_case: CaseStyle,

    /// Prefix/suffix form used when reformatting hex literals.
    #[serde(default = "default_hexadecimal_encoding")]
    #[builder(default = default_hexadecimal_encoding())]
    #[cfg_attr(feature = "cmdline", arg(long, default_value_t = HexEncoding::Untouched))]
    pub hexadecimal_encoding: HexEncoding,

    /// Prefix form used when reformatting octal literals.
    #[serde(default = "default_octal_encoding")]
    #[builder(default = default_octal_encoding())]
    #[cfg_attr(feature = "cmdline", arg(long, default_value_t = OctalEncoding::Untouched))]
    pub octal_encoding: OctalEncoding,

    /// Prefix form used when reformatting binary literals.
    #[serde(default = "default_binary_encoding")]
    #[builder(default = default_binary_encoding())]
    #[cfg_attr(feature = "cmdline", arg(long, default_value_t = BinaryEncoding::Untouched))]
    pub binary_encoding: BinaryEncoding,

    /// Whether label definitions are emitted with or without a trailing `:`.
    #[serde(default = "default_label_postfix")]
    #[builder(default = default_label_postfix())]
    #[cfg_attr(feature = "cmdline", arg(long, default_value_t = LabelPostfix::WithColumn))]
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
