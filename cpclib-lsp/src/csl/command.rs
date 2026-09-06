//! The "▶ Run in emulator" code lens for `.csl` files.

use tower_lsp::lsp_types::*;

use super::CslAnalyzer;
use crate::common::document::Document;

impl CslAnalyzer {
    /// A single fixed lens at the top of the file, wired to
    /// `cpclib.runCsl` (`server/backend.rs`). Mirrors
    /// `locomotive::command::BasicAnalyzer::code_lens`'s shape, minus the
    /// debug lens - CSL scripts have no debug-adapter integration.
    ///
    /// Whether this is shown at all (`config().code_lens`) is checked by
    /// the caller (`server/backend.rs`'s `code_lens` dispatch), same
    /// convention as every other language's analyzer here - this method
    /// only decides the lens's own content.
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
        vec![CodeLens {
            range: Range {
                start: Position::new(0, 0),
                end: Position::new(0, 0)
            },
            command: Some(Command {
                title: "▶ Run in emulator".to_string(),
                command: "cpclib.runCsl".to_string(),
                arguments: Some(vec![serde_json::json!(file_path)])
            }),
            data: None
        }]
    }
}

#[cfg(test)]
mod tests {
    use tower_lsp::lsp_types::Url;

    use super::*;

    fn doc(text: &str) -> Document {
        Document::new_with_language(
            Url::parse("file:///t.csl").unwrap(),
            text.to_string(),
            1,
            Some("csl")
        )
    }

    #[test]
    fn a_non_empty_document_gets_a_run_lens() {
        let analyzer = CslAnalyzer::default();
        let lenses = analyzer.code_lens(&doc("wait 1000\n"));
        assert_eq!(lenses.len(), 1);
        let run = lenses[0].command.as_ref().unwrap();
        assert_eq!(run.command, "cpclib.runCsl");
        assert_eq!(run.arguments.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn an_empty_document_gets_no_lens() {
        let analyzer = CslAnalyzer::default();
        let lenses = analyzer.code_lens(&doc("   \n"));
        assert!(lenses.is_empty());
    }
}
