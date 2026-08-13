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
        let disabled_parser_categories =
            super::parse::disabled_parser_warning_categories(&self.config().warnings);
        let full_text = document.text();
        let total_lines = full_text.lines().count();
        // Byte offset of the start of each line, indexed by 0-based line
        // number, precomputed once so `remaining` below can be a zero-copy
        // slice instead of `full_text.lines().skip(n).collect::<Vec<_>>().join("\n")`
        // rebuilt on every one of up to `MAX_RECOVERY_ATTEMPTS` retries -
        // for a file with errors scattered throughout it, that used to be
        // O(n²) in the worst case (each retry re-copying the whole
        // remaining tail). `line_starts.len()` is always >= `total_lines`
        // (an extra trailing entry for a final `\n` is harmless, never
        // indexed), so `line_starts[start_line]` is safe for every
        // `start_line < total_lines`.
        let mut line_starts: Vec<usize> = vec![0];
        line_starts.extend(full_text.match_indices('\n').map(|(i, _)| i + 1));

        let mut diagnostics = Vec::new();
        let mut start_line = 0usize;
        let mut attempts = 0usize;

        while start_line < total_lines && attempts < MAX_RECOVERY_ATTEMPTS {
            attempts += 1;

            let remaining = &full_text[line_starts[start_line]..];
            if remaining.trim().is_empty() {
                break;
            }

            let listing_with_errors = match Self::parse_source(
                remaining,
                Some(&document.uri),
                disabled_parser_categories
            ) {
                Ok(_) => break, // the rest of the file parses cleanly
                Err(e) => e
            };
            let error = listing_with_errors.cpclib_error_unchecked();

            let mut chunk = Vec::new();
            collect_asm_diagnostics(error, None, document, &mut chunk);
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

        // The document parsed cleanly (the loop above only reaches here once
        // `parse_source` succeeds, or the file is empty) - only then is it
        // worth actually assembling to surface the assembler's own warnings
        // (e.g. `ld hl, de`, a "fake instruction" basm accepts but which
        // isn't real Z80). A document with syntax errors can't assemble
        // meaningfully, so skip the extra dry-run pass for it. `dry_run_env_cached`
        // (a real assemble, needed here for actual assembler warnings -
        // unlike most hover value-substitution, which only needs
        // `local_symbols_env_cached`'s lightweight local `EQU`/`SET`
        // resolution) is cached per document version, so this only pays
        // the real cost once per edit even though `analyze` and hover can
        // both request it for the same version.
        //
        // Uses `self.parse_document(document)` (the cached listing), not a
        // fresh `Self::parse_source` call the way the recovery loop above
        // does: `Env::address_trace` (`cpclib-asm`) keys recorded addresses
        // by the exact parse that produced them, not by source text alone,
        // so an address-aware constraint (`reachableByJr`, i.e. `jp2jr`)
        // silently resolves to nothing if the `Env` handed to
        // `collect_peephole_warnings` was cached against a *different*
        // parse than the `listing` passed alongside it - which a fresh
        // re-parse here would be whenever some other request (e.g. the
        // quickfix, or hover) already populated the env cache first via
        // `self.parse_document`. Reusing the same cached listing everywhere
        // guarantees they're always the same parse.
        if diagnostics.is_empty()
            && let Ok(listing) = self.parse_document(document)
        {
            let (mut env, own_complete) = self.dry_run_env_cached_checked(document, &listing);
            collect_assembler_warnings(&env, document, &mut diagnostics);
            enrich_fake_instruction_diagnostics(document, &mut diagnostics);
            Self::enrich_overflow_diagnostics(&listing, &mut env, &mut diagnostics);
            if self.config().warnings.unused_bindings {
                collect_unused_binding_warnings(&listing, document, &mut diagnostics);
            }
            // Either the warning class is on, or the user asked for this
            // document by hand (`cpclib.analyzePeephole`). The default is off
            // because answering costs a full project assemble.
            if self.peephole_wanted(&document.uri) {
                let (peephole_addresses, own_env) =
                    super::peephole::address_source(self, document, &listing);
                let before = diagnostics.len();
                super::peephole::collect_peephole_warnings(
                    &listing,
                    self.config().peephole_goal.into(),
                    peephole_addresses.as_addresses(own_env.as_ref()),
                    &document.uri,
                    &mut diagnostics
                );
                // A request narrowed to a selection reports only what falls
                // inside it - but note the analysis above ran over the whole
                // document regardless, because a match's safety depends on
                // the code around it.
                if let Some(scope) = self.peephole_scope(&document.uri) {
                    let mut kept = diagnostics.split_off(before);
                    kept.retain(|d| super::peephole::overlaps(&d.range, &scope));
                    diagnostics.extend(kept);
                }
            }
        }

        diagnostics
    }
}

/// Walk `env.warnings()` (populated by a real assembling pass, e.g. via
/// `dry_run_env`) into `Diagnostic`s. Distinct from `collect_asm_diagnostics`
/// (which walks a *parse* error tree): warnings only exist after actually
/// assembling, never from parsing alone.
///
/// Forces every diagnostic produced here to `WARNING` severity, regardless
/// of which `AssemblerError` variant a given warning currently wears.
/// `collect_asm_diagnostics` has an explicit `RelocatedWarning`/
/// `AlreadyRenderedWarningWithLocation` arm that already gets this right,
/// but `Env::render_warnings()` flattens anything else (e.g. the
/// pre-existing `OverrideMemory`, or a `checked_byte`/`checked_word`
/// overflow warning once `render_warnings()` has run) into a plain
/// `AssemblingError{msg}` - a shape `collect_asm_diagnostics`'s catch-all
/// arm has no way to distinguish from a real error, so it defaults to
/// `ERROR`. Since everything walked here came from `env.warnings()`, never
/// the error tree, it's unconditionally a warning by construction -
/// overriding after the fact is more robust than trying to enumerate every
/// possible flattened shape in `collect_asm_diagnostics` itself (which is
/// also used for real parse *errors* via a different caller, where `ERROR`
/// is correct for most variants there).
pub(super) fn collect_assembler_warnings(
    env: &cpclib_asm::assembler::Env,
    document: &Document,
    out: &mut Vec<Diagnostic>
) {
    let start = out.len();
    for warning in env.warnings() {
        collect_asm_diagnostics(warning, None, document, out);
    }
    for diag in &mut out[start..] {
        diag.severity = Some(DiagnosticSeverity::WARNING);
    }
}

/// Walk `cpclib_asm::unused_bindings::find_unused_bindings` into
/// `Diagnostic`s - one `WARNING` per declared-but-never-referenced FUNCTION
/// parameter. The detection itself lives in `cpclib-asm` (see that crate's
/// own module doc comment) so any other consumer can reuse it without
/// recoding it; this is purely the "turn a structured finding into an LSP
/// diagnostic" glue, matching `collect_assembler_warnings`'s own role for
/// real assembler warnings.
///
/// Only `FunctionParameter` is reported here, even though
/// `find_unused_bindings` also detects unused MACRO parameters and REPEAT/
/// ITERATE/FOR loop counters. Those other four kinds have a *real-time*
/// equivalent check wired into `cpclib-asm`'s own assembler
/// (`Env::warn_if_counter_unused`, and `visit_macro_definition`'s own
/// `unused_macro_parameter_indices` call) that fires during the real
/// `dry_run_env_cached` assemble a few lines above and already reaches this
/// same `diagnostics` vector via `collect_assembler_warnings`'s
/// `env.warnings()` walk - reporting them again here would duplicate every
/// one of those warnings. FUNCTION parameters are the one kind with no
/// real-time equivalent (`leave_function` never clears `used_symbols`, see
/// the module doc comment on `unused_bindings` for why), so the static
/// method is still the only source of truth for that kind specifically.
fn collect_unused_binding_warnings(
    listing: &cpclib_asm::parser::obtained::LocatedListing,
    document: &Document,
    out: &mut Vec<Diagnostic>
) {
    use cpclib_asm::unused_bindings::UnusedBindingKind;

    for binding in cpclib_asm::unused_bindings::find_unused_bindings(listing.iter()) {
        if binding.kind != UnusedBindingKind::FunctionParameter {
            continue;
        }
        let line = binding.line.saturating_sub(1) as u32;
        let col = binding.column.saturating_sub(1);
        let line_text = document.line(line as usize).unwrap_or_default();
        let start_char = crate::common::document::byte_offset_to_utf16_col(&line_text, col) as u32;
        let end_char =
            crate::common::document::byte_offset_to_utf16_col(&line_text, col + binding.len.max(1))
                as u32;
        let construct = match binding.kind {
            UnusedBindingKind::MacroParameter => "macro",
            UnusedBindingKind::FunctionParameter => "function",
            UnusedBindingKind::RepeatCounter => "REPEAT loop",
            UnusedBindingKind::IterateCounter => "ITERATE loop",
            UnusedBindingKind::ForCounter => "FOR loop"
        };
        out.push(Diagnostic {
            range: Range {
                start: Position {
                    line,
                    character: start_char
                },
                end: Position {
                    line,
                    character: end_char
                }
            },
            severity: Some(DiagnosticSeverity::WARNING),
            source: Some("basm".to_string()),
            message: format!(
                "'{}' is never used in this {construct}'s body",
                binding.name
            ),
            ..Default::default()
        });
    }
}

/// A "fake instruction" warning's range is exactly the offending
/// instruction's own source span (see `AlreadyRenderedWarningWithLocation`
/// in `cpclib-asm`, which captures `line`/`column`/`len` straight from the
/// token's `Z80Span`) - so re-extracting that same text from the live
/// document, re-assembling it in isolation, and disassembling the result
/// shows exactly what real opcode(s) it expands to, without hardcoding any
/// per-fake-instruction knowledge. Fake instructions are always plain
/// register-to-register forms (`ld hl, de`, `sub de, bc`, ...), so the
/// extracted text never depends on a symbol table and is always safe to
/// assemble on its own.
///
/// Deliberately kept to this one short line (unlike the numbered
/// bytes/flags breakdown `hover.rs`'s `format_fake_instruction_hover`
/// shows) - the Problems panel / warning-squiggle tooltip only ever
/// displays `Diagnostic.message` as plain text, never the rich hover
/// response, so this is the only place a VSCode user browsing warnings
/// (rather than hovering the instruction itself) ever sees which real
/// instructions a fake one stands for.
fn enrich_fake_instruction_diagnostics(document: &Document, diagnostics: &mut [Diagnostic]) {
    for diag in diagnostics.iter_mut() {
        if diag.severity != Some(DiagnosticSeverity::WARNING)
            || !diag
                .message
                .contains(cpclib_asm::parser::instructions::FAKE_INSTRUCTION_WARNING)
        {
            continue;
        }
        let Some(line) = document.line(diag.range.start.line as usize)
        else {
            continue;
        };
        // `diag.range`'s `character` fields are UTF-16 code units (LSP
        // convention - see `collect_asm_diagnostics`'s
        // `AlreadyRenderedWarningWithLocation` arm, which produces them)
        // - convert back to byte offsets to slice `line`.
        let start = document.byte_column(diag.range.start);
        let end = document.byte_column(diag.range.end);
        let Some(snippet) = line.get(start..end)
        else {
            continue;
        };
        if let Some(disassembled) = super::disassemble::disassemble_snippet(snippet) {
            diag.message = format!("{} (assembles as: {disassembled})", diag.message);
        }
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
/// error with the closest known source location. `document` is only needed
/// by the `AlreadyRenderedWarningWithLocation` arm (it carries a raw
/// `line`/`column`/`len`, not a `Z80Span`, so converting its byte-based
/// column to UTF-16 needs the actual line text) - threaded through every
/// recursive call the same way `parent_span` already is.
pub(super) fn collect_asm_diagnostics(
    error: &cpclib_asm::AssemblerError,
    parent_span: Option<&cpclib_asm::parser::Z80Span>,
    document: &Document,
    out: &mut Vec<Diagnostic>
) {
    use cpclib_asm::AssemblerError;
    match error {
        AssemblerError::MultipleErrors { errors } => {
            for e in errors {
                collect_asm_diagnostics(e, parent_span, document, out);
            }
        },
        AssemblerError::RelocatedError { span, error: inner } => {
            collect_asm_diagnostics(inner, Some(span), document, out);
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
            collect_asm_diagnostics(inner, Some(span), document, out);
        },
        AssemblerError::ForIssue { span, error: inner } => {
            collect_asm_diagnostics(inner, span.as_ref(), document, out);
        },
        AssemblerError::RepeatIssue {
            span, error: inner, ..
        } => {
            collect_asm_diagnostics(inner, span.as_ref(), document, out);
        },
        AssemblerError::WhileIssue { span, error: inner } => {
            collect_asm_diagnostics(inner, span.as_ref(), document, out);
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
            collect_asm_diagnostics(root, parent_span, document, &mut sub);
            for mut d in sub {
                d.message = format!("{}{}", prefix, d.message);
                out.push(d);
            }
        },
        AssemblerError::CrunchedSectionError { error: inner } => {
            collect_asm_diagnostics(inner, parent_span, document, out);
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
                let col = col_1.saturating_sub(1);
                let len = end_off.saturating_sub(span.offset_from_start());
                // Byte offsets above -> UTF-16 code units, per the LSP spec
                // (same conversion as `asm_diag`, using the same
                // `complete_line()`-derived line text `col`/`len` were
                // computed against).
                let line_text = span.complete_line();
                let start_char =
                    crate::common::document::byte_offset_to_utf16_col(line_text, col) as u32;
                let end_char =
                    crate::common::document::byte_offset_to_utf16_col(line_text, col + len.max(1))
                        as u32;
                out.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line,
                            character: start_char
                        },
                        end: Position {
                            line,
                            character: end_char
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
        // Warnings collected from a real assembling pass (`env.warnings()`,
        // see `collect_assembler_warnings`) always arrive as this variant -
        // it carries its own location (captured eagerly at construction
        // time in cpclib-asm, since holding onto the originating `Z80Span`
        // itself is not always safe - see its doc comment) rather than a
        // `Z80Span`, so build the `Range` directly instead of going through
        // `asm_diag`. Always `WARNING`: nothing pushes this variant into
        // `env.warnings()` except an actual warning.
        AssemblerError::AlreadyRenderedWarningWithLocation {
            msg,
            line,
            column,
            len
        } => {
            let line = line.saturating_sub(1);
            let col = column.saturating_sub(1);
            // `column`/`len` are byte-based (see this variant's own field
            // docs in cpclib-asm) - convert to UTF-16 code units, per the
            // LSP spec, via the document's own line text.
            // `enrich_fake_instruction_diagnostics` converts back on read.
            let line_text = document.line(line as usize).unwrap_or_default();
            let start_char =
                crate::common::document::byte_offset_to_utf16_col(&line_text, col as usize) as u32;
            let end_char = crate::common::document::byte_offset_to_utf16_col(
                &line_text,
                (col + (*len).max(1)) as usize
            ) as u32;
            out.push(Diagnostic {
                range: Range {
                    start: Position {
                        line,
                        character: start_char
                    },
                    end: Position {
                        line,
                        character: end_char
                    }
                },
                severity: Some(DiagnosticSeverity::WARNING),
                source: Some("basm".to_string()),
                message: strip_ansi(msg),
                ..Default::default()
            });
        },
        other => {
            out.push(asm_diag(
                parent_span,
                strip_ansi(&format!("{other}")),
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
        let col = col_1.saturating_sub(1);
        let span_text: &str = s.as_ref();
        // Highlight to end of the current instruction (next `:` separator) or end of line.
        let first_line = span_text.lines().next().unwrap_or(span_text);
        let len = first_line.find(':').unwrap_or(first_line.len()).max(1);
        // `col`/`len` above are byte offsets/lengths (from `relative_line_and_column`,
        // byte-based like the rest of `line-col`) - convert to UTF-16 code
        // units, per the LSP spec, via the same line text they were computed
        // against.
        let line_text = s.complete_line();
        let start_char = crate::common::document::byte_offset_to_utf16_col(line_text, col) as u32;
        let end_char =
            crate::common::document::byte_offset_to_utf16_col(line_text, col + len) as u32;
        Range {
            start: Position {
                line,
                character: start_char
            },
            end: Position {
                line,
                character: end_char
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
    use cpclib_tokens::ListingElement;

    use super::*;
    use crate::basm::AssemblyAnalyzer;

    fn diagnostics_for(text: &str) -> Vec<Diagnostic> {
        let uri = Url::parse("file:///t.asm").unwrap();
        let document = Document::new(uri, text.to_string(), 1);
        AssemblyAnalyzer::new().analyze(&document)
    }

    /// The peephole pass is off by default (a full project assemble is not a
    /// keystroke-time cost), so a test about it has to ask for it.
    fn diagnostics_with_peephole(text: &str) -> Vec<Diagnostic> {
        let mut config = crate::common::config::AsmConfig::default();
        config.warnings.peephole_optimizer = true;
        diagnostics_for_with_config(text, config)
    }

    fn diagnostics_for_with_config(
        text: &str,
        config: crate::common::config::AsmConfig
    ) -> Vec<Diagnostic> {
        let uri = Url::parse("file:///t.asm").unwrap();
        let document = Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        analyzer.set_config(config);
        analyzer.analyze(&document)
    }

    #[test]
    fn valid_file_yields_no_diagnostics() {
        let text = "org 0x4000\n ld a, 1\n ret\n";
        assert!(diagnostics_for(text).is_empty());
    }

    /// Regression test for `asm_diag` treating `relative_line_and_column`'s
    /// byte-based column as a raw UTF-16 `Position.character` (an LSP-spec
    /// violation for any line with non-ASCII content before the span). Uses
    /// a real `Z80Span` from a cleanly-parsed source rather than a synthetic
    /// one, since `Z80Span` isn't hand-constructible outside `cpclib-asm`.
    /// "xcaféxx: " (10 chars, all in the Basic Multilingual Plane) precedes
    /// `nop` - `é` is 2 UTF-8 bytes but 1 UTF-16 unit, so the byte column
    /// (11, 0-based) and the correct UTF-16 column (10) differ by exactly 1.
    #[test]
    fn asm_diag_range_is_utf16_aware_with_a_multibyte_char_before_the_span() {
        use cpclib_asm::MayHaveSpan;
        let text = "org 0x4000\n xcaf\u{e9}xx: nop\n ret\n";
        let listing = AssemblyAnalyzer::parse_source(text, None, Default::default())
            .expect("should parse cleanly");
        let tokens: Vec<_> = super::super::token::flatten_listing(listing.iter()).collect();
        let nop_span = tokens
            .iter()
            .find(|t| t.span().as_ref() as &str == "nop")
            .expect("expected a nop token")
            .span();
        let diag = asm_diag(
            Some(nop_span),
            "test".to_string(),
            DiagnosticSeverity::ERROR
        );
        assert_eq!(diag.range.start.line, 1);
        assert_eq!(diag.range.start.character, 10, "{diag:?}");
        assert_eq!(diag.range.end.character, 13, "{diag:?}");
    }

    /// Regression test for `AlreadyRenderedWarningWithLocation`'s `column`
    /// (also byte-based, per its own field docs in cpclib-asm) being used
    /// directly as `Position.character` - constructs the `AssemblerError`
    /// directly (its fields are public) rather than trying to trigger this
    /// exact variant through a real assembling pass, since it's only ever
    /// produced deep inside `cpclib-asm`'s warning-rendering pipeline. Same
    /// "xcaféxx: " prefix and byte/UTF-16 divergence as the `asm_diag` test
    /// above.
    #[test]
    fn already_rendered_warning_with_location_column_is_utf16_aware() {
        let uri = Url::parse("file:///t.asm").unwrap();
        let text = "org 0x4000\n xcaf\u{e9}xx: nop\n ret\n";
        let document = Document::new(uri, text.to_string(), 1);
        let error = cpclib_asm::AssemblerError::AlreadyRenderedWarningWithLocation {
            msg: "test warning".to_string(),
            line: 2,
            column: 12,
            len: 3
        };
        let mut out = Vec::new();
        collect_asm_diagnostics(&error, None, &document, &mut out);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(out[0].range.start.line, 1);
        assert_eq!(out[0].range.start.character, 10, "{out:?}");
        assert_eq!(out[0].range.end.character, 13, "{out:?}");
    }

    /// Regression test for `enrich_fake_instruction_diagnostics`'s read-back:
    /// once its input `Diagnostic.range` is UTF-16 (as produced by
    /// `AlreadyRenderedWarningWithLocation` above), slicing the source line
    /// with the raw `character` values as byte offsets would grab the wrong
    /// (or a panicking, non-char-boundary) substring on a line with
    /// multi-byte content before the warning - must convert back to bytes
    /// via `Document::byte_column` first.
    #[test]
    fn enrich_fake_instruction_diagnostics_reads_the_correct_byte_slice_via_utf16_range() {
        let uri = Url::parse("file:///t.asm").unwrap();
        let text = "org 0x4000\n xcaf\u{e9}xx: ld hl, de\n ret\n";
        let document = Document::new(uri, text.to_string(), 1);
        // UTF-16 range of "ld hl, de" on line 1: "xcaféxx: " is 10 chars
        // (all BMP), so it spans UTF-16 columns 10..19.
        let mut diags = vec![Diagnostic {
            range: Range {
                start: Position {
                    line: 1,
                    character: 10
                },
                end: Position {
                    line: 1,
                    character: 19
                }
            },
            severity: Some(DiagnosticSeverity::WARNING),
            message: format!(
                "{} something",
                cpclib_asm::parser::instructions::FAKE_INSTRUCTION_WARNING
            ),
            ..Default::default()
        }];
        enrich_fake_instruction_diagnostics(&document, &mut diags);
        assert!(
            diags[0].message.contains("assembles as"),
            "{:?}",
            diags[0].message
        );
        assert!(
            diags[0].message.contains("LD L, E") && diags[0].message.contains("LD H, D"),
            "{:?}",
            diags[0].message
        );
    }

    #[test]
    fn override_memory_warning_is_reported_as_a_warning_not_an_error() {
        // Regression test: `Env::render_warnings()` flattens anything that
        // isn't already `AssemblingError`/`AlreadyRenderedWarningWithLocation`
        // into a plain `AssemblingError{msg}` - a shape
        // `collect_asm_diagnostics`'s catch-all arm can't distinguish from a
        // real error, so before this fix `OverrideMemory` (a real,
        // pre-existing warning kind, never previously exercised through
        // this path since the LSP didn't call `env.warnings()` at all until
        // recently) rendered as `ERROR`.
        let text = "org 0x4000\n db 1,2,3,4,5\n org 0x4002\n db 9,9\n";
        let diags = diagnostics_for(text);
        let override_diag = diags
            .iter()
            .find(|d| d.message.to_lowercase().contains("override"))
            .unwrap_or_else(|| panic!("expected an override-memory diagnostic: {diags:?}"));
        assert_eq!(override_diag.severity, Some(DiagnosticSeverity::WARNING));
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

    /// Regression test for the `analyze` recovery loop's byte-offset
    /// rewrite (`line_starts`, replacing a `lines().skip().collect().join()`
    /// rebuild every retry): three scattered errors, several lines apart,
    /// with a multi-byte comment on an intervening line - the byte slice
    /// must still land exactly on each `start_line`'s own text, and never
    /// panic on a non-UTF-8 boundary (line starts are always right after a
    /// single-byte `\n`, so this is safe regardless of what's on the line).
    #[test]
    fn three_scattered_errors_with_multibyte_content_are_all_reported_correctly() {
        let text = "org 0x4000\n\
                     @#$ garbage1 @#$\n\
                     ld a, 1\n\
                     ; caf\u{e9} commentaire\n\
                     @#$ garbage2 @#$\n\
                     ld b, 2\n\
                     @#$ garbage3 @#$\n\
                     ret\n";
        let diags = diagnostics_for(text);
        assert_eq!(diags.len(), 3, "{diags:?}");
        let lines: Vec<u32> = diags.iter().map(|d| d.range.start.line).collect();
        assert_eq!(lines, vec![1, 4, 6], "{diags:?}");
    }

    #[test]
    fn diagnostic_message_shows_the_real_document_path_not_no_file() {
        // Regression test: the LSP never threaded the document's own URI
        // into the parser's `current_filename`, so every basm error message
        // showed the placeholder "no file"/"no file specified" instead of
        // the real path.
        let text = "org 0x4000\n@#$ garbage @#$\n ld a, 1\n ret\n";
        let diags = diagnostics_for(text);
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert!(diags[0].message.contains("t.asm"), "{:?}", diags[0].message);
        assert!(
            !diags[0].message.to_lowercase().contains("no file"),
            "{:?}",
            diags[0].message
        );
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

    #[test]
    fn fake_ld_instruction_is_reported_as_a_warning() {
        // `ld hl, de` is accepted by basm (assembled using several real
        // opcodes) but isn't a genuine Z80 instruction - the assembler
        // already flags it as a warning; this exercises the LSP actually
        // surfacing it, which required a real assembling pass (`analyze`
        // used to only ever parse, never assemble).
        let text = "org 0x4000\n ld hl, de\n ret\n";
        let diags = diagnostics_for(text);
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert_eq!(diags[0].severity, Some(DiagnosticSeverity::WARNING));
        assert!(
            diags[0].message.contains("fake instruction"),
            "{:?}",
            diags[0].message
        );
        assert_eq!(diags[0].range.start.line, 1);
        // The Problems panel / warning-squiggle tooltip only ever shows
        // this plain message text, never the rich hover response - so the
        // replacement instructions must still be visible here too, even
        // though `hover.rs` shows a much richer numbered breakdown for the
        // same fake instruction when hovering it directly.
        assert!(
            diags[0].message.contains("LD L, E") && diags[0].message.contains("LD H, D"),
            "{:?}",
            diags[0].message
        );
    }

    #[test]
    fn a_valid_instruction_is_not_reported_as_a_fake_one() {
        let text = "org 0x4000\n ld a, e\n ret\n";
        assert!(diagnostics_for(text).is_empty());
    }

    /// The redundant explicit `A,` accumulator prefix (`CP A,r`, and
    /// likewise for `ADD`/`ADC`/`SBC`/`SUB`/`AND`/`OR`/`XOR`) is real, valid
    /// Z80 syntax - unlike a fake instruction, it needs no special
    /// `enrich_*_diagnostics` pass at all: it already surfaces automatically
    /// through `collect_assembler_warnings`'s generic walk over
    /// `env.warnings()`, exactly like any other real assembler warning.
    #[test]
    fn redundant_accumulator_prefix_is_reported_as_a_warning_not_an_error() {
        let text = "org 0x4000\n cp a, c\n ret\n";
        let diags = diagnostics_for(text);
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert_eq!(diags[0].severity, Some(DiagnosticSeverity::WARNING));
        assert!(
            diags[0].message.contains("not mandatory"),
            "{:?}",
            diags[0].message
        );
        assert_eq!(diags[0].range.start.line, 1);
    }

    #[test]
    fn the_bare_implicit_accumulator_form_is_not_reported() {
        let text = "org 0x4000\n cp c\n ret\n";
        assert!(diagnostics_for(text).is_empty());
    }

    /// Regression test for the parser-level fix (see
    /// `cpclib_asm::parser::instructions::wrap_optional_accumulator_warning`):
    /// disabling `redundant_accumulator_prefix` must suppress *only* that
    /// diagnostic, while a `fake_instructions` warning elsewhere in the same
    /// file still fires normally.
    #[test]
    fn disabling_redundant_accumulator_prefix_suppresses_only_that_diagnostic() {
        let text = "org 0x4000\n add de, bc\n cp a, c\n ret\n";
        let mut config = crate::common::config::AsmConfig::default();
        config.warnings.redundant_accumulator_prefix = false;
        let diags = diagnostics_for_with_config(text, config);
        assert!(
            diags.iter().any(|d| d.message.contains("fake instruction")),
            "{diags:?}"
        );
        assert!(
            !diags.iter().any(|d| d.message.contains("not mandatory")),
            "{diags:?}"
        );
    }

    /// Same fix, `fake_instructions` disabled instead - and the underlying
    /// token itself must no longer be `is_warning()` (proves the fix landed
    /// in the parser, not just a post-hoc diagnostic filter).
    #[test]
    fn disabling_fake_instructions_suppresses_only_that_diagnostic_and_unwraps_the_token() {
        let text = "org 0x4000\n add de, bc\n cp a, c\n ret\n";
        let mut config = crate::common::config::AsmConfig::default();
        config.warnings.fake_instructions = false;
        let diags = diagnostics_for_with_config(text, config.clone());
        assert!(
            !diags.iter().any(|d| d.message.contains("fake instruction")),
            "{diags:?}"
        );
        assert!(
            diags.iter().any(|d| d.message.contains("not mandatory")),
            "{diags:?}"
        );

        let uri = Url::parse("file:///t.asm").unwrap();
        let document = Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        analyzer.set_config(config);
        let listing = analyzer.parse_document(&document).ok().unwrap();
        let fake_token = super::super::token::flatten_listing(listing.iter())
            .find(|t| t.mnemonic() == Some(&cpclib_tokens::Mnemonic::Add))
            .expect("expected the add de,bc token");
        assert!(!fake_token.is_warning(), "{fake_token:?}");
    }

    /// Reproduces a real bug a user hit: `peephole_quickfix_action` (and any
    /// other `self.parse_document`-based feature - hover, definitions, ...)
    /// always populates the `dry_run_env_cached` cache using the *cached*
    /// listing. If `analyze()` computed its diagnostics against a
    /// *different* parse of the same text (as it used to, via a fresh
    /// `Self::parse_source` call), `Env::address_trace`'s span-identity
    /// keying (`cpclib-asm`) would silently miss every lookup for that
    /// mismatched parse, and `jp2jr` would vanish from the diagnostics list
    /// even though it's genuinely reachable - while the quickfix, always
    /// self-consistent with its own cached listing, kept working. Exactly
    /// the asymmetry (bulb but no squiggly) the user actually observed.
    #[test]
    fn analyze_finds_an_address_aware_match_even_when_something_else_primed_the_env_cache_first() {
        let text = "start:\n    jp target\ntarget:\n    ret\n";
        let uri = Url::parse("file:///t.asm").unwrap();
        let document = Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        // This test is about the diagnostic, so the pass has to be on.
        let mut config = crate::common::config::AsmConfig::default();
        config.warnings.peephole_optimizer = true;
        analyzer.set_config(config);

        // Simulate a quickfix request (or hover, or anything else built on
        // `self.parse_document`) running before diagnostics ever do.
        let cursor = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 0 }
        };
        let _ = analyzer.peephole_quickfix_action(&document, cursor);

        let diags = analyzer.analyze(&document);
        assert!(
            diags.iter().any(|d| d.message.contains("jr target")),
            "{diags:?}"
        );
    }

    #[test]
    fn disabling_unused_bindings_suppresses_that_diagnostic() {
        let text = "FUNCTION f, a, b\n    IF {a} > 0\n        RETURN 1\n    ENDIF\n    RETURN 0\nENDFUNCTION\nval equ f(1, 2)\n";
        let enabled = diagnostics_for(text);
        assert!(
            enabled.iter().any(|d| d.message.contains("is never used")),
            "{enabled:?}"
        );

        let mut config = crate::common::config::AsmConfig::default();
        config.warnings.unused_bindings = false;
        let disabled = diagnostics_for_with_config(text, config);
        assert!(
            !disabled.iter().any(|d| d.message.contains("is never used")),
            "{disabled:?}"
        );
    }

    /// `unnecessary-ld-to-itself` (a real, built-in `cpclib-asmoptim` rule)
    /// firing through the real `analyze()` pipeline - proves the whole
    /// chain (dry-run assemble, address recording, `flatten_listing`,
    /// `find_matches_with_resolver`) is wired correctly, not just that the
    /// engine works in isolation (already covered by `cpclib-asmoptim`'s own
    /// tests).
    #[test]
    fn a_peephole_optimisation_is_reported_as_a_warning() {
        let text = "org 0x4000\n ld b, b\n ret\n";
        let diags = diagnostics_with_peephole(text);
        let peephole: Vec<_> = diags
            .iter()
            .filter(|d| d.source.as_deref() == Some("basm-peephole"))
            .collect();
        assert_eq!(peephole.len(), 1, "{diags:?}");
        assert!(peephole[0].message.contains("ld b,b"), "{peephole:?}");
        assert_eq!(peephole[0].severity, Some(DiagnosticSeverity::WARNING));
        // "ld b, b" is on line 1 (0-based), starting right after the leading
        // space.
        assert_eq!(peephole[0].range.start.line, 1);
    }

    #[test]
    fn already_optimal_source_reports_no_peephole_warning() {
        let text = "org 0x4000\n xor a\n ld (hl), a\n ret\n";
        let diags = diagnostics_for(text);
        assert!(
            !diags.iter().any(|d| d.source.as_deref() == Some("basm-peephole")),
            "{diags:?}"
        );
    }

    #[test]
    fn peephole_optimizer_is_off_unless_asked_for() {
        let text = "org 0x4000\n ld b, b\n ret\n";
        // The default: nothing, because deciding this needs a whole-project
        // assemble and the user did not ask for one.
        let by_default = diagnostics_for(text);
        assert!(
            !by_default
                .iter()
                .any(|d| d.source.as_deref() == Some("basm-peephole")),
            "the peephole pass must not run unasked: {by_default:?}"
        );

        let enabled = diagnostics_with_peephole(text);
        assert!(
            enabled
                .iter()
                .any(|d| d.source.as_deref() == Some("basm-peephole")),
            "{enabled:?}"
        );
    }

    /// The other way in: leave the warning class off, and ask for this one
    /// document by hand the way `cpclib.analyzePeephole` does.
    #[test]
    fn an_explicit_request_reports_peephole_matches_with_the_class_still_off() {
        let uri = Url::parse("file:///t.asm").unwrap();
        let document = Document::new(uri.clone(), "org 0x4000\n ld b, b\n ret\n".into(), 1);
        let analyzer = AssemblyAnalyzer::new();

        let peephole = |diags: Vec<Diagnostic>| {
            diags
                .into_iter()
                .filter(|d| d.source.as_deref() == Some("basm-peephole"))
                .count()
        };

        assert_eq!(peephole(analyzer.analyze(&document)), 0);

        analyzer.request_peephole(&uri, None);
        assert_eq!(
            peephole(analyzer.analyze(&document)),
            1,
            "asking for this document must be enough, with the class off"
        );

        // Only this document: the request is per-URI, not global.
        let other_uri = Url::parse("file:///other.asm").unwrap();
        let other = Document::new(other_uri, "org 0x4000\n ld b, b\n ret\n".into(), 1);
        assert_eq!(peephole(analyzer.analyze(&other)), 0);

        analyzer.clear_peephole_request(Some(&uri));
        assert_eq!(peephole(analyzer.analyze(&document)), 0);
    }

    /// A request narrowed to a selection reports only what falls inside it.
    /// The analysis itself still covers the whole document - a match is only
    /// safe because of the code around it - so this is a filter on the
    /// output, which is what the test checks: the same document reports more
    /// when the scope is widened, not something different.
    #[test]
    fn a_scoped_request_reports_only_matches_inside_the_selection() {
        let uri = Url::parse("file:///t.asm").unwrap();
        let text = "org 0x4000\n ld b, b\n nop\n ld c, c\n ret\n";
        let document = Document::new(uri.clone(), text.into(), 1);
        let analyzer = AssemblyAnalyzer::new();

        let peephole_lines = |diags: Vec<Diagnostic>| {
            let mut lines: Vec<u32> = diags
                .into_iter()
                .filter(|d| d.source.as_deref() == Some("basm-peephole"))
                .map(|d| d.range.start.line)
                .collect();
            lines.sort();
            lines
        };

        analyzer.request_peephole(&uri, None);
        assert_eq!(peephole_lines(analyzer.analyze(&document)), vec![1, 3]);

        // Just the `ld b, b` line.
        analyzer.request_peephole(&uri, Some(Range {
            start: Position {
                line: 1,
                character: 0
            },
            end: Position {
                line: 1,
                character: 8
            }
        }));
        assert_eq!(peephole_lines(analyzer.analyze(&document)), vec![1]);
    }

    #[test]
    fn immediate_overflow_into_an_8bit_register_is_a_warning() {
        let text = "org 0x4000\n ld b, 300\n ret\n";
        let diags = diagnostics_for(text);
        let overflow: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::WARNING))
            .collect();
        assert_eq!(overflow.len(), 1, "{diags:?}");
        assert!(
            overflow[0].message.contains("does not fit"),
            "{:?}",
            overflow[0].message
        );
        assert_eq!(overflow[0].range.start.line, 1);
        // Enriched with the real, truncated value it actually assembles to
        // (300 & 0xFF == 44 == 0x2c) - derived by re-assembling `ld b, 300`
        // with 300 substituted by its resolved value and disassembling the
        // result, not by hardcoding the truncation arithmetic here.
        assert!(
            overflow[0].message.contains("assembles as")
                && overflow[0].message.contains("LD B")
                && overflow[0].message.contains("0x2c"),
            "{:?}",
            overflow[0].message
        );
        // The offending value itself is shown in decimal, matching how the
        // source actually wrote it ("300", not "0x12c").
        assert!(
            overflow[0].message.contains("value 300"),
            "{:?}",
            overflow[0].message
        );
    }

    #[test]
    fn overflow_value_is_shown_in_the_same_base_the_source_used() {
        let text = "org 0x4000\n ld b, 0x12C\n ret\n";
        let diags = diagnostics_for(text);
        let overflow: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("does not fit"))
            .collect();
        assert_eq!(overflow.len(), 1, "{diags:?}");
        assert!(
            overflow[0].message.contains("value 0x12c"),
            "{:?}",
            overflow[0].message
        );
    }

    #[test]
    fn overflow_value_defaults_to_hex_when_the_source_is_a_symbol() {
        // Per the original request: a bare literal keeps the source's own
        // base, but a symbol/expression has no single "original base" of
        // its own to preserve - default to hex.
        let text = "org 0x4000\nval equ 300\n ld b, val\n ret\n";
        let diags = diagnostics_for(text);
        let overflow: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("does not fit"))
            .collect();
        assert_eq!(overflow.len(), 1, "{diags:?}");
        assert!(
            overflow[0].message.contains("value 0x12c"),
            "{:?}",
            overflow[0].message
        );
    }

    #[test]
    fn a_value_that_fits_an_8bit_register_is_not_reported() {
        let text = "org 0x4000\n ld b, 200\n ret\n";
        assert!(diagnostics_for(text).is_empty());
    }

    #[test]
    fn overflow_through_a_variable_is_also_reported() {
        // Per the original request: overflow detection must work "for
        // indirection with variables", not just literal immediates -
        // resolving `val` against the fully-assembled `Env`'s symbol table
        // covers this the same way it covers a literal.
        let text = "org 0x4000\nval equ 300\n ld b, val\n ret\n";
        let diags = diagnostics_for(text);
        let overflow: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("does not fit"))
            .collect();
        assert_eq!(overflow.len(), 1, "{diags:?}");
    }

    #[test]
    fn a_16bit_immediate_load_is_not_flagged_as_an_8bit_overflow() {
        let text = "org 0x4000\n ld bc, 40000\n ret\n";
        assert!(diagnostics_for(text).is_empty());
    }

    #[test]
    fn defb_item_overflow_is_reported() {
        let text = "org 0x4000\n db 1, 2, 300\n ret\n";
        let diags = diagnostics_for(text);
        assert_eq!(
            diags
                .iter()
                .filter(|d| d.message.contains("does not fit"))
                .count(),
            1,
            "{diags:?}"
        );
    }

    #[test]
    fn unused_macro_parameter_is_a_warning() {
        let text = "MACRO foo, a, b, c\n    ld a, {a}\n    ld b, {b}\nENDM\nfoo(1, 2, 3)\n";
        let diags = diagnostics_for(text);
        let found: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("is never used"))
            .collect();
        assert_eq!(found.len(), 1, "{diags:?}");
        assert_eq!(found[0].severity, Some(DiagnosticSeverity::WARNING));
        assert!(found[0].message.contains('c'), "{}", found[0].message);
    }

    #[test]
    fn unused_function_parameter_is_a_warning() {
        let text = "FUNCTION f, a, b\n    IF {a} > 0\n        RETURN 1\n    ENDIF\n    RETURN 0\nENDFUNCTION\nval equ f(1, 2)\n";
        let diags = diagnostics_for(text);
        let found: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("is never used"))
            .collect();
        assert_eq!(found.len(), 1, "{diags:?}");
        assert!(found[0].message.contains('b'), "{}", found[0].message);
    }

    #[test]
    fn unused_repeat_counter_is_a_warning() {
        let text = "org 0x4000\nREPEAT 3, i\n    nop\nENDR\n";
        let diags = diagnostics_for(text);
        let found: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("is never used"))
            .collect();
        assert_eq!(found.len(), 1, "{diags:?}");
        assert!(found[0].message.contains('i'), "{}", found[0].message);
    }

    #[test]
    fn no_false_positive_when_every_macro_function_and_loop_binding_is_used() {
        let text = "MACRO foo, a\n    ld a, {a}\nENDM\nfoo(1)\nREPEAT 3, i\n    db {i}\nENDR\n";
        let diags = diagnostics_for(text);
        assert!(
            !diags.iter().any(|d| d.message.contains("is never used")),
            "{diags:?}"
        );
    }
}
