//! Goto-definition and references for Locomotive BASIC: line-number jump
//! targets (GOTO/GOSUB/...), FOR/NEXT pairing, first variable assignment.
//!
//! `locomotive_basic_goto_definition` is `pub(crate)` because the basm module
//! reuses it for BASIC blocks embedded in assembly (`LOCOMOTIVE` directive).

use cpclib_basic::located::{LocatedBasicLine, LocatedBasicProgram, LocatedTokenKind};
use cpclib_basic::tokens::BasicTokenNoPrefix;
use tower_lsp::lsp_types::*;

use super::BasicAnalyzer;
use super::token::*;
use crate::common::document::Document;

impl BasicAnalyzer {
    pub fn goto_definition(&self, document: &Document, position: Position) -> Option<Location> {
        let text = document.text();
        let prog = LocatedBasicProgram::parse(&text).ok()?;

        // Find which token the cursor is on.
        let cursor_line = position.line;
        let cursor_col = position.character;

        let bline = prog.lines.iter().find(|l| l.source_line == cursor_line)?;

        // Find the token at the cursor.
        let tok_idx = bline
            .tokens
            .iter()
            .position(|t| t.span.col <= cursor_col && cursor_col < t.span.col + t.span.len)?;

        let tok = &bline.tokens[tok_idx];

        match &tok.kind {
            // ── NEXT keyword: find matching FOR ─────────────────────────────
            LocatedTokenKind::Keyword(BasicTokenNoPrefix::Next) => {
                let next_var = skip_spaces_then_var_name(&bline.tokens, tok_idx + 1);
                let (target, for_col) =
                    find_for_matching_next(&prog, bline.source_line, next_var.as_deref())?;
                Some(Location {
                    uri: document.uri.clone(),
                    range: single_pos(target.source_line, for_col)
                })
            },

            // ── GOTO/GOSUB keyword: jump to the named line ───────────────────
            LocatedTokenKind::Keyword(BasicTokenNoPrefix::Goto)
            | LocatedTokenKind::Keyword(BasicTokenNoPrefix::Gosub) => {
                let num_text = skip_spaces_then_number(&bline.tokens, tok_idx + 1)?;
                let target_num: u16 = num_text.parse().ok()?;
                let target = prog.find_line(target_num)?;
                Some(Location {
                    uri: document.uri.clone(),
                    range: single_pos(target.source_line, 0)
                })
            },

            // ── Variable: goto first assignment ─────────────────────────────
            LocatedTokenKind::Variable(name) => {
                let key = name.to_uppercase();
                // Find first assignment of this variable.
                let target_line = first_assignment_line(&prog, &key)?;
                Some(Location {
                    uri: document.uri.clone(),
                    range: single_pos(
                        target_line.source_line,
                        target_line
                            .tokens
                            .iter()
                            .find(|t| {
                                if let LocatedTokenKind::Variable(n) = &t.kind {
                                    n.to_uppercase() == key
                                }
                                else {
                                    false
                                }
                            })
                            .map(|t| t.span.col)
                            .unwrap_or(0)
                    )
                })
            },

            // ── Number after GOTO/GOSUB/RESTORE/RESUME/RUN ──────────────────
            LocatedTokenKind::Number(text) => {
                // Check that a line-jump keyword precedes this number.
                if !is_jump_target(&bline.tokens, tok_idx) {
                    return None;
                }
                let target_num: u16 = text.parse().ok()?;
                let target = prog.find_line(target_num)?;
                Some(Location {
                    uri: document.uri.clone(),
                    range: single_pos(target.source_line, 0)
                })
            },

            _ => None
        }
    }

    pub fn find_references(&self, document: &Document, position: Position) -> Vec<Location> {
        let text = document.text();
        let prog = match LocatedBasicProgram::parse(&text) {
            Ok(p) => p,
            Err(_) => return vec![]
        };

        let Some(var_key) = variable_at_position(&prog, position)
        else {
            return vec![];
        };

        variable_occurrence_spans(&prog, &var_key)
            .into_iter()
            .map(|(line, col, len)| {
                Location {
                    uri: document.uri.clone(),
                    range: span_range(line, col, len)
                }
            })
            .collect()
    }

    /// The range of the variable occurrence under the cursor, or `None` if
    /// the cursor isn't on a variable — used by `textDocument/prepareRename`
    /// to decide whether to offer renaming at all.
    pub fn prepare_rename(&self, document: &Document, position: Position) -> Option<Range> {
        let text = document.text();
        let prog = LocatedBasicProgram::parse(&text).ok()?;
        let tok = token_at_position(&prog, position)?;
        match &tok.kind {
            LocatedTokenKind::Variable(_) => {
                Some(span_range(tok.span.line, tok.span.col, tok.span.len))
            },
            _ => None
        }
    }

    /// Rename every occurrence of the variable under the cursor to
    /// `new_name`, within this single document — BASIC variables have no
    /// cross-file consequences.
    pub fn rename(
        &self,
        document: &Document,
        position: Position,
        new_name: &str
    ) -> Option<WorkspaceEdit> {
        let text = document.text();
        let prog = LocatedBasicProgram::parse(&text).ok()?;
        let var_key = variable_at_position(&prog, position)?;
        variable_occurrences_to_workspace_edit(&prog, &var_key, 0, document.uri.clone(), new_name)
    }
}

// ─── Shared variable-occurrence helpers (find_references, rename) ───────────

/// The token at `position` (document/block-relative, whichever `prog` was
/// parsed against), if any.
fn token_at_position(
    prog: &LocatedBasicProgram,
    position: Position
) -> Option<&cpclib_basic::located::LocatedBasicToken> {
    let bline = prog.lines.iter().find(|l| l.source_line == position.line)?;
    bline
        .tokens
        .iter()
        .find(|t| t.span.col <= position.character && position.character < t.span.col + t.span.len)
}

/// The uppercased variable name under the cursor, or `None` if the cursor
/// isn't on a `Variable` token.
fn variable_at_position(prog: &LocatedBasicProgram, position: Position) -> Option<String> {
    match &token_at_position(prog, position)?.kind {
        LocatedTokenKind::Variable(n) => Some(n.to_uppercase()),
        _ => None
    }
}

/// Every occurrence of `var_key` (already uppercased) as a `Variable` token
/// in `prog`, as raw `(line, col, len)` spans — shared by `find_references`
/// and `rename`, which just wrap these in a `Location`/`TextEdit`, possibly
/// at a line offset (for BASIC embedded in a `LOCOMOTIVE` block).
fn variable_occurrence_spans(prog: &LocatedBasicProgram, var_key: &str) -> Vec<(u32, u32, u32)> {
    let mut spans = Vec::new();
    for bline in &prog.lines {
        for t in &bline.tokens {
            if let LocatedTokenKind::Variable(n) = &t.kind {
                if n.to_uppercase() == var_key {
                    spans.push((t.span.line, t.span.col, t.span.len));
                }
            }
        }
    }
    spans
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

/// Build a single-file `WorkspaceEdit` renaming every occurrence of
/// `var_key` to `new_name`, with each occurrence's line offset by
/// `line_offset` (0 for a standalone `.bas` document, the `LOCOMOTIVE`
/// block's starting line when embedded in a `.asm` document).
fn variable_occurrences_to_workspace_edit(
    prog: &LocatedBasicProgram,
    var_key: &str,
    line_offset: u32,
    uri: Url,
    new_name: &str
) -> Option<WorkspaceEdit> {
    let edits: Vec<TextEdit> = variable_occurrence_spans(prog, var_key)
        .into_iter()
        .map(|(line, col, len)| {
            TextEdit {
                range: span_range(line_offset + line, col, len),
                new_text: new_name.to_string()
            }
        })
        .collect();
    if edits.is_empty() {
        return None;
    }
    Some(WorkspaceEdit {
        changes: Some(std::collections::HashMap::from([(uri, edits)])),
        ..Default::default()
    })
}

// ─── Goto-definition helpers ──────────────────────────────────────────────────

/// Returns a `Range` pointing at a single character position.
fn single_pos(line: u32, col: u32) -> Range {
    Range {
        start: Position {
            line,
            character: col
        },
        end: Position {
            line,
            character: col
        }
    }
}

/// Returns true if the token at `tok_idx` is preceded (ignoring spaces) by a
/// line-jump keyword (GOTO, GOSUB, RESTORE, RESUME, RUN, MERGE, CHAIN, etc.).
fn is_jump_target(tokens: &[cpclib_basic::located::LocatedBasicToken], tok_idx: usize) -> bool {
    // Walk backwards, skipping spaces, commas (ON n GOTO x,y lists).
    let mut i = tok_idx;
    while i > 0 {
        i -= 1;
        let kind = &tokens[i].kind;
        match kind {
            LocatedTokenKind::Space => continue,
            LocatedTokenKind::Number(_) => continue, // other numbers in a list
            LocatedTokenKind::Other(',') => continue, // comma in ON…GOTO list
            LocatedTokenKind::Keyword(k) => {
                use BasicTokenNoPrefix::*;
                return matches!(
                    k,
                    Goto | Gosub
                        | Restore
                        | Resume
                        | Run
                        | Merge
                        | Chain
                        | Delete
                        | List
                        | Renum
                        | Auto
                        | OnErrorGoto
                );
            },
            _ => return false
        }
    }
    false
}

/// Find the FOR that matches a NEXT on `next_source_line`.
/// Returns `(line, for_col)` — the source line containing FOR and the column of the FOR token.
/// If `next_var` is Some, only match FORs with that variable (case-insensitive).
fn find_for_matching_next<'a>(
    prog: &'a LocatedBasicProgram,
    next_source_line: u32,
    next_var: Option<&str>
) -> Option<(&'a LocatedBasicLine, u32)> {
    // Stack of (var_name_upper, line, for_col).
    let mut stack: Vec<(String, &LocatedBasicLine, u32)> = Vec::new();

    for bline in prog
        .lines
        .iter()
        .filter(|l| l.source_line < next_source_line)
    {
        for tok in &bline.tokens {
            match &tok.kind {
                LocatedTokenKind::Keyword(BasicTokenNoPrefix::For) => {
                    let tok_pos = bline
                        .tokens
                        .iter()
                        .position(|t| std::ptr::eq(t, tok))
                        .unwrap_or(0);
                    let var_name = skip_spaces_then_var_name(&bline.tokens, tok_pos + 1)
                        .unwrap_or_default()
                        .to_uppercase();
                    stack.push((var_name, bline, tok.span.col));
                },
                LocatedTokenKind::Keyword(BasicTokenNoPrefix::Next) => {
                    let tok_pos = bline
                        .tokens
                        .iter()
                        .position(|t| std::ptr::eq(t, tok))
                        .unwrap_or(0);
                    let nv = skip_spaces_then_var_name(&bline.tokens, tok_pos + 1)
                        .map(|s| s.to_uppercase())
                        .unwrap_or_default();
                    if let Some(pos) = stack
                        .iter()
                        .rposition(|(v, ..)| nv.is_empty() || v.is_empty() || *v == nv)
                    {
                        stack.remove(pos);
                    }
                },
                _ => {}
            }
        }
    }

    if let Some(nv) = next_var {
        let nv_upper = nv.to_uppercase();
        for (var, line, col) in stack.iter().rev() {
            if var.is_empty() || var == &nv_upper {
                return Some((line, *col));
            }
        }
        None
    }
    else {
        stack.last().map(|(_, l, c)| (*l, *c))
    }
}

/// Find the BASIC line on which `var_key` (uppercase) is first assigned.
fn first_assignment_line<'a>(
    prog: &'a LocatedBasicProgram,
    var_key: &str
) -> Option<&'a LocatedBasicLine> {
    for bline in &prog.lines {
        let toks = &bline.tokens;
        let n = toks.len();
        let mut i = 0;

        while i < n {
            match &toks[i].kind {
                LocatedTokenKind::Keyword(BasicTokenNoPrefix::Let)
                | LocatedTokenKind::Keyword(BasicTokenNoPrefix::For) => {
                    if let Some(vt) = skip_spaces_then_var(toks, i + 1) {
                        if let LocatedTokenKind::Variable(name) = &vt.kind {
                            if name.to_uppercase() == var_key {
                                return Some(bline);
                            }
                        }
                    }
                    i += 1;
                },
                LocatedTokenKind::Keyword(BasicTokenNoPrefix::Input)
                | LocatedTokenKind::Keyword(BasicTokenNoPrefix::Read) => {
                    // Scan for all variables after the keyword.
                    let mut j = i + 1;
                    while j < n {
                        match &toks[j].kind {
                            LocatedTokenKind::Space
                            | LocatedTokenKind::Other(',')
                            | LocatedTokenKind::Other(';')
                            | LocatedTokenKind::StringLit(_) => {},
                            LocatedTokenKind::Variable(name) => {
                                if name.to_uppercase() == var_key {
                                    return Some(bline);
                                }
                            },
                            LocatedTokenKind::Separator => break,
                            _ => {}
                        }
                        j += 1;
                    }
                    i += 1;
                },
                LocatedTokenKind::Variable(name) => {
                    if name.to_uppercase() == var_key && is_followed_by_eq(toks, i + 1) {
                        return Some(bline);
                    }
                    i += 1;
                },
                _ => {
                    i += 1;
                }
            }
        }
    }
    None
}

/// - `basic_text`       — BASIC source lines joined with `\n` (0-indexed within the block)
/// - `position`         — cursor position in **document** coordinates
/// - `block_start_line` — document line index of the first BASIC content line
/// - `document_uri`     — URI to use in the returned `Location`
pub(crate) fn locomotive_basic_goto_definition(
    basic_text: &str,
    position: Position,
    block_start_line: u32,
    document_uri: &Url
) -> Option<Location> {
    let prog = LocatedBasicProgram::parse(basic_text).ok()?;

    let cursor_line = position.line.checked_sub(block_start_line)?;
    let cursor_col = position.character;

    let bline = prog.lines.iter().find(|l| l.source_line == cursor_line)?;

    let tok_idx = bline
        .tokens
        .iter()
        .position(|t| t.span.col <= cursor_col && cursor_col < t.span.col + t.span.len)?;
    let tok = &bline.tokens[tok_idx];

    let to_loc = |src_line: u32, col: u32| {
        Location {
            uri: document_uri.clone(),
            range: single_pos(block_start_line + src_line, col)
        }
    };

    match &tok.kind {
        LocatedTokenKind::Keyword(BasicTokenNoPrefix::Next) => {
            let next_var = skip_spaces_then_var_name(&bline.tokens, tok_idx + 1);
            let (target, for_col) =
                find_for_matching_next(&prog, cursor_line, next_var.as_deref())?;
            Some(to_loc(target.source_line, for_col))
        },
        LocatedTokenKind::Keyword(BasicTokenNoPrefix::Goto)
        | LocatedTokenKind::Keyword(BasicTokenNoPrefix::Gosub) => {
            let num_text = skip_spaces_then_number(&bline.tokens, tok_idx + 1)?;
            let target_num: u16 = num_text.parse().ok()?;
            let target = prog.find_line(target_num)?;
            Some(to_loc(target.source_line, 0))
        },
        LocatedTokenKind::Number(text) => {
            if !is_jump_target(&bline.tokens, tok_idx) {
                return None;
            }
            let target_num: u16 = text.parse().ok()?;
            let target = prog.find_line(target_num)?;
            Some(to_loc(target.source_line, 0))
        },
        LocatedTokenKind::Variable(name) => {
            let key = name.to_uppercase();
            let target_line = first_assignment_line(&prog, &key)?;
            let col = target_line
                .tokens
                .iter()
                .find(
                    |t| matches!(&t.kind, LocatedTokenKind::Variable(n) if n.to_uppercase() == key)
                )
                .map(|t| t.span.col)
                .unwrap_or(0);
            Some(to_loc(target_line.source_line, col))
        },
        _ => None
    }
}

/// As [`BasicAnalyzer::prepare_rename`], for BASIC embedded in a `LOCOMOTIVE`
/// block inside a `.asm` file. See [`locomotive_basic_goto_definition`] for
/// what the parameters mean.
pub(crate) fn locomotive_basic_prepare_rename(
    basic_text: &str,
    position: Position,
    block_start_line: u32
) -> Option<Range> {
    let prog = LocatedBasicProgram::parse(basic_text).ok()?;
    let cursor_line = position.line.checked_sub(block_start_line)?;
    let local_position = Position {
        line: cursor_line,
        character: position.character
    };
    let tok = token_at_position(&prog, local_position)?;
    match &tok.kind {
        LocatedTokenKind::Variable(_) => {
            Some(span_range(
                block_start_line + tok.span.line,
                tok.span.col,
                tok.span.len
            ))
        },
        _ => None
    }
}

/// As [`BasicAnalyzer::rename`], for BASIC embedded in a `LOCOMOTIVE` block
/// inside a `.asm` file. See [`locomotive_basic_goto_definition`] for what
/// the parameters mean.
pub(crate) fn locomotive_basic_rename(
    basic_text: &str,
    position: Position,
    block_start_line: u32,
    document_uri: &Url,
    new_name: &str
) -> Option<WorkspaceEdit> {
    let prog = LocatedBasicProgram::parse(basic_text).ok()?;
    let cursor_line = position.line.checked_sub(block_start_line)?;
    let local_position = Position {
        line: cursor_line,
        character: position.character
    };
    let var_key = variable_at_position(&prog, local_position)?;
    variable_occurrences_to_workspace_edit(
        &prog,
        &var_key,
        block_start_line,
        document_uri.clone(),
        new_name
    )
}

#[cfg(test)]
mod rename_tests {
    use super::*;
    use crate::common::document::Document;

    fn doc(text: &str) -> Document {
        let uri = Url::parse("file:///t.bas").unwrap();
        Document::new(uri, text.to_string(), 1)
    }

    #[test]
    fn rename_updates_every_occurrence_of_a_variable() {
        // "10 LET X=5" / "20 PRINT X" / "30 X=X+1" — X appears 4 times:
        // the LET target, the PRINT reference, and both sides of X=X+1.
        let text = "10 LET X=5\n20 PRINT X\n30 X=X+1\n";
        let d = doc(text);
        let analyzer = BasicAnalyzer::new();
        // Cursor on the "X" in "LET X=5" (column 7).
        let edit = analyzer
            .rename(
                &d,
                Position {
                    line: 0,
                    character: 7
                },
                "Y"
            )
            .expect("expected a workspace edit");
        let edits = edit.changes.unwrap().remove(&d.uri).unwrap();
        assert_eq!(edits.len(), 4, "{edits:?}");
        for e in &edits {
            assert_eq!(e.new_text, "Y");
        }
    }

    #[test]
    fn prepare_rename_rejects_a_non_variable_word() {
        let text = "10 PRINT \"HELLO\"\n";
        let d = doc(text);
        let analyzer = BasicAnalyzer::new();
        // Cursor on the PRINT keyword.
        assert!(
            analyzer
                .prepare_rename(
                    &d,
                    Position {
                        line: 0,
                        character: 4
                    }
                )
                .is_none()
        );
    }

    #[test]
    fn prepare_rename_accepts_a_variable() {
        let text = "10 LET X=5\n";
        let d = doc(text);
        let analyzer = BasicAnalyzer::new();
        let range = analyzer
            .prepare_rename(
                &d,
                Position {
                    line: 0,
                    character: 7
                }
            )
            .expect("expected a range");
        assert_eq!(range.start.character, 7);
        assert_eq!(range.end.character, 8);
    }

    #[test]
    fn renaming_inside_a_locomotive_block_produces_absolute_document_coordinates() {
        // Mirrors how `basm/definition.rs`'s `AssemblyAnalyzer::rename`
        // delegates to this for `LOCOMOTIVE` block content embedded in a
        // `.asm` file — the block starts at document line 3.
        let basic_text = "10 LET X=5\n20 PRINT X\n";
        let uri = Url::parse("file:///embedded.asm").unwrap();
        let edit = locomotive_basic_rename(
            basic_text,
            Position {
                line: 3,
                character: 7
            },
            3,
            &uri,
            "Y"
        )
        .expect("expected a workspace edit");
        let edits = edit.changes.unwrap().remove(&uri).unwrap();
        assert_eq!(edits.len(), 2, "{edits:?}");
        assert_eq!(edits[0].range.start.line, 3);
        assert_eq!(edits[1].range.start.line, 4);
    }
}
