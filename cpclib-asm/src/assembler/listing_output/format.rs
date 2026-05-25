use std::collections::HashSet;

use cpclib_common::itertools::Itertools;

pub const MAX_RENDERED_SOURCE_COLUMN_CHARS: usize = 80;
pub const DEFAULT_LISTING_LINE_TEMPLATE: &str = "{A} {P} {C} {L4} {S}";

#[derive(Clone, Debug)]
pub enum ListingAddressRadix {
    Hex,
    Dec
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListingSourceFileOutputMode {
    None,
    Header,
    FileMap
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListingOutputKind {
    Text,
    Html
}

#[derive(Clone, Debug)]
pub struct ListingOutputFormat {
    pub bytes_per_line: usize,
    pub show_physical_address: bool,
    pub uppercase_hex: bool,
    pub address_radix: ListingAddressRadix,
    pub logical_address_width: usize,
    pub physical_address_width: usize,
    pub line_number_width: usize,
    pub show_line_numbers: bool,
    pub show_context_header: bool,
    pub listing_line_template: String,
    pub source_file_output_mode: ListingSourceFileOutputMode,
    pub output_kind: ListingOutputKind
}

impl Default for ListingOutputFormat {
    fn default() -> Self {
        Self {
            bytes_per_line: 8,
            show_physical_address: true,
            uppercase_hex: true,
            address_radix: ListingAddressRadix::Hex,
            logical_address_width: 4,
            physical_address_width: 5,
            line_number_width: 4,
            show_line_numbers: true,
            show_context_header: true,
            listing_line_template: DEFAULT_LISTING_LINE_TEMPLATE.to_string(),
            source_file_output_mode: ListingSourceFileOutputMode::Header,
            output_kind: ListingOutputKind::Text
        }
    }
}

fn logical_address_width_impl(format: &ListingOutputFormat) -> usize {
    format.logical_address_width.max(1)
}

fn physical_address_width_impl(format: &ListingOutputFormat) -> usize {
    format.physical_address_width.max(1)
}

fn physical_field_width_impl(format: &ListingOutputFormat) -> usize {
    physical_address_width_impl(format) + 1
}

fn format_address_for_impl(format: &ListingOutputFormat, value: u32, width: usize) -> String {
    match format.address_radix {
        ListingAddressRadix::Hex => {
            if format.uppercase_hex {
                format!("{:0width$X}", value, width = width)
            }
            else {
                format!("{:0width$x}", value, width = width)
            }
        },
        ListingAddressRadix::Dec => format!("{:0width$}", value, width = width)
    }
}

fn blank_impl(width: usize) -> String {
    " ".repeat(width)
}

fn hex_byte_for_impl(format: &ListingOutputFormat, b: u8) -> String {
    if format.uppercase_hex {
        format!("{b:02X}")
    }
    else {
        format!("{b:02x}")
    }
}

fn format_bytes_raw_for_impl(format: &ListingOutputFormat, bytes: &[u8]) -> String {
    bytes.iter().map(|b| hex_byte_for_impl(format, *b)).join(" ")
}

fn format_bytes_for_impl(format: &ListingOutputFormat, bytes_per_line: usize, bytes: &[u8]) -> String {
    let rendered = format_bytes_raw_for_impl(format, bytes);
    let full_width = bytes_per_line * 3;
    format!("{rendered:<full_width$}")
}

fn render_source_column_impl(line: Option<&str>) -> String {
    line.map(|line| {
        line.trim_start()
            .chars()
            .take(MAX_RENDERED_SOURCE_COLUMN_CHARS)
            .collect()
    })
    .unwrap_or_default()
}

fn format_line_with_template_for_impl(
    format: &ListingOutputFormat,
    bytes_per_line: usize,
    file_index: usize,
    logical_address: Option<u32>,
    physical_address_repr: &str,
    bytes: &[u8],
    line_number: Option<u32>,
    source_line_raw: Option<&str>,
    source_line_expanded: Option<&str>
) -> String {
    let template = format.listing_line_template.as_str();
    let mut output = String::with_capacity(template.len() + 32);
    let mut consumed = HashSet::<String>::new();
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '{' {
            output.push(ch);
            continue;
        }

        let mut placeholder = String::new();
        while let Some(&next) = chars.peek() {
            chars.next();
            if next == '}' {
                break;
            }
            placeholder.push(next);
        }

        if placeholder.is_empty() {
            output.push('{');
            output.push('}');
            continue;
        }

        if !consumed.insert(placeholder.clone()) {
            continue;
        }

        let rendered = match placeholder.as_str() {
            "A" => logical_address
                .map(|value| format_address_for_impl(format, value, logical_address_width_impl(format)))
                .unwrap_or_else(|| blank_impl(logical_address_width_impl(format))),
            "P" => {
                if format.show_physical_address {
                    physical_address_repr.to_string()
                }
                else {
                    String::new()
                }
            },
            "C" => format_bytes_for_impl(format, bytes_per_line, bytes),
            "CX" => format_bytes_for_impl(format, bytes_per_line, bytes),
            "F" => file_index.to_string(),
            "F2" => format!("{:>2}", file_index),
            "F3" => format!("{:>3}", file_index),
            "L" => {
                if format.show_line_numbers {
                    line_number.map(|value| value.to_string()).unwrap_or_default()
                }
                else {
                    String::new()
                }
            },
            "L3" => {
                if format.show_line_numbers {
                    line_number.map(|value| format!("{:>3}", value)).unwrap_or_default()
                }
                else {
                    String::new()
                }
            },
            "L4" => {
                if format.show_line_numbers {
                    line_number.map(|value| format!("{:>4}", value)).unwrap_or_default()
                }
                else {
                    String::new()
                }
            },
            "L5" => {
                if format.show_line_numbers {
                    line_number.map(|value| format!("{:>5}", value)).unwrap_or_default()
                }
                else {
                    String::new()
                }
            },
            "S" => render_source_column_impl(source_line_expanded),
            "SR" => render_source_column_impl(source_line_raw),
            _ => format!("{{{placeholder}}}")
        };

        output.push_str(&rendered);
    }

    output
}

fn format_deferred_line_with_template_for_impl(
    format: &ListingOutputFormat,
    bytes_per_line: usize,
    file_index: usize,
    specific_content: &str,
    line_number: Option<u32>,
    source_line_raw: &str,
    source_line_expanded: &str
) -> Vec<String> {
    let source_marker_raw = "\u{1f}SOURCE_RAW\u{1f}";
    let source_marker_expanded = "\u{1f}SOURCE_EXPANDED\u{1f}";
    let rendered = format_line_with_template_for(
        format,
        bytes_per_line,
        file_index,
        None,
        &blank_impl(physical_field_width_impl(format)),
        &[],
        line_number,
        Some(source_marker_raw),
        Some(source_marker_expanded)
    );

    let anchor = if format.show_line_numbers {
        line_number
            .and_then(|value| rendered.find(&value.to_string()))
            .or_else(|| rendered.find(source_marker_expanded))
            .or_else(|| rendered.find(source_marker_raw))
    }
    else {
        rendered
            .find(source_marker_expanded)
            .or_else(|| rendered.find(source_marker_raw))
    };

    let rendered = rendered
        .replace(source_marker_raw, source_line_raw.trim_start())
        .replace(source_marker_expanded, source_line_expanded.trim_start());

    match anchor {
        Some(anchor) => {
            let suffix = &rendered[anchor..];
            if !specific_content.is_empty() && specific_content.len() > anchor {
                let continuation_prefix = anchor.saturating_sub(1);
                vec![
                    specific_content.to_string(),
                    format!(">{:continuation_prefix$}{suffix}", "")
                ]
            }
            else {
                vec![format!("{specific_content:<anchor$}{suffix}")]
            }
        },
        None if rendered.trim().is_empty() => vec![specific_content.to_string()],
        None if specific_content.is_empty() => vec![rendered],
        None => vec![format!("{specific_content} {rendered}")]
    }
}

pub(crate) fn logical_address_width(format: &ListingOutputFormat) -> usize {
    logical_address_width_impl(format)
}

pub(crate) fn physical_address_width(format: &ListingOutputFormat) -> usize {
    physical_address_width_impl(format)
}

pub(crate) fn physical_field_width(format: &ListingOutputFormat) -> usize {
    physical_field_width_impl(format)
}

pub(crate) fn format_address(format: &ListingOutputFormat, value: u32, width: usize) -> String {
    format_address_for_impl(format, value, width)
}

pub(crate) fn format_address_for(format: &ListingOutputFormat, value: u32, width: usize) -> String {
    format_address_for_impl(format, value, width)
}

pub(crate) fn blank(width: usize) -> String {
    blank_impl(width)
}

pub(crate) fn hex_byte(format: &ListingOutputFormat, b: u8) -> String {
    hex_byte_for_impl(format, b)
}

pub(crate) fn hex_byte_for(format: &ListingOutputFormat, b: u8) -> String {
    hex_byte_for_impl(format, b)
}

pub(crate) fn format_bytes_raw(format: &ListingOutputFormat, bytes: &[u8]) -> String {
    format_bytes_raw_for_impl(format, bytes)
}

pub(crate) fn format_bytes_raw_for(format: &ListingOutputFormat, bytes: &[u8]) -> String {
    format_bytes_raw_for_impl(format, bytes)
}

pub(crate) fn format_bytes(format: &ListingOutputFormat, bytes_per_line: usize, bytes: &[u8]) -> String {
    format_bytes_for_impl(format, bytes_per_line, bytes)
}

pub(crate) fn format_bytes_for(format: &ListingOutputFormat, bytes_per_line: usize, bytes: &[u8]) -> String {
    format_bytes_for_impl(format, bytes_per_line, bytes)
}

pub(crate) fn render_source_column_for(line: Option<&str>) -> String {
    render_source_column_impl(line)
}

pub(crate) fn render_source_column(line: Option<&str>) -> String {
    render_source_column_impl(line)
}

pub(crate) fn format_line_with_template(
    format: &ListingOutputFormat,
    bytes_per_line: usize,
    file_index: usize,
    logical_address: Option<u32>,
    physical_address_repr: &str,
    bytes: &[u8],
    line_number: Option<u32>,
    source_line_raw: Option<&str>,
    source_line_expanded: Option<&str>
) -> String {
    format_line_with_template_for_impl(
        format,
        bytes_per_line,
        file_index,
        logical_address,
        physical_address_repr,
        bytes,
        line_number,
        source_line_raw,
        source_line_expanded
    )
}

pub(crate) fn format_line_with_template_for(
    format: &ListingOutputFormat,
    bytes_per_line: usize,
    file_index: usize,
    logical_address: Option<u32>,
    physical_address_repr: &str,
    bytes: &[u8],
    line_number: Option<u32>,
    source_line_raw: Option<&str>,
    source_line_expanded: Option<&str>
) -> String {
    format_line_with_template_for_impl(
        format,
        bytes_per_line,
        file_index,
        logical_address,
        physical_address_repr,
        bytes,
        line_number,
        source_line_raw,
        source_line_expanded
    )
}

pub(crate) fn format_deferred_line_with_template(
    format: &ListingOutputFormat,
    bytes_per_line: usize,
    file_index: usize,
    specific_content: &str,
    line_number: Option<u32>,
    source_line_raw: &str,
    source_line_expanded: &str
) -> Vec<String> {
    format_deferred_line_with_template_for_impl(
        format,
        bytes_per_line,
        file_index,
        specific_content,
        line_number,
        source_line_raw,
        source_line_expanded
    )
}

pub(crate) fn logical_width(format: &ListingOutputFormat) -> usize {
    logical_address_width_impl(format)
}

pub(crate) fn physical_width(format: &ListingOutputFormat) -> usize {
    physical_address_width_impl(format)
}

pub(crate) fn physical_field_width_for(format: &ListingOutputFormat) -> usize {
    physical_field_width_impl(format)
}

pub(crate) fn format_deferred_line_with_template_for(
    format: &ListingOutputFormat,
    bytes_per_line: usize,
    file_index: usize,
    specific_content: &str,
    line_number: Option<u32>,
    source_line_raw: &str,
    source_line_expanded: &str
) -> Vec<String> {
    format_deferred_line_with_template_for_impl(
        format,
        bytes_per_line,
        file_index,
        specific_content,
        line_number,
        source_line_raw,
        source_line_expanded
    )
}
