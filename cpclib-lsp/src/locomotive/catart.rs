//! CatArt-whitelist diagnostics for `.CAT`/`.ASC` documents.
//!
//! Deliberately does NOT re-derive the CatArt command whitelist by hand: it
//! reuses `cpclib_catart::convert::basic_to_commands` directly, the same
//! converter CatArt itself uses, so this can never drift out of sync with
//! what CatArt actually accepts. Runs it once per BASIC source line
//! (re-parsing just that line in isolation) rather than once for the whole
//! document, because `basic_to_commands` stops at the first error (a
//! `Result`, not a collected `Vec`) - per-line re-invocation turns that into
//! "one diagnostic per offending line" instead of just the first.

use cpclib_basic::BasicProgram;
use cpclib_catart::basic_command::BasicCommand;
use cpclib_catart::convert::basic_to_commands;
use cpclib_catart::error::CatArtError;
use tower_lsp::lsp_types::*;

use super::BasicAnalyzer;
use crate::common::document::Document;

impl BasicAnalyzer {
    /// CatArt-specific diagnostics for `document`, on top of whatever
    /// `analyze` already reports (parse errors, undefined-line warnings).
    /// Returns `Vec::new()` if the document doesn't even parse as located
    /// BASIC - that failure is `analyze`'s diagnostic to report, not
    /// duplicated here.
    pub fn catart_diagnostics(&self, document: &Document) -> Vec<Diagnostic> {
        let Ok(prog) = self.parse_cached(document)
        else {
            return Vec::new();
        };

        let mut diagnostics = Vec::new();
        for bline in &prog.lines {
            let Some(source_text) = document.line(bline.source_line as usize)
            else {
                continue;
            };
            // If the isolated line doesn't even parse as plain BASIC, leave
            // it to `analyze`'s own parse-error diagnostic.
            let Ok(line_program) = BasicProgram::parse(&source_text)
            else {
                continue;
            };
            let line_len = source_text.chars().count() as u32;
            let range = Range {
                start: Position {
                    line: bline.source_line,
                    character: 0
                },
                end: Position {
                    line: bline.source_line,
                    character: line_len
                }
            };
            match basic_to_commands(&line_program) {
                Err(err) => {
                    let detail = match &err {
                        CatArtError::IncompatibleBasicCommand(_, msg) => msg.clone(),
                        CatArtError::NotEnoughTokens(msg) => msg.clone(),
                        CatArtError::InvalidParameter(_, msg) => msg.clone()
                    };
                    diagnostics.push(Diagnostic {
                        range,
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: format!("Not a valid CatArt command: {detail}"),
                        source: Some("cpclib-lsp".into()),
                        ..Default::default()
                    });
                },
                Ok(commands) => {
                    for cmd in commands.iter() {
                        let noop_name = match cmd {
                            BasicCommand::CursorOn | BasicCommand::CursorOff => Some("CURSOR"),
                            BasicCommand::Symbol(..) => Some("SYMBOL"),
                            _ => None
                        };
                        if let Some(name) = noop_name {
                            diagnostics.push(Diagnostic {
                                range,
                                severity: Some(DiagnosticSeverity::WARNING),
                                message: format!(
                                    "{name} is a no-op in CatArt (ignored by the renderer)"
                                ),
                                source: Some("cpclib-lsp".into()),
                                ..Default::default()
                            });
                        }
                    }
                },
            }
        }
        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///t.asc").unwrap(), text.to_string(), 1)
    }

    #[test]
    fn valid_catart_source_has_no_diagnostics() {
        let analyzer = BasicAnalyzer::new();
        let d = doc("10 INK 1,26\n20 PAPER 1\n30 PEN 0\n40 PRINT \"HI\"\n50 CLS\n");
        assert!(analyzer.catart_diagnostics(&d).is_empty());
    }

    #[test]
    fn catart2_asc_fixture_has_no_error_diagnostics() {
        let analyzer = BasicAnalyzer::new();
        let d = doc(include_str!("../../tests/fixtures/CATART2.ASC"));
        let diags = analyzer.catart_diagnostics(&d);
        assert!(
            diags
                .iter()
                .all(|d| d.severity != Some(DiagnosticSeverity::ERROR)),
            "{diags:?}"
        );
    }

    #[test]
    fn goto_outside_the_whitelist_is_flagged_as_an_error() {
        let analyzer = BasicAnalyzer::new();
        let d = doc("10 GOTO 20\n20 PRINT \"X\"\n");
        let diags = analyzer.catart_diagnostics(&d);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diags[0].range.start.line, 0);
    }

    #[test]
    fn cursor_and_symbol_are_valid_but_flagged_as_no_op_warnings() {
        let analyzer = BasicAnalyzer::new();
        let d = doc("10 CURSOR 1\n20 SYMBOL 65,1,2,3,4,5,6,7,8\n");
        let diags = analyzer.catart_diagnostics(&d);
        assert_eq!(diags.len(), 2, "{diags:?}");
        assert!(
            diags
                .iter()
                .all(|d| d.severity == Some(DiagnosticSeverity::WARNING))
        );
        assert!(diags[0].message.contains("CURSOR"));
        assert!(diags[1].message.contains("SYMBOL"));
    }

    #[test]
    fn comment_and_blank_lines_are_not_flagged() {
        let analyzer = BasicAnalyzer::new();
        let d = doc("10 REM hello\n20 \n30 CLS\n");
        assert!(analyzer.catart_diagnostics(&d).is_empty());
    }
}
