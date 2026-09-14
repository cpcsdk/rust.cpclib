use std::process;

use clap::Parser;
use cpclib_basmopt::cli::Cli;
use cpclib_basmopt::{Suggestion, analyze_file, apply_fixes_in_place};

fn print_suggestion(source: &camino::Utf8Path, s: &Suggestion) {
    let rule = s.rule_name.as_deref().unwrap_or("<unnamed>");
    println!("{}:{}:{}: [{rule}] {}", source, s.line, s.column, s.message);
    // Indented under the finding: what makes it safe, and where to look. A
    // suggestion nobody can check is a suggestion nobody should apply.
    for reason in &s.reasons {
        match (reason.line, reason.column) {
            (Some(line), Some(column)) => {
                println!("    because {} (at {}:{})", reason.text, line, column)
            },
            _ => println!("    because {}", reason.text)
        }
    }
}

fn run() -> i32 {
    let cli = Cli::parse();

    if cli.in_place {
        return run_in_place(&cli);
    }

    let outcome = match analyze_file(&cli.source, &cli.options()) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let cpclib_basmopt::AnalyzeOutcome {
        source: _,
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
        println!("{}: no optimization opportunities found", cli.source);
        return 0;
    }

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

/// `-i`/`--in-place`: analyze-and-apply, up to a fixed 2 passes so a fix one
/// pass applies that exposes a genuinely new opportunity gets caught in the
/// same run - see `apply_fixes_in_place`'s own doc comment. Only bulk-safe
/// suggestions ever get applied unreviewed - see
/// `cpclib_asmoptim::engine::PeepholeMatch::bulk_unsafe`'s own doc comment
/// for why: an instruction whose entire output is dead is, on the CPC,
/// plausibly deliberate timing padding, and `-i` applies every match with
/// nobody looking at each site. The plain (non-`-i`) report still lists
/// them - a human reading it can judge each one individually, which is
/// exactly what a bulk rewrite cannot do.
fn run_in_place(cli: &Cli) -> i32 {
    let outcome = match apply_fixes_in_place(&cli.source, &cli.options()) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    if let Some(warning) = &outcome.assemble_warning {
        eprintln!(
            "warning: {}: could not fully assemble, address-aware suggestions skipped: {warning}",
            cli.source
        );
    }

    // Silent on nothing found, matching this flag's own established
    // behavior (the plain report path is where "nothing found" gets
    // announced) - useful for scripting `-i` over many files unattended.
    if outcome.total_applied == 0 && outcome.remaining_skipped == 0 {
        return 0;
    }

    if let Err(e) = fs_err::write(&cli.source, &outcome.final_source) {
        eprintln!("error: cannot write {}: {e}", cli.source);
        return 2;
    }
    let passes = if outcome.passes_run > 1 {
        format!(" across {} passes", outcome.passes_run)
    }
    else {
        String::new()
    };
    println!(
        "{}: applied {} fix{}{passes}",
        cli.source,
        outcome.total_applied,
        if outcome.total_applied == 1 { "" } else { "es" }
    );
    if outcome.remaining_skipped > 0 {
        println!(
            "{}: {} suggestion{} skipped (needs individual review - rerun without -i to see \
             {})",
            cli.source,
            outcome.remaining_skipped,
            if outcome.remaining_skipped == 1 { "" } else { "s" },
            if outcome.remaining_skipped == 1 { "it" } else { "them" }
        );
    }
    0
}

fn main() {
    process::exit(run());
}
