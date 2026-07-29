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
    /// Returns `Vec::new()` if the whole document doesn't even parse as
    /// located BASIC - that failure is `analyze`'s diagnostic to report, not
    /// duplicated here. A single malformed *line* is different: it's
    /// reported here directly (not deferred to `analyze`), since
    /// `LocatedBasicProgram` (what `analyze` uses) tokenizes more
    /// permissively than the strict grammar this function re-checks each
    /// line against, and would otherwise miss it entirely.
    pub fn catart_diagnostics(&self, document: &Document) -> Vec<Diagnostic> {
        let Ok(prog) = self.parse_cached(document)
        else {
            return Vec::new();
        };

        let mut diagnostics = Vec::new();
        let warn_catart_no_op = self.config().warnings.catart_no_op;
        for bline in &prog.lines {
            let Some(source_text) = document.line(bline.source_line as usize)
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
            // Re-parse just this line in isolation via the strict grammar
            // (`BasicProgram::parse`/`parse_basic_line`), not the more
            // permissive `LocatedBasicProgram` tokenizer `analyze` itself
            // relies on - the latter doesn't validate a statement's exact
            // argument count/shape (confirmed directly: a `SYMBOL` missing
            // one of its 9 required arguments tokenizes "successfully" with
            // `analyze` producing no diagnostic at all), so a genuine syntax
            // error here would otherwise be invisible end-to-end rather than
            // just mis-attributed.
            let line_program = match BasicProgram::parse(&source_text) {
                Ok(p) => p,
                Err(e) => {
                    diagnostics.push(Diagnostic {
                        range,
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: format!("Not a valid CatArt command: {e}"),
                        source: Some("cpclib-lsp".into()),
                        ..Default::default()
                    });
                    continue;
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
                        if warn_catart_no_op && let Some(name) = noop_name {
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
    fn disabling_catart_no_op_warnings_suppresses_them() {
        let analyzer = BasicAnalyzer::new();
        let d = doc("10 CURSOR 1\n20 SYMBOL 65,1,2,3,4,5,6,7,8\n");
        assert_eq!(analyzer.catart_diagnostics(&d).len(), 2);

        let mut config = crate::common::config::BasicConfig::default();
        config.warnings.catart_no_op = false;
        analyzer.set_config(config);
        assert!(analyzer.catart_diagnostics(&d).is_empty());
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

    /// Regression test for a real report: `SYMBOL` missing one of its 9
    /// required arguments (8 given instead of 9) produced no diagnostic at
    /// all end-to-end - not because it was accepted, but because
    /// `LocatedBasicProgram` (what `analyze` uses) tokenizes more
    /// permissively than the strict grammar and silently produced zero
    /// tokens for the malformed line, while `catart_diagnostics`'s own
    /// isolated per-line reparse used to defer silently to `analyze` on any
    /// parse failure. Fixed on two levels: `cpclib-basic`'s `parse_basic_line`
    /// no longer silently discards a statement that committed to a keyword
    /// (via `cut_err`) and then failed its argument grammar, and
    /// `catart_diagnostics` now reports that failure itself rather than
    /// assuming `analyze` will.
    #[test]
    fn a_symbol_statement_missing_one_argument_is_flagged_as_an_error() {
        let analyzer = BasicAnalyzer::new();
        let d = doc("1100 SYMBOL 0,0,0,0,0,0,0,0\n");
        let diags = analyzer.catart_diagnostics(&d);
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert_eq!(diags[0].severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diags[0].range.start.line, 0);
    }

    #[test]
    fn comment_and_blank_lines_are_not_flagged() {
        let analyzer = BasicAnalyzer::new();
        let d = doc("10 REM hello\n20 \n30 CLS\n");
        assert!(analyzer.catart_diagnostics(&d).is_empty());
    }

    /// Cross-validation for `autocomplete::CATART_ALLOWED_KEYWORDS` (the
    /// completion-filtering whitelist): every keyword it lists must actually
    /// be accepted by the real `basic_to_commands` whitelist enforced here,
    /// or completion would offer a keyword this module then flags as an
    /// error - the two lists are maintained independently (one keyed by
    /// keyword text for filtering, one implicit in `cpclib_catart`'s match
    /// arms) and must not be allowed to silently drift apart.
    #[test]
    fn every_completion_whitelisted_keyword_is_accepted_without_error() {
        let analyzer = BasicAnalyzer::new();
        let minimal_usage: &[(&str, &str)] = &[
            ("INK", "10 INK 1,1"),
            ("PAPER", "10 PAPER 1"),
            ("PEN", "10 PEN 1"),
            ("PRINT", "10 PRINT \"X\""),
            ("CURSOR", "10 CURSOR 1"),
            ("MODE", "10 MODE 1"),
            ("LOCATE", "10 LOCATE 1,1"),
            ("WINDOW", "10 WINDOW 1,1,1,1"),
            ("BORDER", "10 BORDER 1"),
            ("CLS", "10 CLS"),
            ("SYMBOL", "10 SYMBOL 1,1,1,1,1,1,1,1,1")
        ];
        assert_eq!(
            minimal_usage.len(),
            super::super::autocomplete::CATART_ALLOWED_KEYWORDS.len(),
            "add a minimal-usage fixture here for every entry in CATART_ALLOWED_KEYWORDS"
        );
        for (keyword, line) in minimal_usage {
            assert!(
                super::super::autocomplete::CATART_ALLOWED_KEYWORDS
                    .iter()
                    .any(|kw| kw.eq_ignore_ascii_case(keyword)),
                "{keyword} is tested here but missing from CATART_ALLOWED_KEYWORDS"
            );
            let diags = analyzer.catart_diagnostics(&doc(line));
            assert!(
                diags
                    .iter()
                    .all(|d| d.severity != Some(DiagnosticSeverity::ERROR)),
                "{keyword} ({line}) should be accepted without error: {diags:?}"
            );
        }
    }
}
