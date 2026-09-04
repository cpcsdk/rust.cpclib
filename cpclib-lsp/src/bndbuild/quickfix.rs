//! Code actions for build files.
//!
//! One so far: a rule depends on something nothing builds, so offer to write
//! the rule that builds it.

use tower_lsp::lsp_types::*;

use super::BuildFileAnalyzer;
use crate::common::document::Document;

impl BuildFileAnalyzer {
    pub fn code_actions(&self, document: &Document, range: Range) -> Vec<CodeAction> {
        self.missing_dependency_actions(document, range)
    }

    /// Offer to write the rule that builds a dependency nothing produces.
    ///
    /// The diagnostic already says the dependency cannot be built; this is the
    /// obvious next step, and writing it by hand means remembering the exact
    /// `basm` invocation every time. Where a source of the same name sits
    /// beside the build file the rule can be written in full; where it does not
    /// there is nothing to guess, so the rule is a placeholder that fails
    /// loudly rather than a command that might be wrong.
    fn missing_dependency_actions(&self, document: &Document, range: Range) -> Vec<CodeAction> {
        let text = document.text();
        let lines: Vec<&str> = text.lines().collect();
        let cursor = range.start.line as usize;
        if cursor >= lines.len() {
            return Vec::new();
        }

        let Some((start, end)) = rule_block_at(&lines, cursor)
        else {
            return Vec::new();
        };

        let targets = declared_targets(&lines);
        let directory = document
            .uri
            .to_file_path()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));

        let mut actions = Vec::new();
        for dependency in dependencies_in(&lines[start..=end]) {
            if targets.contains(&dependency) {
                continue;
            }
            if directory
                .as_ref()
                .is_some_and(|d| d.join(&dependency).exists())
            {
                continue;
            }

            let rule = rule_building(&dependency, directory.as_deref());
            // After the faulty rule, with a blank line between them - a build
            // file reads as a list of rules, and two run together read as one.
            let at = Position {
                line: (end + 1) as u32,
                character: 0
            };
            let mut changes = std::collections::HashMap::new();
            changes.insert(
                document.uri.clone(),
                vec![TextEdit {
                    range: Range { start: at, end: at },
                    new_text: format!("\n{rule}")
                }]
            );

            actions.push(CodeAction {
                title: format!("Create a rule that builds '{dependency}'"),
                kind: Some(CodeActionKind::QUICKFIX),
                edit: Some(WorkspaceEdit {
                    changes: Some(changes),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }
        actions
    }
}

/// The span of lines of the rule containing `cursor`.
///
/// A rule starts at a `- ` at the left margin and runs until the next one.
fn rule_block_at(lines: &[&str], cursor: usize) -> Option<(usize, usize)> {
    let starts_rule = |line: &str| line.trim_start() != line.trim() || line.starts_with("- ");
    let start = (0..=cursor)
        .rev()
        .find(|index| lines[*index].starts_with("- "))?;
    let _ = starts_rule;

    let end = ((start + 1)..lines.len())
        .find(|index| lines[*index].starts_with("- "))
        .map(|next| next - 1)
        .unwrap_or(lines.len() - 1);

    // Trailing blank lines belong to the gap, not to the rule.
    let end = (start..=end)
        .rev()
        .find(|index| !lines[*index].trim().is_empty())
        .unwrap_or(end);
    Some((start, end))
}

/// Every `tgt:` in the file, however it is spelled.
fn declared_targets(lines: &[&str]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|line| value_of(line, &["tgt", "target"]))
        .flat_map(|value| {
            value
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Every `dep:` in this rule.
fn dependencies_in(lines: &[&str]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|line| value_of(line, &["dep", "deps", "requires"]))
        .flat_map(|value| {
            value
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The value of `key:` on this line, for any of `keys`.
fn value_of(line: &str, keys: &[&str]) -> Option<String> {
    let trimmed = line.trim_start().trim_start_matches("- ");
    let (key, value) = trimmed.split_once(':')?;
    keys.contains(&key.trim())
        .then(|| value.trim().trim_matches('"').to_string())
        .filter(|value| !value.is_empty())
}

/// The rule that would build `target`.
fn rule_building(target: &str, directory: Option<&std::path::Path>) -> String {
    let stem = target
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(target);
    let source = format!("{stem}.asm");
    let buildable = directory.is_some_and(|d| d.join(&source).exists());

    if buildable {
        format!(
            "- tgt: {target}\n  dep: {source}\n  cmd: basm --snapshot --output {target} {source}\n"
        )
    }
    else {
        // Nothing to guess from. A placeholder that says so beats a command
        // that looks right and builds the wrong thing.
        format!("- tgt: {target}\n  cmd: echo \"Need to finish\"\n")
    }
}

#[cfg(test)]
mod tests {
    use tower_lsp::lsp_types::Url;

    use super::*;

    fn at(line: u32) -> Range {
        Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 0 }
        }
    }

    fn actions_in(dir: &camino_tempfile::Utf8TempDir, text: &str) -> Vec<CodeAction> {
        let path = dir.path().join("bndbuild.yml");
        std::fs::write(&path, text).unwrap();
        let document = Document::new(
            Url::from_file_path(path.as_std_path()).unwrap(),
            text.to_string(),
            1
        );
        BuildFileAnalyzer::new().code_actions(&document, at(0))
    }

    const RULE: &str = "- tgt: test\n  dep: hello.sna\n  cmd: emu --snapshot=hello.sna run\n";

    /// A source of the same name is beside the build file, so the rule can be
    /// written in full.
    #[test]
    fn a_buildable_dependency_gets_a_real_rule() {
        let dir = camino_tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hello.asm"), "\tnop\n").unwrap();

        let actions = actions_in(&dir, RULE);
        assert_eq!(actions.len(), 1, "{actions:?}");
        assert_eq!(actions[0].title, "Create a rule that builds 'hello.sna'");

        let edit = &actions[0].edit.as_ref().unwrap().changes.as_ref().unwrap();
        let (_, edits) = edit.iter().next().unwrap();
        assert_eq!(
            edits[0].new_text,
            "\n- tgt: hello.sna\n  dep: hello.asm\n  cmd: basm --snapshot --output hello.sna hello.asm\n"
        );
        // After the faulty rule, with a blank line between: a build file reads
        // as a list of rules, and two run together read as one.
        assert_eq!(edits[0].range.start.line, 3, "{edits:?}");
    }

    /// Nothing to guess from: a placeholder that says so beats a command that
    /// looks right and builds the wrong thing.
    #[test]
    fn an_unbuildable_dependency_gets_a_placeholder() {
        let dir = camino_tempfile::tempdir().unwrap();
        let actions = actions_in(&dir, RULE);

        let edit = &actions[0].edit.as_ref().unwrap().changes.as_ref().unwrap();
        let (_, edits) = edit.iter().next().unwrap();
        assert_eq!(
            edits[0].new_text,
            "\n- tgt: hello.sna\n  cmd: echo \"Need to finish\"\n"
        );
    }

    /// A dependency something already builds needs nothing.
    #[test]
    fn a_dependency_with_a_rule_is_left_alone() {
        let dir = camino_tempfile::tempdir().unwrap();
        let actions = actions_in(
            &dir,
            "- tgt: test\n  dep: hello.sna\n  cmd: emu run\n\n- tgt: hello.sna\n  cmd: basm\n"
        );
        assert!(actions.is_empty(), "{actions:?}");
    }

    /// ...and so does one that is already on disk.
    #[test]
    fn a_dependency_that_exists_is_left_alone() {
        let dir = camino_tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hello.sna"), "").unwrap();
        assert!(actions_in(&dir, RULE).is_empty());
    }
}
