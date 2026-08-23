//! Code actions for Locomotive BASIC (renumbering) and the "▶ Run in
//! emulator" code lens.

use tower_lsp::lsp_types::*;

use super::BasicAnalyzer;
use crate::common::document::Document;

impl BasicAnalyzer {
    /// "▶ Run in emulator" and "🐞 Debug in emulator" lenses at the top of
    /// the file, wired to `cpclib.runBasic` (`server/backend.rs`) and the
    /// client-side `cpclib.debugBasic` (`cpclib-vscode/src/debug.ts`).
    /// Mirrors `BuildFileAnalyzer::code_lens`'s shape
    /// (`bndbuild/semantic_tokens.rs`) but with fixed lenses instead of one
    /// per target, and `embedded_bndbuild.rs`'s Run-then-Debug pairing for
    /// `.asm` files.
    pub fn code_lens(&self, document: &Document) -> Vec<CodeLens> {
        if document.text().trim().is_empty() {
            return vec![];
        }
        let file_path = document
            .uri
            .to_file_path()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(0, 0)
        };
        vec![
            CodeLens {
                range,
                command: Some(Command {
                    title: "▶ Run in emulator".to_string(),
                    command: "cpclib.runBasic".to_string(),
                    arguments: Some(vec![serde_json::json!(file_path)])
                }),
                data: None
            },
            CodeLens {
                range,
                command: Some(Command {
                    title: "🐞 Debug in emulator".to_string(),
                    command: "cpclib.debugBasic".to_string(),
                    arguments: Some(vec![serde_json::json!(file_path)])
                }),
                data: None
            },
        ]
    }

    pub fn code_actions(&self, document: &Document, _range: Range) -> Vec<CodeAction> {
        use cpclib_basic::renum::Renumber;
        let mut actions = Vec::new();
        let prog = match self.parse_cached(document) {
            Ok(p) => p,
            Err(_) => return actions
        };
        if prog.lines.is_empty() {
            return actions;
        }
        let subs = prog.renum_substitutions(10, 10);
        if subs.is_empty() {
            return actions;
        }
        let edits: Vec<TextEdit> = subs
            .iter()
            .map(|(line, col, len, new_text)| {
                TextEdit {
                    range: Range {
                        start: Position {
                            line: *line,
                            character: *col
                        },
                        end: Position {
                            line: *line,
                            character: col + len
                        }
                    },
                    new_text: new_text.clone()
                }
            })
            .collect();
        let edit = WorkspaceEdit {
            changes: Some(std::collections::HashMap::from([(
                document.uri.clone(),
                edits
            )])),
            ..Default::default()
        };
        actions.push(CodeAction {
            title: "Renumber BASIC lines (10, 20, 30…)".to_string(),
            kind: Some(CodeActionKind::REFACTOR_REWRITE),
            edit: Some(edit),
            ..Default::default()
        });
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///t.bas").unwrap(), text.to_string(), 1)
    }

    #[test]
    fn a_non_empty_document_gets_a_run_lens_and_a_debug_lens() {
        let analyzer = BasicAnalyzer::default();
        let lenses = analyzer.code_lens(&doc("10 PRINT \"HI\"\n"));
        assert_eq!(lenses.len(), 2);
        let run = lenses[0].command.as_ref().unwrap();
        assert_eq!(run.command, "cpclib.runBasic");
        assert_eq!(run.arguments.as_ref().unwrap().len(), 1);
        let debug = lenses[1].command.as_ref().unwrap();
        assert_eq!(debug.command, "cpclib.debugBasic");
        assert_eq!(debug.arguments.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn an_empty_document_gets_no_lens() {
        let analyzer = BasicAnalyzer::default();
        let lenses = analyzer.code_lens(&doc("   \n"));
        assert!(lenses.is_empty());
    }
}
