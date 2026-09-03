//! Lexical data for Z80/basm assembly: instruction/directive/register sets,
//! semantic-token indices, the tokens legend, and word-extraction helpers.

use std::collections::HashSet;
use std::sync::LazyLock;

use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;

// Semantic token type indices — must match `semantic_tokens_legend()` order
pub(super) const TT_KEYWORD: u32 = 0; // Z80 instructions
pub(super) const TT_MACRO: u32 = 1; // assembler directives (EQU, DEFB, MACRO…)
pub(super) const TT_FUNCTION: u32 = 2; // macro invocation names
pub(super) const TT_NAMESPACE: u32 = 3; // module names
pub(super) const TT_VARIABLE: u32 = 4; // registers / condition codes
pub(super) const TT_NUMBER: u32 = 5; // numeric literals
pub(super) const TT_STRING: u32 = 6; // string literals
pub(super) const TT_COMMENT: u32 = 7; // line comments
pub(super) const TT_OPERATOR: u32 = 8; // operators
pub(super) const TT_ENUM_MEMBER: u32 = 9; // EQU / assign constants
pub(super) const TT_LABEL: u32 = 10; // jump / procedure labels
pub(super) const TT_PARAMETER: u32 = 11; // macro parameters {param}

pub(super) const MOD_DECLARATION: u32 = 1 << 0;
pub(super) const MOD_READONLY: u32 = 1 << 1;
/// A token inside an `IF`/`ELSEIF`/`ELSE` branch statically known (from a
/// dry-run assembly pass) not to be the one that actually assembles - maps
/// to the standard `deprecated` modifier (most themes render it with
/// strikethrough) rather than a custom modifier name, which most themes
/// silently ignore with no visible effect at all.
pub(super) const MOD_INACTIVE: u32 = 1 << 2;

/// One semantic token in absolute (not delta-encoded) document coordinates
/// - the shared accumulation shape every source (the ASM tokenizer itself,
/// LOCOMOTIVE-embedded BASIC, bndbuild-embedded rules) pushes into before
/// the final sort + delta-encode pass in `semantic_tokens.rs`.
pub(super) struct RawSemanticToken {
    pub(super) line: u32,
    pub(super) col: u32,
    pub(super) len: u32,
    pub(super) token_type: u32,
    pub(super) modifiers: u32
}

// Full Z80 register set + condition codes used as operands
pub(super) const REGISTER_LIST: &[&str] = &[
    "AF'", "AF", "BC", "DE", "HL", "IX", "IY", "SP", "PC", "IXH", "IXL", "IYH", "IYL", "A", "B",
    "C", "D", "E", "H", "L", "F", "I", "R", "NZ", "Z", "NC", "PE", "PO", "P", "M"
];

// Static lookup sets — built once, shared across all tokenizer calls
pub(super) static INSTRUCTION_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| cpclib_asm::lsp::Z80_INSTRUCTIONS.iter().copied().collect());

pub(super) static DIRECTIVE_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    for d in cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_STANDALONE {
        s.insert(*d);
    }
    for d in cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_START {
        s.insert(*d);
    }
    for d in cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_END {
        s.insert(*d);
    }
    s
});

pub(super) static REGISTER_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| REGISTER_LIST.iter().copied().collect());

/// Returns the SemanticTokensLegend that must be advertised in `initialize()`.
///
/// `cpclib-vscode/package.json`'s `contributes.semanticTokenScopes` pins most
/// of these types to the exact TextMate scope `syntaxes/z80-asm.tmLanguage.json`
/// already uses for the equivalent construct, so a semantic-token-capable
/// client renders identically to plain TextMate highlighting. One pair is
/// deliberately *not* kept visually distinct: the TextMate grammar has no
/// dedicated scope for an `EQU`/`=` constant's name — it's colored by the
/// same generic "bare identifier at column 0" rule as an ordinary label — so
/// `ENUM_MEMBER` (EQU/assign constants) is pinned to that same
/// `entity.name.function.label.z80` scope rather than left to invent a new
/// one. Giving constants their own distinct color would need a new,
/// more-specific TextMate pattern (matching only when the identifier is
/// followed by `EQU`/`=`) ahead of the generic label pattern in the
/// grammar — a real, if small, grammar change, not just a scope rename;
/// left for whenever that visual distinction is actually wanted rather than
/// bundled into a colors-should-match fix.
pub(crate) fn semantic_tokens_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::KEYWORD,     // 0  Z80 instructions
            SemanticTokenType::MACRO,       // 1  assembler directives
            SemanticTokenType::FUNCTION,    // 2  macro invocation names
            SemanticTokenType::NAMESPACE,   // 3  module names
            SemanticTokenType::VARIABLE,    // 4  registers / condition codes
            SemanticTokenType::NUMBER,      // 5  numeric literals
            SemanticTokenType::STRING,      // 6  string literals
            SemanticTokenType::COMMENT,     // 7  comments
            SemanticTokenType::OPERATOR,    // 8  operators
            SemanticTokenType::ENUM_MEMBER, // 9  EQU / assign constants
            SemanticTokenType::TYPE, // 10 jump / procedure labels (teal — avoids theme blue)
            SemanticTokenType::DECORATOR, // 11 macro parameters {param}
        ],
        token_modifiers: vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::READONLY,
            SemanticTokenModifier::DEPRECATED,
        ]
    }
}

/// The Z80/basm identifier word (and its `[start, end)` `char`-count column
/// range) at `column` in `line`, or `None` if `column` doesn't sit on one.
/// Z80/basm identifier characters: alphanumeric, `_`, `.`, `@` — the dot
/// allows `.local` labels and qualified names like `module.symbol`. The
/// single source of truth for this scan; `AssemblyAnalyzer::extract_word_at_position`
/// is a thin wrapper discarding the range for callers that don't need it.
pub(super) fn word_range_at_position(line: &str, column: usize) -> Option<(String, u32, u32)> {
    let chars: Vec<char> = line.chars().collect();
    if column >= chars.len() {
        return None;
    }
    let is_word = |c: char| c.is_alphanumeric() || c == '_' || c == '.' || c == '@';
    let mut start = column;
    let mut end = column;
    while start > 0 && is_word(chars[start - 1]) {
        start -= 1;
    }
    while end < chars.len() && is_word(chars[end]) {
        end += 1;
    }
    if start < end {
        Some((chars[start..end].iter().collect(), start as u32, end as u32))
    }
    else {
        None
    }
}

impl AssemblyAnalyzer {
    // Helper methods

    pub(super) fn extract_word_at_position(&self, line: &str, column: usize) -> Option<String> {
        word_range_at_position(line, column).map(|(word, ..)| word)
    }

    // ── Code actions ──────────────────────────────────────────────────────────
}

pub(super) fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'.' || b == b'@'
}

// ─── Listing flattening ────────────────────────────────────────────────────────

/// Recursively flatten a parsed listing, descending into every kind of
/// nested block (`IF`/`IFDEF`/`IFNDEF`, `MODULE`, `REPEAT`/`REPEAT...UNTIL`,
/// `WHILE`, `RORG`, `FOR`, `ITERATE`, `SWITCH`, `CONFINED`, crunched
/// sections, assembler-control blocks).
///
/// `listing.iter()` alone only sees the *top-level* statements — a file
/// entirely wrapped in an `ifndef GUARD ... endif` header guard (extremely
/// common in real-world sources, including several of basm's own
/// `inner://` resources, e.g. `ga.asm`) would otherwise expose exactly one
/// top-level `IF` token and none of the labels/constants/macros inside it.
/// Symbol lookups (completion, hover, goto-definition) should use this
/// instead of a shallow `.iter()` so they see everything, not just what
/// happens to sit outside every conditional/loop/module.
///
/// Thin re-export of `cpclib_asm::flatten::flatten_listing` under this
/// module's own established name - kept so the many existing call sites in
/// this crate don't all need their imports touched. The real implementation
/// lives in `cpclib-asm` now (promoted there so `cpclib-asmoptim`/
/// `cpclib-basmopt` can share it instead of each maintaining their own copy;
/// see that crate's `flatten.rs` for the actual traversal logic).
pub(super) use cpclib_asm::flatten::flatten_listing;

/// 0-based line of `token`'s own span.
pub(super) fn span_line<T: cpclib_asm::parser::obtained::MayHaveSpan>(token: &T) -> u32 {
    let (line_1based, _col) = token.span().relative_line_and_column();
    line_1based.saturating_sub(1) as u32
}

/// Every 0-based line `token`'s span covers, not only the line it starts on.
///
/// A one-instruction token is one line and this is `span_line`. A block
/// directive is not: an `ENUM ... ENDENUM`, a `MACRO ... ENDM` or a nested
/// `IF` is a single token whose span really does run from its opening keyword
/// to its closing one, so anything reasoning about "which lines does this
/// token occupy" (dimming an inactive branch, say) has to walk the whole
/// extent or it greys the first line and leaves the body looking live.
///
/// The trailing newline is trimmed first: some parsers consume the line ending
/// into the span, which would otherwise dim one line too many.
pub(super) fn span_lines<T: cpclib_asm::parser::obtained::MayHaveSpan>(
    token: &T
) -> std::ops::RangeInclusive<u32> {
    let start = span_line(token);
    let extra = {
        use cpclib_asm::SourceString;
        token.span().as_str().trim_end().matches('\n').count() as u32
    };
    start..=start + extra
}

/// `token`'s own span as a precise LSP `Range` (start and end line +
/// character) - generalizes `span_line` to the full span, not just its
/// start line. Assumes a single-line span (true for any one instruction
/// token) - end column is the start column plus the rendered text's own
/// byte length, since `LocatedToken`'s `Display` (and any other
/// `MayHaveSpan` token's) renders exactly its own span text, not "rest of
/// file from this point" (confirmed via `Z80Span::as_str` returning a
/// bounded fragment).
pub(super) fn token_lsp_range<T>(token: &T) -> Range
where T: cpclib_asm::parser::obtained::MayHaveSpan + std::fmt::Display {
    let (line_1based, col_1based) = token.span().relative_line_and_column();
    let line = line_1based.saturating_sub(1) as u32;
    let start_char = col_1based.saturating_sub(1) as u32;
    let end_char = start_char + token.to_string().len() as u32;
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

/// Every token in `listing` whose own [`token_lsp_range`] is *fully
/// contained* within `range` - character-accurate: a token only partially
/// covered (e.g. sitting right before the selection actually starts, on
/// the same line) is excluded entirely, not partially included. Descends
/// into every nested block via [`flatten_listing`], so a selection
/// entirely inside a `MODULE`/`IF`/`REPEAT`/etc. body is still found
/// correctly.
pub(super) fn tokens_in_range<'a, T>(
    listing: impl IntoIterator<Item = &'a T> + 'a,
    range: Range
) -> Vec<&'a T>
where
    T: cpclib_tokens::ListingElement
        + cpclib_asm::parser::obtained::MayHaveSpan
        + std::fmt::Display
        + 'a
{
    flatten_listing(listing)
        .filter(|t| {
            let r = token_lsp_range(*t);
            r.start >= range.start && r.end <= range.end
        })
        .collect()
}

/// Every token in `listing` whose own span line falls within
/// `[start_line, end_line]` (inclusive, 0-based) - whole-line granularity,
/// ignoring any character/column position within those lines. A thin
/// wrapper over [`tokens_in_range`] covering the entirety of both boundary
/// lines.
///
/// Currently only exercised by this module's own test (both `cycles.rs` and
/// `stabilize.rs` deliberately use character-precise [`tokens_in_range`]
/// instead - see their own doc comments), but kept and test-covered since
/// it documents a real, distinct behavior a future whole-line-selection
/// caller may need.
#[allow(dead_code)]
pub(super) fn tokens_in_lines<'a, T>(
    listing: impl IntoIterator<Item = &'a T> + 'a,
    start_line: u32,
    end_line: u32
) -> Vec<&'a T>
where
    T: cpclib_tokens::ListingElement
        + cpclib_asm::parser::obtained::MayHaveSpan
        + std::fmt::Display
        + 'a
{
    tokens_in_range(
        listing,
        Range {
            start: Position {
                line: start_line,
                character: 0
            },
            end: Position {
                line: end_line + 1,
                character: 0
            }
        }
    )
}

/// For a `.local`-shaped label at `line` (0-based) — its own definition, or
/// any reference within its scope — the name of the owning (non-dotted)
/// global label and that scope's line range (`start..end`, exclusive):
/// from the global's own definition line up to (but not including) the
/// next non-dotted global label's definition line, or to end-of-file
/// (`u32::MAX`). Returns `None` when `line` isn't within any global's
/// scope at all (e.g. before the first global label).
///
/// Used by rename to confine a local-label rename to its own global's
/// scope — a `.foo` under a *different* global is basm's own rules make
/// into a wholly different symbol, and must not be touched. Reimplements,
/// rather than shares, the `current_global`-tracking walk duplicated in
/// `symbols.rs`'s `document_symbols` and `autocomplete.rs`'s
/// `collect_symbols` — those build a display string per label as they go,
/// this only needs scope boundaries.
pub(super) fn label_scope_at_line<'a, T>(
    listing: impl IntoIterator<Item = &'a T> + 'a,
    line: u32
) -> Option<(String, std::ops::Range<u32>)>
where
    T: cpclib_asm::parser::obtained::MayHaveSpan + cpclib_tokens::ListingElement + 'a
{
    scope_containing(&global_label_scopes(listing), line)
}

/// Every global label's own `(name, start..end)` scope in `listing`, in
/// document order — the boundary computation `label_scope_at_line` does,
/// but for every label at once. Callers that need to resolve many lines to
/// their enclosing label (e.g. `call_hierarchy.rs::incoming_calls_in`, once
/// per matched call site) should compute this once via this function and
/// look up each line with `scope_containing`, rather than calling
/// `label_scope_at_line` (a full listing walk) per line — O(labels) to
/// build plus O(labels) per lookup, instead of O(tokens) per lookup.
pub(super) fn global_label_scopes<'a, T>(
    listing: impl IntoIterator<Item = &'a T> + 'a
) -> Vec<(String, std::ops::Range<u32>)>
where T: cpclib_asm::parser::obtained::MayHaveSpan + cpclib_tokens::ListingElement + 'a {
    let mut scopes = Vec::new();
    let mut current_global: Option<(String, u32)> = None;
    for token in flatten_listing(listing) {
        if !token.is_label() {
            continue;
        }
        let raw = token.label_symbol();
        if raw.starts_with('.') {
            continue;
        }
        let (tok_line_1based, _col) = token.span().relative_line_and_column();
        let tok_line = tok_line_1based.saturating_sub(1) as u32;
        if let Some((name, start)) = current_global.take() {
            scopes.push((name, start..tok_line));
        }
        current_global = Some((raw.to_string(), tok_line));
    }
    if let Some((name, start)) = current_global {
        scopes.push((name, start..u32::MAX));
    }
    scopes
}

/// The scope in `scopes` (as computed by `global_label_scopes`) containing
/// `line`, if any.
pub(super) fn scope_containing(
    scopes: &[(String, std::ops::Range<u32>)],
    line: u32
) -> Option<(String, std::ops::Range<u32>)> {
    scopes
        .iter()
        .find(|(_, range)| range.start <= line && line < range.end)
        .cloned()
}

// ─── Block-scope helpers (FUNCTION / REPEAT / ITERATE) ────────────────────

/// The line just past the end (exclusive) of a block starting at
/// `start_line` (0-based, the opening keyword's own line) — found by
/// pairing any of `open_words` against any of `close_words` textually,
/// tracking nesting depth so a block that itself contains a nested instance
/// of the same construct is handled correctly (mirrors
/// `embedded_basic::extract_locomotive_blocks`'s block-matching style).
/// Falls back to end-of-file if no matching close is found (e.g. a document
/// with a syntax error).
fn block_end_line(text: &str, start_line: u32, open_words: &[&str], close_words: &[&str]) -> u32 {
    let lines: Vec<&str> = text.lines().collect();
    let mut depth = 1i32;
    let mut i = start_line as usize + 1;
    while i < lines.len() {
        let upper = lines[i].trim().to_uppercase();
        let first_word = upper.split_whitespace().next().unwrap_or("");
        if open_words.contains(&first_word) {
            depth += 1;
        }
        else if close_words.contains(&first_word) {
            depth -= 1;
            if depth == 0 {
                // exclusive end — include the closing keyword's own line
                return clamp_to_last_addressable_line(text, i as u32 + 1);
            }
        }
        i += 1;
    }
    clamp_to_last_addressable_line(text, lines.len() as u32)
}

/// Clamps a computed "one past this line" position to the last line a real
/// editor's own line model can actually address.
///
/// `text.lines()` and an editor's line count only agree when `text` ends
/// with a trailing newline (both then treat everything up to and including
/// that final, empty line as addressable). When it doesn't, `text.lines()`
/// under-counts by one relative to the editor (there's no trailing empty
/// line to land on), so a caller computing "one past the last real line"
/// lands on a `Position` the editor considers out of bounds.
///
/// A real, user-reported bug: Sticky Scroll silently failed for the very
/// last symbol (a `MACRO`) in a file that didn't end with a trailing
/// newline, while the exact same symbol still showed up fine in the
/// Outline panel — which only needs the much narrower `selection_range`,
/// never this out-of-range end-of-body position.
pub(super) fn clamp_to_last_addressable_line(text: &str, line: u32) -> u32 {
    let editor_line_count = text.matches('\n').count() as u32 + 1;
    line.min(editor_line_count - 1)
}

/// As [`block_end_line`], specialized for `FUNCTION`'s single opening
/// keyword and its two closing aliases (`ENDFUNCTION`/`ENDF` - per the real
/// parser, `cpclib-asm/src/parser/directives.rs`'s `parse_function`; `FEND`
/// is a common mix-up but actually closes `FOR`, not `FUNCTION`).
///
/// `ENDF` is *also* one of `FOR`'s own closing aliases
/// (`ENDFOR`/`FEND`/`ENDF`) - a `FUNCTION` body containing a nested `FOR`
/// loop that itself closes with the bare `ENDF` spelling (rather than
/// `ENDFOR`/`FEND`) will be mismatched as ending the function early. Not
/// resolved here: doing so needs simultaneous depth-tracking of both
/// keyword sets, not just a wider close-word list.
pub(super) fn function_body_end_line(text: &str, start_line: u32) -> u32 {
    block_end_line(text, start_line, &["FUNCTION"], &["ENDFUNCTION", "ENDF"])
}

/// As [`block_end_line`], specialized for `REPEAT`'s (`REPEAT`/`REPT`/`REP`
/// opening; `ENDREPEAT`/`ENDREPT`/`ENDREP`/`ENDR`/`REND` closing) or
/// `ITERATE`'s (`ITERATE`/`ITER` opening; `ENDITERATE`/`ENDITER`/`ENDI`/
/// `IEND` closing) several keyword aliases — whichever `is_repeat` selects.
fn loop_body_end_line(text: &str, start_line: u32, is_repeat: bool) -> u32 {
    if is_repeat {
        block_end_line(
            text,
            start_line,
            &["REPEAT", "REPT", "REP"],
            &["ENDREPEAT", "ENDREPT", "ENDREP", "ENDR", "REND"]
        )
    }
    else {
        block_end_line(
            text,
            start_line,
            &["ITERATE", "ITER"],
            &["ENDITERATE", "ENDITER", "ENDI", "IEND"]
        )
    }
}

/// As [`block_end_line`], specialized for `MACRO`'s single opening keyword
/// and its three closing aliases (`ENDM`/`ENDMACRO`/`MEND`).
pub(super) fn macro_body_end_line(text: &str, start_line: u32) -> u32 {
    block_end_line(text, start_line, &["MACRO"], &["ENDM", "ENDMACRO", "MEND"])
}

/// As [`block_end_line`], specialized for `MODULE`'s single opening keyword
/// and its single closing keyword (`ENDMODULE` - confirmed the only one the
/// real parser accepts, `cpclib-asm/src/parser/directives.rs`'s
/// `parse_module`, unlike `FUNCTION`'s `FEND`/`ENDF` mix-up).
pub(super) fn module_body_end_line(text: &str, start_line: u32) -> u32 {
    block_end_line(text, start_line, &["MODULE"], &["ENDMODULE"])
}

/// The reverse of [`block_end_line`]: the line (0-based) of the *opening*
/// keyword matching a closing keyword's own line at `end_line` - scans
/// backward, tracking nesting depth the same way (a nested instance of the
/// same construct is handled correctly). `None` if no matching open is
/// found (e.g. a stray closing keyword with no opener, or a document with a
/// syntax error).
fn block_start_line(
    text: &str,
    end_line: u32,
    open_words: &[&str],
    close_words: &[&str]
) -> Option<u32> {
    let lines: Vec<&str> = text.lines().collect();
    let mut depth = 1i32;
    let mut i = end_line as i64 - 1;
    while i >= 0 {
        let upper = lines[i as usize].trim().to_uppercase();
        let first_word = upper.split_whitespace().next().unwrap_or("");
        if close_words.contains(&first_word) {
            depth += 1;
        }
        else if open_words.contains(&first_word) {
            depth -= 1;
            if depth == 0 {
                return Some(i as u32);
            }
        }
        i -= 1;
    }
    None
}

/// Open/close keyword pairs for every basm block directive this codebase
/// recognizes, taken directly from `cpclib-asm/build.rs`'s
/// `START_DIRECTIVE`/`END_DIRECTIVE` tables (the parser's own canonical
/// list) - shared by [`matching_opening_line`] below.
const BLOCK_KEYWORD_PAIRS: &[(&[&str], &[&str])] = &[
    (
        &[
            "IF", "IFDEF", "IFEXIST", "IFNDEF", "IFNOT", "IFUSED", "IFNUSED"
        ],
        &["ENDIF"]
    ),
    (&["MACRO"], &["ENDM", "ENDMACRO", "MEND"]),
    // KNOWN BUG, deliberately not fixed: `FEND` doesn't actually close
    // `FUNCTION` in the real parser (`cpclib-asm/src/parser/directives.rs`'s
    // `parse_function` only accepts `ENDFUNCTION`/`ENDF`) - it closes `FOR`
    // instead (see that pair below, which already correctly claims `ENDF`).
    // `function_body_end_line` (a separate, standalone helper - not this
    // table) was already fixed to use the correct `ENDFUNCTION`/`ENDF` pair
    // for the Sticky Scroll range fix. This table backs a *different*
    // feature (ctrl+click / inlay-hint navigation to a closing directive's
    // opener, `matching_opening_line`) and is left wrong on purpose: fixing
    // it naively (swapping `FEND` for `ENDF`) would create a *new*
    // regression, since `ENDF` is genuinely ambiguous between `FUNCTION` and
    // `FOR` - `FOR`'s own pair already correctly claims it, and this table's
    // first-match-by-array-order lookup means FUNCTION coming first would
    // steal it, breaking the currently-correct FOR/`ENDF` case. A real fix
    // needs simultaneous depth-tracking of both keyword sets, not a wider
    // close-word list here - not attempted.
    (&["FUNCTION"], &["ENDFUNCTION", "FEND"]),
    (&["REPEAT", "REPT"], &["ENDR", "ENDREP", "ENDREPEAT"]),
    (
        &["ITER", "ITERATE"],
        &["ENDI", "ENDITER", "ENDITERATE", "IEND"]
    ),
    (&["FOR"], &["ENDF", "ENDFOR"]),
    (&["MODULE"], &["ENDMODULE"]),
    (&["STRUCT"], &["ENDS"]),
    (&["SWITCH"], &["ENDSWITCH"]),
    (&["CONFINED"], &["ENDC", "ENDCONFINED"]),
    (&["ENUM"], &["ENDENUM"]),
    (&["WHILE"], &["ENDW", "WEND"]),
    (&["ASMCONTROLENV"], &["ENDA", "ENDASMCONTROLENV"])
];

/// `ELSE`/`ELSEIF`-family keywords - not real closing tokens (an `IF` can
/// have several, and the block continues after them), but ctrl+click should
/// still jump to the `IF` they belong to, the same as `ENDIF` does. Handled
/// as a special case in [`matching_opening_line`] rather than added to
/// [`BLOCK_KEYWORD_PAIRS`]'s own `IF` close-word list, since adding them
/// there would make `block_end_line`/`block_start_line`'s depth-tracking
/// treat an `ELSE` as if it actually closed the block (ending nesting
/// early) instead of just being a midpoint within it.
const IF_ELSE_WORDS: &[&str] = &[
    "ELSE",
    "ELSEIF",
    "ELSEIFDEF",
    "ELSEIFEXIST",
    "ELSEIFNDEF",
    "ELSEIFNOT",
    "ELSEIFUSED"
];

/// If `line` (0-based) starts with a known block-closing keyword
/// (`ENDIF`/`ENDM`/`ENDMACRO`/`MEND`/`ENDFUNCTION`/`ENDREPEAT`/.../
/// `ENDITERATE`/...) or an `ELSE`/`ELSEIF`-family keyword, the matching
/// opening keyword's own line - for ctrl+click/hover navigation from a
/// closing (or `ELSE`) directive back to what it belongs to. basm's AST has
/// no discrete closing token for these constructs (`Token::If`/`MACRO`/etc.
/// are each one nested node for the whole block, with `ENDIF`/`ENDM`/etc.
/// only implied by where the body ends), so this works from raw text
/// instead, mirroring `block_end_line`'s own already-established
/// text-based approach (used for MACRO/REPEAT/FUNCTION parameter renaming)
/// rather than inventing a second mechanism.
pub(super) fn matching_opening_line(text: &str, line: u32) -> Option<u32> {
    let lines: Vec<&str> = text.lines().collect();
    let line_text = lines.get(line as usize)?;
    let upper = line_text.trim().to_uppercase();
    let first_word = upper.split_whitespace().next().unwrap_or("");

    if IF_ELSE_WORDS.contains(&first_word) {
        let (if_open_words, if_close_words) = BLOCK_KEYWORD_PAIRS[0];
        return block_start_line(text, line, if_open_words, if_close_words);
    }

    for (open_words, close_words) in BLOCK_KEYWORD_PAIRS {
        if close_words.contains(&first_word) {
            return block_start_line(text, line, open_words, close_words);
        }
    }
    None
}

/// If `word_upper` (already uppercased) is a declared parameter of the
/// `MACRO` enclosing `line` (0-based), the macro's name, the parameter's
/// own spelling, and the macro body's line scope (`start` inclusive, `end`
/// exclusive — covering the `MACRO` line itself, since its parameter list
/// lives there). Unlike `FUNCTION`, a `MACRO` body is pure text
/// substitution with no restricted grammar of its own — any `EQU`/label
/// inside it becomes part of the real program at the *call* site once
/// expanded, not a locally-scoped symbol — so only declared parameters are
/// checked here, not body-defined symbols.
pub(super) fn macro_scoped_symbol_at<'a, T>(
    listing: impl IntoIterator<Item = &'a T> + 'a,
    text: &str,
    line: u32,
    word_upper: &str
) -> Option<(String, String, std::ops::Range<u32>)>
where
    T: cpclib_asm::parser::obtained::MayHaveSpan + cpclib_tokens::ListingElement + 'a
{
    for token in flatten_listing(listing) {
        if !token.is_macro_definition() {
            continue;
        }
        let (start_1based, _col) = token.span().relative_line_and_column();
        let start_line = start_1based.saturating_sub(1) as u32;
        let end_line = macro_body_end_line(text, start_line);
        if line < start_line || line >= end_line {
            continue;
        }
        for raw_param in token.macro_definition_arguments() {
            let param = raw_param.trim_start_matches('(').trim_end_matches(')');
            if param.to_uppercase() == word_upper {
                return Some((
                    token.macro_definition_name().to_string(),
                    param.to_string(),
                    start_line..end_line
                ));
            }
        }
    }
    None
}

/// Every `MACRO` body's line range in `listing`/`text` that shadows
/// `word_upper` (already uppercased) as one of its own declared
/// parameters — used to exclude a workspace-wide `Global` rename of that
/// name from reaching inside a macro that redefines it, mirroring
/// `all_function_shadow_ranges`/`all_loop_shadow_ranges`.
pub(super) fn all_macro_shadow_ranges<'a, T>(
    listing: impl IntoIterator<Item = &'a T> + 'a,
    text: &str,
    word_upper: &str
) -> Vec<std::ops::Range<u32>>
where
    T: cpclib_asm::parser::obtained::MayHaveSpan + cpclib_tokens::ListingElement + 'a
{
    let mut ranges = Vec::new();
    for token in flatten_listing(listing) {
        if !token.is_macro_definition() {
            continue;
        }
        let is_shadowed = token.macro_definition_arguments().iter().any(|raw_param| {
            raw_param
                .trim_start_matches('(')
                .trim_end_matches(')')
                .to_uppercase()
                == word_upper
        });
        if !is_shadowed {
            continue;
        }
        let (start_1based, _col) = token.span().relative_line_and_column();
        let start_line = start_1based.saturating_sub(1) as u32;
        let end_line = macro_body_end_line(text, start_line);
        ranges.push(start_line..end_line);
    }
    ranges
}

/// If `word_upper` (already uppercased) is the counter variable of the
/// `REPEAT`/`ITERATE` loop enclosing `line` (0-based) — REPEAT's counter is
/// optional (`REPEAT 5 ... ENDREPEAT` names none), ITERATE's is mandatory —
/// the loop's keyword (`"REPEAT"`/`"ITERATE"`, for diagnostics), the
/// counter's own spelling, and the loop body's line scope (`start`
/// inclusive, `end` exclusive — covering the opening line itself, since
/// that's where the counter is declared).
///
/// When loops nest and an outer and inner loop happen to share a counter
/// name, the *innermost* enclosing one wins (matching normal lexical
/// scoping): `flatten_listing` visits outer blocks before the inner blocks
/// nested in them, and both candidates' scopes contain `line` by
/// definition, so the last (innermost) match found is kept rather than
/// returning on the first hit.
pub(super) fn loop_scoped_symbol_at<'a, T>(
    listing: impl IntoIterator<Item = &'a T> + 'a,
    text: &str,
    line: u32,
    word_upper: &str
) -> Option<(String, String, std::ops::Range<u32>)>
where
    T: cpclib_asm::parser::obtained::MayHaveSpan + cpclib_tokens::ListingElement + 'a
{
    let mut result = None;
    for token in flatten_listing(listing) {
        let (keyword, counter_name, is_repeat) = if token.is_repeat() {
            (
                "REPEAT",
                token.repeat_counter_name().map(str::to_string),
                true
            )
        }
        else if token.is_iterate() {
            (
                "ITERATE",
                Some(token.iterate_counter_name().to_string()),
                false
            )
        }
        else {
            continue;
        };
        let Some(counter_name) = counter_name
        else {
            continue;
        };
        if counter_name.to_uppercase() != word_upper {
            continue;
        }

        let (start_1based, _col) = token.span().relative_line_and_column();
        let start_line = start_1based.saturating_sub(1) as u32;
        let end_line = loop_body_end_line(text, start_line, is_repeat);
        if line < start_line || line >= end_line {
            continue;
        }

        result = Some((keyword.to_string(), counter_name, start_line..end_line));
    }
    result
}

/// Every `REPEAT`/`ITERATE` loop body's line range in `listing`/`text` that
/// shadows `word_upper` (already uppercased) as its own counter variable —
/// used to exclude a workspace-wide `Global` rename of that name from
/// reaching inside a loop that redefines it, mirroring
/// `all_function_shadow_ranges`.
pub(super) fn all_loop_shadow_ranges<'a, T>(
    listing: impl IntoIterator<Item = &'a T> + 'a,
    text: &str,
    word_upper: &str
) -> Vec<std::ops::Range<u32>>
where
    T: cpclib_asm::parser::obtained::MayHaveSpan + cpclib_tokens::ListingElement + 'a
{
    let mut ranges = Vec::new();
    for token in flatten_listing(listing) {
        let (counter_name, is_repeat) = if token.is_repeat() {
            (token.repeat_counter_name().map(str::to_string), true)
        }
        else if token.is_iterate() {
            (Some(token.iterate_counter_name().to_string()), false)
        }
        else {
            continue;
        };
        let Some(counter_name) = counter_name
        else {
            continue;
        };
        if counter_name.to_uppercase() != word_upper {
            continue;
        }

        let (start_1based, _col) = token.span().relative_line_and_column();
        let start_line = start_1based.saturating_sub(1) as u32;
        let end_line = loop_body_end_line(text, start_line, is_repeat);
        ranges.push(start_line..end_line);
    }
    ranges
}

// ─── FUNCTION parameters ───────────────────────────────────────────────────

/// `true` when some line within `text[start..end)` (0-based, exclusive end)
/// defines `word_upper` (already uppercased) via `EQU` or bare `=` (not
/// `==`) — basm's `FUNCTION` bodies can't contain a genuine label
/// definition (`ParsingState::FunctionLimited` doesn't accept `Token::Label`,
/// only `Equ`/`Let`), so this is the only shape a function-local "variable"
/// definition takes. Mirrors `AssemblyAnalyzer::find_definition_by_text`'s
/// EQU/`=` detection.
fn symbol_defined_via_equ_or_assign_in(text: &str, start: u32, end: u32, word_upper: &str) -> bool {
    for (i, line) in text.lines().enumerate() {
        let i = i as u32;
        if i < start || i >= end {
            continue;
        }
        let trimmed = line.trim_start();
        let upper = trimmed.to_uppercase();
        let Some(rest) = upper.strip_prefix(word_upper)
        else {
            continue;
        };
        if rest.as_bytes().first().is_some_and(|&b| is_ident_byte(b)) {
            continue; // not a whole-word match (e.g. "yy" when looking for "y")
        }
        let rest_trimmed = rest.trim_start();
        let is_equ = rest_trimmed.starts_with("EQU")
            && !rest_trimmed
                .as_bytes()
                .get(3)
                .is_some_and(|&b| is_ident_byte(b));
        let is_assign = rest_trimmed.starts_with('=') && !rest_trimmed.starts_with("==");
        if is_equ || is_assign {
            return true;
        }
    }
    false
}

/// If `word_upper` (already uppercased) is scoped to the `FUNCTION`
/// enclosing `line` (0-based) — either one of its declared parameters, or a
/// symbol `EQU`/`=`-defined within its body (basm functions can't contain
/// genuine label definitions, only these — see
/// `symbol_defined_via_equ_or_assign_in`) — the function's name, `word`'s
/// own spelling, and the function body's line scope (`start` inclusive,
/// `end` exclusive — covering the `FUNCTION` line itself, since its
/// parameter list lives there). Any such symbol is local to the function:
/// a same-named definition outside it (even elsewhere in the same file) is
/// a different symbol and must not be touched by a rename confined to this
/// scope.
///
/// Declared parameters are matched with any enclosing `(`/`)` trimmed:
/// basm's `FUNCTION name(params)` grammar doesn't strip the parens itself —
/// a single-parameter `FUNCTION double(x)` stores its one declared
/// parameter as the raw text `"(x)"` — so `(x` / `x)` / `(x)` / `x` all
/// normalize to `x` here.
pub(super) fn function_scoped_symbol_at<'a, T>(
    listing: impl IntoIterator<Item = &'a T> + 'a,
    text: &str,
    line: u32,
    word_upper: &str
) -> Option<(String, String, std::ops::Range<u32>)>
where
    T: cpclib_asm::parser::obtained::MayHaveSpan + cpclib_tokens::ListingElement + 'a
{
    for token in flatten_listing(listing) {
        if !token.is_function_definition() {
            continue;
        }
        let (start_1based, _col) = token.span().relative_line_and_column();
        let start_line = start_1based.saturating_sub(1) as u32;
        let end_line = function_body_end_line(text, start_line);
        if line < start_line || line >= end_line {
            continue;
        }
        for raw_param in token.function_definition_params() {
            let param = raw_param.trim_start_matches('(').trim_end_matches(')');
            if param.to_uppercase() == word_upper {
                return Some((
                    token.function_definition_name().to_string(),
                    param.to_string(),
                    start_line..end_line
                ));
            }
        }
        if symbol_defined_via_equ_or_assign_in(text, start_line, end_line, word_upper) {
            return Some((
                token.function_definition_name().to_string(),
                word_upper.to_string(),
                start_line..end_line
            ));
        }
    }
    None
}

/// `true` when the `FUNCTION` token `token` (whose own body spans
/// `start_line..end_line`) shadows `word_upper` (already uppercased) —
/// declares it as one of its own parameters, or `EQU`/`=`-defines it
/// within its body. Shared by `function_scoped_symbol_at` (checking the
/// function enclosing the cursor) and `all_function_shadow_ranges`
/// (checking every function in a document, for excluding a workspace-wide
/// rename from reaching inside one that shadows the renamed name).
fn function_shadows<T: cpclib_tokens::ListingElement>(
    token: &T,
    text: &str,
    start_line: u32,
    end_line: u32,
    word_upper: &str
) -> bool {
    let is_param = token.function_definition_params().iter().any(|raw_param| {
        raw_param
            .trim_start_matches('(')
            .trim_end_matches(')')
            .to_uppercase()
            == word_upper
    });
    is_param || symbol_defined_via_equ_or_assign_in(text, start_line, end_line, word_upper)
}

/// Every `FUNCTION` body's line range in `listing`/`text` that shadows
/// `word_upper` (already uppercased) — used to exclude a workspace-wide
/// `Global` rename of that name from reaching inside a function that
/// redefines it as its own parameter or a local `EQU`/`=`-defined symbol
/// (see `RenameTarget::FunctionLocal`): inside such a function, the name is
/// a different symbol entirely, and a rename of the *outer* one must not
/// touch it.
pub(super) fn all_function_shadow_ranges<'a, T>(
    listing: impl IntoIterator<Item = &'a T> + 'a,
    text: &str,
    word_upper: &str
) -> Vec<std::ops::Range<u32>>
where
    T: cpclib_asm::parser::obtained::MayHaveSpan + cpclib_tokens::ListingElement + 'a
{
    let mut ranges = Vec::new();
    for token in flatten_listing(listing) {
        if !token.is_function_definition() {
            continue;
        }
        let (start_1based, _col) = token.span().relative_line_and_column();
        let start_line = start_1based.saturating_sub(1) as u32;
        let end_line = function_body_end_line(text, start_line);
        if function_shadows(token, text, start_line, end_line, word_upper) {
            ranges.push(start_line..end_line);
        }
    }
    ranges
}

// ─── RANGE / DEFSECTION (section definitions) ─────────────────────────────────

/// `true` when `token`'s own statement starts with the `RANGE` or
/// `DEFSECTION` keyword. `ListingElement` has no dedicated `is_range`/
/// `range_*` accessors, so extracting a section's name goes through the
/// (documented-costly, and only partially implemented) `to_token()`
/// conversion — this text-based pre-check, read from the token's own source
/// rather than any parsed representation, keeps that call off the hot path
/// of ordinary opcode/directive lines and away from `to_token()`'s
/// unimplemented (`todo!()`-panicking) fallback for other directive kinds.
pub(super) fn starts_with_range_keyword<T: cpclib_asm::parser::obtained::MayHaveSpan>(
    token: &T
) -> bool {
    let span_text: &str = token.span().as_ref();
    let first_line = span_text.lines().next().unwrap_or(span_text);
    let first_word: String = first_line
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    let upper = first_word.to_uppercase();
    upper == "RANGE" || upper == "DEFSECTION"
}

/// Best-effort `(line, column)` of `name` within `token`'s own statement —
/// used for `RANGE`/`DEFSECTION`, where the token's span points at the
/// keyword (`RANGE start, stop, name`), not at the name argument itself.
/// Falls back to the keyword's own position if `name` can't be found on the
/// statement's first line (shouldn't happen for a well-formed `Token::Range`
/// just extracted from this same token).
pub(super) fn locate_name_in_statement<T: cpclib_asm::parser::obtained::MayHaveSpan>(
    token: &T,
    name: &str
) -> (u32, u32) {
    let span = token.span();
    let span_text: &str = span.as_ref();
    let first_line = span_text.lines().next().unwrap_or(span_text);
    let (line_1based, col_1based) = span.relative_line_and_column();
    let lsp_line = line_1based.saturating_sub(1) as u32;
    let base_col = col_1based.saturating_sub(1) as u32;
    let col = first_line
        .find(name)
        .map(|byte_off| base_col + byte_off as u32)
        .unwrap_or(base_col);
    (lsp_line, col)
}

/// A numeral literal's radix — used to locate a 16-bit literal's high/low
/// byte sub-spans (2 hex digits or 8 binary digits per byte; decimal is
/// never byte-split, since decimal digit boundaries don't align to bytes).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum NumeralStyle {
    Hex,
    Decimal,
    Binary
}

impl NumeralStyle {
    pub(super) fn digits_per_byte(self) -> usize {
        match self {
            NumeralStyle::Hex => 2,
            NumeralStyle::Binary => 8,
            NumeralStyle::Decimal => 0
        }
    }
}

/// A numeral literal found while lexing, with enough detail to both
/// evaluate it and precisely place/reformat a color swatch: `token_start`/
/// `token_end` bound the whole literal (base prefix and suffix included,
/// e.g. all of `0x54` or `7F40h`); `prefix`/`suffix` are those captured
/// verbatim (whichever the source actually used — `0x`/`#`/`$`/`&`/`0b`/`%`,
/// or a trailing `h`/`b`) so an edit can preserve the exact same notation;
/// `digits_start`/`digits_end` bound the digit run alone, for locating a
/// 16-bit literal's byte midpoint.
pub(super) struct NumeralLiteral {
    pub(super) line: u32,
    pub(super) token_start: u32,
    pub(super) token_end: u32,
    pub(super) prefix: String,
    pub(super) suffix: String,
    pub(super) digits_start: u32,
    pub(super) digits_end: u32,
    pub(super) value: u32,
    pub(super) style: NumeralStyle
}

/// Lex every numeral literal in `text` (skipping `skip_lines` — LOCOMOTIVE
/// block lines, tokenised separately as BASIC), delegating the actual
/// lexing/parsing to `cpclib_common::parse::scan_numeric_literals` — the
/// same numeral grammar the real assembler uses (every prefix/suffix form:
/// `0x`/`0X`/`#`/`$`/`&` hex, `0b`/`0B`/`%` binary, trailing `h`/`b`
/// suffixes — a hand-rolled lexer here would drift from it). That function
/// already refuses to start a numeral right after an identifier character,
/// so a label like `STATE84` is never misread as ending in a literal `84`.
/// Only its own line-scoped double-quote-string skipping is built in, so
/// `;` comments are stripped first via `format::strip_asm_comment`.
pub(super) fn scan_numeral_literals(
    text: &str,
    skip_lines: &std::collections::HashSet<usize>
) -> Vec<NumeralLiteral> {
    let mut out = Vec::new();

    for (line_idx, line) in text.lines().enumerate() {
        if skip_lines.contains(&line_idx) {
            continue;
        }
        let code = super::format::strip_asm_comment(line);
        for (start, end, value, kind) in cpclib_common::parse::scan_numeric_literals(code) {
            let style = match kind {
                cpclib_common::parse::EncodingKind::Hex => NumeralStyle::Hex,
                cpclib_common::parse::EncodingKind::Bin => NumeralStyle::Binary,
                cpclib_common::parse::EncodingKind::Dec => NumeralStyle::Decimal,
                // Octal has no GA-byte meaning; the ambiguous/unknown
                // states are internal to the parser and never returned.
                _ => continue
            };
            let token_text = &code[start..end];
            let (prefix_len, suffix_len) = prefix_and_suffix_len(token_text, style);
            out.push(NumeralLiteral {
                line: line_idx as u32,
                token_start: start as u32,
                token_end: end as u32,
                prefix: token_text[..prefix_len].to_string(),
                suffix: token_text[token_text.len() - suffix_len..].to_string(),
                digits_start: (start + prefix_len) as u32,
                digits_end: (end - suffix_len) as u32,
                value,
                style
            });
        }
    }

    out
}

/// How many of a numeral token's leading/trailing characters are its base
/// prefix/suffix (as opposed to digits) — e.g. `("0x", 2, 0)`, `("$", 1,
/// 0)`, `("7F40h", 0, 1)`, `("84", 0, 0)` for a bare decimal.
fn prefix_and_suffix_len(text: &str, style: NumeralStyle) -> (usize, usize) {
    match style {
        NumeralStyle::Hex => {
            if text.len() >= 2 && matches!(&text[..2], "0x" | "0X") {
                (2, 0)
            }
            else if text.starts_with(['#', '$', '&']) {
                (1, 0)
            }
            else if text.ends_with(['h', 'H']) {
                (0, 1)
            }
            else {
                (0, 0)
            }
        },
        NumeralStyle::Binary => {
            if text.len() >= 2 && matches!(&text[..2], "0b" | "0B") {
                (2, 0)
            }
            else if text.starts_with('%') {
                (1, 0)
            }
            else if text.ends_with(['b', 'B']) {
                (0, 1)
            }
            else {
                (0, 0)
            }
        },
        NumeralStyle::Decimal => (0, 0)
    }
}

// ─── Generated directive documentation ────────────────────────────────────────

// `DIRECTIVE_DOCS` + `DIRECTIVE_FILE_ARGS`, generated from docs/basm/directives.md.
include!(concat!(env!("OUT_DIR"), "/directive_docs_generated.rs"));

/// Look up a directive by name (case-insensitive) and return a one-line
/// summary of its documentation, for use as a completion item's `.detail`.
pub(super) fn directive_first_doc_line(word_upper: &str) -> Option<String> {
    DIRECTIVE_DOCS
        .iter()
        .find(|(names, _)| names.iter().any(|n| n.to_uppercase() == word_upper))
        .map(|(_, doc)| crate::common::render::first_doc_line(doc))
}

#[cfg(test)]
mod global_label_scope_tests {
    use super::*;
    use crate::common::document::Document;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///t.asm").unwrap(), text.to_string(), 1)
    }

    #[test]
    fn global_label_scopes_covers_every_label_in_order() {
        let text = "start:\n  call target\n  ret\ntarget:\n  ret\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let listing = analyzer.parse_document(&d).unwrap();
        let scopes = global_label_scopes(listing.iter());
        assert_eq!(scopes.len(), 2, "{scopes:?}");
        assert_eq!(scopes[0].0, "start");
        assert_eq!(scopes[0].1, 0..3);
        assert_eq!(scopes[1].0, "target");
        assert_eq!(scopes[1].1, 3..u32::MAX);
    }

    #[test]
    fn scope_containing_matches_label_scope_at_line_for_every_line() {
        let text = "start:\n  call target\n  ret\ntarget:\n  ret\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let listing = analyzer.parse_document(&d).unwrap();
        let scopes = global_label_scopes(listing.iter());
        for line in 0..5u32 {
            assert_eq!(
                scope_containing(&scopes, line),
                label_scope_at_line(listing.iter(), line),
                "mismatch at line {line}"
            );
        }
    }

    #[test]
    fn scope_containing_returns_none_before_the_first_label() {
        let text = "  nop\nstart:\n  ret\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let listing = analyzer.parse_document(&d).unwrap();
        let scopes = global_label_scopes(listing.iter());
        assert_eq!(scope_containing(&scopes, 0), None);
    }
}

#[cfg(test)]
mod token_selection_tests {
    use cpclib_asm::parser::obtained::LocatedToken;
    use cpclib_tokens::ListingElement;

    use super::*;
    use crate::common::document::Document;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///t.asm").unwrap(), text.to_string(), 1)
    }

    /// `tokens_in_lines` includes everything on a boundary line regardless
    /// of column - `tokens_in_range`, given the exact same lines expressed
    /// as a precise `Range`, must behave identically when that `Range`
    /// covers the boundary lines in full.
    #[test]
    fn tokens_in_lines_and_a_matching_full_range_agree() {
        let text = "start:\n  ld a,b\n  nop\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let listing = analyzer.parse_document(&d).unwrap();

        let by_lines = tokens_in_lines(listing.iter(), 0, 2);
        let by_range: Vec<&LocatedToken> = tokens_in_range(
            listing.iter(),
            Range {
                start: Position {
                    line: 0,
                    character: 0
                },
                end: Position {
                    line: 3,
                    character: 0
                }
            }
        );
        assert_eq!(by_lines.len(), by_range.len());
        assert_eq!(by_lines.len(), 3, "{by_lines:?}"); // start:, ld a,b, nop
    }

    /// The real behavior difference from `tokens_in_lines`: a token that
    /// only partially overlaps the selection's start (on the same line) is
    /// excluded entirely, not partially included.
    #[test]
    fn tokens_in_range_excludes_a_token_only_partially_covered_at_the_boundary() {
        let text = "junk: ld a,b\n  nop\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let listing = analyzer.parse_document(&d).unwrap();

        // Start right at "ld a,b" (column 6), excluding "junk:" even
        // though it's on the very first (included) line.
        let tokens: Vec<&LocatedToken> = tokens_in_range(
            listing.iter(),
            Range {
                start: Position {
                    line: 0,
                    character: 6
                },
                end: Position {
                    line: 2,
                    character: 0
                }
            }
        );
        assert!(
            tokens.iter().all(|t| !t.is_label()),
            "the partially-covered \"junk:\" label leaked in: {tokens:?}"
        );
        assert_eq!(tokens.len(), 2, "{tokens:?}"); // ld a,b, nop
    }
}
