use cpclib_asm::{AssemblerError, LocatedListing, MayHaveSpan, parse_z80_str};

use crate::options::{
    AsmFormatOptions, BinaryEncoding, CaseStyle, HexEncoding, LabelPostfix, OctalEncoding,
    SpaceAroundColumn
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
    fn new(source: &'src str, opt: &AsmFormatOptions) -> Self {
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
            hexadecimal_case: opt.hexadecimal_case,
            hexadecimal_encoding: opt.hexadecimal_encoding,
            octal_encoding: opt.octal_encoding,
            binary_encoding: opt.binary_encoding,
            label_definition_postfix_with_column: opt.label_definition_postfix_with_column,
            current_line: 0,
            output: String::new(),
            function_nesting: 0,
            disabled_ranges: pragma::disabled_ranges(source),
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
    let mut fmt = Formatter::new(source, opt);
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
