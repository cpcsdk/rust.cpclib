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
use cpclib_tokens::ListingElement;
use tower_lsp::lsp_types::{Position, Range, TextEdit};

use crate::common::document::Document;

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
/// `condition` is the red dot's, when the editor says (`Some`): written as the
/// directive's `CONDITION="..."`, kept in step with it - added, changed, or
/// taken off (`Some(None)`) on a directive already there. Without it (`None`,
/// an older editor) a directive already there is left as it is.
///
/// The single entry point the server calls: it parses, so callers do not need
/// access to the analyzer's internals.
pub(crate) fn breakpoint_edit(
    analyzer: &super::AssemblyAnalyzer,
    document: &Document,
    line: u32,
    enable: bool,
    condition: Option<Option<&str>>
) -> Option<TextEdit> {
    let listing = analyzer.parse_document(document).ok()?;
    let line_text = document.line(line as usize)?;
    let line_text = line_text.trim_end_matches(['\r', '\n']);
    if !enable {
        return remove_breakpoint_edit(&listing, line_text, line);
    }
    match directive_start(&listing, line_text, line) {
        Some(start) => set_condition_edit(line_text, line, start, condition?),
        None if commented_directive(line_text).is_some() => {
            restore_breakpoint_edit(&listing, line_text, line, condition)
        },
        None => {
            let config = analyzer.config();
            let directive = match condition.flatten() {
                Some(condition) => {
                    format!("{} {} : ", config.breakpoint_directive, condition_argument(condition)?)
                },
                None => directive(&config)
            };
            add_breakpoint_edit(&listing, line_text, line, &directive)
        }
    }
}

/// Where the `breakpoint` word of the directive on `line` starts, in bytes,
/// if the line has one: from its token, or in a macro body (no tokens there)
/// from the text.
fn directive_start(listing: &LocatedListing, line_text: &str, line: u32) -> Option<usize> {
    let token = super::token::flatten_listing(listing.iter())
        .find(|t| line_and_column(*t).0 == line && t.is_breakpoint());
    match token {
        Some(token) => {
            let start_byte = line_and_column(token).1 as usize;
            let rest = line_text.get(start_byte..)?;
            Some(start_byte + rest.len() - rest.trim_start().len())
        },
        None if inside_a_macro_body(&macro_body_ranges(listing), line) => {
            directive_in_text(line_text).map(|(start, _)| start)
        },
        None => None
    }
}

/// Where the directive starting at `start` ends, in bytes: its arguments
/// included, up to a statement separator or a comment that is not inside a
/// string (a name or a condition may hold either), trailing blanks left out.
fn directive_end(line_text: &str, start: usize) -> usize {
    let mut quote = Quote::default();
    let mut end = line_text.len();
    for (offset, c) in line_text[start..].char_indices() {
        if quote.outside(c) && matches!(c, ':' | ';') {
            end = start + offset;
            break;
        }
    }
    start + line_text[start..end].trim_end().len()
}

/// Whether a scan is inside a string: `"..."` or `'...'` (rasm writes
/// `NAME='...'`), each closed by its own quote.
#[derive(Default)]
struct Quote(Option<char>);

impl Quote {
    /// Take `c` in; whether it is outside any string (a quote itself is not).
    fn outside(&mut self, c: char) -> bool {
        match self.0 {
            Some(open) => {
                if c == open {
                    self.0 = None;
                }
                false
            },
            None if c == '"' || c == '\'' => {
                self.0 = Some(c);
                false
            },
            None => true
        }
    }
}

/// The directive's arguments, split on the commas outside strings.
fn directive_arguments(arguments: &str) -> Vec<String> {
    let mut parts = vec![String::new()];
    let mut quote = Quote::default();
    for c in arguments.chars() {
        if quote.outside(c) && c == ',' {
            parts.push(String::new());
        }
        else {
            parts.last_mut().unwrap().push(c);
        }
    }
    parts.into_iter().map(|part| part.trim().to_owned()).filter(|part| !part.is_empty()).collect()
}

/// Whether an argument is `CONDITION=...`.
fn is_condition_argument(argument: &str) -> bool {
    argument
        .split_once('=')
        .is_some_and(|(key, _)| key.trim().eq_ignore_ascii_case("condition"))
}

/// `CONDITION="..."` for `condition` - between single quotes when it holds a
/// `"` - or `None` when it holds both quotes and cannot be written.
fn condition_argument(condition: &str) -> Option<String> {
    if !condition.contains('"') {
        Some(format!("CONDITION=\"{condition}\""))
    }
    else if !condition.contains('\'') {
        Some(format!("CONDITION='{condition}'"))
    }
    else {
        None
    }
}

/// The condition of a directive (`breakpoint ..., CONDITION="A = 5"`), if it
/// has one.
fn condition_of(directive: &str) -> Option<String> {
    let arguments = directive.trim_start().split_once(char::is_whitespace)?.1;
    directive_arguments(arguments)
        .into_iter()
        .find(|argument| is_condition_argument(argument))
        .and_then(|argument| argument.split_once('=').map(|(_, value)| unquoted(value).to_owned()))
        .filter(|condition| !condition.is_empty())
}

/// The directive with `condition` as its condition, or none: its other
/// arguments kept, in their order.
fn with_condition(directive: &str, condition: Option<&str>) -> Option<String> {
    let directive = directive.trim();
    let (word, arguments) = directive.split_once(char::is_whitespace).unwrap_or((directive, ""));
    let mut arguments: Vec<String> =
        directive_arguments(arguments).into_iter().filter(|argument| !is_condition_argument(argument)).collect();
    if let Some(condition) = condition.filter(|condition| !condition.trim().is_empty()) {
        arguments.push(condition_argument(condition.trim())?);
    }
    Some(if arguments.is_empty() { word.to_owned() } else { format!("{word} {}", arguments.join(", ")) })
}

/// The edit that gives the directive starting at `start` the condition
/// `condition` (none: takes its condition off), or `None` when it already
/// has it.
fn set_condition_edit(line_text: &str, line: u32, start: usize, condition: Option<&str>) -> Option<TextEdit> {
    let end = directive_end(line_text, start);
    let directive = &line_text[start..end];
    let wanted = condition.map(str::trim).filter(|condition| !condition.is_empty());
    if condition_of(directive).as_deref() == wanted {
        return None;
    }
    let new_text = with_condition(directive, wanted)?;
    let at = |byte: usize| {
        Position {
            line,
            character: crate::common::document::byte_offset_to_utf16_col(line_text, byte) as u32
        }
    };
    Some(TextEdit {
        range: Range { start: at(start), end: at(end) },
        new_text
    })
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
        let character = crate::common::document::byte_offset_to_utf16_col(line_text, column) as u32;
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
    let character =
        crate::common::document::byte_offset_to_utf16_col(line_text, column as usize) as u32;
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
    // The whole directive, its arguments with it: `BREAKPOINT TYPE=MEM,
    // ADDR=buffer` loses all of it, not only its first word.
    let mut start = directive_start(listing, line_text, line)?;
    let mut end = directive_end(line_text, start);
    let directive = line_text[start..end].to_owned();

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

    // A directive with arguments is commented out rather than lost: the red
    // dot set again on the line brings it back whole.
    if has_arguments(&directive) {
        let rest = format!("{}{}", &line_text[..start], &line_text[end..]);
        let (code, comment) = split_comment(&rest);
        let code = code.trim_end();
        let mut new_line = if code.trim().is_empty() {
            // alone on its line: commented where it was
            let indent = &line_text[..line_text.len() - line_text.trim_start().len()];
            format!("{indent}; {directive}")
        }
        else {
            format!("{code} ; {directive}")
        };
        if !comment.is_empty() {
            new_line.push(' ');
            new_line.push_str(comment);
        }
        return Some(whole_line_edit(line_text, line, new_line));
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

/// A string's text without its quotes, `"` or `'` (the pair only: a quote
/// inside is the string's own).
fn unquoted(value: &str) -> &str {
    let value = value.trim();
    for quote in ['"', '\''] {
        if let Some(inner) = value.strip_prefix(quote).and_then(|v| v.strip_suffix(quote)) {
            return inner;
        }
    }
    value
}

/// Whether a directive carries arguments (`BREAKPOINT TYPE=MEM, ...`), not
/// only its word.
fn has_arguments(directive: &str) -> bool {
    directive.trim().split_once(char::is_whitespace).is_some_and(|(_, arguments)| !arguments.trim().is_empty())
}

/// A line's code and its comment (from the first `;` outside a string, `""`
/// when it has none).
fn split_comment(line_text: &str) -> (&str, &str) {
    let mut quote = Quote::default();
    for (index, c) in line_text.char_indices() {
        if quote.outside(c) && c == ';' {
            return (&line_text[..index], &line_text[index..]);
        }
    }
    (line_text, "")
}

/// A directive commented out on the line (`; BREAKPOINT TYPE=MEM, ...`), as
/// clearing its red dot leaves it: where its comment starts, the directive,
/// and the comment after it (`; why`, or `""`).
fn commented_directive(line_text: &str) -> Option<(usize, String, String)> {
    let (code, comment) = split_comment(line_text);
    let text = comment.strip_prefix(';')?.trim_start();
    let word = text.get(..text.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(text.len()))?;
    if !word.eq_ignore_ascii_case("breakpoint") {
        return None;
    }
    // up to the comment that followed it, if any
    let (directive, after) = split_comment(text);
    // a comment that only talks about a breakpoint is not one
    if !looks_like_arguments(&directive.trim()[word.len()..]) {
        return None;
    }
    Some((code.len(), directive.trim_end().to_owned(), after.trim().to_owned()))
}

/// Whether the text after a `breakpoint` word reads as its arguments (as
/// clearing a red dot comments out only a directive with some): each a word
/// of the directive (`MEM`, `WRITE`, `STOP`...), a `KEY=value` of it, or -
/// the first only - an address, one expression without blanks.
fn looks_like_arguments(arguments: &str) -> bool {
    const WORDS: &[&str] = &[
        "EXEC", "EXECUTE", "MEM", "MEMORY", "IO", "R", "READ", "W", "WRITE", "RW", "READWRITE", "STOP", "STOPPER",
        "WATCH", "WATCHER"
    ];
    const KEYS: &[&str] = &[
        "TYPE", "ACCESS", "RUNMODE", "ADDR", "ADDRESS", "MASK", "SIZE", "VALUE", "VALMASK", "CONDITION", "NAME", "STEP"
    ];
    let arguments = directive_arguments(arguments);
    !arguments.is_empty()
        && arguments.iter().enumerate().all(|(index, argument)| {
            match argument.split_once('=') {
                Some((key, _)) => KEYS.iter().any(|known| key.trim().eq_ignore_ascii_case(known)),
                None => {
                    WORDS.iter().any(|known| argument.eq_ignore_ascii_case(known))
                        || (index == 0 && !argument.contains(char::is_whitespace))
                }
            }
        })
}

/// An edit replacing the whole of `line`.
fn whole_line_edit(line_text: &str, line: u32, new_text: String) -> TextEdit {
    TextEdit {
        range: Range {
            start: Position { line, character: 0 },
            end: Position {
                line,
                character: crate::common::document::byte_offset_to_utf16_col(line_text, line_text.len()) as u32
            }
        },
        new_text
    }
}

/// The edit that brings back a directive commented out on `line` (see
/// [`remove_breakpoint_edit`]), with `condition` when the red dot carries one:
/// in front of the line's first instruction, or where it was when it had the
/// line to itself.
fn restore_breakpoint_edit(
    listing: &LocatedListing,
    line_text: &str,
    line: u32,
    condition: Option<Option<&str>>
) -> Option<TextEdit> {
    let (comment_at, directive, after) = commented_directive(line_text)?;
    let directive = match condition {
        Some(condition) => with_condition(&directive, condition)?,
        None => directive
    };
    let code = line_text[..comment_at].trim_end();
    let first_instruction = super::token::flatten_listing(listing.iter())
        .filter(|t| line_and_column(*t).0 == line)
        .find(|t| t.mnemonic().is_some())
        .map(|t| line_and_column(t).1 as usize)
        .or_else(|| {
            // a macro body has no tokens: its first word, when the line has code
            (inside_a_macro_body(&macro_body_ranges(listing), line) && !code.trim().is_empty())
                .then(|| code.len() - code.trim_start().len())
        });
    let mut new_line = match first_instruction {
        Some(column) if column <= code.len() => format!("{}{directive} : {}", &code[..column], &code[column..]),
        // no instruction: alone (or after a label), where it was
        _ if code.trim().is_empty() => format!("{}{directive}", &line_text[..line_text.len() - line_text.trim_start().len()]),
        _ => format!("{code} {directive}")
    };
    if !after.is_empty() {
        new_line.push(' ');
        new_line.push_str(&after);
    }
    Some(whole_line_edit(line_text, line, new_line))
}

/// Every directive of `document`: its 0-based line and its condition, for the
/// editor to draw its red dot with that condition - the dot's condition is
/// written back into the directive when it changes.
pub(crate) fn breakpoint_directives(
    analyzer: &super::AssemblyAnalyzer,
    document: &Document
) -> Vec<(u32, Option<String>)> {
    let Ok(listing) = analyzer.parse_document(document)
    else {
        return Vec::new();
    };
    breakpoint_lines(analyzer, document)
        .into_iter()
        .map(|line| {
            let condition = document.line(line as usize).and_then(|text| {
                let text = text.trim_end_matches(['\r', '\n']);
                let start = directive_start(&listing, text, line)?;
                condition_of(&text[start..directive_end(text, start)])
            });
            (line, condition)
        })
        .collect()
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

    /// The line once `breakpoint_edit` is applied, with a red dot's condition.
    fn edited(text: &str, line: u32, enable: bool, condition: Option<Option<&str>>) -> Option<String> {
        let d = Document::new(Url::parse("file:///t.asm").unwrap(), text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        let edit = breakpoint_edit(&analyzer, &d, line, enable, condition)?;
        let line_text = d.line(line as usize).unwrap_or_default();
        let line_text = line_text.trim_end_matches(['\r', '\n']);
        let start = edit.range.start.character as usize;
        let end = edit.range.end.character as usize;
        let mut out: String = line_text.chars().take(start).collect();
        out.push_str(&edit.new_text);
        out.extend(line_text.chars().skip(end));
        Some(out)
    }

    /// A directive with arguments is commented out, not lost, and never left
    /// half-written: the arguments come with it, quoted `:` and `;` included.
    #[test]
    fn clearing_an_advanced_directive_comments_it_out() {
        let program = "\tbreakpoint type=mem, access=write, addr=buffer, name=\"a: b; c\" ; why\n\tld a,1\nbuffer\n\tdefb 0\n";
        assert_eq!(
            edited(program, 0, false, None).as_deref(),
            Some("\t; breakpoint type=mem, access=write, addr=buffer, name=\"a: b; c\" ; why")
        );
        let program = "\tbreakpoint condition=\"A = 5\" : inc a ; count\n";
        assert_eq!(edited(program, 0, false, None).as_deref(), Some("\tinc a ; breakpoint condition=\"A = 5\" ; count"));
        let program = "\tld a,0 : breakpoint #7f00, type=io\n";
        assert_eq!(edited(program, 0, false, None).as_deref(), Some("\tld a,0 ; breakpoint #7f00, type=io"));
        let program = "loop\tld a,0 : breakpoint #7f00, type=io : inc a\n";
        assert_eq!(edited(program, 0, false, None).as_deref(), Some("loop\tld a,0 : inc a ; breakpoint #7f00, type=io"));
        // a plain one has nothing worth keeping
        assert_eq!(edited("\tbreakpoint : inc a\n", 0, false, None).as_deref(), Some("\tinc a"));
    }

    /// Clearing the red dot then setting it again gives the directive back as
    /// it was written, wherever it was on the line.
    #[test]
    fn a_commented_directive_comes_back_with_its_red_dot() {
        for original in [
            "\tbreakpoint type=mem, access=write, addr=buffer, name=\"a: b\" ; why",
            "\tbreakpoint condition=\"A = 5\" : inc a ; count",
            "loop\tbreakpoint #7f00, type=io : ld a,0",
            "\tbreakpoint #7f00, type=io",
        ] {
            let program = |line: &str| format!("{line}\n\tld a,1\nbuffer\n\tdefb 0\n");
            let cleared = edited(&program(original), 0, false, None).expect("commented out");
            assert!(cleared.contains("; breakpoint"), "{cleared}");
            let back = edited(&program(&cleared), 0, true, None).expect("restored");
            assert_eq!(back, original, "round trip of {original:?} through {cleared:?}");
        }
    }

    /// rasm's spelling, single quotes, and what they hold kept whole.
    #[test]
    fn single_quoted_arguments_as_rasm_writes_them() {
        let program = "\tbreakpoint MEM,WRITE,STOP,addr=buffer,name='a: b; c',condition='A = 5' ; why\n\tld a,1\nbuffer\n\tdefb 0\n";
        let d = Document::new(Url::parse("file:///t.asm").unwrap(), program.to_string(), 1);
        assert_eq!(breakpoint_directives(&AssemblyAnalyzer::new(), &d), vec![(0, Some("A = 5".to_owned()))]);
        let cleared = edited(program, 0, false, None).expect("commented out");
        assert_eq!(cleared, "\t; breakpoint MEM,WRITE,STOP,addr=buffer,name='a: b; c',condition='A = 5' ; why");
        assert_eq!(edited(&program.replacen("\tbreakpoint", "\t; breakpoint", 1), 0, true, None).as_deref(),
                   Some("\tbreakpoint MEM,WRITE,STOP,addr=buffer,name='a: b; c',condition='A = 5' ; why"));
        assert_eq!(
            edited(program, 0, true, Some(Some("A = 6"))).as_deref(),
            Some("\tbreakpoint MEM, WRITE, STOP, addr=buffer, name='a: b; c', CONDITION=\"A = 6\" ; why")
        );
    }

    /// A comment that only mentions a breakpoint is not a directive to bring
    /// back: setting a red dot on its line writes a new one, the comment kept.
    #[test]
    fn a_comment_about_a_breakpoint_is_not_restored() {
        assert_eq!(
            edited("\tinc a ; breakpoint here because the loop is wrong\n", 0, true, Some(None)).as_deref(),
            Some("\tBREAKPOINT : inc a ; breakpoint here because the loop is wrong")
        );
        assert_eq!(
            edited("\tinc a ; breakpoint, then step\n", 0, true, Some(None)).as_deref(),
            Some("\tBREAKPOINT : inc a ; breakpoint, then step")
        );
    }

    /// A condition's own quotes are kept, only the pair around it goes.
    #[test]
    fn a_condition_keeps_its_own_quotes() {
        assert_eq!(condition_of("breakpoint CONDITION='A = \"x\"'").as_deref(), Some("A = \"x\""));
        assert_eq!(condition_of("breakpoint name=\"n\", condition=A=5").as_deref(), Some("A=5"));
    }

    /// Inside a macro body (no tokens there) too.
    #[test]
    fn a_directive_in_a_macro_body_is_commented_and_restored() {
        let program = |line: &str| format!("\tmacro DEBUG\n{line}\n\tendm\n\tDEBUG()\n");
        let original = "\tbreakpoint condition=\"A = 1\" : ld bc,0";
        let cleared = edited(&program(original), 1, false, None).expect("commented out");
        assert_eq!(cleared, "\tld bc,0 ; breakpoint condition=\"A = 1\"");
        assert_eq!(edited(&program(&cleared), 1, true, None).as_deref(), Some(original));
    }

    /// Restored with the red dot's condition when it has one; a commented
    /// directive has no red dot of its own.
    #[test]
    fn a_commented_directive_restored_takes_the_dots_condition() {
        let program = "\tinc a ; breakpoint name=\"n\", condition=\"A = 3\"\n";
        assert_eq!(
            edited(program, 0, true, Some(Some("A = 4"))).as_deref(),
            Some("\tbreakpoint name=\"n\", CONDITION=\"A = 4\" : inc a")
        );
        let d = Document::new(Url::parse("file:///t.asm").unwrap(), program.to_string(), 1);
        assert!(breakpoint_directives(&AssemblyAnalyzer::new(), &d).is_empty(), "inactive: no red dot");
    }

    /// A red dot with a condition writes a directive with it.
    #[test]
    fn a_red_dot_with_a_condition_writes_it() {
        assert_eq!(
            edited("\tinc a\n", 0, true, Some(Some("A = 5"))).as_deref(),
            Some("\tBREAKPOINT CONDITION=\"A = 5\" : inc a")
        );
        // without one, the plain directive
        assert_eq!(edited("\tinc a\n", 0, true, Some(None)).as_deref(), Some("\tBREAKPOINT : inc a"));
        // a `"` in it: written between single quotes; both: not written
        assert_eq!(
            edited("\tinc a\n", 0, true, Some(Some("A = \"x\""))).as_deref(),
            Some("\tBREAKPOINT CONDITION='A = \"x\"' : inc a")
        );
        assert_eq!(edited("\tinc a\n", 0, true, Some(Some("\"'"))), None);
    }

    /// The dot's condition changed: the directive's follows - added, changed,
    /// taken off - its other arguments kept.
    #[test]
    fn a_directives_condition_follows_its_red_dot() {
        let plain = "\tbreakpoint : inc a\n";
        assert_eq!(
            edited(plain, 0, true, Some(Some("A = 5"))).as_deref(),
            Some("\tbreakpoint CONDITION=\"A = 5\" : inc a")
        );
        let advanced = "\tbreakpoint type=mem, addr=buffer, condition=\"A = 3\", name=\"n\"\n\tinc a\nbuffer\n\tdefb 0\n";
        assert_eq!(
            edited(advanced, 0, true, Some(Some("A = 7"))).as_deref(),
            Some("\tbreakpoint type=mem, addr=buffer, name=\"n\", CONDITION=\"A = 7\"")
        );
        assert_eq!(
            edited(advanced, 0, true, Some(None)).as_deref(),
            Some("\tbreakpoint type=mem, addr=buffer, name=\"n\"")
        );
        // the same condition: nothing to do
        assert_eq!(edited(advanced, 0, true, Some(Some("A = 3"))), None);
        // an editor that does not say: left alone
        assert_eq!(edited(advanced, 0, true, None), None);
    }

    /// The directives of a file, each with its condition, for their red dots.
    #[test]
    fn directives_are_listed_with_their_conditions() {
        let text = "\tbreakpoint : ld b, 1\n\tbreakpoint condition=\"A = 5\", name=\"x\"\n\tnop\n";
        let d = Document::new(Url::parse("file:///t.asm").unwrap(), text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        assert_eq!(
            breakpoint_directives(&analyzer, &d),
            vec![(0, None), (1, Some("A = 5".to_owned()))]
        );
    }
}
