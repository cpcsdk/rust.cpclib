//! Code actions for Locomotive BASIC (renumbering) and the code-lens stub.

use cpclib_basic::located::LocatedBasicProgram;
use tower_lsp::lsp_types::*;

use super::BasicAnalyzer;
use crate::common::document::Document;

impl BasicAnalyzer {
    pub fn code_lens(&self, _document: &Document) -> Vec<CodeLens> {
        vec![]
    }

    pub fn code_actions(&self, document: &Document, _range: Range) -> Vec<CodeAction> {
        use cpclib_basic::renum::Renumber;
        let mut actions = Vec::new();
        let text = document.text();
        let prog = match LocatedBasicProgram::parse(&text) {
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
