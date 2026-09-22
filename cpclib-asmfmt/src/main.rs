use std::io::Read;
use std::path::{Path, PathBuf};
use std::process;

use clap::CommandFactory;
use cpclib_asmfmt::AsmFormatOptions;
use cpclib_asmfmt::cli::{Cli, apply_cli_overrides};

fn format_source(
    source: &str,
    options: &AsmFormatOptions,
    lines: Option<(usize, usize)>
) -> Result<String, String> {
    match lines {
        Some((start, end)) => {
            cpclib_asmfmt::format_range(source, options, start, end).map_err(|e| e.to_string())
        },
        None => cpclib_asmfmt::format(source, options).map_err(|e| e.to_string())
    }
}

// Parse `"START:END"` (1-based, inclusive) as given to `--lines`.
fn parse_lines_arg(raw: &str) -> Result<(usize, usize), String> {
    let (start, end) = raw
        .split_once(':')
        .ok_or_else(|| format!("--lines expects START:END, got {raw:?}"))?;
    let start: usize = start
        .trim()
        .parse()
        .map_err(|_| format!("--lines: invalid start line {start:?}"))?;
    let end: usize = end
        .trim()
        .parse()
        .map_err(|_| format!("--lines: invalid end line {end:?}"))?;
    if start == 0 || end == 0 {
        return Err("--lines: line numbers are 1-based, 0 is not valid".to_string());
    }
    if start > end {
        return Err(format!("--lines: start ({start}) must not be after end ({end})"));
    }
    Ok((start, end))
}

// Print what changed between `source` and `formatted` as a unified-style diff
// instead of the formatted text itself - the same idea as `rustfmt
// --check --color` or `git diff`, for reviewing a reformat before trusting
// it wholesale.
fn print_diff(label: &str, source: &str, formatted: &str) {
    let original_name = format!("{label} (original)");
    let formatted_name = format!("{label} (formatted)");
    let diff = prettydiff::diff_lines(source, formatted)
        .names(&original_name, &formatted_name)
        .set_show_lines(true)
        .format();
    println!("{diff}");
}

struct RunConfig {
    files: Vec<PathBuf>,
    inplace: bool,
    check: bool,
    diff: bool,
    lines: Option<(usize, usize)>,
    options: AsmFormatOptions
}

fn process_stdin(cfg: &RunConfig) -> Result<bool, String> {
    let mut source = String::new();
    std::io::stdin()
        .read_to_string(&mut source)
        .map_err(|e| format!("cannot read stdin: {e}"))?;
    let formatted = format_source(&source, &cfg.options, cfg.lines)?;
    if cfg.diff {
        if formatted != source {
            print_diff("<stdin>", &source, &formatted);
            return Ok(false);
        }
    }
    else if cfg.check {
        if formatted != source {
            eprintln!("<stdin> would be reformatted");
            return Ok(false);
        }
    }
    else {
        print!("{formatted}");
    }
    Ok(true)
}

fn process_file(path: &Path, cfg: &RunConfig) -> Result<bool, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let formatted = format_source(&source, &cfg.options, cfg.lines)?;
    if cfg.diff {
        if formatted != source {
            print_diff(&path.display().to_string(), &source, &formatted);
            return Ok(false);
        }
    }
    else if cfg.check {
        if formatted != source {
            eprintln!("{}: would be reformatted", path.display());
            return Ok(false);
        }
    }
    else if cfg.inplace {
        if formatted != source {
            std::fs::write(path, &formatted)
                .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        }
    }
    else {
        print!("{formatted}");
    }
    Ok(true)
}

fn run() -> i32 {
    let matches = Cli::command().get_matches();

    let config_path = cpclib_asmfmt::find_config_file();
    let base = match &config_path {
        None => AsmFormatOptions::default(),
        Some(path) => {
            match cpclib_asmfmt::load_config_from(path) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("warning: {}: {e}", path.display());
                    AsmFormatOptions::default()
                }
            }
        },
    };

    // Patterns are relative to the config file's own directory (rustfmt's
    // own convention for its `ignore` key). No config file found at all
    // means nothing to ignore, same as an empty list.
    let ignore_matcher = config_path.as_ref().and_then(|path| {
        let patterns = cpclib_asmfmt::load_ignore_patterns_from(path);
        if patterns.is_empty() {
            return None;
        }
        let base_dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
        let mut builder = globset::GlobSetBuilder::new();
        for pattern in &patterns {
            match globset::Glob::new(pattern) {
                Ok(g) => {
                    builder.add(g);
                },
                Err(e) => eprintln!("warning: {}: invalid ignore pattern {pattern:?}: {e}", path.display())
            }
        }
        builder.build().ok().map(|set| (base_dir, set))
    });

    let options = apply_cli_overrides(base, &matches);
    let given_files: Vec<PathBuf> = matches
        .get_many::<PathBuf>("files")
        .unwrap_or_default()
        .cloned()
        .collect();
    let files: Vec<PathBuf> = given_files
        .iter()
        .filter(|f| {
            let Some((base_dir, set)) = &ignore_matcher
            else {
                return true;
            };
            let rel = f.strip_prefix(base_dir).unwrap_or(f);
            !set.is_match(rel)
        })
        .cloned()
        .collect();
    // Files were explicitly given but every one of them was ignored - that's
    // "nothing to do", not "read from stdin instead" (falling through to the
    // stdin path here would otherwise hang waiting for input that was never
    // meant to be piped in).
    if !given_files.is_empty() && files.is_empty() {
        return 0;
    }
    let inplace = matches.get_flag("inplace");
    let check = matches.get_flag("check");
    let diff = matches.get_flag("diff");

    let lines = match matches.get_one::<String>("lines") {
        None => None,
        Some(raw) => {
            // A single range can only ever apply to one source - given more
            // than one file it's ambiguous which file's own line numbers are
            // meant, so refuse rather than silently apply the same range to
            // every file.
            if files.len() > 1 {
                eprintln!("error: --lines can only be used with a single input file");
                return 2;
            }
            match parse_lines_arg(raw) {
                Ok(range) => Some(range),
                Err(e) => {
                    eprintln!("error: {e}");
                    return 2;
                }
            }
        }
    };

    let cfg = RunConfig {
        files,
        inplace,
        check,
        diff,
        lines,
        options
    };

    if cfg.files.is_empty() {
        match process_stdin(&cfg) {
            Ok(true) => return 0,
            Ok(false) => return 1,
            Err(e) => {
                eprintln!("error: {e}");
                return 2;
            }
        }
    }

    let mut all_ok = true;
    for path in &cfg.files {
        if path.as_os_str() == "-" {
            match process_stdin(&cfg) {
                Ok(true) => {},
                Ok(false) => all_ok = false,
                Err(e) => {
                    eprintln!("error: {e}");
                    all_ok = false;
                }
            }
        }
        else {
            match process_file(path, &cfg) {
                Ok(true) => {},
                Ok(false) => all_ok = false,
                Err(e) => {
                    eprintln!("error: {e}");
                    all_ok = false;
                }
            }
        }
    }

    if all_ok { 0 } else { 1 }
}

fn main() {
    process::exit(run());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lines_arg_accepts_a_valid_range() {
        assert_eq!(parse_lines_arg("2:5"), Ok((2, 5)));
        assert_eq!(parse_lines_arg("3:3"), Ok((3, 3)));
        assert_eq!(parse_lines_arg(" 2 : 5 "), Ok((2, 5)));
    }

    #[test]
    fn parse_lines_arg_rejects_a_backwards_range() {
        assert!(parse_lines_arg("5:2").is_err());
    }

    #[test]
    fn parse_lines_arg_rejects_zero_line_numbers() {
        assert!(parse_lines_arg("0:5").is_err());
        assert!(parse_lines_arg("1:0").is_err());
    }

    #[test]
    fn parse_lines_arg_rejects_garbage() {
        assert!(parse_lines_arg("abc").is_err());
        assert!(parse_lines_arg("2-5").is_err());
        assert!(parse_lines_arg("2:x").is_err());
    }
}
