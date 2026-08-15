//! Turning a rule that *runs* a program into one that *debugs* it.
//!
//! A project already has a rule that launches the emulator - `emu --snapshot
//! demo.sna run`. Asking to debug it should not mean writing a second rule that
//! drifts from the first, so the existing one is rewritten: `run` becomes
//! `debug`, and the emulator is forced to the one that can be debugged.
//!
//! The rewrite works on **argv**, never on the YAML text. A build file is a
//! template, its commands are quoted, and a textual substitution would corrupt
//! any of `--emulator=ace`, `--emu ace` or a path containing the word `run`.

/// The emulator a debug session needs: the only one that speaks DAP.
pub const DEBUG_EMULATOR: &str = "1984js";

/// Every spelling of the emulator option, so an existing choice is replaced
/// rather than duplicated.
const EMULATOR_FLAGS: &[&str] = &["-e", "--emulator", "--emu"];

/// Rewrite one `emu ...` argument list for debugging.
///
/// Returns `None` when the command is not an emulator invocation with a `run`
/// subcommand - there is nothing to debug in that case, and guessing would be
/// worse than declining.
pub fn debug_arguments(arguments: &str) -> Option<String> {
    let mut argv = shlex::split(arguments)?;

    // The subcommand is the last bare word; `run` is what we replace.
    let run_position = argv.iter().rposition(|a| a == "run")?;
    argv[run_position] = "debug".to_string();

    // Drop any existing emulator choice, in every spelling it can take.
    let mut cleaned: Vec<String> = Vec::with_capacity(argv.len() + 2);
    let mut skip_next = false;
    for argument in argv {
        if skip_next {
            skip_next = false;
            continue;
        }
        if EMULATOR_FLAGS.contains(&argument.as_str()) {
            skip_next = true; // its value follows
            continue;
        }
        if EMULATOR_FLAGS
            .iter()
            .any(|flag| argument.starts_with(&format!("{flag}=")))
        {
            continue;
        }
        cleaned.push(argument);
    }

    // ...and put ours in, before the subcommand so it reads as an option.
    let subcommand = cleaned.pop()?;
    cleaned.push("--emulator".to_string());
    cleaned.push(DEBUG_EMULATOR.to_string());
    cleaned.push(subcommand);

    Some(shlex::try_join(cleaned.iter().map(String::as_str)).ok()?)
}

/// The snapshot a rewritten command will boot, if it names one.
///
/// The adapter needs it to serve the program to the emulator, and reading it
/// off the rule means the user does not have to state it twice.
pub fn snapshot_of(arguments: &str) -> Option<String> {
    let argv = shlex::split(arguments)?;
    let mut iter = argv.iter();
    while let Some(argument) = iter.next() {
        if argument == "--snapshot" {
            return iter.next().cloned();
        }
        if let Some(value) = argument.strip_prefix("--snapshot=") {
            return Some(value.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_becomes_debug_and_the_emulator_is_forced() {
        assert_eq!(
            debug_arguments("--snapshot demo.sna run").as_deref(),
            Some("--snapshot demo.sna --emulator 1984js debug")
        );
    }

    /// An emulator the rule already chose is replaced, not duplicated - in
    /// every spelling the CLI accepts.
    #[test]
    fn an_existing_emulator_choice_is_replaced() {
        for original in [
            "--snapshot d.sna --emulator ace run",
            "--snapshot d.sna --emu ace run",
            "--snapshot d.sna -e ace run",
            "--snapshot d.sna --emulator=ace run"
        ] {
            let rewritten = debug_arguments(original).expect("rewrites");
            assert!(
                !rewritten.contains("ace"),
                "{original} still selects ace: {rewritten}"
            );
            assert_eq!(rewritten.matches("1984js").count(), 1, "{rewritten}");
            assert!(rewritten.ends_with("debug"), "{rewritten}");
        }
    }

    /// The real rules in the wild carry more than a snapshot.
    #[test]
    fn other_options_survive_untouched() {
        let rewritten =
            debug_arguments("--snapshot polar.sna --debug polar.rasm --background run").unwrap();
        assert!(rewritten.contains("--debug polar.rasm"));
        assert!(rewritten.contains("--background"));
        assert!(rewritten.contains("--snapshot polar.sna"));
    }

    /// A path that merely contains the word must not be mistaken for the
    /// subcommand - the reason this works on argv and not on text.
    #[test]
    fn a_path_containing_run_is_not_the_subcommand() {
        let rewritten = debug_arguments("--snapshot build/run/demo.sna run").unwrap();
        assert!(rewritten.contains("build/run/demo.sna"), "{rewritten}");
        assert!(rewritten.ends_with("debug"), "{rewritten}");
    }

    /// Quoted arguments survive the round trip.
    #[test]
    fn a_quoted_path_survives() {
        let rewritten = debug_arguments("--snapshot 'my demo.sna' run").unwrap();
        assert_eq!(snapshot_of(&rewritten).as_deref(), Some("my demo.sna"));
    }

    /// Nothing to debug: decline rather than invent.
    #[test]
    fn a_command_without_run_is_declined() {
        assert!(debug_arguments("--snapshot d.sna orgams").is_none());
        assert!(debug_arguments("").is_none());
    }

    #[test]
    fn the_snapshot_is_read_back_in_both_spellings() {
        assert_eq!(
            snapshot_of("--snapshot demo.sna run").as_deref(),
            Some("demo.sna")
        );
        assert_eq!(
            snapshot_of("--snapshot=demo.sna run").as_deref(),
            Some("demo.sna")
        );
        assert_eq!(snapshot_of("--drivea d.dsk run"), None);
    }
}
