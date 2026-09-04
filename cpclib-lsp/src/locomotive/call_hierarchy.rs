//! Call hierarchy for Locomotive BASIC: `GOSUB` (including `ON n GOSUB
//! a,b,c` and `AFTER`/`EVERY n GOSUB line`, which fall out of the same scan
//! for free - see `gosub_targets_in_line`) as the "call" relation, with a
//! BASIC line as the "function". No `RETURN`-side logic: a single `RETURN`
//! can dynamically return to many different `GOSUB` call sites, so it is
//! never itself a call-hierarchy node - this only models "what does this
//! line call" / "what calls this line".

use cpclib_basic::located::{LocatedBasicLine, LocatedBasicProgram, LocatedTokenKind};
use cpclib_basic::tokens::BasicTokenNoPrefix;
use tower_lsp::lsp_types::*;

use super::BasicAnalyzer;
use crate::common::call_hierarchy::CallHierarchyData;
use crate::common::document::Document;

/// `(line, column, length)` of one call-site token span.
type CallSiteSpan = (u32, u32, u32);
/// One `GOSUB` target line number, and every call-site span on the scanned
/// line that jumps to it.
type GosubTargetGroup = (u16, Vec<CallSiteSpan>);

/// Builds the `CallHierarchyItem` for one BASIC line. `line_offset` is added
/// to `bline.source_line` for the absolute document line (0 for standalone,
/// the `LOCOMOTIVE` block's start line when embedded); `embedded_block_start`
/// is what goes into the `data` tag (`None` for standalone, `Some(block_start_line)`
/// when embedded) - kept as an explicit parameter rather than inferred from
/// `line_offset == 0`, since a real embedded block's start line is never `0`
/// anyway but inferring it would still be needlessly implicit.
///
/// `line_at` fetches a single BASIC-text line by its `source_line` index -
/// the only line this function ever needs. It's a callback rather than a
/// plain `&str` so document-backed callers can fetch just the matched line
/// (via `Document::line`) instead of cloning the whole document just to
/// satisfy this signature; embedded-BASIC-in-`.asm` callers, which only ever
/// have a joined-block `&str` and no `Document`, still scan that string.
fn basic_line_to_call_hierarchy_item(
    line_at: &dyn Fn(u32) -> String,
    bline: &LocatedBasicLine,
    line_offset: u32,
    embedded_block_start: Option<u32>,
    document_uri: &Url
) -> CallHierarchyItem {
    let raw_line = line_at(bline.source_line);
    let abs_line = line_offset + bline.source_line;
    let ln_span = bline.tokens[0].span; // always the LineNumber token, per located.rs

    CallHierarchyItem {
        name: format!("Line {}", bline.line_number),
        kind: SymbolKind::FUNCTION,
        tags: None,
        detail: None,
        uri: document_uri.clone(),
        range: Range {
            start: Position {
                line: abs_line,
                character: 0
            },
            end: Position {
                line: abs_line,
                character: raw_line.len() as u32
            }
        },
        selection_range: span_range(line_offset + ln_span.line, ln_span.col, ln_span.len),
        data: Some(
            CallHierarchyData::BasicLine {
                line_number: bline.line_number,
                block_start_line: embedded_block_start
            }
            .to_json()
        )
    }
}

fn span_range(line: u32, col: u32, len: u32) -> Range {
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

/// Every GOSUB target on a single BASIC line, grouped by target line number
/// (a repeated `ON n GOSUB a,a,b` target collapses into one entry's spans) -
/// mirrors `Renumber::renum_substitutions`'s (`cpclib-basic/src/renum.rs`)
/// `after_jump` state machine, filtered to trigger on `Gosub` only.
///
/// `ON n GOSUB a,b,c` and `AFTER n [,timer] GOSUB line` / `EVERY n [,timer]
/// GOSUB line` are handled for free with no extra cases: the trigger is the
/// literal `Gosub` keyword token itself, not whatever precedes it - `ON`,
/// `AFTER`, `EVERY` are their own separate keyword tokens that simply reset
/// `after_gosub` to `false` (falling into the wildcard arm below), and the
/// following `Gosub` token sets it back to `true` regardless.
fn gosub_targets_in_line(bline: &LocatedBasicLine) -> Vec<GosubTargetGroup> {
    let mut groups: Vec<GosubTargetGroup> = Vec::new();
    let mut after_gosub = false;

    for tok in &bline.tokens {
        match &tok.kind {
            LocatedTokenKind::Keyword(BasicTokenNoPrefix::Gosub) => after_gosub = true,
            LocatedTokenKind::Number(n) if after_gosub => {
                if let Ok(target) = n.parse::<u16>() {
                    let span = (tok.span.line, tok.span.col, tok.span.len);
                    match groups.iter_mut().find(|(t, _)| *t == target) {
                        Some(g) => g.1.push(span),
                        None => groups.push((target, vec![span]))
                    }
                }
                // keep after_gosub = true: a comma may follow for ON...GOSUB lists.
            },
            LocatedTokenKind::Other(',') | LocatedTokenKind::Space => {}, // keep state
            _ => after_gosub = false // any other token resets (mirrors renum.rs)
        }
    }
    groups
}

/// Every BASIC line in `prog` whose GOSUB target(s) include `line_number`,
/// as `(caller_line, call-site spans)`.
fn gosub_callers_of(
    prog: &LocatedBasicProgram,
    line_number: u16
) -> Vec<(&LocatedBasicLine, Vec<CallSiteSpan>)> {
    prog.lines
        .iter()
        .filter_map(|bline| {
            let spans: Vec<_> = gosub_targets_in_line(bline)
                .into_iter()
                .filter(|(t, _)| *t == line_number)
                .flat_map(|(_, spans)| spans)
                .collect();
            (!spans.is_empty()).then_some((bline, spans))
        })
        .collect()
}

fn gosub_targets_to_outgoing_calls(
    line_at: &dyn Fn(u32) -> String,
    prog: &LocatedBasicProgram,
    line_number: u16,
    line_offset: u32,
    embedded_block_start: Option<u32>,
    document_uri: &Url
) -> Vec<CallHierarchyOutgoingCall> {
    let Some(bline) = prog.find_line(line_number)
    else {
        return Vec::new();
    };
    gosub_targets_in_line(bline)
        .into_iter()
        .filter_map(|(target, spans)| {
            let target_line = prog.find_line(target)?;
            let to = basic_line_to_call_hierarchy_item(
                line_at,
                target_line,
                line_offset,
                embedded_block_start,
                document_uri
            );
            let from_ranges = spans
                .into_iter()
                .map(|(l, c, len)| span_range(line_offset + l, c, len))
                .collect();
            Some(CallHierarchyOutgoingCall { to, from_ranges })
        })
        .collect()
}

fn gosub_callers_to_incoming_calls(
    line_at: &dyn Fn(u32) -> String,
    prog: &LocatedBasicProgram,
    line_number: u16,
    line_offset: u32,
    embedded_block_start: Option<u32>,
    document_uri: &Url
) -> Vec<CallHierarchyIncomingCall> {
    gosub_callers_of(prog, line_number)
        .into_iter()
        .map(|(caller_line, spans)| {
            let from = basic_line_to_call_hierarchy_item(
                line_at,
                caller_line,
                line_offset,
                embedded_block_start,
                document_uri
            );
            let from_ranges = spans
                .into_iter()
                .map(|(l, c, len)| span_range(line_offset + l, c, len))
                .collect();
            CallHierarchyIncomingCall { from, from_ranges }
        })
        .collect()
}

/// A single document line, without the trailing `\n`/`\r` that `ropey`'s
/// `Document::line` includes but `str::lines()` (what the embedded-BASIC
/// path scans) doesn't - keeps `raw_line.len()` in
/// `basic_line_to_call_hierarchy_item` consistent between both paths.
fn document_line_trimmed(document: &Document, idx: u32) -> String {
    document
        .line(idx as usize)
        .map(|l| l.trim_end_matches(['\n', '\r']).to_string())
        .unwrap_or_default()
}

impl BasicAnalyzer {
    pub fn prepare_call_hierarchy(
        &self,
        document: &Document,
        position: Position
    ) -> Option<CallHierarchyItem> {
        let prog = self.parse_cached(document).ok()?;
        let bline = prog.lines.iter().find(|l| l.source_line == position.line)?;
        let line_at = |idx: u32| document_line_trimmed(document, idx);
        Some(basic_line_to_call_hierarchy_item(
            &line_at,
            bline,
            0,
            None,
            &document.uri
        ))
    }

    pub fn incoming_calls(
        &self,
        document: &Document,
        line_number: u16
    ) -> Vec<CallHierarchyIncomingCall> {
        let Ok(prog) = self.parse_cached(document)
        else {
            return Vec::new();
        };
        let line_at = |idx: u32| document_line_trimmed(document, idx);
        gosub_callers_to_incoming_calls(&line_at, &prog, line_number, 0, None, &document.uri)
    }

    pub fn outgoing_calls(
        &self,
        document: &Document,
        line_number: u16
    ) -> Vec<CallHierarchyOutgoingCall> {
        let Ok(prog) = self.parse_cached(document)
        else {
            return Vec::new();
        };
        let line_at = |idx: u32| document_line_trimmed(document, idx);
        gosub_targets_to_outgoing_calls(&line_at, &prog, line_number, 0, None, &document.uri)
    }
}

/// As [`BasicAnalyzer::prepare_call_hierarchy`], for BASIC embedded in a
/// `LOCOMOTIVE` block inside a `.asm` file - `basic_text` is that block's
/// own joined source, `position` is in absolute document coordinates,
/// `block_start_line` is the block's first BASIC-content line (0-based,
/// document-relative).
pub(crate) fn locomotive_basic_prepare_call_hierarchy(
    basic_text: &str,
    position: Position,
    block_start_line: u32,
    document_uri: &Url
) -> Option<CallHierarchyItem> {
    let prog = LocatedBasicProgram::parse(basic_text).ok()?;
    let cursor_line = position.line.checked_sub(block_start_line)?;
    let bline = prog.lines.iter().find(|l| l.source_line == cursor_line)?;
    let line_at = |idx: u32| {
        basic_text
            .lines()
            .nth(idx as usize)
            .unwrap_or("")
            .to_string()
    };
    Some(basic_line_to_call_hierarchy_item(
        &line_at,
        bline,
        block_start_line,
        Some(block_start_line),
        document_uri
    ))
}

/// As [`BasicAnalyzer::incoming_calls`], for BASIC embedded in a `LOCOMOTIVE` block.
pub(crate) fn locomotive_basic_incoming_calls(
    basic_text: &str,
    line_number: u16,
    block_start_line: u32,
    document_uri: &Url
) -> Vec<CallHierarchyIncomingCall> {
    let Ok(prog) = LocatedBasicProgram::parse(basic_text)
    else {
        return Vec::new();
    };
    let line_at = |idx: u32| {
        basic_text
            .lines()
            .nth(idx as usize)
            .unwrap_or("")
            .to_string()
    };
    gosub_callers_to_incoming_calls(
        &line_at,
        &prog,
        line_number,
        block_start_line,
        Some(block_start_line),
        document_uri
    )
}

/// As [`BasicAnalyzer::outgoing_calls`], for BASIC embedded in a `LOCOMOTIVE` block.
pub(crate) fn locomotive_basic_outgoing_calls(
    basic_text: &str,
    line_number: u16,
    block_start_line: u32,
    document_uri: &Url
) -> Vec<CallHierarchyOutgoingCall> {
    let Ok(prog) = LocatedBasicProgram::parse(basic_text)
    else {
        return Vec::new();
    };
    let line_at = |idx: u32| {
        basic_text
            .lines()
            .nth(idx as usize)
            .unwrap_or("")
            .to_string()
    };
    gosub_targets_to_outgoing_calls(
        &line_at,
        &prog,
        line_number,
        block_start_line,
        Some(block_start_line),
        document_uri
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::document::Document;

    fn doc(text: &str) -> Document {
        let uri = Url::parse("file:///t.bas").unwrap();
        Document::new(uri, text.to_string(), 1)
    }

    #[test]
    fn prepare_call_hierarchy_names_the_line() {
        let text = "10 GOSUB 100\n100 PRINT 1\n110 RETURN\n";
        let d = doc(text);
        let analyzer = BasicAnalyzer::new();
        let item = analyzer
            .prepare_call_hierarchy(
                &d,
                Position {
                    line: 1,
                    character: 0
                }
            )
            .expect("expected an item at line 100");
        assert_eq!(item.name, "Line 100");
    }

    #[test]
    fn incoming_calls_finds_the_gosub_caller() {
        let text = "10 GOSUB 100\n100 PRINT 1\n110 RETURN\n";
        let d = doc(text);
        let analyzer = BasicAnalyzer::new();
        let calls = analyzer.incoming_calls(&d, 100);
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert_eq!(calls[0].from.name, "Line 10");
    }

    #[test]
    fn outgoing_calls_finds_the_gosub_target() {
        let text = "10 GOSUB 100\n100 PRINT 1\n110 RETURN\n";
        let d = doc(text);
        let analyzer = BasicAnalyzer::new();
        let calls = analyzer.outgoing_calls(&d, 10);
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert_eq!(calls[0].to.name, "Line 100");
    }

    #[test]
    fn on_gosub_list_yields_one_outgoing_entry_per_distinct_target() {
        let text = "10 ON X GOSUB 100,200,300\n100 RETURN\n200 RETURN\n300 RETURN\n";
        let d = doc(text);
        let analyzer = BasicAnalyzer::new();
        let calls = analyzer.outgoing_calls(&d, 10);
        assert_eq!(calls.len(), 3, "{calls:?}");
    }

    #[test]
    fn a_repeated_target_in_an_on_gosub_list_collapses_to_one_entry() {
        let text = "10 ON X GOSUB 100,100,200\n100 RETURN\n200 RETURN\n";
        let d = doc(text);
        let analyzer = BasicAnalyzer::new();
        let calls = analyzer.outgoing_calls(&d, 10);
        let to_100 = calls.iter().find(|c| c.to.name == "Line 100").unwrap();
        assert_eq!(to_100.from_ranges.len(), 2);
    }

    #[test]
    fn line_with_no_callers_yields_no_incoming_calls() {
        let text = "10 PRINT 1\n";
        let d = doc(text);
        let analyzer = BasicAnalyzer::new();
        assert!(analyzer.incoming_calls(&d, 10).is_empty());
    }

    #[test]
    fn embedded_block_twin_uses_absolute_document_coordinates() {
        // Simulates a LOCOMOTIVE block starting at document line 5.
        let basic_text = "10 GOSUB 100\n100 PRINT 1\n110 RETURN\n";
        let uri = Url::parse("file:///main.asm").unwrap();

        let item = locomotive_basic_prepare_call_hierarchy(
            basic_text,
            Position {
                line: 6,
                character: 0
            },
            5,
            &uri
        )
        .expect("expected an item at line 100 (doc line 6)");
        assert_eq!(item.name, "Line 100");
        assert_eq!(item.range.start.line, 6);
        match CallHierarchyData::from_json(item.data.as_ref().unwrap()) {
            Some(CallHierarchyData::BasicLine {
                block_start_line, ..
            }) => {
                assert_eq!(block_start_line, Some(5));
            },
            other => panic!("unexpected data: {other:?}")
        }

        let calls = locomotive_basic_incoming_calls(basic_text, 100, 5, &uri);
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert_eq!(calls[0].from.range.start.line, 5); // line 10 -> doc line 5
    }

    /// Regression test for the switch from `document.text()` + `str::lines()`
    /// to per-line `Document::line` fetches: a supplementary-plane character
    /// (😀) on an earlier line must not desync line-index lookups (line
    /// splitting is always on `\n`, unaffected by multi-byte content), and
    /// the target line's own byte length (used for `range.end.character`,
    /// this file's pre-existing byte-based convention, unrelated to the
    /// UTF-16 fix elsewhere in this audit) must match what `str::lines()`
    /// would have produced - i.e. `Document::line`'s trailing `\n`/`\r` must
    /// be trimmed correctly.
    #[test]
    fn call_hierarchy_resolves_correctly_with_multibyte_content_on_an_earlier_line() {
        let text = "10 GOSUB 100\n15 REM 😀 comment\n100 PRINT 1\n110 RETURN\n";
        let d = doc(text);
        let analyzer = BasicAnalyzer::new();

        let item = analyzer
            .prepare_call_hierarchy(
                &d,
                Position {
                    line: 2,
                    character: 0
                }
            )
            .expect("expected an item at line 100 (doc line 2)");
        assert_eq!(item.name, "Line 100");
        assert_eq!(item.range.start.line, 2);
        assert_eq!(item.range.end.character, "100 PRINT 1".len() as u32);

        let outgoing = analyzer.outgoing_calls(&d, 10);
        assert_eq!(outgoing.len(), 1, "{outgoing:?}");
        assert_eq!(outgoing[0].to.name, "Line 100");

        let incoming = analyzer.incoming_calls(&d, 100);
        assert_eq!(incoming.len(), 1, "{incoming:?}");
        assert_eq!(incoming[0].from.name, "Line 10");
    }
}
