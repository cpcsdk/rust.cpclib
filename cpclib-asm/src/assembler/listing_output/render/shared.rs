use std::collections::HashMap;
use std::io::Write;

use cpclib_common::itertools::Itertools;

use super::super::TokenKind;
use super::super::format::{
    blank, format_address_for, format_deferred_line_with_template_for,
    format_line_with_template_for, hex_byte_for, logical_address_width,
    render_source_column, ListingOutputFormat, ListingOutputKind
};

pub(crate) struct ListingLineRender<'a> {
    pub(crate) row_id: usize,
    pub(crate) file_index: usize,
    pub(crate) logical_address: Option<u32>,
    pub(crate) physical_address_repr: &'a str,
    pub(crate) bytes: &'a [u8],
    pub(crate) fallback_bytes: &'a str,
    pub(crate) line_number: Option<u32>,
    pub(crate) source_line_raw: &'a str,
    pub(crate) source_line_expanded: &'a str,
    pub(crate) is_multiline_continuation: bool,
    pub(crate) token_kind: &'a TokenKind,
    pub(crate) tokens: &'a [ListingTokenRender<'a>],
    pub(crate) definition_target: Option<usize>,
    pub(crate) highlighted_symbols: &'a [String],
    pub(crate) collapsible: bool,
    pub(crate) collapsed_block: bool
}

pub(crate) struct ListingTokenRender<'a> {
    pub(crate) raw_text: &'a str,
    pub(crate) expanded_text: &'a str,
    pub(crate) bytes: &'a [u8],
    pub(crate) token_kind: &'a TokenKind
}

pub(crate) struct ListingDeferredRender<'a> {
    pub(crate) row_id: usize,
    pub(crate) file_index: usize,
    pub(crate) specific_content: &'a str,
    pub(crate) line_number: Option<u32>,
    pub(crate) source_line_raw: &'a str,
    pub(crate) source_line_expanded: &'a str,
    pub(crate) token_kind: &'a TokenKind,
    pub(crate) definition_target: Option<usize>,
    pub(crate) highlighted_symbols: &'a [String],
    pub(crate) collapsible: bool,
    pub(crate) collapsed_block: bool
}

pub(crate) enum ListingNotice<'a> {
    RawLine(&'a str),
    ContextHeader { file_index: usize, fname: &'a str },
    FileMapHeader { file_index: usize, fname: &'a str },
    FileMapEntry { file_index: usize, fname: &'a str }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum HtmlBlockKind {
    MacroDefinition,
    Repeat
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HtmlBlock {
    row_id: usize,
    kind: HtmlBlockKind,
    title: String,
    source_row_ids: Vec<usize>
}

pub(crate) enum ListingRenderer {
    Text(TextListingRenderer),
    Html(HtmlListingRenderer)
}

pub(crate) struct TextListingRenderer;

#[derive(Default)]
pub(crate) struct HtmlListingRenderer {
    pub(crate) next_row_id: usize,
    pub(crate) active_block: Option<HtmlBlock>,
    pub(crate) blocks: Vec<HtmlBlock>,
    pub(crate) symbol_targets: HashMap<String, usize>
}

pub(crate) fn render_html_bytes(format: &ListingOutputFormat, bytes: &[u8]) -> String {
    render_html_bytes_for_row(format, bytes, None)
}

pub(crate) fn render_html_bytes_for_row(format: &ListingOutputFormat, bytes: &[u8], row_id: Option<usize>) -> String {
    let hover_attr = row_id
        .map(|row_id| format!(" data-hover-row=\"row-{row_id}\""))
        .unwrap_or_default();

    bytes
        .iter()
        .map(|byte| format!("<span class=\"token byte\"{hover_attr}>{}</span>", hex_byte_for(format, *byte)))
        .join(" ")
}

pub(crate) fn escape_html(text: &str) -> String {
    text.chars()
        .map(|ch| match ch {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#39;".to_string(),
            _ => ch.to_string()
        })
        .collect()
}

pub(crate) fn render_html_tokenized_text(text: &str, class_name: &str) -> String {
    let mut out = String::new();
    let mut token = String::new();
    let mut in_whitespace = None;

    for ch in text.chars() {
        let is_whitespace = ch.is_whitespace();
        if in_whitespace == Some(is_whitespace) || in_whitespace.is_none() {
            token.push(ch);
            in_whitespace = Some(is_whitespace);
            continue;
        }

        if in_whitespace == Some(true) {
            out.push_str(&escape_html(&token));
        }
        else {
            out.push_str(&format!("<span class=\"token {class_name}\">{}</span>", escape_html(&token)));
        }

        token.clear();
        token.push(ch);
        in_whitespace = Some(is_whitespace);
    }

    if !token.is_empty() {
        if in_whitespace == Some(true) {
            out.push_str(&escape_html(&token));
        }
        else {
            out.push_str(&format!("<span class=\"token {class_name}\">{}</span>", escape_html(&token)));
        }
    }

    out
}

pub(crate) fn is_identifier_char(ch: char) -> bool {
    ch.is_alphanumeric() || matches!(ch, '_' | '.' | '@' | '?')
}

impl ListingRenderer {
    pub(crate) fn from_format(format: &ListingOutputFormat) -> Self {
        match format.output_kind {
            ListingOutputKind::Text => Self::Text(TextListingRenderer),
            ListingOutputKind::Html => Self::Html(HtmlListingRenderer::default())
        }
    }

    pub(crate) fn start(&mut self, writer: &mut dyn Write) {
        match self {
            Self::Text(_) => {},
            Self::Html(renderer) => renderer.start(writer)
        }
    }

    pub(crate) fn render_notice(&mut self, writer: &mut dyn Write, notice: ListingNotice<'_>) {
        match self {
            Self::Text(renderer) => renderer.render_notice(writer, notice),
            Self::Html(renderer) => renderer.render_notice(writer, notice)
        }
    }

    pub(crate) fn render_deferred(&mut self, writer: &mut dyn Write, format: &ListingOutputFormat, bytes_per_line: usize, deferred: ListingDeferredRender<'_>) {
        match self {
            Self::Text(renderer) => renderer.render_deferred(writer, format, bytes_per_line, deferred),
            Self::Html(renderer) => renderer.render_deferred(writer, format, deferred)
        }
    }

    pub(crate) fn render_line(&mut self, writer: &mut dyn Write, format: &ListingOutputFormat, bytes_per_line: usize, line: ListingLineRender<'_>) {
        match self {
            Self::Text(renderer) => renderer.render_line(writer, format, bytes_per_line, line),
            Self::Html(renderer) => renderer.render_line(writer, format, line)
        }
    }

    pub(crate) fn finish(&mut self, writer: &mut dyn Write) {
        match self {
            Self::Text(_) => {},
            Self::Html(renderer) => renderer.finish(writer)
        }
    }
}
