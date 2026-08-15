//! Turning VS Code's red dot into a `breakpoint` directive.
//!
//! The editor's own breakpoints are just marks on a line; nothing in a `.asm`
//! file knows about them. Rather than teach an emulator to accept a list of
//! addresses at launch - each emulator does it differently, and getting the
//! address means mapping a line back to a PC - this writes the breakpoint into
//! the source, where basm already understands it: `breakpoint` is a directive,
//! and it survives into the snapshot for the emulator to honour.
//!
//! So toggling the red dot edits the line, and untoggling it puts the line
//! back. That is the whole mechanism.
//!
//! The insertion point is the first *instruction* on the line, not column
//! zero: a line may start with a label, and `loop breakpoint : ld a,0` keeps
//! `loop` pointing where it did (the directive emits no bytes) while
//! `breakpoint : loop ld a,0` would not even parse.

use cpclib_asm::parser::obtained::{LocatedListing, MayHaveSpan};
use crate::common::document::Document;
use cpclib_tokens::ListingElement;
use tower_lsp::lsp_types::{Position, Range, TextEdit};

/// What gets written in, separator included.
const DIRECTIVE: &str = "breakpoint : ";

/// 0-based (line, column) a token starts at.
fn line_and_column(token: &impl MayHaveSpan) -> (u32, u32) {
    let (line, column) = token.span().relative_line_and_column();
    (
        line.saturating_sub(1) as u32,
        column.saturating_sub(1) as u32
    )
}

/// The edit that toggles a `breakpoint` on `line` of `document`, or `None`
/// when the line cannot carry one or already says what is being asked.
///
/// The single entry point the server calls: it parses, so callers do not need
/// access to the analyzer's internals.
pub(crate) fn breakpoint_edit(
    analyzer: &super::AssemblyAnalyzer,
    document: &Document,
    line: u32,
    enable: bool
) -> Option<TextEdit> {
    let listing = analyzer.parse_document(document).ok()?;
    let line_text = document.line(line as usize)?;
    let line_text = line_text.trim_end_matches(['\r', '\n']);
    if enable {
        add_breakpoint_edit(&listing, line_text, line)
    }
    else {
        remove_breakpoint_edit(&listing, line_text, line)
    }
}

/// The edit that adds a `breakpoint` to `line`, or `None` when there is
/// nothing to put one on (a blank, comment-only or label-only line) or one is
/// already there.
pub(super) fn add_breakpoint_edit(
    listing: &LocatedListing,
    line_text: &str,
    line: u32
) -> Option<TextEdit> {
    let on_line: Vec<_> = super::token::flatten_listing(listing.iter())
        .filter(|t| line_and_column(*t).0 == line)
        .collect();

    if on_line.iter().any(|t| t.is_breakpoint()) {
        return None;
    }
    // Something has to execute for a breakpoint to stop at it.
    let first = on_line.iter().find(|t| t.mnemonic().is_some())?;

    let column = line_and_column(*first).1;
    let character = crate::common::document::byte_offset_to_utf16_col(line_text, column as usize)
        as u32;
    let position = Position { line, character };
    Some(TextEdit {
        range: Range {
            start: position,
            end: position
        },
        new_text: DIRECTIVE.to_string()
    })
}

/// The edit that takes the `breakpoint` back off `line`, or `None` if there is
/// none there.
///
/// Removes what it finds in the source rather than assuming the exact spelling
/// this module writes: the user may have reformatted the line, or typed the
/// directive themselves.
pub(super) fn remove_breakpoint_edit(
    listing: &LocatedListing,
    line_text: &str,
    line: u32
) -> Option<TextEdit> {
    let token = super::token::flatten_listing(listing.iter())
        .find(|t| line_and_column(*t).0 == line && t.is_breakpoint())?;

    let start_byte = line_and_column(token).1 as usize;
    let rest = line_text.get(start_byte..)?;

    // The directive itself, then the statement separator and the spacing
    // around it, so the line closes back up instead of leaving `  : ld a, 0`.
    let mut end_byte = start_byte + rest.len() - rest.trim_start().len();
    let after_word = rest.trim_start();
    let word_len = after_word
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(after_word.len());
    end_byte += word_len;

    let tail = line_text.get(end_byte..)?;
    let trimmed = tail.trim_start();
    if let Some(without_separator) = trimmed.strip_prefix(':') {
        end_byte += tail.len() - without_separator.len();
        let tail = line_text.get(end_byte..)?;
        end_byte += tail.len() - tail.trim_start().len();
    }

    Some(TextEdit {
        range: Range {
            start: Position {
                line,
                character: crate::common::document::byte_offset_to_utf16_col(
                    line_text, start_byte
                ) as u32
            },
            end: Position {
                line,
                character: crate::common::document::byte_offset_to_utf16_col(line_text, end_byte)
                    as u32
            }
        },
        new_text: String::new()
    })
}

#[cfg(test)]
mod tests {
    use tower_lsp::lsp_types::Url;

    use super::*;
    use crate::basm::AssemblyAnalyzer;
    use crate::common::document::Document;

    /// Apply whichever edit the toggle produces, and hand back the new line -
    /// what the user would actually see in the editor.
    fn toggled(text: &str, line: u32, enable: bool) -> Option<String> {
        let d = Document::new(Url::parse("file:///t.asm").unwrap(), text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        let listing = analyzer.parse_document(&d).expect("must parse");
        let line_text = d.line(line as usize).unwrap_or_default();
        let line_text = line_text.trim_end_matches(['\r', '\n']);
        let edit = if enable {
            add_breakpoint_edit(&listing, line_text, line)?
        }
        else {
            remove_breakpoint_edit(&listing, line_text, line)?
        };
        let start = edit.range.start.character as usize;
        let end = edit.range.end.character as usize;
        let mut out: String = line_text.chars().take(start).collect();
        out.push_str(&edit.new_text);
        out.extend(line_text.chars().skip(end));
        Some(out)
    }

    #[test]
    fn a_breakpoint_goes_in_front_of_the_instruction() {
        assert_eq!(
            toggled("\tld a, 0\n", 0, true).as_deref(),
            Some("\tbreakpoint : ld a, 0")
        );
    }

    /// A label has to keep pointing at the code, so the directive goes after
    /// it - not at column zero.
    #[test]
    fn a_label_on_the_line_keeps_its_place() {
        assert_eq!(
            toggled("loop\tld a, 0\n", 0, true).as_deref(),
            Some("loop\tbreakpoint : ld a, 0")
        );
    }

    /// Nothing executes on these, so there is nothing to break on.
    #[test]
    fn lines_with_no_instruction_are_refused() {
        assert!(toggled("\n", 0, true).is_none());
        assert!(toggled("; just a comment\n", 0, true).is_none());
        assert!(toggled("a_label\n\tnop\n", 0, true).is_none());
    }

    /// Toggling twice is a no-op, not two directives.
    #[test]
    fn a_line_that_already_has_one_is_left_alone() {
        assert!(toggled("\tbreakpoint : ld a, 0\n", 0, true).is_none());
    }

    /// Removing puts the line back exactly as it was.
    #[test]
    fn removing_closes_the_line_back_up() {
        assert_eq!(
            toggled("\tbreakpoint : ld a, 0\n", 0, false).as_deref(),
            Some("\tld a, 0")
        );
        assert_eq!(
            toggled("loop\tbreakpoint : ld a, 0\n", 0, false).as_deref(),
            Some("loop\tld a, 0")
        );
    }

    /// Add then remove is the identity - the property that matters when a
    /// user clicks the same red dot twice.
    #[test]
    fn adding_then_removing_restores_the_line() {
        for original in ["\tld a, 0", "loop\tld a, 0", "\tld a, 0 ; keep me"] {
            let with = toggled(&format!("{original}\n"), 0, true).expect("must add");
            let without = toggled(&format!("{with}\n"), 0, false).expect("must remove");
            assert_eq!(without, original, "round trip failed for {original:?}");
        }
    }

    /// Nothing to remove where there is none.
    #[test]
    fn removing_from_a_plain_line_does_nothing() {
        assert!(toggled("\tld a, 0\n", 0, false).is_none());
    }
}
