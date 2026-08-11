use std::process;

use clap::Parser;
use cpclib_basmopt::cli::Cli;
use cpclib_basmopt::{Suggestion, analyze_file, apply_fixes};

fn print_suggestion(source: &camino::Utf8Path, s: &Suggestion) {
    let rule = s.rule_name.as_deref().unwrap_or("<unnamed>");
    println!("{}:{}:{}: [{rule}] {}", source, s.line, s.column, s.message);
}

fn run() -> i32 {
    let cli = Cli::parse();

    let outcome = match analyze_file(&cli.source, &cli.options()) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let cpclib_basmopt::AnalyzeOutcome {
        source,
        suggestions,
        assemble_warning
    } = outcome;
    if let Some(warning) = &assemble_warning {
        eprintln!(
            "warning: {}: could not fully assemble, address-aware suggestions skipped: {warning}",
            cli.source
        );
    }

    if suggestions.is_empty() {
        if !cli.in_place {
            println!("{}: no optimization opportunities found", cli.source);
        }
        return 0;
    }

    if cli.in_place {
        let fixed = apply_fixes(&source, &suggestions);
        if let Err(e) = fs_err::write(&cli.source, fixed) {
            eprintln!("error: cannot write {}: {e}", cli.source);
            return 2;
        }
        println!(
            "{}: applied {} fix{}",
            cli.source,
            suggestions.len(),
            if suggestions.len() == 1 { "" } else { "es" }
        );
        0
    }
    else {
        for s in &suggestions {
            print_suggestion(&cli.source, s);
        }
        println!(
            "{} optimization opportunit{} found",
            suggestions.len(),
            if suggestions.len() == 1 { "y" } else { "ies" }
        );
        1
    }
}

fn main() {
    process::exit(run());
}
