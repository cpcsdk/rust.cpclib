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
///
/// The word comes from `asm.breakpoint_directive`; the ` : ` is basm's
/// statement separator and is not a matter of taste.
fn directive(config: &crate::common::config::AsmConfig) -> String {
    format!("{} : ", config.breakpoint_directive)
}

/// The 0-based line ranges covered by macro definition bodies.
///
/// Inclusive of the `MACRO` and `ENDM` lines themselves, which is harmless:
/// neither carries an instruction, so both are refused for other reasons.
fn macro_body_ranges(listing: &LocatedListing) -> Vec<(u32, u32)> {
    super::token::flatten_listing(listing.iter())
        .filter(|t| t.is_macro_definition())
        .map(|t| {
            let start = line_and_column(t).0;
            let height = t.macro_definition_code().lines().count() as u32;
            (start, start + height + 1)
        })
        .collect()
}

fn inside_a_macro_body(ranges: &[(u32, u32)], line: u32) -> bool {
    ranges.iter().any(|(from, to)| line >= *from && line <= *to)
}

/// The `breakpoint` word in a line, as a byte range, ignoring any comment.
///
/// Only for macro-body lines, where there is no token to ask. Matched
/// case-insensitively and only as a whole word, so `breakpoint_count` is not
/// mistaken for one.
fn directive_in_text(line: &str) -> Option<(usize, usize)> {
    let code = line.split(';').next().unwrap_or(line);
    let lowered = code.to_ascii_lowercase();
    let mut from = 0;
    while let Some(found) = lowered[from..].find("breakpoint") {
        let start = from + found;
        let end = start + "breakpoint".len();
        let before_ok = start == 0
            || !code[..start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.');
        let after_ok = !code[end..]
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_');
        if before_ok && after_ok {
            return Some((start, end));
        }
        from = end;
    }
    None
}

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
        add_breakpoint_edit(&listing, line_text, line, &directive(&analyzer.config()))
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
    line: u32,
    directive: &str
) -> Option<TextEdit> {
    let on_line: Vec<_> = super::token::flatten_listing(listing.iter())
        .filter(|t| line_and_column(*t).0 == line)
        .collect();

    if on_line.iter().any(|t| t.is_breakpoint()) {
        return None;
    }

    // No AST here at all: a macro body is text until it is expanded.
    if on_line.is_empty() && inside_a_macro_body(&macro_body_ranges(listing), line) {
        if directive_in_text(line_text).is_some() {
            return None;
        }
        let code = line_text.split(';').next().unwrap_or(line_text);
        if code.trim().is_empty() {
            return None;
        }
        let column = code.len() - code.trim_start().len();
        let character =
            crate::common::document::byte_offset_to_utf16_col(line_text, column) as u32;
        let position = Position { line, character };
        return Some(TextEdit {
            range: Range {
                start: position,
                end: position
            },
            new_text: directive.to_string()
        });
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
        new_text: directive.to_string()
    })
}

/// The edit that takes the `breakpoint` back off `line`, or `None` if there is
/// none there.
///
/// Removes what it finds in the source rather than assuming the exact spelling
/// this module writes: the user may have reformatted the line, typed the
/// directive themselves, or put it somewhere other than the front - a
/// breakpoint is a breakpoint wherever on the line it sits.
///
/// Which is why the statement separator is taken from whichever side actually
/// has one. `BREAKPOINT : ld a,0` gives up the `:` after it; `ld a,0 :
/// BREAKPOINT` the one before; a directive alone on its line has neither.
/// Removing the word alone would leave a dangling `:` that does not assemble.
pub(super) fn remove_breakpoint_edit(
    listing: &LocatedListing,
    line_text: &str,
    line: u32
) -> Option<TextEdit> {
    let token = super::token::flatten_listing(listing.iter())
        .find(|t| line_and_column(*t).0 == line && t.is_breakpoint());

    let (mut start, mut end) = match token {
        Some(token) => {
            let start_byte = line_and_column(token).1 as usize;
            let rest = line_text.get(start_byte..)?;
            let start = start_byte + rest.len() - rest.trim_start().len();
            let after_word = rest.trim_start();
            let word_len = after_word
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .unwrap_or(after_word.len());
            (start, start + word_len)
        },
        // Inside a macro body there is no token to find it by.
        None if inside_a_macro_body(&macro_body_ranges(listing), line) => {
            directive_in_text(line_text)?
        },
        None => return None
    };

    // A separator on the right closes the line up after the removal.
    let tail = line_text.get(end..)?;
    let trimmed = tail.trim_start();
    if let Some(without_separator) = trimmed.strip_prefix(':') {
        end += tail.len() - without_separator.len();
        let tail = line_text.get(end..)?;
        end += tail.len() - tail.trim_start().len();
    }
    else {
        // Nothing to the right, so take the one on the left - otherwise
        // `ld a,0 : BREAKPOINT` would be left as `ld a,0 :`.
        let head = line_text.get(..start)?;
        let before = head.trim_end();
        if let Some(without_separator) = before.strip_suffix(':') {
            start = without_separator.trim_end().len();
        }
    }

    Some(TextEdit {
        range: Range {
            start: Position {
                line,
                character: crate::common::document::byte_offset_to_utf16_col(line_text, start)
                    as u32
            },
            end: Position {
                line,
                character: crate::common::document::byte_offset_to_utf16_col(line_text, end) as u32
            }
        },
        new_text: String::new()
    })
}

/// Every 0-based line of `document` carrying a `breakpoint` directive.
///
/// The other direction of the same idea: a file that already has directives in
/// it - written by hand, or by an earlier session - should show its red dots
/// when opened, not only after the user clicks one.
pub(crate) fn breakpoint_lines(
    analyzer: &super::AssemblyAnalyzer,
    document: &Document
) -> Vec<u32> {
    let Ok(listing) = analyzer.parse_document(document)
    else {
        return Vec::new();
    };
    let mut lines: Vec<u32> = super::token::flatten_listing(listing.iter())
        .filter(|t| t.is_breakpoint())
        .map(|t| line_and_column(t).0)
        .collect();

    // Macro bodies never became tokens, so they are read straight from the
    // source - otherwise a breakpoint inside a macro would have no red dot.
    for (from, to) in macro_body_ranges(&listing) {
        for line in from..=to {
            if let Some(text) = document.line(line as usize)
                && directive_in_text(&text).is_some()
            {
                lines.push(line);
            }
        }
    }
    // A line could hold two, and the editor wants one dot per line.
    lines.sort_unstable();
    lines.dedup();
    lines
}

#[cfg(test)]
mod tests {
    use tower_lsp::lsp_types::Url;

    use super::*;
    use crate::basm::AssemblyAnalyzer;
    use crate::common::config::AsmConfig;
    use crate::common::document::Document;

    /// What the shipped default writes, so the tests below assert against the
    /// real thing rather than a spelling of their own.
    const DEFAULT: &str = "BREAKPOINT : ";

    /// Apply whichever edit the toggle produces, and hand back the new line -
    /// what the user would actually see in the editor.
    fn toggled(text: &str, line: u32, enable: bool) -> Option<String> {
        let d = Document::new(Url::parse("file:///t.asm").unwrap(), text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        let listing = analyzer.parse_document(&d).expect("must parse");
        let line_text = d.line(line as usize).unwrap_or_default();
        let line_text = line_text.trim_end_matches(['\r', '\n']);
        let edit = if enable {
            add_breakpoint_edit(&listing, line_text, line, DEFAULT)?
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
            toggled("\tld a, 0\n", 0, true),
            Some(format!("\t{DEFAULT}ld a, 0"))
        );
    }

    /// A label has to keep pointing at the code, so the directive goes after
    /// it - not at column zero.
    #[test]
    fn a_label_on_the_line_keeps_its_place() {
        assert_eq!(
            toggled("loop\tld a, 0\n", 0, true),
            Some(format!("loop\t{DEFAULT}ld a, 0"))
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
        assert!(toggled("\tBREAKPOINT : ld a, 0\n", 0, true).is_none());
    }

    /// Removing puts the line back exactly as it was - whichever case the
    /// directive is written in, since removal reads the parsed token rather
    /// than matching the text this module happens to write.
    #[test]
    fn removing_closes_the_line_back_up() {
        for spelling in ["breakpoint", "BREAKPOINT", "Breakpoint"] {
            assert_eq!(
                toggled(&format!("\t{spelling} : ld a, 0\n"), 0, false).as_deref(),
                Some("\tld a, 0"),
                "failed for {spelling}"
            );
        }
        assert_eq!(
            toggled("loop\tBREAKPOINT : ld a, 0\n", 0, false).as_deref(),
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

    /// The default the tests above assume is the one the config actually
    /// ships - otherwise they would be pinning a fiction.
    #[test]
    fn the_default_directive_is_what_gets_written() {
        assert_eq!(directive(&AsmConfig::default()), DEFAULT);
    }

    /// A project that spells its directives in lower case gets that.
    #[test]
    fn the_directive_follows_the_configuration() {
        let config = AsmConfig {
            breakpoint_directive: "breakpoint".to_string(),
            ..AsmConfig::default()
        };
        assert_eq!(directive(&config), "breakpoint : ");
    }

    /// A directive the user put at the end of the line still comes off
    /// cleanly - taking the separator *before* it, since there is none after.
    #[test]
    fn a_trailing_directive_takes_the_separator_before_it() {
        assert_eq!(
            toggled("\tld a, 0 : BREAKPOINT\n", 0, false).as_deref(),
            Some("\tld a, 0")
        );
        assert_eq!(
            toggled("\tld a, 0 : BREAKPOINT : ld b, 1\n", 0, false).as_deref(),
            Some("\tld a, 0 : ld b, 1")
        );
    }

    /// One on its own line has no separator on either side.
    #[test]
    fn a_directive_alone_on_its_line_leaves_the_line_empty() {
        assert_eq!(toggled("\tBREAKPOINT\n", 0, false).as_deref(), Some("\t"));
    }

    /// Whatever the position, adding is refused when one is already there -
    /// which is also what stops the editor and the file feeding each other.
    #[test]
    fn a_trailing_directive_still_counts_as_present() {
        assert!(toggled("\tld a, 0 : BREAKPOINT\n", 0, true).is_none());
    }

    /// Directives already in a file are reported so the editor can show their
    /// red dots on open.
    #[test]
    fn every_line_carrying_a_directive_is_listed() {
        let text = "\tld a, 0\n\
                    \tBREAKPOINT : ld b, 1\n\
                    \tld c, 2 : BREAKPOINT\n\
                    \tnop\n";
        let d = Document::new(Url::parse("file:///t.asm").unwrap(), text.to_string(), 1);
        assert_eq!(breakpoint_lines(&AssemblyAnalyzer::new(), &d), vec![1, 2]);
    }

    /// A file with none reports none - the editor must not be handed a dot to
    /// draw where there is nothing.
    #[test]
    fn a_file_without_directives_lists_nothing() {
        let d = Document::new(
            Url::parse("file:///t.asm").unwrap(),
            "\tld a, 0\n\tnop\n".to_string(),
            1
        );
        assert!(breakpoint_lines(&AssemblyAnalyzer::new(), &d).is_empty());
    }

    /// Reported: the red dot appears but no directive is written. A macro
    /// body is text, not tokens, so nothing was found on the line.
    #[test]
    fn a_line_inside_a_macro_body_can_take_one() {
        let text = "\tmacro FOO\n\t\tld a, 0\n\tendm\n";
        assert_eq!(
            toggled(text, 1, true),
            Some(format!("\t\t{DEFAULT}ld a, 0"))
        );
    }

    /// And comes back off again.
    #[test]
    fn a_breakpoint_inside_a_macro_body_can_be_removed() {
        let text = "\tmacro FOO\n\t\tBREAKPOINT : ld a, 0\n\tendm\n";
        assert_eq!(toggled(text, 1, false).as_deref(), Some("\t\tld a, 0"));
    }

    /// Round trip, the property that matters when the same dot is clicked
    /// twice.
    #[test]
    fn adding_and_removing_inside_a_macro_restores_the_line() {
        let body = "\t\tld a, 0";
        let with = toggled(&format!("\tmacro FOO\n{body}\n\tendm\n"), 1, true).unwrap();
        let back = toggled(&format!("\tmacro FOO\n{with}\n\tendm\n"), 1, false).unwrap();
        assert_eq!(back, body);
    }

    /// Twice in a row is still once.
    #[test]
    fn a_macro_body_line_that_already_has_one_is_left_alone() {
        let text = "\tmacro FOO\n\t\tbreakpoint : ld a, 0\n\tendm\n";
        assert!(toggled(text, 1, true).is_none());
    }

    /// Blank and comment-only body lines have nothing to break on.
    #[test]
    fn empty_macro_body_lines_are_refused() {
        let text = "\tmacro FOO\n\n\t; nothing here\n\tendm\n";
        assert!(toggled(text, 1, true).is_none());
        assert!(toggled(text, 2, true).is_none());
    }

    /// A directive inside a macro body must get its red dot too.
    #[test]
    fn a_directive_inside_a_macro_body_is_listed() {
        let text = "\tmacro FOO\n\t\tBREAKPOINT : ld a, 0\n\tendm\n\tnop\n";
        let d = Document::new(Url::parse("file:///t.asm").unwrap(), text.to_string(), 1);
        assert_eq!(breakpoint_lines(&AssemblyAnalyzer::new(), &d), vec![1]);
    }

    /// A word that merely contains "breakpoint" is not one.
    #[test]
    fn a_similar_word_is_not_mistaken_for_the_directive() {
        assert!(directive_in_text("\t\tld a, breakpoint_count").is_none());
        assert!(directive_in_text("\t\t; breakpoint here later").is_none());
        assert!(directive_in_text("\t\tBREAKPOINT : nop").is_some());
    }

    /// Nothing to remove where there is none.
    #[test]
    fn removing_from_a_plain_line_does_nothing() {
        assert!(toggled("\tld a, 0\n", 0, false).is_none());
    }
}
