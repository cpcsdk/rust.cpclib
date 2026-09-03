use std::collections::HashSet;

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

const HEX_DIGITS_LOWER: &[u8; 16] = b"0123456789abcdef";
const HEX_DIGITS_UPPER: &[u8; 16] = b"0123456789ABCDEF";

/// Writes each byte as two hex digits, space-separated, straight into `out`.
/// Used instead of `format!("{b:02x}")` per byte + `itertools::join` (the
/// old `format_bytes_raw_for_impl`/`format_bytes_for_impl` shape): that was
/// `bytes.len() + 1` allocations just to render one byte column, on the
/// most frequently rendered field in the listing (every line with output
/// bytes goes through this). DHAT profiling of a real project's listing
/// generation found it among the largest remaining allocation sites after
/// fixing the bigger per-identifier/per-placeholder hotspots.
fn write_hex_bytes(out: &mut String, format: &ListingOutputFormat, bytes: &[u8]) {
    let digits = if format.uppercase_hex {
        HEX_DIGITS_UPPER
    }
    else {
        HEX_DIGITS_LOWER
    };
    for (i, &b) in bytes.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push(digits[(b >> 4) as usize] as char);
        out.push(digits[(b & 0xF) as usize] as char);
    }
}

fn format_bytes_for_impl(
    format: &ListingOutputFormat,
    bytes_per_line: usize,
    bytes: &[u8]
) -> String {
    let full_width = bytes_per_line * 3;
    let mut out = String::with_capacity(full_width.max(bytes.len() * 3));
    write_hex_bytes(&mut out, format, bytes);
    // `format!("{rendered:<full_width$}")`'s equivalent: pad, never truncate.
    // Every char pushed above is ASCII, so byte length tracks char count.
    while out.len() < full_width {
        out.push(' ');
    }
    out
}

fn render_source_column_impl(line: Option<&str>) -> String {
    let Some(line) = line else {
        return String::new();
    };
    let trimmed = line.trim_start();
    // A byte length within the char cap can't possibly need truncating (byte
    // count >= char count always), so the vast majority of lines - far
    // shorter than MAX_RENDERED_SOURCE_COLUMN_CHARS - skip straight to a
    // single-copy `to_owned()` instead of walking char boundaries to
    // rebuild the same string one char at a time.
    if trimmed.len() <= MAX_RENDERED_SOURCE_COLUMN_CHARS {
        return trimmed.to_owned();
    }
    trimmed.chars().take(MAX_RENDERED_SOURCE_COLUMN_CHARS).collect()
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
    // Borrows placeholder names straight out of `template` instead of
    // collecting each into its own `String` (see the char-by-char build-up
    // below): the template is a fixed, short, small-alphabet string re-scanned
    // for every listing line, so a `String::new()` + push-per-char build-up
    // and a per-occurrence `.clone()` into this set were pure per-line
    // overhead - DHAT profiling of a real project's listing generation
    // found this loop's `String::push` growth among the top allocation
    // sites.
    let mut consumed = HashSet::<&str>::new();
    let mut chars = template.char_indices().peekable();

    while let Some((start, ch)) = chars.next() {
        if ch != '{' {
            output.push(ch);
            continue;
        }

        let placeholder_start = start + 1;
        let mut placeholder_end = placeholder_start;
        while let Some(&(next_idx, next_ch)) = chars.peek() {
            chars.next();
            if next_ch == '}' {
                placeholder_end = next_idx;
                break;
            }
            placeholder_end = next_idx + next_ch.len_utf8();
        }
        let placeholder = &template[placeholder_start..placeholder_end];

        if placeholder.is_empty() {
            output.push('{');
            output.push('}');
            continue;
        }

        if !consumed.insert(placeholder) {
            continue;
        }

        let rendered = match placeholder {
            "A" => {
                logical_address
                    .map(|value| {
                        format_address_for_impl(format, value, logical_address_width_impl(format))
                    })
                    .unwrap_or_else(|| blank_impl(logical_address_width_impl(format)))
            },
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
                    line_number
                        .map(|value| value.to_string())
                        .unwrap_or_default()
                }
                else {
                    String::new()
                }
            },
            "L3" => {
                if format.show_line_numbers {
                    line_number
                        .map(|value| format!("{:>3}", value))
                        .unwrap_or_default()
                }
                else {
                    String::new()
                }
            },
            "L4" => {
                if format.show_line_numbers {
                    line_number
                        .map(|value| format!("{:>4}", value))
                        .unwrap_or_default()
                }
                else {
                    String::new()
                }
            },
            "L5" => {
                if format.show_line_numbers {
                    line_number
                        .map(|value| format!("{:>5}", value))
                        .unwrap_or_default()
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
                    format!(">{:continuation_prefix$}{suffix}", ""),
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

pub(crate) fn format_address_for(format: &ListingOutputFormat, value: u32, width: usize) -> String {
    format_address_for_impl(format, value, width)
}

pub(crate) fn blank(width: usize) -> String {
    blank_impl(width)
}

pub(crate) fn hex_byte_for(format: &ListingOutputFormat, b: u8) -> String {
    hex_byte_for_impl(format, b)
}

pub(crate) fn render_source_column(line: Option<&str>) -> String {
    render_source_column_impl(line)
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
