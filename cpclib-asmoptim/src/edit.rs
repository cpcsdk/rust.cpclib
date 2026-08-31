//! Turning a match into the source edit that applies it.
//!
//! Every consumer that *acts* on a suggestion - `cpclib-basmopt` rewriting a
//! file in place, the LSP offering a quickfix - has to answer the same
//! questions, and they are fiddlier than they look:
//!
//! * **Which bytes does this match actually occupy?** Not "the lines it is on":
//!   basm puts several instructions on a line (`ld bc,#7F00 : out (c),c : ld
//!   a,#54`), so replacing whole lines destroys whatever else shares them. That
//!   was not hypothetical - it silently deleted three instructions per line
//!   from a real palette-setup routine.
//! * **What happens to the `:` separators?** Deleting an instruction that
//!   shares a line must take one separator with it, or a dangling `:` is left
//!   behind. *Replacing* one must leave them alone, or the new instructions get
//!   glued to the surviving ones.
//! * **Where can a comment go?** A comment runs to end of line, so anything
//!   emitted after one on the same line is silently commented out.
//!
//! Getting any of these wrong corrupts the user's file, so it lives here once
//! rather than in each consumer - the same reason `flatten_listing` sits in
//! `cpclib-asm` rather than being copied into everything that walks a listing.

use std::ops::Range;

use cpclib_asm::parser::MayHaveSpan;
use cpclib_tokens::ListingElement;

use crate::engine::PeepholeMatch;

/// A replacement to splice into the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEdit {
    /// The byte range to replace.
    pub range: Range<usize>,
    /// What to put there. Empty for a deletion.
    pub text: String
}

/// The edit that applies `m` to `source`.
///
/// `tokens` must be the same slice the match was produced against, since the
/// match's indices point into it.
pub fn edit_for_match<T>(source: &str, tokens: &[&T], m: &PeepholeMatch) -> Option<SourceEdit>
where T: ListingElement + MayHaveSpan {
    let first = tokens.get(m.start)?.possible_span()?;
    let last = tokens.get(m.end.checked_sub(1)?)?.possible_span()?;

    let token_start = first.offset_from_start();
    // `trim_end`: a token's span runs up to the next one, so it carries the
    // whitespace before the following `:` along with it. Leaving that
    // whitespace outside the range yields `inc bc : out (c),c` rather than
    // `inc bc: out (c),c`, where `bc:` reads like a label definition.
    let token_end = last.offset_from_start() + last.to_string().trim_end().len();
    if token_end > source.len() || token_start > token_end {
        return None;
    }

    let ls = line_start(source, token_start);
    let le = line_end(source, token_end);
    let alone = source[ls..token_start].trim().is_empty() && source[token_end..le].trim().is_empty();

    let is_deletion = m.replacement.is_empty();

    let (start, mut end) = if alone {
        // The match has these lines to itself, so the whole-line span is both
        // correct and tidier: it keeps the original indentation for a
        // replacement, and lets a deletion take the lines away entirely.
        (ls, le)
    }
    else if is_deletion {
        absorb_separator(source, tokens, m.start, token_start, token_end, le)
    }
    else {
        (token_start, token_end)
    };

    // A deletion of lines it owned takes the trailing newline too, so nothing
    // is left behind but the lines that remain.
    if alone && is_deletion && source[end..].starts_with('\n') {
        end += 1;
    }

    let text = if is_deletion {
        String::new()
    }
    else if alone {
        let indent = leading_whitespace(&source[ls..]);
        m.replacement
            .iter()
            .map(|line| format!("{indent}{line}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
    else if m.replacement.iter().any(|l| is_comment_line(l)) {
        // A comment runs to end of line, so these cannot be strung together
        // with `:` at all - whichever entry follows a comment would be
        // swallowed by it. Give each its own line instead.
        //
        // Found by the upstream corpus: a preserved region beginning with the
        // comment that had trailed the matched instruction rendered as
        // `; should be removed : ld a,(bc) : ld (val),a`, quietly commenting
        // out both instructions the rule had promised to keep.
        let indent = leading_whitespace(&source[ls..]);
        m.replacement
            .iter()
            .map(|line| format!("{indent}{line}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
    else {
        // Mid-line: stay on the line, using basm's own separator, because the
        // instructions around this one are still part of it.
        m.replacement.join(" : ")
    };

    // A comment swallows the rest of its line, so anything still to come has
    // to start a new one.
    let text = if m.replacement.last().is_some_and(|l| is_comment_line(l))
        && !source[end..line_end(source, end)].trim().is_empty()
    {
        let indent = leading_whitespace(&source[ls..]).to_owned();
        format!("{text}\n{indent}")
    }
    else {
        text
    };

    Some(SourceEdit {
        range: start..end,
        text
    })
}

/// Whether a rendered replacement entry is a comment - i.e. something that
/// claims the rest of its line.
fn is_comment_line(line: &str) -> bool {
    line.trim_start().starts_with(';')
}

/// Widen a deletion's range to take in the `:` separating it from the
/// instructions staying on the line.
///
/// Prefers the *following* separator. Falling back to the preceding one is only
/// safe when what precedes really is another instruction: `start: ld b,b` ends
/// in a `:` too, and absorbing that would delete the label. The token list
/// settles it, since a label is its own token.
fn absorb_separator<T>(
    source: &str,
    tokens: &[&T],
    match_start: usize,
    token_start: usize,
    token_end: usize,
    le: usize
) -> (usize, usize)
where T: ListingElement {
    let after = &source[token_end..le];
    let after_trimmed = after.trim_start_matches([' ', '\t']);
    if after_trimmed.starts_with(':') {
        let mut end = le - after_trimmed.len() + 1;
        while source[end..].starts_with([' ', '\t']) {
            end += 1;
        }
        return (token_start, end);
    }

    let ls = line_start(source, token_start);
    let before_trimmed = source[ls..token_start].trim_end_matches([' ', '\t']);
    let preceded_by_label = match_start
        .checked_sub(1)
        .and_then(|i| tokens.get(i))
        .is_some_and(|t| t.is_label());
    if before_trimmed.ends_with(':') && !preceded_by_label {
        let mut start = ls + before_trimmed.len() - 1;
        while start > ls && source[..start].ends_with([' ', '\t']) {
            start -= 1;
        }
        return (start, token_end);
    }

    (token_start, token_end)
}

/// Byte offset of the start of the line containing `offset`.
pub fn line_start(source: &str, offset: usize) -> usize {
    source[..offset].rfind('\n').map_or(0, |i| i + 1)
}

/// Byte offset of the end of the line containing `offset`, excluding its
/// newline.
pub fn line_end(source: &str, offset: usize) -> usize {
    source[offset..]
        .find('\n')
        .map_or(source.len(), |i| offset + i)
}

/// The leading spaces/tabs of the first line of `text`.
pub fn leading_whitespace(text: &str) -> &str {
    let line = &text[..line_end(text, 0)];
    let trimmed = line.trim_start_matches([' ', '\t']);
    &line[..line.len() - trimmed.len()]
}
