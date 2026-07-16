//! Completion for bndbuild files.
//!
//! Logic: cursor-context detection (`value_prefix_before_cursor`,
//! `command_argv_at_cursor`, `filename_prefix_at_cursor`).
//! Rendering: the `get_*_completions` functions, which map candidates
//! (task names, clap flags, scraped help options, filesystem entries,
//! Jinja snippets) to LSP `CompletionItem`s.

use tower_lsp::lsp_types::*;

use super::BuildFileAnalyzer;
use crate::common::document::Document;

impl BuildFileAnalyzer {
    /// Provide completion suggestions for build files
    pub fn completion(&self, document: &Document, position: Position) -> Vec<CompletionItem> {
        let mut completions = Vec::new();

        let line_idx = position.line as usize;
        if let Some(line) = document.line(line_idx) {
            let indent = line.chars().take_while(|c| c.is_whitespace()).count();
            let cursor = position.character as usize;

            // If we're at the start of a line or after whitespace, suggest top-level keys
            if indent == 0 || line.trim().is_empty() {
                completions.extend(self.get_top_level_completions());
            }
            else if let Some((cmd_name, args, arg_index)) =
                self.command_argv_at_cursor(&line, cursor)
            {
                let prefix = args[arg_index].to_string_lossy().into_owned();
                if super::internal_commands::get_command_for(cmd_name).is_some() {
                    // Internal command: offer real flag/value completion driven
                    // by its actual clap::Command.
                    completions.extend(
                        self.get_internal_command_arg_completions(cmd_name, args, arg_index)
                    );
                }
                else {
                    // Delegated (third-party) command: best-effort completion
                    // scraped from its `--help` output, if already installed.
                    completions
                        .extend(self.get_delegated_command_arg_completions(cmd_name, &prefix));
                }
                // Most task arguments are filenames - offer those too, in
                // addition to the command-specific completions above.
                completions.extend(self.get_filename_completions(document, &prefix));
            }
            else if let Some(prefix) = self.filename_prefix_at_cursor(&line, cursor) {
                // Inside a `targets:`/`dependencies:` (or aliases) scalar value
                completions.extend(self.get_filename_completions(document, &prefix));
            }
            else if line.trim_start().starts_with("- ") {
                // Inside a list - suggest task types
                completions.extend(self.get_task_completions());
            }
        }

        // Add Jinja template completions
        completions.extend(self.get_jinja_completions());

        completions
    }

    fn get_top_level_completions(&self) -> Vec<CompletionItem> {
        // Use cpclib-bndbuild's build keywords
        cpclib_bndbuild::lsp::BUILD_KEYWORDS
            .iter()
            .map(|(keyword, description)| {
                CompletionItem {
                    label: keyword.to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some(description.to_string()),
                    insert_text: Some(format!("{}:\n  ", keyword)),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    ..Default::default()
                }
            })
            .collect()
    }

    fn get_task_completions(&self) -> Vec<CompletionItem> {
        // Use cpclib-bndbuild's task types
        cpclib_bndbuild::lsp::TASK_TYPES
            .iter()
            .flat_map(|task| {
                task.names.iter().map(|name| {
                    CompletionItem {
                        label: name.to_string(),
                        kind: Some(CompletionItemKind::CLASS),
                        detail: Some(task.description.to_string()),
                        insert_text: Some(format!("{} ", name)),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        ..Default::default()
                    }
                })
            })
            .collect()
    }

    /// Strip a leading `"- "` list marker or a `key:` prefix (only when `key`
    /// is one of `allowed_keys`) from `line_text`, and rebase `cursor_column`
    /// onto the remaining value text. Returns the value substring from the
    /// start of the value up to the cursor.
    ///
    /// Returns `None` when the line isn't a `- ` item nor a matching `key:`
    /// line, or the cursor sits before the value even starts (e.g. still on
    /// the key itself).
    fn value_prefix_before_cursor(
        &self,
        line_text: &str,
        cursor_column: usize,
        allowed_keys: &[&str]
    ) -> Option<String> {
        let rest = line_text.trim_start();
        let leading_ws = line_text.chars().count() - rest.chars().count();

        let (value_text, consumed_chars) = if let Some(after_dash) = rest.strip_prefix("- ") {
            (after_dash, leading_ws + 2)
        }
        else if let Some(colon_idx) = rest.find(':') {
            let key = rest[..colon_idx].trim();
            if !allowed_keys.contains(&key) {
                return None;
            }
            let after_colon = &rest[colon_idx + 1..];
            let after_colon_trimmed = after_colon.trim_start();
            // Note: an empty `after_colon_trimmed` here (`key:` alone, or `key:
            // ` with nothing typed yet) is intentionally NOT special-cased as
            // "list items follow on later lines" - callers that care (like the
            // tasks tokenizer) already end up bailing out via their own
            // subsequent empty-tokens check, while callers that want to offer
            // completion right after the colon (like filename completion) get
            // a valid empty prefix instead of a premature `None`.
            let ws_after_colon = after_colon.chars().count() - after_colon_trimmed.chars().count();
            (
                after_colon_trimmed,
                leading_ws + key.chars().count() + 1 + ws_after_colon
            )
        }
        else {
            return None;
        };

        let cursor_in_value = cursor_column.checked_sub(consumed_chars)?;
        let value_chars: Vec<char> = value_text.chars().collect();
        if cursor_in_value > value_chars.len() {
            return None;
        }
        Some(value_chars[..cursor_in_value].iter().collect())
    }

    /// If the cursor sits inside the value of a task-invocation line (a `- `
    /// list item, or a scalar `cmd:`/`tasks:`/`command:`/`launch:`/`run:` value)
    /// whose first word is a *recognized task command* (internal or
    /// delegated), return that command's canonical name plus the tokenized
    /// argv. `args[arg_index]` is always the partial/in-progress token.
    ///
    /// Returns `None` when the cursor is still inside the command-name token
    /// itself (falls back to plain command-name completion), the command word
    /// is unrecognized, or the line isn't a task-invocation line at all.
    /// Callers decide separately whether the resolved command is internal
    /// (`super::internal_commands`) or delegated (`super::delegated_help`).
    fn command_argv_at_cursor(
        &self,
        line_text: &str,
        cursor_column: usize
    ) -> Option<(&'static str, Vec<std::ffi::OsString>, usize)> {
        let task_keys: Vec<&str> = cpclib_bndbuild::lsp::RULE_KEYS
            .iter()
            .find(|k| k.names.contains(&"tasks"))
            .map(|k| k.names.to_vec())
            .unwrap_or_default();
        let prefix_before_cursor =
            self.value_prefix_before_cursor(line_text, cursor_column, &task_keys)?;

        // Tokenize with the streaming lexer (not shlex::split, which just
        // returns None on an unterminated quote - exactly the state a user is
        // in while mid-typing a quoted argument).
        let mut lexer = shlex::Shlex::new(&prefix_before_cursor);
        let mut tokens: Vec<String> = (&mut lexer).collect();
        if lexer.had_error {
            return None;
        }
        if tokens.is_empty() {
            return None; // cursor is still on/before the command name itself
        }

        let ends_in_whitespace = prefix_before_cursor.ends_with(char::is_whitespace);
        let cmd_word = tokens.remove(0);
        if tokens.is_empty() && !ends_in_whitespace {
            // Cursor is still within the command-name token: let the existing
            // command-name completion branch handle it.
            return None;
        }

        let canonical = cpclib_bndbuild::lsp::TASK_TYPES
            .iter()
            .find(|t| t.names.contains(&cmd_word.as_str()))
            .map(|t| t.names[0])?;

        if ends_in_whitespace {
            tokens.push(String::new());
        }
        let arg_index = tokens.len() - 1;
        let args = tokens.into_iter().map(std::ffi::OsString::from).collect();
        Some((canonical, args, arg_index))
    }

    /// If the cursor sits inside the value of a `targets:`/`tgt:`/`target:`/
    /// `build:` or `dependencies:`/`dep:`/`dependency:`/`requires:` line
    /// (scalar form only - see note below), return the partial filename token
    /// at the cursor, for filesystem-based completion.
    ///
    /// Only the scalar `key: value` form is handled here (not multi-line `- `
    /// lists): unlike `tasks:`/`cmd:`, real-world build files essentially
    /// always write targets/dependencies as a single space-separated scalar
    /// string, and a bare `- ` line's enclosing key can't be determined
    /// without scanning back through prior lines, which isn't done here.
    fn filename_prefix_at_cursor(&self, line_text: &str, cursor_column: usize) -> Option<String> {
        let file_keys: Vec<&str> = cpclib_bndbuild::lsp::RULE_KEYS
            .iter()
            .filter(|k| k.names.contains(&"targets") || k.names.contains(&"dependencies"))
            .flat_map(|k| k.names.iter().copied())
            .collect();

        // Only the scalar `key: value` form applies here, so bypass the `- `
        // handling in `value_prefix_before_cursor` by requiring a `:` on this
        // line before delegating to it.
        if !line_text.trim_start().contains(':') {
            return None;
        }
        let prefix_before_cursor =
            self.value_prefix_before_cursor(line_text, cursor_column, &file_keys)?;

        let mut lexer = shlex::Shlex::new(&prefix_before_cursor);
        let mut tokens: Vec<String> = (&mut lexer).collect();
        if lexer.had_error {
            return None;
        }

        if prefix_before_cursor.is_empty() || prefix_before_cursor.ends_with(char::is_whitespace) {
            Some(String::new())
        }
        else {
            tokens.pop()
        }
    }

    /// List filesystem entries matching `prefix` (which may include a
    /// directory component, e.g. `"src/mai"`), relative to the directory of
    /// the currently open document. Only the final path segment is used as
    /// `label`/`insert_text` - editors treat `/` as a word boundary, so this
    /// naturally replaces just the in-progress segment.
    fn get_filename_completions(&self, document: &Document, prefix: &str) -> Vec<CompletionItem> {
        let Ok(doc_path) = document.uri.to_file_path()
        else {
            return Vec::new();
        };
        let Some(base_dir) = doc_path.parent()
        else {
            return Vec::new();
        };

        let (dir_part, file_prefix) = match prefix.rfind('/') {
            Some(idx) => (&prefix[..idx], &prefix[idx + 1..]),
            None => ("", prefix)
        };
        let search_dir = if dir_part.is_empty() {
            base_dir.to_path_buf()
        }
        else {
            base_dir.join(dir_part)
        };

        let Ok(entries) = std::fs::read_dir(&search_dir)
        else {
            return Vec::new();
        };

        let mut items: Vec<CompletionItem> = entries
            .filter_map(|e| e.ok())
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !name.starts_with(file_prefix) {
                    return None;
                }
                if !file_prefix.starts_with('.') && name.starts_with('.') {
                    return None;
                }
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let insert_text = if is_dir {
                    format!("{name}/")
                }
                else {
                    name.clone()
                };
                Some(CompletionItem {
                    label: name,
                    kind: Some(if is_dir {
                        CompletionItemKind::FOLDER
                    }
                    else {
                        CompletionItemKind::FILE
                    }),
                    insert_text: Some(insert_text),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    ..Default::default()
                })
            })
            .take(200)
            .collect();

        items.sort_by(|a, b| a.label.cmp(&b.label));
        items
    }

    fn get_internal_command_arg_completions(
        &self,
        cmd_name: &str,
        args: Vec<std::ffi::OsString>,
        arg_index: usize
    ) -> Vec<CompletionItem> {
        let Some(mut cmd) = super::internal_commands::get_command_for(cmd_name)
        else {
            return Vec::new();
        };
        match clap_complete::engine::complete(&mut cmd, args, arg_index, None) {
            Ok(candidates) => {
                candidates
                    .into_iter()
                    .map(|c| {
                        let value = c.get_value().to_string_lossy().into_owned();
                        let kind = if value.starts_with('-') {
                            CompletionItemKind::PROPERTY
                        }
                        else {
                            CompletionItemKind::VALUE
                        };
                        CompletionItem {
                            label: value.clone(),
                            kind: Some(kind),
                            detail: c.get_help().map(|h| h.to_string()),
                            insert_text: Some(value),
                            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                            ..Default::default()
                        }
                    })
                    .collect()
            },
            Err(e) => {
                tracing::debug!("clap_complete engine::complete failed for {cmd_name}: {e}");
                Vec::new()
            }
        }
    }

    /// Argument completion for *delegated* (third-party) commands, scraped
    /// from their `--help` output - see `super::delegated_help`. Only offers
    /// something when the tool is already installed locally; otherwise (or
    /// when its help text had no recognizable options) returns nothing, same
    /// as today's behavior.
    fn get_delegated_command_arg_completions(
        &self,
        cmd_name: &str,
        current_prefix: &str
    ) -> Vec<CompletionItem> {
        super::delegated_help::get_completions_for(cmd_name)
            .into_iter()
            .filter(|(flag, _)| flag.starts_with(current_prefix))
            .map(|(flag, comment)| {
                CompletionItem {
                    label: flag.clone(),
                    kind: Some(CompletionItemKind::PROPERTY),
                    detail: comment,
                    insert_text: Some(flag),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    ..Default::default()
                }
            })
            .collect()
    }

    fn get_jinja_completions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "{{ }}".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Jinja variable".to_string()),
                insert_text: Some("{{ $0 }}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "{% %}".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Jinja statement".to_string()),
                insert_text: Some("{% $0 %}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "{% if %}".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Jinja if statement".to_string()),
                insert_text: Some("{% if $1 %}\n  $0\n{% endif %}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "{% for %}".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Jinja for loop".to_string()),
                insert_text: Some("{% for $1 in $2 %}\n  $0\n{% endfor %}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
        ]
    }
}

#[cfg(test)]
mod command_argv_at_cursor_tests {
    use super::*;

    fn argv_at(line: &str, cursor: usize) -> Option<(&'static str, Vec<String>, usize)> {
        BuildFileAnalyzer::new()
            .command_argv_at_cursor(line, cursor)
            .map(|(name, args, idx)| {
                (
                    name,
                    args.into_iter()
                        .map(|a| a.to_string_lossy().into_owned())
                        .collect(),
                    idx
                )
            })
    }

    #[test]
    fn list_item_partial_flag() {
        let line = "  - basm --sn";
        let result = argv_at(line, line.chars().count());
        assert_eq!(result, Some(("basm", vec!["--sn".to_string()], 0)));
    }

    #[test]
    fn list_item_trailing_whitespace_yields_empty_token() {
        let line = "  - basm ";
        let result = argv_at(line, line.chars().count());
        assert_eq!(result, Some(("basm", vec![String::new()], 0)));
    }

    #[test]
    fn scalar_cmd_form_with_several_args() {
        let line = "  cmd: basm --output foo.sna --";
        let result = argv_at(line, line.chars().count());
        assert_eq!(
            result,
            Some((
                "basm",
                vec![
                    "--output".to_string(),
                    "foo.sna".to_string(),
                    "--".to_string()
                ],
                2
            ))
        );
    }

    #[test]
    fn cursor_still_inside_command_name_falls_back() {
        let line = "  - basm";
        let result = argv_at(line, line.chars().count());
        assert_eq!(result, None);
    }

    #[test]
    fn delegated_command_is_still_recognized_and_tokenized() {
        // The tokenizer resolves & tokenizes regardless of internal/delegated;
        // `completion()` is what decides which completion mechanism to use.
        let line = "  - rasm foo.asm --";
        let result = argv_at(line, line.chars().count());
        assert_eq!(
            result,
            Some(("rasm", vec!["foo.asm".to_string(), "--".to_string()], 1))
        );
    }

    #[test]
    fn unknown_command_word_is_ignored() {
        let line = "  - notacommand --x";
        let result = argv_at(line, line.chars().count());
        assert_eq!(result, None);
    }

    #[test]
    fn non_task_key_is_ignored() {
        let line = "  targets: basm --sn";
        let result = argv_at(line, line.chars().count());
        assert_eq!(result, None);
    }
}

#[cfg(test)]
mod filename_prefix_at_cursor_tests {
    use super::*;

    fn prefix_at(line: &str, cursor: usize) -> Option<String> {
        BuildFileAnalyzer::new().filename_prefix_at_cursor(line, cursor)
    }

    #[test]
    fn targets_scalar_partial_filename() {
        let line = "  tgt: HELLO2.BIN hel";
        let result = prefix_at(line, line.chars().count());
        assert_eq!(result, Some("hel".to_string()));
    }

    #[test]
    fn dependencies_scalar_partial_filename() {
        let line = "  dep: hello.asm";
        let result = prefix_at(line, line.chars().count());
        assert_eq!(result, Some("hello.asm".to_string()));
    }

    #[test]
    fn trailing_whitespace_yields_empty_prefix() {
        let line = "  dep: hello.asm ";
        let result = prefix_at(line, line.chars().count());
        assert_eq!(result, Some(String::new()));
    }

    #[test]
    fn right_after_colon_yields_empty_prefix() {
        let line = "  dep: ";
        let result = prefix_at(line, line.chars().count());
        assert_eq!(result, Some(String::new()));
    }

    #[test]
    fn non_target_dep_key_is_ignored() {
        let line = "  cmd: basm hel";
        let result = prefix_at(line, line.chars().count());
        assert_eq!(result, None);
    }

    #[test]
    fn dash_form_is_not_handled_here() {
        let line = "  - hello.asm";
        let result = prefix_at(line, line.chars().count());
        assert_eq!(result, None);
    }
}

#[cfg(test)]
mod get_filename_completions_tests {
    use super::*;

    #[test]
    fn lists_matching_files_and_directories_in_document_dir() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("hello.asm"), "").unwrap();
        std::fs::write(tmp.path().join("hello2.dsk"), "").unwrap();
        std::fs::write(tmp.path().join("other.txt"), "").unwrap();
        std::fs::create_dir(tmp.path().join("helpers")).unwrap();

        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        let document = Document::new(uri, String::new(), 1);

        let items = BuildFileAnalyzer::new().get_filename_completions(&document, "hel");
        let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();

        assert!(labels.contains(&"hello.asm".to_string()));
        assert!(labels.contains(&"hello2.dsk".to_string()));
        assert!(labels.contains(&"helpers".to_string()));
        assert!(!labels.contains(&"other.txt".to_string()));
    }

    #[test]
    fn directory_insert_text_has_trailing_slash() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("src")).unwrap();

        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        let document = Document::new(uri, String::new(), 1);

        let items = BuildFileAnalyzer::new().get_filename_completions(&document, "src");
        let src_item = items.iter().find(|i| i.label == "src").unwrap();
        assert_eq!(src_item.insert_text, Some("src/".to_string()));
    }
}
