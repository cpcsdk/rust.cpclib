//! Diagnostics for assembly files: parse/assembly errors mapped to LSP
//! diagnostics (recursive walk of the `AssemblerError` tree).

use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use crate::common::document::Document;

/// Safety cap on the number of recovery re-parses `analyze` will attempt: a
/// pathological file with an error on (almost) every line would otherwise
/// trigger one re-parse per line, each parsing an ever-shrinking suffix of
/// the file - bounding it keeps worst-case cost roughly linear instead of
/// quadratic, and keeps the diagnostics list from growing unbounded.
const MAX_RECOVERY_ATTEMPTS: usize = 200;

impl AssemblyAnalyzer {
    /// Analyze the document and return diagnostics.
    ///
    /// basm's parser stops at the first syntax error, like most recursive-
    /// descent parsers - a single `parse_document` call only ever surfaces
    /// that first error. To report more of them at once, on failure this
    /// resumes parsing from the line right after the last error reported,
    /// against the rest of the file, and repeats until either the remainder
    /// parses cleanly, no error location can be determined (nothing to
    /// safely resume from), or the recovery cap is hit.
    pub fn analyze(&self, document: &Document) -> Vec<Diagnostic> {
        let full_text = document.text();
        let total_lines = full_text.lines().count();

        let mut diagnostics = Vec::new();
        let mut start_line = 0usize;
        let mut attempts = 0usize;

        while start_line < total_lines && attempts < MAX_RECOVERY_ATTEMPTS {
            attempts += 1;

            let remaining: String = if start_line == 0 {
                full_text.clone()
            }
            else {
                full_text
                    .lines()
                    .skip(start_line)
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            if remaining.trim().is_empty() {
                break;
            }

            let listing_with_errors = match Self::parse_source(&remaining) {
                Ok(_) => break, // the rest of the file parses cleanly
                Err(e) => e
            };
            let error = listing_with_errors.cpclib_error_unchecked();

            let mut chunk = Vec::new();
            collect_asm_diagnostics(error, None, &mut chunk);
            if chunk.is_empty() {
                chunk.push(Diagnostic {
                    range: NO_LOCATION_RANGE,
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("basm".to_string()),
                    message: strip_ansi(&format!("{error}")),
                    ..Default::default()
                });
            }

            // `collect_asm_diagnostics`/`asm_diag` fall back to
            // `NO_LOCATION_RANGE` themselves whenever a given error carries
            // no span (e.g. a top-level `FunctionError` with no parent
            // span). Only diagnostics with a *real* location can tell us
            // where it's safe to resume parsing; a sentinel's line 0 must
            // not pull the resume point backwards, and if nothing in this
            // chunk has a real location there is nothing to safely resume
            // from at all - stop after recording it, rather than risk an
            // infinite loop or re-reporting the same error forever.
            let max_located_line = chunk
                .iter()
                .filter(|d| d.range != NO_LOCATION_RANGE)
                .map(|d| d.range.end.line)
                .max();

            for mut d in chunk {
                d.range.start.line += start_line as u32;
                d.range.end.line += start_line as u32;
                diagnostics.push(d);
            }

            let Some(max_located_line) = max_located_line
            else {
                break;
            };
            start_line += max_located_line as usize + 1;
        }

        diagnostics
    }
}

/// Sentinel range used whenever an error carries no source-location
/// information at all - see `analyze`'s recovery loop and `asm_diag`.
const NO_LOCATION_RANGE: Range = Range {
    start: Position {
        line: 0,
        character: 0
    },
    end: Position {
        line: 0,
        character: 100
    }
};

// ─── Per-error diagnostics ─────────────────────────────────────────────────────

/// Recursively walk an `AssemblerError` tree, emitting one `Diagnostic` per leaf
/// error with the closest known source location.
pub(super) fn collect_asm_diagnostics(
    error: &cpclib_asm::AssemblerError,
    parent_span: Option<&cpclib_asm::parser::Z80Span>,
    out: &mut Vec<Diagnostic>
) {
    use cpclib_asm::AssemblerError;
    match error {
        AssemblerError::MultipleErrors { errors } => {
            for e in errors {
                collect_asm_diagnostics(e, parent_span, out);
            }
        },
        AssemblerError::RelocatedError { span, error: inner } => {
            collect_asm_diagnostics(inner, Some(span), out);
        },
        AssemblerError::RelocatedWarning { warning, span } => {
            out.push(asm_diag(
                Some(span),
                format!("{warning}"),
                DiagnosticSeverity::WARNING
            ));
        },
        AssemblerError::RelocatedInfo { info, span } => {
            out.push(asm_diag(
                Some(span),
                format!("{info}"),
                DiagnosticSeverity::INFORMATION
            ));
        },
        AssemblerError::IncludedFileError { span, error: inner } => {
            out.push(asm_diag(
                Some(span),
                format!("In included file: {inner}"),
                DiagnosticSeverity::ERROR
            ));
        },
        AssemblerError::IfIssue { span, error: inner } => {
            collect_asm_diagnostics(inner, Some(span), out);
        },
        AssemblerError::ForIssue { span, error: inner } => {
            collect_asm_diagnostics(inner, span.as_ref(), out);
        },
        AssemblerError::RepeatIssue {
            span, error: inner, ..
        } => {
            collect_asm_diagnostics(inner, span.as_ref(), out);
        },
        AssemblerError::WhileIssue { span, error: inner } => {
            collect_asm_diagnostics(inner, span.as_ref(), out);
        },
        AssemblerError::MacroError {
            name,
            location,
            root
        } => {
            let prefix = if let Some(loc) = location {
                format!("Macro {} (defined at {}): ", name, loc)
            }
            else {
                format!("Macro {}: ", name)
            };
            let mut sub = Vec::new();
            collect_asm_diagnostics(root, parent_span, &mut sub);
            for mut d in sub {
                d.message = format!("{}{}", prefix, d.message);
                out.push(d);
            }
        },
        AssemblerError::CrunchedSectionError { error: inner } => {
            collect_asm_diagnostics(inner, parent_span, out);
        },
        AssemblerError::FunctionError(name, inner) => {
            let msg = format!("Function {name}: {inner}");
            out.push(asm_diag(parent_span, msg, DiagnosticSeverity::ERROR));
        },
        AssemblerError::SyntaxError { error: parse_err } => {
            let message = strip_ansi(&format!("{error}"));
            // primary_span_and_end gives exact source byte offsets — no tab expansion issues.
            if let Some((span, end_off)) = parse_err.primary_span_and_end() {
                let (line_1, col_1) = span.relative_line_and_column();
                let line = line_1.saturating_sub(1) as u32;
                let col = col_1.saturating_sub(1) as u32;
                let len = end_off.saturating_sub(span.offset_from_start()) as u32;
                out.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line,
                            character: col
                        },
                        end: Position {
                            line,
                            character: col + len.max(1)
                        }
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("basm".to_string()),
                    message,
                    ..Default::default()
                });
                return;
            }
            let owned_span = parse_err.primary_z80span();
            let span_ref = owned_span.as_ref().or(parent_span);
            out.push(asm_diag(span_ref, message, DiagnosticSeverity::ERROR));
        },
        AssemblerError::AlreadyRenderedError(s) => {
            out.push(asm_diag(
                parent_span,
                strip_ansi(s),
                DiagnosticSeverity::ERROR
            ));
        },
        other => {
            out.push(asm_diag(
                parent_span,
                format!("{other}"),
                DiagnosticSeverity::ERROR
            ));
        }
    }
}

pub(super) fn asm_diag(
    span: Option<&cpclib_asm::parser::Z80Span>,
    message: String,
    severity: DiagnosticSeverity
) -> Diagnostic {
    let range = if let Some(s) = span {
        let (line_1, col_1) = s.relative_line_and_column();
        let line = line_1.saturating_sub(1) as u32;
        let col = col_1.saturating_sub(1) as u32;
        let span_text: &str = s.as_ref();
        // Highlight to end of the current instruction (next `:` separator) or end of line.
        let first_line = span_text.lines().next().unwrap_or(span_text);
        let len = (first_line.find(':').unwrap_or(first_line.len()) as u32).max(1);
        Range {
            start: Position {
                line,
                character: col
            },
            end: Position {
                line,
                character: col + len
            }
        }
    }
    else {
        NO_LOCATION_RANGE
    };
    Diagnostic {
        range,
        severity: Some(severity),
        source: Some("basm".to_string()),
        message,
        ..Default::default()
    }
}

pub(super) fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek().copied() {
                Some('[') => {
                    chars.next(); // consume '['
                    // CSI: consume until final byte in 0x40..=0x7E ('@'..='~')
                    for c2 in chars.by_ref() {
                        if ('@'..='~').contains(&c2) {
                            break;
                        }
                    }
                },
                Some(c2) if ('\x40'..='\x5F').contains(&c2) => {
                    chars.next(); // 2-char Fe sequence
                },
                _ => {}
            }
        }
        else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basm::AssemblyAnalyzer;

    fn diagnostics_for(text: &str) -> Vec<Diagnostic> {
        let uri = Url::parse("file:///t.asm").unwrap();
        let document = Document::new(uri, text.to_string(), 1);
        AssemblyAnalyzer::new().analyze(&document)
    }

    #[test]
    fn valid_file_yields_no_diagnostics() {
        let text = "org 0x4000\n ld a, 1\n ret\n";
        assert!(diagnostics_for(text).is_empty());
    }

    #[test]
    fn single_syntax_error_is_reported_on_its_own_line() {
        let text = "org 0x4000\n@#$ garbage @#$\n ld a, 1\n ret\n";
        let diags = diagnostics_for(text);
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert_eq!(diags[0].range.start.line, 1);
        assert_eq!(diags[0].severity, Some(DiagnosticSeverity::ERROR));
    }

    #[test]
    fn several_syntax_errors_across_the_file_are_all_reported() {
        // Regression test for the "only the first error is ever shown"
        // report: basm's parser stops at the first syntax error like most
        // recursive-descent parsers, so this exercises `analyze`'s recovery
        // loop, which re-parses the remainder of the file after each error.
        let text = "org 0x4000\n@#$ garbage1 @#$\n ld a, 1\n@#$ garbage2 @#$\n ld b, 2\n ret\n";
        let diags = diagnostics_for(text);
        assert_eq!(diags.len(), 2, "{diags:?}");

        let lines: Vec<u32> = diags.iter().map(|d| d.range.start.line).collect();
        assert_eq!(lines, vec![1, 3], "{diags:?}");
        for d in &diags {
            assert_eq!(d.severity, Some(DiagnosticSeverity::ERROR));
        }
    }

    #[test]
    fn recovery_gives_up_gracefully_when_the_last_error_has_no_usable_location() {
        // Not a behavioral requirement so much as a safety guard: a
        // location-less trailing error must not spin the recovery loop or
        // panic - it should just be recorded once and stop.
        let text = "org 0x4000\n@#$ garbage @#$\n ret\n";
        let diags = diagnostics_for(text);
        assert!(!diags.is_empty(), "{diags:?}");
    }
}
