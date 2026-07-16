//! Best-effort flag/option completion for *delegated* bndbuild task commands
//! (third-party binaries like rasm, sjasmplus, ace, martine, ... that bndbuild
//! downloads/installs on demand - as opposed to "internal" commands, which are
//! completed from their real `clap::Command` via `super::internal_commands`).
//!
//! There is no `clap::Command` available for these tools, so instead this runs
//! `<command> --help` through bndbuild's own task execution machinery (the same
//! path `bndbuild --help <command>` uses, see `cpclib_bndbuild::app`'s
//! `execute_help`) and scrapes option flags out of the captured help text.
//!
//! Safety/UX constraints this deliberately respects:
//! - Only runs when the tool is **already installed/cached locally**
//!   (`DelegateApplicationDescription::is_cached()`); never triggers a download
//!   from an editor completion request.
//! - Spawning the process is a blocking call, so the result is memoized per
//!   command name for the lifetime of the LSP process - it only runs once per
//!   distinct delegated command, not on every keystroke.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, LazyLock, Mutex};

use cpclib_bndbuild::task::Task;
use cpclib_common::event::CapturingObserver;

/// Parse option flags out of a chunk of `--help` output.
///
/// Heuristic (deliberately simple - this is best-effort scraping of arbitrary
/// third-party help text, not a real argument-syntax parser): for each line,
/// strip leading whitespace; if what remains starts with `-`, the leading run
/// of whitespace-separated tokens that look like flags (start with `-`, a
/// trailing `,` is stripped so "`-h,`" next to "`--help`" is handled) are the
/// option(s) for that line, and whatever text follows them (if any) is treated
/// as their description/comment.
fn parse_help_options(text: &str) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('-') {
            continue;
        }

        let mut tokens = trimmed.split_whitespace().peekable();
        let mut flags = Vec::new();
        while let Some(tok) = tokens.peek() {
            let candidate = tok.trim_end_matches(',');
            if candidate.starts_with('-') && candidate.len() > 1 {
                flags.push(candidate.to_string());
                tokens.next();
            }
            else {
                break;
            }
        }

        if flags.is_empty() {
            continue;
        }

        let comment: Vec<&str> = tokens.collect();
        let comment = if comment.is_empty() {
            None
        }
        else {
            Some(comment.join(" "))
        };

        for flag in flags {
            out.push((flag, comment.clone()));
        }
    }

    out.sort();
    out.dedup();
    out
}

/// Actually fetch `<name> --help` (only if already installed) and parse it.
/// Returns an empty vec on any failure (unknown command, not cached, process
/// error, ...) - there is simply nothing to offer in that case, no worse than
/// today's lack of argument completion for delegated commands.
fn fetch_and_parse_help(name: &str) -> Vec<(String, Option<String>)> {
    let Ok(task) = Task::from_str(&format!("{name} --help"))
    else {
        return Vec::new();
    };

    let Some(conf) = task.configuration::<CapturingObserver>()
    else {
        return Vec::new(); // not actually a delegated command
    };

    if !conf.is_cached() {
        // Never trigger a download from an editor completion request.
        return Vec::new();
    }

    let observer = Arc::new(CapturingObserver::new());
    let _ = task.execute(&observer); // `--help` commonly exits non-zero; ignore the Result

    let text = format!("{}\n{}", observer.stdout_joined(), observer.stderr_joined());
    parse_help_options(&text)
}

static HELP_CACHE: LazyLock<Mutex<HashMap<String, Vec<(String, Option<String>)>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Returns the `(flag, description)` pairs scraped from `<name> --help`,
/// memoized after the first call. Empty when the tool isn't a recognized
/// delegated command, isn't installed locally, or its help text had no
/// recognizable option lines.
pub fn get_completions_for(name: &str) -> Vec<(String, Option<String>)> {
    let mut cache = HELP_CACHE.lock().unwrap();
    if let Some(cached) = cache.get(name) {
        return cached.clone();
    }
    let result = fetch_and_parse_help(name);
    cache.insert(name.to_string(), result.clone());
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_short_and_long_flags_with_comment() {
        let text = "  -h, --help     Print help\n  -V, --version  Print version\n";
        let result = parse_help_options(text);
        assert!(result.contains(&("-h".to_string(), Some("Print help".to_string()))));
        assert!(result.contains(&("--help".to_string(), Some("Print help".to_string()))));
        assert!(result.contains(&("-V".to_string(), Some("Print version".to_string()))));
        assert!(result.contains(&("--version".to_string(), Some("Print version".to_string()))));
    }

    #[test]
    fn parses_flag_with_no_comment() {
        let text = "    --sna\n";
        let result = parse_help_options(text);
        assert_eq!(result, vec![("--sna".to_string(), None)]);
    }

    #[test]
    fn ignores_non_option_lines() {
        let text = "Usage: rasm <inputfile> [options]\n\nOptions:\n  -o <file>    output file\n";
        let result = parse_help_options(text);
        assert_eq!(
            result,
            vec![("-o".to_string(), Some("<file> output file".to_string()))]
        );
    }

    #[test]
    fn unknown_or_uninstalled_command_yields_no_completions() {
        // In a test environment this tool is neither a recognized command with
        // a matching name, nor (if it were) plausibly installed/cached, so this
        // must never spawn a process or attempt a download.
        assert_eq!(get_completions_for("not-a-real-command-xyz"), Vec::new());
    }
}
