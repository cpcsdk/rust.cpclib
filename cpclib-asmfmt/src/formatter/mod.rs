use cpclib_asm::{AssemblerError, LocatedListing, MayHaveSpan, parse_z80_str};

use crate::options::{
    AsmFormatOptions, BinaryEncoding, CaseStyle, HexEncoding, LabelPostfix, OctalEncoding,
    SpaceAroundColumn
};

mod case;
mod emit;
mod numeric;
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
    // Per-source-line segment cache (`:` splitting for one_instruction_per_line)
    pub(super) seg_line: usize, // which source line is currently cached (usize::MAX = none)
    pub(super) seg_idx: usize,  // next segment to consume
    pub(super) seg_items: Vec<String>, // content segments (before trailing `;` comment)
    pub(super) seg_trailing: Option<String>  // the trailing `;` comment of the whole source line
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
            seg_line: usize::MAX,
            seg_idx: 0,
            seg_items: Vec::new(),
            seg_trailing: None
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
    fmt.output
}

pub fn format(asm: &str, opt: &AsmFormatOptions) -> Result<String, AssemblerError> {
    let listing = parse_z80_str(asm)?;
    Ok(format_listing(&listing, asm, 1, opt))
}
