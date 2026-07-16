//! Diagnostics for assembly files: parse/assembly errors mapped to LSP
//! diagnostics (recursive walk of the `AssemblerError` tree).

use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use crate::common::document::Document;

impl AssemblyAnalyzer {
    /// Analyze the document and return diagnostics
    pub fn analyze(&self, document: &Document) -> Vec<Diagnostic> {
        match self.parse_document(document) {
            Ok(_) => vec![],
            Err(listing_with_errors) => {
                let error = listing_with_errors.cpclib_error_unchecked();
                let mut diagnostics = Vec::new();
                collect_asm_diagnostics(error, None, &mut diagnostics);
                if diagnostics.is_empty() {
                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: 0,
                                character: 0
                            },
                            end: Position {
                                line: 0,
                                character: 100
                            }
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        source: Some("basm".to_string()),
                        message: strip_ansi(&format!("{error}")),
                        ..Default::default()
                    });
                }
                diagnostics
            }
        }
    }
}

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
    let (start, end_pos) = if let Some(s) = span {
        let (line_1, col_1) = s.relative_line_and_column();
        let line = line_1.saturating_sub(1) as u32;
        let col = col_1.saturating_sub(1) as u32;
        let span_text: &str = s.as_ref();
        // Highlight to end of the current instruction (next `:` separator) or end of line.
        let first_line = span_text.lines().next().unwrap_or(span_text);
        let len = (first_line.find(':').unwrap_or(first_line.len()) as u32).max(1);
        (
            Position {
                line,
                character: col
            },
            Position {
                line,
                character: col + len
            }
        )
    }
    else {
        (
            Position {
                line: 0,
                character: 0
            },
            Position {
                line: 0,
                character: 100
            }
        )
    };
    Diagnostic {
        range: Range {
            start,
            end: end_pos
        },
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
