//! Build-progress tracking shared by every caller that drives a
//! [`crate::builder::BndBuilder`] and wants to report how far along it is -
//! the LSP server (`$/progress`), the `bndbuild` CLI's own `--progress`
//! flag, and `cpclib-dap`'s "preparing the debug session" step. Lives here
//! (rather than in any one of those callers) because none of them may
//! depend on another: `cpclib-lsp` depends on `cpclib-dap`, so anything
//! `cpclib-dap` needs cannot live inside `cpclib-lsp`, and this crate is the
//! one thing all three already depend on.

/// A build-progress update, combining bndbuild's own rule-level "step N of
/// M" signal with basm's finer-grained internal phase events (parse/load/
/// pass/save), so the consumer can report one coherent progress bar instead
/// of an indeterminate spinner for the whole build.
#[derive(Debug, Clone)]
pub enum ProgressUpdate {
    /// A new rule (of `out_of` total in this build) started running.
    Rule {
        rule: String,
        nb: usize,
        out_of: usize
    },
    /// A task within the current rule started - `command` is its exact,
    /// already-expanded command text (e.g. `basm main.asm -o main.bin`),
    /// the same text a terminal's own `$ ...` echo shows. Surfaced
    /// explicitly rather than left implicit in the rule name: knowing
    /// *which rule* is building doesn't tell you *what command* is
    /// actually running, and that's the thing worth seeing at a glance
    /// without digging through the terminal's own scrollback.
    Task { command: String },
    /// basm's own internal progress within the rule currently running.
    Asm(cpclib_asm::progress::AsmProgressEvent)
}

/// Turns a stream of [`ProgressUpdate`]s into `(message, percentage)` pairs
/// for a client's own progress UI (an LSP `WorkDoneProgressReport`, a DAP
/// `progressUpdate` event, or a parsed line from the CLI's `--progress`
/// marker output), tracked across the whole build (one instance per build,
/// fed every update in order).
///
/// The percentage is driven primarily by the rule-level "step N of M"
/// signal: rule `nb` (1-based, out of `out_of`) owns the percentage slot
/// `[(nb-1)/out_of, nb/out_of)`. basm's own parse/load/save events (which do
/// carry a real done/total) interpolate within that slot; `PassStarted`
/// carries no known total (basm iterates until addresses stabilize, so the
/// number of passes isn't known ahead of time) and only updates the message,
/// leaving percentage where it was rather than asserting false precision.
#[derive(Debug, Default)]
pub struct ProgressState {
    rule: String,
    nb: usize,
    out_of: usize,
    /// The current task's own command text (e.g. `basm main.asm -o
    /// main.bin`), if a `Task` update has been seen for the rule currently
    /// running - shown in place of the bare rule name once known, since it
    /// says what is actually executing rather than just which rule asked
    /// for it. Cleared on every `Rule` transition (a new rule hasn't
    /// started any of its own tasks yet).
    command: String
}

impl ProgressState {
    pub fn new() -> Self {
        Self::default()
    }

    /// What to show in place of the bare rule name once a task's own
    /// command is known - the command itself is more useful at a glance
    /// than the rule that happened to ask for it.
    fn subject(&self) -> &str {
        if self.command.is_empty() {
            &self.rule
        }
        else {
            &self.command
        }
    }

    /// The three-line report a client should show:
    ///
    /// 1. what is running (the rule, or the command once known) with its
    ///    own bar for "how many of the build's rules are done";
    /// 2. the current phase's name alone (`"pass 3"`, `"parsing main.asm"`,
    ///    ...) - kept on its own line, under line 1, so it stays legible
    ///    even when line 1 (a real command can be long: `-D` flags, paths,
    ///    ...) gets truncated or the display is narrow;
    /// 3. that phase's own bar, one line under its name - blank when the
    ///    phase has no known total yet (`PassStarted`; basm iterates until
    ///    addresses stabilize, so a pass's own total isn't known ahead of
    ///    time) rather than a bar frozen at a misleading 0%.
    ///
    /// `percentage` is the single blended number a native percentage
    /// indicator (VS Code's own progress bar, DAP's
    /// `progressUpdate.percentage`) should use - the two text bars in the
    /// message are for the *two separate things* (rule-of-build vs
    /// phase-of-task) a single native indicator can't distinguish.
    pub fn apply(&mut self, update: ProgressUpdate) -> (String, Option<u32>) {
        match update {
            ProgressUpdate::Rule { rule, nb, out_of } => {
                self.out_of = out_of.max(1);
                self.nb = nb;
                self.rule = rule;
                self.command.clear();
                let percentage = slot_start(self.nb, self.out_of);
                let line1 = self.rule_line();
                (format!("{line1}\nstarting…\n"), Some(percentage))
            },
            ProgressUpdate::Task { command } => {
                self.command = command;
                let percentage = slot_start(self.nb, self.out_of.max(1));
                let line1 = self.rule_line();
                (format!("{line1}\nstarting…\n"), Some(percentage))
            },
            ProgressUpdate::Asm(event) => {
                let out_of = self.out_of.max(1);
                let base = slot_start(self.nb, out_of) as f64;
                let slot_width = 100.0 / out_of as f64;

                let (phase, within_slot) = match &event {
                    cpclib_asm::progress::AsmProgressEvent::Parse { item, done, total } => {
                        (format!("parsing {item}"), fraction(*done, *total))
                    },
                    cpclib_asm::progress::AsmProgressEvent::Load { item, done, total } => {
                        (format!("loading {item}"), fraction(*done, *total))
                    },
                    cpclib_asm::progress::AsmProgressEvent::PassStarted { pass } => {
                        (format!("pass {pass}"), None)
                    },
                    cpclib_asm::progress::AsmProgressEvent::PassProgress {
                        pass,
                        visited,
                        expected
                    } => {
                        // No raw `visited`/`expected` counts in the text -
                        // that's exactly what the bar right below already
                        // shows; repeating it as numbers is noise.
                        (format!("pass {pass}"), fraction(*visited, *expected))
                    },
                    cpclib_asm::progress::AsmProgressEvent::Save { item, done, total } => {
                        (format!("saving {item}"), fraction(*done, *total))
                    },
                    cpclib_asm::progress::AsmProgressEvent::SaveFinished => {
                        ("save complete".to_string(), Some(1.0))
                    }
                };

                let percentage = within_slot
                    .map(|f| (base + slot_width * f).round() as u32)
                    .unwrap_or(base as u32);
                let line1 = self.rule_line();
                let line3 = within_slot.map(|f| bar(f, BAR_WIDTH)).unwrap_or_default();
                (format!("{line1}\n{phase}\n{line3}"), Some(percentage))
            }
        }
    }

    /// Line 1: `[nb/out_of] subject {bar}` - `subject` is the command once
    /// a `Task` has started, else the bare rule name.
    fn rule_line(&self) -> String {
        let out_of = self.out_of.max(1);
        let fraction = (self.nb.saturating_sub(1)) as f64 / out_of as f64;
        format!(
            "[{}/{out_of}] {} {}",
            self.nb,
            self.subject(),
            bar(fraction, BAR_WIDTH)
        )
    }
}

/// Width (in characters) of a text-rendered bar - short enough to stay on
/// one line next to its label in a notification of ordinary width.
const BAR_WIDTH: usize = 10;

/// A plain-text progress bar (`"███████░░░"` at 70%) - a client whose UI
/// doesn't (or, empirically, doesn't reliably) render a native percentage
/// indicator next to arbitrary message text still shows *something*
/// visually bar-shaped, not just a number.
fn bar(fraction: f64, width: usize) -> String {
    let filled = ((fraction.clamp(0.0, 1.0) * width as f64).round() as usize).min(width);
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

fn slot_start(nb: usize, out_of: usize) -> u32 {
    (100 * nb.saturating_sub(1) / out_of.max(1)) as u32
}

fn fraction(done: u64, total: u64) -> Option<f64> {
    if total == 0 {
        None
    }
    else {
        Some((done as f64 / total as f64).clamp(0.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use cpclib_asm::progress::AsmProgressEvent;

    use super::*;

    /// Splits a report's message into its three lines, failing loudly if it
    /// isn't exactly three - every caller of `apply` relies on that shape
    /// (a client always has a "name"/"phase"/"phase bar" line to place),
    /// so a test that silently tolerated a different line count would let
    /// that shape drift unnoticed.
    fn lines(message: &str) -> (&str, &str, &str) {
        let mut it = message.split('\n');
        let (Some(line1), Some(line2), Some(line3), None) =
            (it.next(), it.next(), it.next(), it.next())
        else {
            panic!("expected exactly three lines, got: {message:?}");
        };
        (line1, line2, line3)
    }

    #[test]
    fn a_rule_update_reports_the_start_of_its_own_slot() {
        let mut state = ProgressState::new();
        let (message, percentage) = state.apply(ProgressUpdate::Rule {
            rule: "main.bin".to_string(),
            nb: 2,
            out_of: 4
        });
        let (line1, line2, line3) = lines(&message);
        // Rule 2 of 4: the previous rule (1 of 4) fully occupies [0,25) of
        // the rule bar, matching the blended percentage below.
        assert_eq!(line1, format!("[2/4] main.bin {}", bar(0.25, BAR_WIDTH)));
        assert_eq!(line2, "starting…");
        assert_eq!(line3, "");
        assert_eq!(percentage, Some(25));
    }

    #[test]
    fn asm_progress_interpolates_within_the_current_rule_s_slot() {
        let mut state = ProgressState::new();
        state.apply(ProgressUpdate::Rule {
            rule: "main.bin".to_string(),
            nb: 1,
            out_of: 2
        });
        // Rule 1 of 2 owns [0, 50). Halfway through parsing -> 25.
        let (message, percentage) = state.apply(ProgressUpdate::Asm(AsmProgressEvent::Parse {
            item: "main.asm".to_string(),
            done: 1,
            total: 2
        }));
        let (line1, line2, line3) = lines(&message);
        assert_eq!(line1, format!("[1/2] main.bin {}", bar(0.0, BAR_WIDTH)));
        assert_eq!(line2, "parsing main.asm");
        assert_eq!(line3, bar(0.5, BAR_WIDTH));
        assert_eq!(percentage, Some(25));
    }

    #[test]
    fn a_pass_with_no_known_total_still_reports_the_slot_start_and_a_blank_bar_line() {
        let mut state = ProgressState::new();
        state.apply(ProgressUpdate::Rule {
            rule: "main.bin".to_string(),
            nb: 1,
            out_of: 1
        });
        let (message, percentage) =
            state.apply(ProgressUpdate::Asm(AsmProgressEvent::PassStarted { pass: 3 }));
        let (line1, line2, line3) = lines(&message);
        assert_eq!(line1, format!("[1/1] main.bin {}", bar(0.0, BAR_WIDTH)));
        assert_eq!(line2, "pass 3");
        // No bar here - basm doesn't know this pass's total ahead of time,
        // and a bar frozen at 0% would be actively misleading.
        assert_eq!(line3, "");
        assert_eq!(percentage, Some(0));
    }

    #[test]
    fn an_asm_event_before_any_rule_started_still_produces_three_lines() {
        let mut state = ProgressState::new();
        let (message, percentage) = state.apply(ProgressUpdate::Asm(AsmProgressEvent::Load {
            item: "data.bin".to_string(),
            done: 0,
            total: 1
        }));
        let (line1, line2, line3) = lines(&message);
        assert_eq!(line1, format!("[0/1]  {}", bar(0.0, BAR_WIDTH)));
        assert_eq!(line2, "loading data.bin");
        assert_eq!(line3, bar(0.0, BAR_WIDTH));
        assert_eq!(percentage, Some(0));
    }

    #[test]
    fn pass_progress_reports_percentage_without_raw_token_counts() {
        let mut state = ProgressState::new();
        state.apply(ProgressUpdate::Rule {
            rule: "main.bin".to_string(),
            nb: 1,
            out_of: 1
        });
        let (message, percentage) =
            state.apply(ProgressUpdate::Asm(AsmProgressEvent::PassProgress {
                pass: 2,
                visited: 40,
                expected: 80
            }));
        assert!(!message.contains("40"), "should not leak raw counts: {message}");
        assert!(!message.contains("80"), "should not leak raw counts: {message}");
        let (_, line2, line3) = lines(&message);
        assert_eq!(line2, "pass 2");
        assert_eq!(line3, bar(0.5, BAR_WIDTH));
        assert_eq!(percentage, Some(50));
    }

    #[test]
    fn a_task_update_shows_its_command_in_place_of_the_bare_rule_name() {
        let mut state = ProgressState::new();
        state.apply(ProgressUpdate::Rule {
            rule: "main.bin".to_string(),
            nb: 1,
            out_of: 1
        });
        let (message, _) = state.apply(ProgressUpdate::Task {
            command: "basm main.asm -o main.bin".to_string()
        });
        let (line1, _, _) = lines(&message);
        assert_eq!(
            line1,
            format!("[1/1] basm main.asm -o main.bin {}", bar(0.0, BAR_WIDTH))
        );

        // Subsequent basm-internal events keep showing the command too, not
        // just the one-off task-start message.
        let (message, _) = state.apply(ProgressUpdate::Asm(AsmProgressEvent::Parse {
            item: "main.asm".to_string(),
            done: 1,
            total: 1
        }));
        let (line1, line2, line3) = lines(&message);
        assert_eq!(
            line1,
            format!("[1/1] basm main.asm -o main.bin {}", bar(0.0, BAR_WIDTH))
        );
        assert_eq!(line2, "parsing main.asm");
        assert_eq!(line3, bar(1.0, BAR_WIDTH));
    }

    #[test]
    fn a_new_rule_clears_the_previous_rule_s_task_command() {
        let mut state = ProgressState::new();
        state.apply(ProgressUpdate::Rule {
            rule: "first".to_string(),
            nb: 1,
            out_of: 2
        });
        state.apply(ProgressUpdate::Task {
            command: "basm first.asm -o first.bin".to_string()
        });
        let (message, _) = state.apply(ProgressUpdate::Rule {
            rule: "second".to_string(),
            nb: 2,
            out_of: 2
        });
        let (line1, _, _) = lines(&message);
        assert_eq!(line1, format!("[2/2] second {}", bar(0.5, BAR_WIDTH)));
    }

    #[test]
    fn the_bar_renders_filled_and_empty_blocks_proportionally() {
        assert_eq!(bar(0.0, 10), "░░░░░░░░░░");
        assert_eq!(bar(1.0, 10), "██████████");
        assert_eq!(bar(0.5, 10), "█████░░░░░");
        // Out-of-range inputs clamp rather than panicking or producing a
        // bar wider/narrower than requested.
        assert_eq!(bar(-1.0, 10).chars().count(), 10);
        assert_eq!(bar(2.0, 10).chars().count(), 10);
    }
}
