use std::path::PathBuf;

use crate::options::AsmFormatOptions;

#[derive(clap::Parser, Debug)]
#[command(
    name = "basm-fmt",
    about = "Z80 assembly formatter for basm",
    after_help = "CONFIGURATION FILE:\n    \
        basm-fmt searches for `basm-fmt.toml` starting from the current directory,\n    \
        walking up to the filesystem root, then in $XDG_CONFIG_HOME/basm-fmt/.\n    \
        When found, the file is loaded first as the base configuration.\n    \
        Flags given on the command line override individual options from the file;\n    \
        omit a flag to keep the config file value for that option."
)]
pub struct Cli {
    /// Source files to format. Use `-` to read from stdin.
    pub files: Vec<PathBuf>,

    /// Rewrite files in-place instead of writing to stdout.
    #[arg(short = 'i', long)]
    pub inplace: bool,

    /// Exit with a non-zero code if any file would be reformatted (do not write).
    #[arg(short = 'c', long)]
    pub check: bool,

    #[command(flatten)]
    pub options: AsmFormatOptions,
}

/// Merge `base` (from config file) with options explicitly set on the command line.
///
/// Fields not present on the command line keep the `base` value, so config file values
/// are never silently overridden by clap defaults.
pub fn apply_cli_overrides(base: AsmFormatOptions, matches: &clap::ArgMatches) -> AsmFormatOptions {
    use clap::{FromArgMatches, parser::ValueSource};
    let explicit = |name: &str| matches!(matches.value_source(name), Some(ValueSource::CommandLine));
    let cli = AsmFormatOptions::from_arg_matches(matches).unwrap_or_default();
    AsmFormatOptions {
        indent_size:                        if explicit("indent-size")                          { cli.indent_size }                        else { base.indent_size },
        comment_column:                     if explicit("comment-column")                       { cli.comment_column }                     else { base.comment_column },
        mnemonic_case:                      if explicit("mnemonic-case")                        { cli.mnemonic_case }                      else { base.mnemonic_case },
        directive_case:                     if explicit("directive-case")                       { cli.directive_case }                     else { base.directive_case },
        register_case:                      if explicit("register-case")                        { cli.register_case }                      else { base.register_case },
        one_instruction_per_line:           if explicit("one-instruction-per-line")             { cli.one_instruction_per_line }           else { base.one_instruction_per_line },
        space_around_column:                if explicit("space-around-column")                  { cli.space_around_column }                else { base.space_around_column },
        space_around_assignment:            if explicit("space-around-assignment")              { cli.space_around_assignment }            else { base.space_around_assignment },
        hexadecimal_case:                   if explicit("hexadecimal-case")                     { cli.hexadecimal_case }                   else { base.hexadecimal_case },
        hexadecimal_encoding:               if explicit("hexadecimal-encoding")                 { cli.hexadecimal_encoding }               else { base.hexadecimal_encoding },
        octal_encoding:                     if explicit("octal-encoding")                       { cli.octal_encoding }                     else { base.octal_encoding },
        binary_encoding:                    if explicit("binary-encoding")                      { cli.binary_encoding }                    else { base.binary_encoding },
        label_definition_postfix_with_column: if explicit("label-definition-postfix-with-column") { cli.label_definition_postfix_with_column } else { base.label_definition_postfix_with_column },
    }
}
