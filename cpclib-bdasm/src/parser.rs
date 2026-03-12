use std::borrow::Cow;
use std::collections::HashMap;
use cpclib_common::camino::Utf8PathBuf;
use cpclib_common::parse_value;
use cpclib_common::winnow::{
    Parser,
    combinator::{alt, preceded, separated},
    token::{take_till, take_while},
    ascii::{space0, space1, line_ending, Caseless},
    error::ErrMode,
};
use cpclib_common::winnow::error::{ContextError, ParseError};
use crate::{BdAsmError, Result, DataBlocString};
use crate::control_file::{ControlDirective, ControlFile};

/// Parse a u16 value supporting multiple bases (hex, decimal, binary, octal)
pub fn parse_u16_value(s: &str) -> std::result::Result<u16, String> {
    let bytes = s.as_bytes();
    let result: std::result::Result<u32, ParseError<_, ()>> = parse_value.parse(bytes);
    result
        .map(|v| v as u16)
        .map_err(|_| format!("Invalid numeric value: {}", s))
}

/// Parse a value that can be either a numeric value or a label name
/// Returns the resolved u16 value, looking up labels in the provided map
pub fn parse_value_or_label(s: &str, labels: &HashMap<u16, Cow<str>>) -> std::result::Result<u16, String> {
    // Try to parse as numeric value first
    if let Ok(value) = parse_u16_value(s) {
        return Ok(value);
    }
    
    // Otherwise, treat as label and look it up
    labels.iter()
        .find(|(_, name)| name.as_ref() == s)
        .map(|(addr, _)| *addr)
        .ok_or_else(|| format!("Label '{}' not found", s))
}

// Winnow parsers for data bloc specifications

/// Parse a value or label name (alphanumeric/hex/etc or identifier)
fn parse_value_or_label_string<'i>(input: &mut &'i [u8]) -> std::result::Result<&'i [u8], ErrMode<ContextError>> {
    use cpclib_common::winnow::Parser;
    use cpclib_common::winnow::token::take_while;
    
    // Match either a numeric value (with optional 0x prefix) or a label name
    take_while(1.., |c: u8| {
        c.is_ascii_alphanumeric() || c == b'_' || c == b'x' || c == b'X'
    }).parse_next(input)
}

/// Winnow parser for DataBlocString
pub fn parse_data_bloc_string<'i>(input: &mut &'i [u8]) -> std::result::Result<DataBlocString, ErrMode<ContextError>> {
    use cpclib_common::winnow::Parser;
    use cpclib_common::winnow::combinator::alt;
    use cpclib_common::winnow::token::literal;
    
    // Try START..=END syntax first (inclusive range)
    let inclusive_range = (
        parse_value_or_label_string,
        literal("..="),
        parse_value_or_label_string
    ).map(|(start, _, end)| {
        DataBlocString::InclusiveRange(
            String::from_utf8_lossy(start).to_string(),
            String::from_utf8_lossy(end).to_string()
        )
    });
    
    // Try START..END syntax (exclusive range)
    let exclusive_range = (
        parse_value_or_label_string,
        literal(".."),
        parse_value_or_label_string
    ).map(|(start, _, end)| {
        DataBlocString::Range(
            String::from_utf8_lossy(start).to_string(),
            String::from_utf8_lossy(end).to_string()
        )
    });
    
    // Try START-LENGTH syntax
    let sized = (
        parse_value_or_label_string,
        literal("-"),
        parse_value_or_label_string
    ).map(|(start, _, length)| {
        DataBlocString::Sized(
            String::from_utf8_lossy(start).to_string(),
            String::from_utf8_lossy(length).to_string()
        )
    });
    
    // Try in order: inclusive range, exclusive range, sized
    alt((inclusive_range, exclusive_range, sized)).parse_next(input)
}

// Winnow parsers for control file format

/// Parse a comment line (starts with ; or #)
fn comment_line<'i>(input: &mut &'i [u8]) -> std::result::Result<(), ErrMode<ContextError>> {
    (space0, alt((b';', b'#')), take_till(0.., |c| c == b'\n')).void().parse_next(input)
}

/// Parse empty line
fn empty_line<'i>(input: &mut &'i [u8]) -> std::result::Result<(), ErrMode<ContextError>> {
    space0.void().parse_next(input)
}

/// Parse a label name (alphanumeric + underscore)
fn label_name<'i>(input: &mut &'i [u8]) -> std::result::Result<&'i [u8], ErrMode<ContextError>> {
    take_while(1.., |c: u8| c.is_ascii_alphanumeric() || c == b'_').parse_next(input)
}

/// Parse origin directive: origin <address> | o <address>
fn origin_directive<'i>(input: &mut &'i [u8]) -> std::result::Result<ControlDirective, ErrMode<ContextError>> {
    alt((Caseless("origin"), Caseless("o"))).parse_next(input)?;
    space1.parse_next(input)?;
    let addr = parse_value::<_, ContextError>.parse_next(input)? as u16;
    Ok(ControlDirective::Origin(addr))
}

/// Parse skip directive: skip <count> | s <count>
fn skip_directive<'i>(input: &mut &'i [u8]) -> std::result::Result<ControlDirective, ErrMode<ContextError>> {
    alt((Caseless("skip"), Caseless("s"))).parse_next(input)?;
    space1.parse_next(input)?;
    let count = parse_value::<_, ContextError>.parse_next(input)? as usize;
    Ok(ControlDirective::Skip(count))
}

/// Parse data directive: data <spec>
fn data_directive<'i>(input: &mut &'i [u8]) -> std::result::Result<ControlDirective, ErrMode<ContextError>> {
    preceded(
        (alt((Caseless("data"), Caseless("d"))), space1),
        parse_data_bloc_string
    )
    .map(|spec| ControlDirective::DataBloc(spec))
    .parse_next(input)
}

/// Parse label directive: label <name>=<address> | l <name>=<address>
fn label_directive<'i>(input: &mut &'i [u8]) -> std::result::Result<ControlDirective, ErrMode<ContextError>> {
    alt((Caseless("label"), Caseless("l"))).parse_next(input)?;
    space1.parse_next(input)?;
    
    let name_bytes = label_name.parse_next(input)?;
    let name = String::from_utf8_lossy(name_bytes).to_string();
    
    b'='.parse_next(input)?;
    space0.parse_next(input)?;
    
    let address = parse_value::<_, ContextError>.parse_next(input)? as u16;
    
    Ok(ControlDirective::Label { name, address })
}

/// Parse cpcstring directive: cpcstring <spec> | cs <spec>
fn cpcstring_directive<'i>(input: &mut &'i [u8]) -> std::result::Result<ControlDirective, ErrMode<ContextError>> {
    preceded(
        (alt((Caseless("cpcstring"), Caseless("cs"))), space1),
        parse_data_bloc_string
    )
    .map(|spec| ControlDirective::CpcString(spec))
    .parse_next(input)
}

/// Parse any directive
fn directive<'i>(input: &mut &'i [u8]) -> std::result::Result<ControlDirective, ErrMode<ContextError>> {
    preceded(
        space0,
        alt((
            origin_directive,
            skip_directive,
            data_directive,
            label_directive,
            cpcstring_directive,
        ))
    )
    .parse_next(input)
}

/// Parse a single line (directive, comment, or empty)
fn control_line<'i>(input: &mut &'i [u8]) -> std::result::Result<Option<ControlDirective>, ErrMode<ContextError>> {
    alt((
        directive.map(Some),
        comment_line.map(|_| None),
        empty_line.map(|_| None),
    ))
    .parse_next(input)
}

/// Parse entire control file
pub fn parse_control_file<'i>(input: &mut &'i [u8]) -> std::result::Result<Vec<ControlDirective>, ErrMode<ContextError>> {
    let lines: Vec<Option<ControlDirective>> = separated(0.., control_line, line_ending).parse_next(input)?;
    Ok(lines.into_iter().flatten().collect())
}

/// Load control file from disk
pub fn load_control_file(path: &Utf8PathBuf) -> Result<ControlFile> {
    let contents = std::fs::read_to_string(path)?;
    let bytes = contents.as_bytes();
    let mut input: &[u8] = bytes;
    
    let directives = parse_control_file(&mut input)
        .map_err(|e| BdAsmError::ControlFile(
            format!("Failed to parse control file '{}': {}", path, e)
        ))?;
    
    Ok(ControlFile { directives })
}
