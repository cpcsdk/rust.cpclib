//! Hover for bndbuild files: rule keywords and task-type documentation.
//!
//! The `get_*_help` functions are rendering helpers (they build markdown);
//! word extraction/lookup is the logic part.

use tower_lsp::lsp_types::*;

use super::BuildFileAnalyzer;
use crate::common::document::Document;
use crate::common::render::make_hover;

impl BuildFileAnalyzer {
    /// Provide hover information for build file keywords
    pub fn hover(&self, document: &Document, position: Position) -> Option<Hover> {
        let line_idx = position.line as usize;
        let line = document.line(line_idx)?;
        // `macro_call_hover` (via `macro_call_at`) indexes by byte, while
        // `extract_word_at_position`/`command_argv_at_cursor` index by
        // `char` - two different conversions of the same UTF-16
        // `position.character`.
        let byte_cursor = document.byte_column(position);
        let cursor = document.char_column(position);

        // A call to a `{% macro %}` defined in this file: show what it
        // expands to for these exact arguments.
        if let Some(md) = super::macro_expand::macro_call_hover(document, &line, byte_cursor) {
            return Some(make_hover(md));
        }

        // Extract word at cursor
        let word = self.extract_word_at_position(&line, cursor)?;

        // A flag (`--snapshot`) or subcommand (`run`) of a task invocation,
        // e.g. `- emu --snapshot test.sna run`: reuse the exact same
        // command resolution as completion, driven by the *actual* cursor
        // position so this only fires when the cursor genuinely sits on an
        // argument - never on the command name itself (handled below by
        // `get_task_type_help`), nor on a YAML key that merely happens to
        // share a word with a task argument (e.g. `run:` is also an alias
        // of `tasks:`, handled below by `get_keyword_help`).
        if let Some((cmd_name, _args, _idx)) = self.command_argv_at_cursor(&line, cursor)
            && let Some(description) = self.get_task_argument_help(cmd_name, &word)
        {
            return Some(make_hover(description));
        }

        // Check if it's a build file keyword
        if let Some(description) = self.get_keyword_help(&word) {
            return Some(make_hover(description));
        }

        // Check if it's a task type
        if let Some(description) = self.get_task_type_help(&word) {
            return Some(make_hover(description));
        }

        None
    }

    /// Documentation for a single flag or subcommand of task command
    /// `cmd_name` (the canonical name, as resolved by
    /// `command_argv_at_cursor`).
    fn get_task_argument_help(&self, cmd_name: &str, word: &str) -> Option<String> {
        if let Some(cmd) = super::internal_commands::get_command_for(cmd_name) {
            // A bare word (no leading `-`) can only be a subcommand, e.g.
            // `run` in `emu --snapshot test.sna run`.
            if !word.starts_with('-') {
                let sub = cmd.get_subcommands().find(|s| s.get_name() == word)?;
                let about = sub
                    .get_about()
                    .map(|h| h.to_string())
                    .or_else(|| sub.get_long_about().map(|h| h.to_string()))?;
                return Some(format!("**{word}**\n\n{about}"));
            }

            let long = word.strip_prefix("--");
            let short = (long.is_none() && word.len() > 1)
                .then(|| word.strip_prefix('-'))
                .flatten()
                .and_then(|s| s.chars().next());

            let arg = cmd.get_arguments().find(|a| {
                (long.is_some() && a.get_long() == long)
                    || (long.is_some()
                        && a.get_visible_aliases()
                            .is_some_and(|al| al.iter().any(|x| Some(*x) == long)))
                    || (long.is_some()
                        && a.get_all_aliases()
                            .is_some_and(|al| al.iter().any(|x| Some(*x) == long)))
                    || (short.is_some() && a.get_short() == short)
            })?;

            let help = arg
                .get_help()
                .map(|h| h.to_string())
                .or_else(|| arg.get_long_help().map(|h| h.to_string()))?;

            let mut header = format!("**{word}**");
            if let Some(names) = arg.get_value_names() {
                let placeholders: Vec<String> = names.iter().map(|n| format!("<{n}>")).collect();
                header.push_str(&format!(" `{}`", placeholders.join(" ")));
            }
            return Some(format!("{header}\n\n{help}"));
        }

        // Delegated (third-party) command: reuse the same scraped `--help`
        // text that already backs its completion items. Only flags are
        // recognized here - there's no structured subcommand data scraped
        // from plain `--help` output.
        if !word.starts_with('-') {
            return None;
        }
        let (matched_flag, comment) = super::delegated_help::get_completions_for(cmd_name)
            .into_iter()
            .find(|(f, _)| f == word)?;
        let comment = comment?;
        Some(format!("**{matched_flag}**\n\n{comment}"))
    }

    fn get_keyword_help(&self, word: &str) -> Option<String> {
        // Rule-level keys (with aliases) take priority
        for key in cpclib_bndbuild::lsp::RULE_KEYS {
            if key.names.contains(&word) {
                let canonical = key.names[0];
                let aliases: Vec<_> = key.names[1..].to_vec();
                let mut md = format!("**{}**\n\n{}", canonical, key.description);
                if !aliases.is_empty() {
                    md.push_str(&format!("\n\nAliases: `{}`", aliases.join("`, `")));
                }
                return Some(md);
            }
        }
        // Top-level file keywords
        for (keyword, description) in cpclib_bndbuild::lsp::BUILD_KEYWORDS {
            if *keyword == word {
                return Some(format!("**{}**\n\n{}", keyword, description));
            }
        }
        None
    }

    fn get_task_type_help(&self, word: &str) -> Option<String> {
        // Use cpclib-bndbuild's task types
        for task in cpclib_bndbuild::lsp::TASK_TYPES {
            for name in task.names {
                if *name == word {
                    let mut md = format!(
                        "**{}**\n\n{}\n\nUsage:\n```\n{}\n```",
                        name, task.description, task.synopsis
                    );
                    if !task.example.is_empty() {
                        let cmd = if task.example.contains('\n') {
                            format!(
                                "cmd:\n{}",
                                task.example
                                    .lines()
                                    .map(|line| format!("  {line}"))
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            )
                        }
                        else {
                            format!("cmd: {}", task.example)
                        };
                        md.push_str(&format!("\n\nExample:\n```\n{cmd}\n```"));
                    }
                    return Some(md);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bndbuild::BuildFileAnalyzer;

    fn hover_at(text: &str, line: u32, character: u32) -> Option<Hover> {
        let uri = Url::parse("file:///t.bnd").unwrap();
        let document = Document::new(uri, text.to_string(), 1);
        BuildFileAnalyzer::new().hover(&document, Position { line, character })
    }

    fn markdown(hover: &Hover) -> &str {
        match &hover.contents {
            HoverContents::Markup(m) => m.value.as_str(),
            _ => panic!("expected markdown hover contents")
        }
    }

    /// Regression test for treating `position.character` (UTF-16 code
    /// units) as a raw byte offset for `macro_call_hover`'s `byte_cursor`:
    /// a supplementary-plane character (😀, 4 UTF-8 bytes / 2 UTF-16 units)
    /// earlier on the line must not desync the two.
    #[test]
    fn hovering_a_macro_call_with_a_supplementary_plane_char_before_it_still_expands() {
        let text = "{% macro assemble(src) %}basm {{ src }}{% endmacro %}\n\
                     - cmd: \u{1F600}{{ assemble(\"main.asm\") }}\n";
        // UTF-16 column 15 on line 1 lands 3 chars into "assemble" once
        // correctly converted to a byte offset.
        let hover = hover_at(text, 1, 15).expect("expected an expansion hover");
        let md = markdown(&hover);
        assert!(md.contains("basm main.asm"), "{md}");
    }

    #[test]
    fn hovering_an_internal_command_flag_shows_its_real_help_text() {
        let text = "- tgt: out.sna\n  cmd: basm --snapshot main.asm\n";
        // Column inside "--snapshot" on line 1.
        let hover = hover_at(text, 1, 15).expect("hover on --snapshot");
        let md = markdown(&hover);
        assert!(md.contains("--snapshot"), "{md}");
        assert!(md.contains("Generate a snapshot"), "{md}");
    }

    #[test]
    fn hovering_a_flag_alias_resolves_to_the_same_underlying_arg() {
        let text = "- tgt: out.sna\n  cmd: basm --sna main.asm\n";
        // Column inside "--sna" (an alias of --snapshot).
        let hover = hover_at(text, 1, 12).expect("hover on --sna alias");
        let md = markdown(&hover);
        assert!(md.contains("Generate a snapshot"), "{md}");
    }

    #[test]
    fn hovering_the_command_name_itself_still_shows_the_task_type_help() {
        let text = "- tgt: out.sna\n  cmd: basm --snapshot main.asm\n";
        // Column inside "basm" itself, not a flag.
        let hover = hover_at(text, 1, 8).expect("hover on basm");
        let md = markdown(&hover);
        assert!(md.contains("basm (built-in assembler)"), "{md}");
    }

    #[test]
    fn hovering_an_unknown_flag_of_a_known_command_yields_no_hover() {
        let text = "- tgt: out.sna\n  cmd: basm --not-a-real-flag main.asm\n";
        assert!(hover_at(text, 1, 15).is_none());
    }

    #[test]
    fn hovering_the_emulator_flag_of_the_emu_facade_command_shows_its_help() {
        // Regression test: `emulator` used to be declared with no `help =`
        // at all in `EmuCli` (cpclib-runner), so hover legitimately had
        // nothing to show. Fixed alongside this feature.
        let text = "- tgt: out.sna\n  cmd: emu --snapshot test2.sna  --emulator {{EMU}} run\n";
        // Column inside "--emulator".
        let hover = hover_at(text, 1, 37).expect("hover on --emulator");
        let md = markdown(&hover);
        assert!(md.contains("--emulator"), "{md}");
        assert!(md.contains("Which emulator to use"), "{md}");
    }

    #[test]
    fn hovering_a_trailing_subcommand_shows_its_own_help_not_a_yaml_keyword() {
        // `run` here is the `emu ... run` *subcommand* (launch and keep the
        // emulator window open), not the `run:` YAML key that happens to be
        // an alias of `tasks:` - hovering it must not show the latter.
        let text = "- tgt: out.sna\n  cmd: emu --snapshot test2.sna  --emulator {{EMU}} run\n";
        // Column inside the trailing "run".
        let hover = hover_at(text, 1, 53).expect("hover on the run subcommand");
        let md = markdown(&hover);
        assert!(md.contains("Launch the emulator"), "{md}");
        assert!(!md.contains("Tasks - define"), "{md}");
    }

    #[test]
    fn hovering_run_as_an_actual_yaml_key_still_shows_the_tasks_keyword_help() {
        // Guard against the reordering above swallowing the legitimate case:
        // `run:` used as the rule's task-list key (alias of `tasks:`).
        let text = "- tgt: out.bin\n  run:\n    - basm main.asm\n";
        // Column inside "run" (the key itself, before the colon).
        let hover = hover_at(text, 1, 3).expect("hover on the run: key");
        let md = markdown(&hover);
        assert!(md.contains("**tasks**"), "{md}");
        assert!(md.contains("List of tasks to execute"), "{md}");
    }

    #[test]
    fn flag_of_an_unrecognized_or_uninstalled_delegated_command_yields_no_help() {
        // Not a real/known delegated command, so `delegated_help` never spawns
        // a process - this must not panic and must not fall back to anything.
        // (Testing this through `hover()` with a *real* delegated command name
        // like `rasm` would be environment-dependent: on a machine where the
        // user already has it installed/cached, the real `--help` output
        // would be scraped and this would flake.)
        assert!(
            BuildFileAnalyzer::new()
                .get_task_argument_help("definitely-not-a-real-delegated-command-xyz", "-o")
                .is_none()
        );
    }
}
