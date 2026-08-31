//! bndbuild rules living inside a `.asm` file's own comments.
//!
//! ```text
//! ; #!bndbuild
//! ; - tgt: demo.sna
//! ;   cmd: basm demo.asm -o demo.sna
//! ```
//!
//! A project small enough not to want a separate build file can keep its rules
//! beside the code they build. Both the language server (which offers them as
//! CodeLenses and runs them) and the debug adapter (which has to build one
//! before it can debug it) need to find these blocks, so the finding lives
//! here rather than in either - two scanners that could disagree about where a
//! block starts would be two answers to "which rules does this file have".
//!
//! Detection walks the already-tokenised `Token::Comment` nodes of a parsed
//! listing rather than re-lexing raw text: that reuses basm's own comment
//! recognition, gets real source positions for free, and cannot mistake a `;`
//! inside a string for the start of a comment.

use cpclib_asm::flatten::flatten_listing;
use cpclib_asm::parser::obtained::{LocatedListing, MayHaveSpan};
use cpclib_tokens::ListingElement;

const MARKER: &str = "#!bndbuild";

/// A `#!bndbuild`-marked run of consecutive `;`/`//` comment lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedBndbuildBlock {
    /// 0-based line of the `#!bndbuild` marker comment itself.
    pub marker_line: usize,
    /// 0-based line of the first YAML content line (== `marker_line + 1`).
    pub yaml_start_line: usize,
    /// One line per content line, `"\n"`-joined, comment prefix stripped -
    /// line-preserving by construction, which is what keeps mapping a
    /// block-relative line back to `yaml_start_line + n` a plain constant add.
    pub yaml_text: String,
    /// Per content line (same indexing as `yaml_text.lines()`), the
    /// outer-document column at which that line's dedented content begins -
    /// how much the `;`/`// ` prefix-stripping peeled off. Needed for
    /// bidirectional column translation in an editor; execution only needs the
    /// lines.
    pub content_start_cols: Vec<u32>
}

/// Every `#!bndbuild` block in `listing`.
///
/// Walks nested `MODULE`/`IF`/`REPEAT` bodies too, so a block is found
/// wherever it was written. Multiple independent blocks in one file are all
/// returned.
pub fn extract_embedded_blocks(listing: &LocatedListing) -> Vec<EmbeddedBndbuildBlock> {
    let mut blocks = Vec::new();
    // (marker_line, last_accepted_line, content lines so far, their columns)
    let mut current: Option<(usize, usize, Vec<&str>, Vec<u32>)> = None;

    for token in flatten_listing(listing.iter()) {
        if !token.is_comment() {
            finish_block(&mut blocks, current.take());
            continue;
        }

        let span = token.span();
        let raw: &str = span.as_ref();

        let (line_1based, col_1based) = span.relative_line_and_column();
        let line = line_1based - 1;
        let start_col = (col_1based - 1) as u32;
        let stripped = strip_comment_prefix(raw);
        let content_col = start_col + (raw.len() - stripped.len()) as u32;

        if let Some((_marker_line, last_line, content, cols)) = current.as_mut() {
            if line == *last_line + 1 {
                content.push(stripped);
                cols.push(content_col);
                *last_line = line;
                continue;
            }
            // Non-consecutive: the current block is done. Fall through so this
            // same comment token is still checked as a possible new block start
            // below.
            finish_block(&mut blocks, current.take());
        }

        if is_marker(stripped) {
            current = Some((line, line, Vec::new(), Vec::new()));
        }
    }
    finish_block(&mut blocks, current.take());

    blocks
}

/// Parse `text` as a `.asm` file and return its blocks.
///
/// For callers that have a path rather than a listing. A file that does not
/// parse has no blocks - which is the right answer here: a build rule cannot be
/// recovered from a file basm itself cannot read.
pub fn blocks_in_source(text: &str) -> Vec<EmbeddedBndbuildBlock> {
    match cpclib_asm::parser::parse_z80_str(text) {
        Ok(listing) => extract_embedded_blocks(&listing),
        Err(_) => Vec::new()
    }
}

fn finish_block(
    blocks: &mut Vec<EmbeddedBndbuildBlock>,
    current: Option<(usize, usize, Vec<&str>, Vec<u32>)>
) {
    if let Some((marker_line, _last_line, content, cols)) = current
        && !content.is_empty()
    {
        blocks.push(EmbeddedBndbuildBlock {
            marker_line,
            yaml_start_line: marker_line + 1,
            yaml_text: content.join("\n"),
            content_start_cols: cols
        });
    }
}

fn is_marker(stripped: &str) -> bool {
    stripped.trim_end().split_whitespace().next() == Some(MARKER)
}

/// Strips a leading `;`/`//` comment prefix and at most one following space.
///
/// `raw` is a real `Token::Comment` span's own text - always one of the two
/// prefixes, possibly after leading whitespace the parser left in the span.
fn strip_comment_prefix(raw: &str) -> &str {
    let trimmed = raw.trim_start();
    let rest = trimmed
        .strip_prefix(';')
        .or_else(|| trimmed.strip_prefix("//"))
        .unwrap_or(trimmed);
    rest.strip_prefix(' ').unwrap_or(rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_marked_comment_run_becomes_a_block() {
        let blocks = blocks_in_source(
            "; #!bndbuild\n; - tgt: demo.sna\n;   cmd: basm demo.asm\n  org 0x4000\n"
        );
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        assert_eq!(blocks[0].marker_line, 0);
        assert_eq!(blocks[0].yaml_start_line, 1);
        assert_eq!(blocks[0].yaml_text, "- tgt: demo.sna\n  cmd: basm demo.asm");
    }

    /// The run ends where the comments do: code after it is not YAML.
    #[test]
    fn a_block_stops_at_the_first_non_comment() {
        let blocks = blocks_in_source("; #!bndbuild\n; - tgt: a\n  nop\n; - tgt: b\n");
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        assert_eq!(blocks[0].yaml_text, "- tgt: a");
    }

    #[test]
    fn a_file_with_no_marker_has_no_blocks() {
        assert!(blocks_in_source("; just a comment\n  nop\n").is_empty());
    }

    /// `//` is a comment in basm too, and the prefix comes off the same way.
    #[test]
    fn slash_comments_work_as_well() {
        let blocks = blocks_in_source("// #!bndbuild\n// - tgt: a\n");
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        assert_eq!(blocks[0].yaml_text, "- tgt: a");
    }

    #[test]
    fn two_blocks_are_both_found() {
        let blocks =
            blocks_in_source("; #!bndbuild\n; - tgt: a\n  nop\n; #!bndbuild\n; - tgt: b\n");
        assert_eq!(blocks.len(), 2, "{blocks:?}");
        assert_eq!(blocks[1].yaml_text, "- tgt: b");
    }
}
