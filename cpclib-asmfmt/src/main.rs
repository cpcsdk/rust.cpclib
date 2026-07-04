use std::io::Read;
use std::path::{Path, PathBuf};
use std::process;

use clap::CommandFactory;
use cpclib_asmfmt::AsmFormatOptions;
use cpclib_asmfmt::cli::{Cli, apply_cli_overrides};

fn format_source(source: &str, options: &AsmFormatOptions) -> Result<String, String> {
    cpclib_asmfmt::format(source, options).map_err(|e| e.to_string())
}

struct RunConfig {
    files: Vec<PathBuf>,
    inplace: bool,
    check: bool,
    options: AsmFormatOptions
}

fn process_stdin(cfg: &RunConfig) -> Result<bool, String> {
    let mut source = String::new();
    std::io::stdin()
        .read_to_string(&mut source)
        .map_err(|e| format!("cannot read stdin: {e}"))?;
    let formatted = format_source(&source, &cfg.options)?;
    if cfg.check {
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
    let formatted = format_source(&source, &cfg.options)?;
    if cfg.check {
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

    let base = match cpclib_asmfmt::find_config_file() {
        None => AsmFormatOptions::default(),
        Some(path) => {
            match cpclib_asmfmt::load_config_from(&path) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("warning: {}: {e}", path.display());
                    AsmFormatOptions::default()
                }
            }
        },
    };

    let options = apply_cli_overrides(base, &matches);
    let files: Vec<PathBuf> = matches
        .get_many::<PathBuf>("files")
        .unwrap_or_default()
        .cloned()
        .collect();
    let inplace = matches.get_flag("inplace");
    let check = matches.get_flag("check");

    let cfg = RunConfig {
        files,
        inplace,
        check,
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
