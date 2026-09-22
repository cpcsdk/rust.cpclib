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
        omit a flag to keep the config file value for that option.\n    \
        \n    \
        A top-level `ignore = [\"glob\", ...]` array (paths relative to the config\n    \
        file's own directory) excludes matching files from a run entirely - no flag\n    \
        equivalent, config file only, the same convention rustfmt.toml itself uses."
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

    /// Print a diff of what would change instead of the formatted output (or rewriting
    /// in place). Implies the same non-zero exit code as `--check` when anything differs.
    #[arg(long)]
    pub diff: bool,

    /// Format only lines START:END (1-based, inclusive); everything else is left
    /// untouched, the same mechanism `; fmt: off`/`on` uses internally. Refused when more
    /// than one file is given (each would need its own range); fine with a single file or
    /// with stdin.
    #[arg(long, value_name = "START:END")]
    pub lines: Option<String>,

    #[command(flatten)]
    pub options: AsmFormatOptions
}

/// Merge `base` (from config file) with options explicitly set on the command line.
///
/// Fields not present on the command line keep the `base` value, so config file values
/// are never silently overridden by clap defaults.
pub fn apply_cli_overrides(base: AsmFormatOptions, matches: &clap::ArgMatches) -> AsmFormatOptions {
    use clap::FromArgMatches;
    use clap::parser::ValueSource;
    let explicit =
        |name: &str| matches!(matches.value_source(name), Some(ValueSource::CommandLine));
    let cli = AsmFormatOptions::from_arg_matches(matches).unwrap_or_default();
    AsmFormatOptions {
        indent_size: if explicit("indent_size") {
            cli.indent_size
        }
        else {
            base.indent_size
        },
        comment_column: if explicit("comment_column") {
            cli.comment_column
        }
        else {
            base.comment_column
        },
        mnemonic_case: if explicit("mnemonic_case") {
            cli.mnemonic_case
        }
        else {
            base.mnemonic_case
        },
        directive_case: if explicit("directive_case") {
            cli.directive_case
        }
        else {
            base.directive_case
        },
        register_case: if explicit("register_case") {
            cli.register_case
        }
        else {
            base.register_case
        },
        one_instruction_per_line: if explicit("one_instruction_per_line") {
            cli.one_instruction_per_line
        }
        else {
            base.one_instruction_per_line
        },
        space_around_column: if explicit("space_around_column") {
            cli.space_around_column
        }
        else {
            base.space_around_column
        },
        space_around_assignment: if explicit("space_around_assignment") {
            cli.space_around_assignment
        }
        else {
            base.space_around_assignment
        },
        space_around_comma: if explicit("space_around_comma") {
            cli.space_around_comma
        }
        else {
            base.space_around_comma
        },
        quote_style: if explicit("quote_style") {
            cli.quote_style
        }
        else {
            base.quote_style
        },
        max_consecutive_blank_lines: if explicit("max_consecutive_blank_lines") {
            cli.max_consecutive_blank_lines
        }
        else {
            base.max_consecutive_blank_lines
        },
        hexadecimal_case: if explicit("hexadecimal_case") {
            cli.hexadecimal_case
        }
        else {
            base.hexadecimal_case
        },
        hexadecimal_encoding: if explicit("hexadecimal_encoding") {
            cli.hexadecimal_encoding
        }
        else {
            base.hexadecimal_encoding
        },
        octal_encoding: if explicit("octal_encoding") {
            cli.octal_encoding
        }
        else {
            base.octal_encoding
        },
        binary_encoding: if explicit("binary_encoding") {
            cli.binary_encoding
        }
        else {
            base.binary_encoding
        },
        label_definition_postfix_with_column: if explicit("label_definition_postfix_with_column") {
            cli.label_definition_postfix_with_column
        }
        else {
            base.label_definition_postfix_with_column
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    /// `apply_cli_overrides` used to look up `matches.value_source(...)` by
    /// the kebab-case *flag* name (`"indent-size"`), but a clap-derive
    /// field's `Id` is its Rust identifier (`"indent_size"`) unless
    /// explicitly renamed - every lookup here panicked, on every real
    /// invocation (`main.rs` calls this unconditionally), regardless of
    /// which flags were actually given. Exercises the real path
    /// (`Cli::command().try_get_matches_from`, exactly what `main.rs`
    /// does), not just `AsmFormatOptions::from_arg_matches` directly, which
    /// is what let this ship unnoticed - that call alone never touches
    /// `value_source` at all.
    #[test]
    fn apply_cli_overrides_does_not_panic_with_no_flags() {
        let matches = Cli::command().try_get_matches_from(["basm-fmt", "file.asm"]).unwrap();
        let base = AsmFormatOptions::default();
        let merged = apply_cli_overrides(base.clone(), &matches);
        assert_eq!(merged.indent_size, base.indent_size, "no flag given - the config-file base must survive untouched");
    }

    /// An explicitly-given flag must still win over the config-file base -
    /// the actual behavior `apply_cli_overrides` exists for.
    #[test]
    fn apply_cli_overrides_lets_an_explicit_flag_win_over_the_base() {
        let matches = Cli::command()
            .try_get_matches_from(["basm-fmt", "--indent-size", "2", "file.asm"])
            .unwrap();
        let base = AsmFormatOptions { indent_size: 8, ..Default::default() };
        let merged = apply_cli_overrides(base, &matches);
        assert_eq!(merged.indent_size, 2);
    }
}
