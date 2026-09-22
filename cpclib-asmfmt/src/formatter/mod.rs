use cpclib_asm::{AssemblerError, LocatedListing, MayHaveSpan, parse_z80_str};

use crate::options::{
    AsmFormatOptions, BinaryEncoding, CaseStyle, HexEncoding, LabelPostfix, OctalEncoding,
    QuoteStyle, SpaceAroundColumn
};

mod case;
mod emit;
mod numeric;
mod pragma;
mod splitting;
mod tokens;

pub(super) struct Formatter<'src> {
    pub(super) source_lines: Vec<&'src str>,
    pub(super) indent_size: usize,
    pub(super) comment_column: usize,
    pub(super) mnemonic_case: CaseStyle,
    pub(super) directive_case: CaseStyle,
    pub(super) register_case: CaseStyle,
    pub(super) one_instruction_per_line: bool,
    pub(super) space_around_column: SpaceAroundColumn,
    pub(super) space_around_assignment: SpaceAroundColumn,
    pub(super) space_around_comma: SpaceAroundColumn,
    pub(super) quote_style: QuoteStyle,
    pub(super) max_consecutive_blank_lines: Option<usize>,
    pub(super) hexadecimal_case: CaseStyle,
    pub(super) hexadecimal_encoding: HexEncoding,
    pub(super) octal_encoding: OctalEncoding,
    pub(super) binary_encoding: BinaryEncoding,
    pub(super) label_definition_postfix_with_column: LabelPostfix,
    pub(super) current_line: usize,
    pub(super) output: String,
    // How many FUNCTION bodies currently enclose the token being formatted
    // (>0 while inside one, however deeply nested inside further IF/REPEAT/...
    // wrapping within it - those don't open their own scope, see
    // `tokens::Formatter::assign_depth`'s own doc comment).
    pub(super) function_nesting: usize,
    // 0-based, inclusive `(start, end)` source-line ranges where formatting is
    // suppressed - between a `; fmt: off` line and its matching `; fmt: on`
    // (both marker lines included) - see `pragma`'s own doc comment.
    pub(super) disabled_ranges: Vec<(usize, usize)>,
    // Kept so a MACRO body (captured as raw text, not a nested token list,
    // since it is genuinely re-parsed fresh on every call) can be formatted
    // by recursing into `format_listing` with these same options, rather
    // than only ever copied through verbatim - see `tokens::format_macro_def`.
    pub(super) opt: AsmFormatOptions
}

impl<'src> Formatter<'src> {
    // Additional 0-based inclusive line ranges (`extra_disabled`, empty for
    // the ordinary whole-file path) are folded into `disabled_ranges`
    // alongside whatever `; fmt: off`/`on` already contributes - the
    // mechanism `format_range` uses to leave everything outside the caller's
    // requested range untouched, without needing a second, parallel "don't
    // touch this" concept of its own.
    fn new_with_extra_disabled(
        source: &'src str,
        opt: &AsmFormatOptions,
        extra_disabled: &[(usize, usize)]
    ) -> Self {
        let mut disabled_ranges = pragma::disabled_ranges(source);
        disabled_ranges.extend_from_slice(extra_disabled);
        Self {
            source_lines: source.lines().collect(),
            indent_size: opt.indent_size,
            comment_column: opt.comment_column,
            mnemonic_case: opt.mnemonic_case,
            directive_case: opt.directive_case,
            register_case: opt.register_case,
            one_instruction_per_line: opt.one_instruction_per_line,
            space_around_column: opt.space_around_column,
            space_around_assignment: opt.space_around_assignment,
            space_around_comma: opt.space_around_comma,
            quote_style: opt.quote_style,
            max_consecutive_blank_lines: opt.max_consecutive_blank_lines,
            hexadecimal_case: opt.hexadecimal_case,
            hexadecimal_encoding: opt.hexadecimal_encoding,
            octal_encoding: opt.octal_encoding,
            binary_encoding: opt.binary_encoding,
            label_definition_postfix_with_column: opt.label_definition_postfix_with_column,
            current_line: 0,
            output: String::new(),
            function_nesting: 0,
            disabled_ranges,
            opt: opt.clone()
        }
    }
}

pub fn format_listing(
    listing: &LocatedListing,
    source: &str,
    depth: usize,
    opt: &AsmFormatOptions
) -> String {
    format_listing_with_extra_disabled(listing, source, depth, opt, &[])
}

fn format_listing_with_extra_disabled(
    listing: &LocatedListing,
    source: &str,
    depth: usize,
    opt: &AsmFormatOptions,
    extra_disabled: &[(usize, usize)]
) -> String {
    let mut fmt = Formatter::new_with_extra_disabled(source, opt, extra_disabled);
    if let Some(first) = listing.iter().next() {
        let (line_1, _) = first.span().relative_line_and_column();
        fmt.current_line = line_1.saturating_sub(1);
    }
    fmt.format_tokens(listing, depth);
    // `format_tokens`/`emit_interstitial` only ever preserve a blank/comment
    // line by walking *up to* the next real token's own line - nothing calls
    // that walk again after the very last token, so a blank line or comment
    // trailing after it (with nothing following, all the way to the end of
    // `source`) would otherwise be silently dropped. Rare for a whole file
    // (most real sources don't end on a meaningful trailing blank with
    // nothing after it) but routine for a MACRO body reformatted through
    // this same function (`tokens::format_macro_def`) - a blank line right
    // before `ENDM` is exactly that shape, and no different in principle
    // from a mid-file one.
    fmt.emit_interstitial(fmt.source_lines.len());
    fmt.output
}

pub fn format(asm: &str, opt: &AsmFormatOptions) -> Result<String, Box<AssemblerError>> {
    let listing = parse_z80_str(asm)?;
    Ok(format_listing(&listing, asm, 1, opt))
}

// Format only 1-based, inclusive lines `[start_line, end_line]` of `asm`,
// leaving everything before and after untouched - reuses the exact
// `disabled_ranges`/`emit_verbatim_through` machinery `; fmt: off`/`on`
// already relies on (see `pragma`'s own doc comment), just driven by a
// caller-given range instead of inline markers. `end_line` past the end of
// the file is clamped rather than treated as an error.
pub fn format_range(
    asm: &str,
    opt: &AsmFormatOptions,
    start_line: usize,
    end_line: usize
) -> Result<String, Box<AssemblerError>> {
    let listing = parse_z80_str(asm)?;
    let total_lines = asm.lines().count();
    let last_line_0 = total_lines.saturating_sub(1);
    let start_0 = start_line.saturating_sub(1);
    let end_0 = end_line.saturating_sub(1).min(last_line_0);
    let mut extra_disabled = Vec::new();
    if start_0 > 0 {
        extra_disabled.push((0, start_0 - 1));
    }
    if end_0 < last_line_0 {
        extra_disabled.push((end_0 + 1, last_line_0));
    }
    Ok(format_listing_with_extra_disabled(&listing, asm, 1, opt, &extra_disabled))
}
