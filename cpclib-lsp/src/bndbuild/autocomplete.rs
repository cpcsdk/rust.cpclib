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

/// What a `- ` list item (or a bare line) belongs to, per the bndbuild
/// schema: the document is an array of rules; a rule's `cmd:` key holds a
/// list of task invocations.
#[derive(Debug, PartialEq)]
enum ListContext {
    /// Item of the document root array, or continuation lines of a rule
    /// mapping: rule keys apply here.
    Rule,
    /// Item nested under a `cmd:`/`tasks:`/... key: task invocations apply.
    Tasks,
    /// Item nested under a `tgt:`/`dep:`/... key, before `- ` has been
    /// typed: filenames apply, not rule keys.
    Files
}

impl BuildFileAnalyzer {
    /// Provide completion suggestions for build files
    pub fn completion(&self, document: &Document, position: Position) -> Vec<CompletionItem> {
        let mut completions = Vec::new();

        let line_idx = position.line as usize;
        if let Some(line) = document.line(line_idx) {
            let cursor = position.character as usize;

            // Inside `{{ }}` / `{% %}`: offer Jinja-context completions only
            // (never the brace snippets - the braces are already there).
            if let Some(ctx) = super::jinja::jinja_context_at(&line, cursor) {
                return self.get_jinja_inner_completions(document, ctx);
            }

            if let Some((cmd_name, args, arg_index)) = self.command_argv_at_cursor(&line, cursor) {
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
                completions.extend(self.get_variable_brace_completions(document));
            }
            else if let Some(prefix) =
                self.filename_prefix_at_cursor(document, line_idx, &line, cursor)
            {
                // Inside a `targets:`/`dependencies:` (or aliases) value -
                // either the scalar form or a multi-line `- ` list item.
                completions.extend(self.get_filename_completions(document, &prefix));
                completions.extend(self.get_variable_brace_completions(document));
            }
            else if let Some(values) = self.boolean_key_value_at_cursor(&line, cursor) {
                // `phony:` takes a boolean
                completions.extend(values);
            }
            else {
                // Key/task-name position: what applies depends on the schema
                // context of this line, not on a blanket "list item" rule.
                match self.list_context_at(document, line_idx) {
                    ListContext::Tasks => completions.extend(self.get_task_completions()),
                    ListContext::Rule => completions.extend(self.get_rule_key_completions()),
                    ListContext::Files => {
                        completions.extend(self.get_filename_completions(document, ""));
                        completions.extend(self.get_variable_brace_completions(document));
                    }
                }
            }
        }

        // Jinja brace snippets - only offered outside existing braces.
        completions.extend(self.get_jinja_completions());

        completions
    }

    /// Decide whether the line belongs to a rule mapping (offer rule keys) or
    /// to a task list under `cmd:`/`tasks:`/... (offer task invocations), by
    /// finding the closest enclosing key (see `enclosing_key_for_list_item`).
    fn list_context_at(&self, document: &Document, line_idx: usize) -> ListContext {
        match Self::enclosing_key_for_list_item(document, line_idx) {
            Some(key) if super::token::TASK_KEY_NAMES.contains(&key.as_str()) => ListContext::Tasks,
            Some(key) if super::token::FILE_KEY_NAMES.contains(&key.as_str()) => ListContext::Files,
            _ => ListContext::Rule
        }
    }

    /// `true`/`false` completion for the boolean `phony:` key.
    fn boolean_key_value_at_cursor(
        &self,
        line: &str,
        cursor: usize
    ) -> Option<Vec<CompletionItem>> {
        let content = line.trim_start();
        let content = content.strip_prefix("- ").unwrap_or(content);
        let (key, _) = content.split_once(':')?;
        if key.trim() != "phony" {
            return None;
        }
        // Cursor must be after the colon.
        let colon_col = line.find(':')?;
        if cursor <= colon_col {
            return None;
        }
        Some(
            ["true", "false"]
                .iter()
                .map(|v| {
                    CompletionItem {
                        label: v.to_string(),
                        kind: Some(CompletionItemKind::VALUE),
                        insert_text: Some(v.to_string()),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        ..Default::default()
                    }
                })
                .collect()
        )
    }

    /// Rule-level keys from the schema (tgt/dep/cmd/help/phony/constraint and
    /// their aliases).
    fn get_rule_key_completions(&self) -> Vec<CompletionItem> {
        cpclib_bndbuild::lsp::RULE_KEYS
            .iter()
            .flat_map(|key| {
                key.names.iter().map(move |name| {
                    CompletionItem {
                        label: name.to_string(),
                        kind: Some(CompletionItemKind::KEYWORD),
                        detail: Some(key.description.to_string()),
                        insert_text: Some(format!("{}: ", name)),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        ..Default::default()
                    }
                })
            })
            .collect()
    }

    /// Completions offered *inside* Jinja braces: known variables (local
    /// `{% set %}` and built-in globals), plus statement keywords when inside
    /// `{% ... %}`.
    fn get_jinja_inner_completions(
        &self,
        document: &Document,
        ctx: super::jinja::JinjaContext
    ) -> Vec<CompletionItem> {
        let mut completions: Vec<CompletionItem> = super::jinja::known_variables(document)
            .into_iter()
            .map(|(name, detail)| {
                CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some(detail),
                    insert_text: Some(name),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    ..Default::default()
                }
            })
            .collect();

        if ctx == super::jinja::JinjaContext::Statement {
            for (kw, detail) in super::jinja::JINJA_STATEMENT_KEYWORDS {
                completions.push(CompletionItem {
                    label: kw.to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some(detail.to_string()),
                    insert_text: Some(kw.to_string()),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    ..Default::default()
                });
            }
        }

        completions
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
    pub(super) fn command_argv_at_cursor(
        &self,
        line_text: &str,
        cursor_column: usize
    ) -> Option<(&'static str, Vec<std::ffi::OsString>, usize)> {
        let prefix_before_cursor = self.value_prefix_before_cursor(
            line_text,
            cursor_column,
            &super::token::TASK_KEY_NAMES
        )?;

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
    /// `build:` or `dependencies:`/`dep:`/`dependency:`/`requires:` field,
    /// return the partial filename token at the cursor, for filesystem-based
    /// completion. Handles both forms these fields can take:
    ///   - the scalar `key: value` form (`tgt: a.bin b.bin`)
    ///   - the multi-line list form (`dep:\n  - a.bin\n  - b.bin`), where a
    ///     bare `- ` item's governing key is resolved via
    ///     `enclosing_key_for_list_item` since the item's own line never
    ///     repeats it
    fn filename_prefix_at_cursor(
        &self,
        document: &Document,
        line_idx: usize,
        line_text: &str,
        cursor_column: usize
    ) -> Option<String> {
        let trimmed = line_text.trim_start();
        if trimmed.starts_with("- ") && !trimmed.contains(':') {
            // Multi-line list form: a bare item with no key of its own.
            // Only offer filenames when the enclosing key is actually
            // tgt/dep - a root-level `- ` item starts a *new rule* (offering
            // rule keys instead, via the `list_context_at` fallback), and a
            // `cmd:`/`tasks:` item is a task invocation, not a filename.
            let key = Self::enclosing_key_for_list_item(document, line_idx)?;
            if !super::token::FILE_KEY_NAMES.contains(&key.as_str()) {
                return None;
            }
        }
        else if !trimmed.contains(':') {
            // Neither a bare list item nor a `key: value` line.
            return None;
        }
        let prefix_before_cursor = self.value_prefix_before_cursor(
            line_text,
            cursor_column,
            &super::token::FILE_KEY_NAMES
        )?;

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

    /// Known-variable completions offered *outside* Jinja braces: a bare
    /// variable reference isn't valid bndbuild YAML, so proposing `root` and
    /// letting the user type it plain would produce broken output. Instead
    /// each candidate is inserted already wrapped as `{{root}}`.
    fn get_variable_brace_completions(&self, document: &Document) -> Vec<CompletionItem> {
        super::jinja::known_variables(document)
            .into_iter()
            .map(|(name, detail)| {
                CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some(detail),
                    insert_text: Some(format!("{{{{{name}}}}}")),
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
    use crate::common::document::Document;

    /// `line` is placed as the *last* line of a synthetic document, with
    /// `preceding` (if any) as the line(s) directly above it — for the
    /// scalar-only tests `preceding` is empty, matching the old single-line
    /// behavior; the multi-line list tests below supply real context.
    fn prefix_at_with_context(preceding: &[&str], line: &str, cursor: usize) -> Option<String> {
        let uri = Url::parse("file:///t.bnd").unwrap();
        let mut lines: Vec<&str> = preceding.to_vec();
        lines.push(line);
        let text = lines.join("\n");
        let document = Document::new(uri, text, 1);
        let line_idx = lines.len() - 1;
        BuildFileAnalyzer::new().filename_prefix_at_cursor(&document, line_idx, line, cursor)
    }

    fn prefix_at(line: &str, cursor: usize) -> Option<String> {
        prefix_at_with_context(&[], line, cursor)
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
    fn dash_form_without_an_enclosing_key_is_not_handled() {
        // No preceding line at all - e.g. a root-level `- ` item starting a
        // new rule, which `enclosing_key_for_list_item` correctly reports as
        // having no governing tgt/dep key.
        let line = "  - hello.asm";
        let result = prefix_at(line, line.chars().count());
        assert_eq!(result, None);
    }

    #[test]
    fn dash_form_under_a_dependencies_key_completes_filenames() {
        let preceding = ["- tgt: out.bin", "  dep:"];
        let line = "    - hel";
        let result = prefix_at_with_context(&preceding, line, line.chars().count());
        assert_eq!(result, Some("hel".to_string()));
    }

    #[test]
    fn dash_form_under_a_targets_key_completes_filenames() {
        let preceding = ["- tgt:"];
        let line = "    - out.b";
        let result = prefix_at_with_context(&preceding, line, line.chars().count());
        assert_eq!(result, Some("out.b".to_string()));
    }

    #[test]
    fn dash_form_under_a_cmd_key_is_not_treated_as_a_filename() {
        let preceding = ["- tgt: out.bin", "  cmd:"];
        let line = "    - hel";
        let result = prefix_at_with_context(&preceding, line, line.chars().count());
        assert_eq!(result, None);
    }
}

#[cfg(test)]
mod list_context_at_tests {
    use super::*;
    use crate::common::document::Document;

    #[test]
    fn blank_line_under_dependencies_key_offers_filenames_not_rule_keys() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("hello.asm"), "").unwrap();
        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        // The `- ` marker hasn't been typed yet on the last (blank) line.
        let text = "- tgt: out.bin\n  dep:\n    ";
        let document = Document::new(uri, text.to_string(), 1);

        let items = BuildFileAnalyzer::new().completion(
            &document,
            Position {
                line: 2,
                character: 4
            }
        );
        let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();

        assert!(
            labels.contains(&"hello.asm".to_string()),
            "expected filename completion, got {labels:?}"
        );
        assert!(
            !labels.contains(&"tgt".to_string()) && !labels.contains(&"dep".to_string()),
            "rule keys should not be offered under an open dep: list, got {labels:?}"
        );
    }

    #[test]
    fn blank_line_under_targets_key_offers_filenames_not_rule_keys() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("out.bin"), "").unwrap();
        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        let text = "- tgt:\n    ";
        let document = Document::new(uri, text.to_string(), 1);

        let items = BuildFileAnalyzer::new().completion(
            &document,
            Position {
                line: 1,
                character: 4
            }
        );
        let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();

        assert!(
            labels.contains(&"out.bin".to_string()),
            "expected filename completion, got {labels:?}"
        );
        assert!(
            !labels.contains(&"cmd".to_string()),
            "rule keys should not be offered under an open tgt: list, got {labels:?}"
        );
    }

    #[test]
    fn blank_line_under_cmd_key_still_offers_task_completions() {
        let uri = Url::parse("file:///build.bnd").unwrap();
        let text = "- tgt: out.bin\n  cmd:\n    ";
        let document = Document::new(uri, text.to_string(), 1);

        assert_eq!(
            BuildFileAnalyzer::new().list_context_at(&document, 2),
            ListContext::Tasks
        );
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

#[cfg(test)]
mod schema_context_tests {
    use super::*;

    fn complete(text: &str, line: u32, character: u32) -> Vec<String> {
        let uri = Url::parse("file:///t.bnd").unwrap();
        let doc = Document::new(uri, text.to_string(), 1);
        BuildFileAnalyzer::new()
            .completion(&doc, Position { line, character })
            .iter()
            .map(|i| i.label.clone())
            .collect()
    }

    #[test]
    fn rule_start_offers_rule_keys_not_tasks() {
        // `- ` at document root starts a new rule: keys, not task names.
        let labels = complete("- tgt: out.bin\n- ", 1, 2);
        assert!(labels.contains(&"tgt".to_string()), "{labels:?}");
        assert!(labels.contains(&"dep".to_string()));
        assert!(
            !labels.contains(&"basm".to_string()),
            "task names don't belong here: {labels:?}"
        );
    }

    #[test]
    fn task_list_offers_task_names_not_keys() {
        let text = "- tgt: out.bin\n  cmd:\n    - ";
        let labels = complete(text, 2, 6);
        assert!(labels.contains(&"basm".to_string()), "{labels:?}");
        assert!(
            !labels.contains(&"tgt".to_string()),
            "rule keys don't belong here: {labels:?}"
        );
    }

    #[test]
    fn rule_continuation_line_offers_rule_keys() {
        let text = "- tgt: out.bin\n  ";
        let labels = complete(text, 1, 2);
        assert!(labels.contains(&"dep".to_string()), "{labels:?}");
        assert!(labels.contains(&"phony".to_string()));
    }

    #[test]
    fn phony_value_offers_booleans() {
        let text = "- tgt: out.bin\n  phony: ";
        let labels = complete(text, 1, 9);
        assert!(labels.contains(&"true".to_string()), "{labels:?}");
        assert!(labels.contains(&"false".to_string()));
    }

    #[test]
    fn no_brace_snippets_inside_jinja() {
        let text = "{% set root = \"src\" %}\n- tgt: {{r";
        let labels = complete(text, 1, 10);
        assert!(
            !labels.iter().any(|l| l.contains("{{")),
            "brace snippets must not be offered inside jinja: {labels:?}"
        );
        assert!(
            labels.contains(&"root".to_string()),
            "set variables offered: {labels:?}"
        );
    }

    #[test]
    fn jinja_statement_offers_keywords_and_variables() {
        let text = "{% set root = \"src\" %}\n{% ";
        let labels = complete(text, 1, 3);
        assert!(labels.contains(&"if".to_string()), "{labels:?}");
        assert!(labels.contains(&"endfor".to_string()));
        assert!(labels.contains(&"root".to_string()));
    }

    #[test]
    fn brace_snippets_offered_outside_jinja() {
        let labels = complete("- tgt: out.bin\n- ", 1, 2);
        assert!(labels.iter().any(|l| l.contains("{{")), "{labels:?}");
    }

    #[test]
    fn known_variable_completion_outside_braces_is_wrapped_in_braces() {
        // A real (empty) tempdir - `get_filename_completions` lists the
        // document's actual directory, and a bare `file:///t.bnd` URI would
        // resolve to the filesystem root, which may itself contain an
        // unrelated `root` entry (e.g. `/root`) that collides with the
        // variable label under test.
        let tmp = camino_tempfile::tempdir().unwrap();
        let text = "{% set root = \"src\" %}\n- tgt: out.bin\n  dep: ";
        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        let doc = Document::new(uri, text.to_string(), 1);
        let items = BuildFileAnalyzer::new().completion(
            &doc,
            Position {
                line: 2,
                character: 7
            }
        );
        let root_item = items
            .iter()
            .find(|i| i.label == "root" && i.kind == Some(CompletionItemKind::VARIABLE))
            .expect("root variable offered as a completion");
        assert_eq!(root_item.insert_text.as_deref(), Some("{{root}}"));
    }

    #[test]
    fn builtin_jinja_globals_offered_outside_braces_in_dep_value() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let text = "- tgt: out.bin\n  dep: ";
        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        let doc = Document::new(uri, text.to_string(), 1);
        let items = BuildFileAnalyzer::new().completion(
            &doc,
            Position {
                line: 1,
                character: 7
            }
        );
        let item = items
            .iter()
            .find(|i| i.label == "AKG_PLAYER_PATH")
            .expect("builtin global offered as a completion");
        assert_eq!(item.insert_text.as_deref(), Some("{{AKG_PLAYER_PATH}}"));
    }
}

#[cfg(test)]
mod jinja_definition_tests {
    use super::*;

    #[test]
    fn jinja_variable_definition_and_references() {
        let uri = Url::parse("file:///t.bnd").unwrap();
        let text =
            "{% set root = \"src\" %}\n- tgt: {{root}}/out.bin\n  cmd: basm {{root}}/main.asm\n";
        let doc = Document::new(uri, text.to_string(), 1);
        let analyzer = BuildFileAnalyzer::new();

        // goto-definition from the use on line 1 goes to the set on line 0
        let def = analyzer.goto_definition(
            &doc,
            Position {
                line: 1,
                character: 10
            }
        );
        assert_eq!(def.expect("definition").range.start.line, 0);

        // references include the set line and both uses
        let refs = analyzer.find_references(
            &doc,
            Position {
                line: 1,
                character: 10
            }
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.range.start.line).collect();
        assert_eq!(lines, vec![0, 1, 2], "{refs:?}");
    }
}
