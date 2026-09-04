//! Diagnostics for Locomotive BASIC: parse errors, FOR/NEXT balance,
//! use-before-assignment variable tracking.

use std::collections::HashMap;

use cpclib_basic::located::{LocatedBasicToken, LocatedTokenKind};
use cpclib_basic::tokens::BasicTokenNoPrefix;
use tower_lsp::lsp_types::*;

use super::BasicAnalyzer;
use crate::common::document::Document;

impl BasicAnalyzer {
    pub fn analyze(&self, document: &Document) -> Vec<Diagnostic> {
        // Parse error → one diagnostic at line 0.
        let prog = match self.parse_cached(document) {
            Ok(p) => p,
            Err(e) => {
                return vec![Diagnostic {
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0
                        },
                        end: Position {
                            line: 0,
                            character: 1
                        }
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: e.to_string(),
                    source: Some("cpclib-lsp".into()),
                    ..Default::default()
                }];
            }
        };

        // Build the set of line numbers that actually exist.
        let defined: std::collections::HashSet<u16> =
            prog.lines.iter().map(|l| l.line_number).collect();

        let mut diagnostics = Vec::new();
        let warn_undefined_line = self.config().warnings.undefined_line;

        for bline in &prog.lines {
            let mut after_jump = false;
            for tok in &bline.tokens {
                match &tok.kind {
                    LocatedTokenKind::Keyword(kw) => {
                        after_jump = matches!(
                            kw,
                            BasicTokenNoPrefix::Goto
                                | BasicTokenNoPrefix::Gosub
                                | BasicTokenNoPrefix::Restore
                                | BasicTokenNoPrefix::Run
                                | BasicTokenNoPrefix::Then
                                | BasicTokenNoPrefix::Else
                                | BasicTokenNoPrefix::OnErrorGoto
                        );
                    },
                    LocatedTokenKind::Number(n) if after_jump => {
                        if let Ok(target) = n.parse::<u16>()
                            && warn_undefined_line && !defined.contains(&target) {
                                diagnostics.push(Diagnostic {
                                    range: Range {
                                        start: Position {
                                            line: tok.span.line,
                                            character: tok.span.col
                                        },
                                        end: Position {
                                            line: tok.span.line,
                                            character: tok.span.col + tok.span.len
                                        }
                                    },
                                    severity: Some(DiagnosticSeverity::WARNING),
                                    message: format!("Undefined BASIC line {target}"),
                                    source: Some("cpclib-lsp".into()),
                                    ..Default::default()
                                });
                            }
                        // Keep after_jump: comma-separated targets for ON GOTO.
                    },
                    LocatedTokenKind::Other(',') => {}, // keep state for ON GOTO n,n,n
                    LocatedTokenKind::Space => {},      // keep state
                    LocatedTokenKind::Separator => {
                        after_jump = false;
                    },
                    _ => {
                        after_jump = false;
                    }
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use tower_lsp::lsp_types::{DiagnosticSeverity, Url};

    use super::*;
    use crate::locomotive::BasicAnalyzer;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///t.bas").unwrap(), text.to_string(), 1)
    }

    #[test]
    fn undefined_goto_target_is_a_warning() {
        let analyzer = BasicAnalyzer::new();
        let d = doc("10 GOTO 20\n");
        let diags = analyzer.analyze(&d);
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert_eq!(diags[0].severity, Some(DiagnosticSeverity::WARNING));
        assert!(diags[0].message.contains("Undefined BASIC line 20"));
    }

    #[test]
    fn disabling_undefined_line_warnings_suppresses_them() {
        let analyzer = BasicAnalyzer::new();
        let d = doc("10 GOTO 20\n");
        assert_eq!(analyzer.analyze(&d).len(), 1);

        let mut config = crate::common::config::BasicConfig::default();
        config.warnings.undefined_line = false;
        analyzer.set_config(config);
        assert!(analyzer.analyze(&d).is_empty());
    }
}

/// Record the first occurrence of a variable in `seen`.
pub(super) fn record_var(seen: &mut HashMap<String, (String, u32, u32)>, tok: &LocatedBasicToken) {
    if let LocatedTokenKind::Variable(name) = &tok.kind {
        let key = name.to_uppercase();
        seen.entry(key)
            .or_insert_with(|| (name.clone(), tok.span.line, tok.span.col));
    }
}

/// Collect variables from a comma-separated list starting at `from`.
pub(super) fn collect_comma_separated_vars(
    toks: &[LocatedBasicToken],
    from: usize,
    seen: &mut HashMap<String, (String, u32, u32)>
) {
    let mut i = from;
    while i < toks.len() {
        match &toks[i].kind {
            LocatedTokenKind::Space | LocatedTokenKind::Other(',') => {
                i += 1;
            },
            LocatedTokenKind::Variable(_) => {
                record_var(seen, &toks[i]);
                i += 1;
            },
            LocatedTokenKind::Separator => break, // `:` ends the statement
            _ => {
                i += 1;
            }
        }
    }
}

/// Collect variables after INPUT (skip optional stream # and prompt string).
pub(super) fn collect_vars_after_input(
    toks: &[LocatedBasicToken],
    from: usize,
    seen: &mut HashMap<String, (String, u32, u32)>
) {
    let mut i = from;
    // Skip optional stream: `#n,`
    if i < toks.len() && matches!(&toks[i].kind, LocatedTokenKind::Other('#')) {
        // skip # digit ,
        while i < toks.len() && !matches!(&toks[i].kind, LocatedTokenKind::Other(',')) {
            i += 1;
        }
        if i < toks.len() {
            i += 1;
        } // skip comma
    }
    // Skip optional prompt string followed by `;` or `,`.
    if i < toks.len()
        && let LocatedTokenKind::StringLit(_) = &toks[i].kind {
            i += 1;
            // Skip the `;` or `,` separator.
            while i < toks.len()
                && matches!(
                    &toks[i].kind,
                    LocatedTokenKind::Space
                        | LocatedTokenKind::Other(';')
                        | LocatedTokenKind::Other(',')
                )
            {
                i += 1;
            }
        }
    collect_comma_separated_vars(toks, i, seen);
}
