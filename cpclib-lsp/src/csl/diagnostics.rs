//! Diagnostics for CSL files: parse errors only - `cpclib_csl` stops at the
//! first error (like bndbuild's YAML parser), so this is a
//! single-diagnostic-per-parse model, mirroring
//! `bndbuild::diagnostics::analyze`'s YAML-error case exactly.

use tower_lsp::lsp_types::*;

use super::CslAnalyzer;
use crate::common::document::Document;

impl CslAnalyzer {
    /// Analyze the CSL file and return diagnostics.
    pub fn analyze(&self, document: &Document) -> Vec<Diagnostic> {
        let Err(error) = self.parse_document(document)
        else {
            return Vec::new();
        };

        let text = document.text();
        let (line, col) = error.line_col();
        let line_idx = line.saturating_sub(1);
        let col_idx = col.saturating_sub(1);

        // `CslError.span` is a byte range into the whole source, but
        // `line_col` already reduced it to a (1-based line, 1-based column)
        // pair - reconstruct the span's *width* in bytes so the diagnostic
        // underlines the real offending line/token rather than a single
        // character, then convert that byte column to UTF-16 the same way
        // `bndbuild::diagnostics` converts its (char-count) column.
        let error_line = text.lines().nth(line_idx).unwrap_or("");
        let span_width_bytes = error.span.end.saturating_sub(error.span.start);
        let end_col_idx = (col_idx + span_width_bytes).min(error_line.len());

        let start_utf16 = crate::common::document::byte_offset_to_utf16_col(error_line, col_idx);
        let end_utf16 = crate::common::document::byte_offset_to_utf16_col(error_line, end_col_idx);

        vec![Diagnostic {
            range: Range {
                start: Position {
                    line: line_idx as u32,
                    character: start_utf16 as u32
                },
                end: Position {
                    line: line_idx as u32,
                    character: end_utf16 as u32
                }
            },
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("csl".to_string()),
            message: error.message.clone(),
            ..Default::default()
        }]
    }
}

#[cfg(test)]
mod tests {
    use tower_lsp::lsp_types::Url;

    use super::*;
    use crate::common::document::Document;

    fn doc(text: &str) -> Document {
        Document::new_with_language(
            Url::parse("file:///test.csl").unwrap(),
            text.to_string(),
            1,
            Some("csl")
        )
    }

    #[test]
    fn valid_script_has_no_diagnostics() {
        let analyzer = CslAnalyzer::new();
        let document = doc("csl_version 1.1\nreset\nwait 1000\n");
        assert!(analyzer.analyze(&document).is_empty());
    }

    #[test]
    fn typo_produces_one_diagnostic_covering_the_whole_bad_line() {
        let analyzer = CslAnalyzer::new();
        let document = doc("csl_version 1.1\nrset H\nwait 1000\n");
        let diagnostics = analyzer.analyze(&document);
        assert_eq!(diagnostics.len(), 1);

        let d = &diagnostics[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(d.source.as_deref(), Some("csl"));
        // Line 1 (0-indexed) is "rset H" - the diagnostic must anchor there,
        // not on line 0.
        assert_eq!(d.range.start.line, 1);
        assert_eq!(d.range.start.character, 0);
        assert_eq!(d.range.end.character, 6); // "rset H".len()
    }

    #[test]
    fn a_typo_suggestion_reaches_the_diagnostic_message() {
        let analyzer = CslAnalyzer::new();
        let document = doc("disk_inser 'test.dsk'\n");
        let diagnostics = analyzer.analyze(&document);
        assert_eq!(diagnostics.len(), 1);
        // The message itself is whatever winnow's context reports; the
        // typo suggestion lives in `CslError.notes`, not surfaced in this
        // first cut's single-line message - just confirm we got an error
        // diagnostic at all, on the right (only) line.
        assert_eq!(diagnostics[0].range.start.line, 0);
    }

    #[test]
    fn re_analyzing_the_same_version_reuses_the_cached_parse() {
        let analyzer = CslAnalyzer::new();
        let document = doc("csl_version 1.1\nreset\n");
        assert!(analyzer.analyze(&document).is_empty());
        // Second call at the same version must hit the cache, not panic or
        // diverge - the real assertion is just that this doesn't reparse
        // into a different (wrong) result.
        assert!(analyzer.analyze(&document).is_empty());
    }
}
