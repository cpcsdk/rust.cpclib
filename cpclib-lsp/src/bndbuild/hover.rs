//! Hover for bndbuild files: rule keywords and task-type documentation.
//!
//! The `get_*_help` functions are rendering helpers (they build markdown);
//! word extraction/lookup is the logic part.

use tower_lsp::lsp_types::*;

use super::BuildFileAnalyzer;
use crate::common::document::Document;

impl BuildFileAnalyzer {
    /// Provide hover information for build file keywords
    pub fn hover(&self, document: &Document, position: Position) -> Option<Hover> {
        let line_idx = position.line as usize;
        let line = document.line(line_idx)?;

        // Extract word at cursor
        let word = self.extract_word_at_position(&line, position.character as usize)?;

        // Check if it's a build file keyword
        if let Some(description) = self.get_keyword_help(&word) {
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: description
                }),
                range: None
            });
        }

        // Check if it's a task type
        if let Some(description) = self.get_task_type_help(&word) {
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: description
                }),
                range: None
            });
        }

        None
    }

    fn get_keyword_help(&self, word: &str) -> Option<String> {
        // Rule-level keys (with aliases) take priority
        for key in cpclib_bndbuild::lsp::RULE_KEYS {
            if key.names.contains(&word) {
                let canonical = key.names[0];
                let aliases: Vec<_> = key.names[1..].iter().copied().collect();
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
