use std::io::Write;

use super::super::TokenKind;
use super::super::format::{
    ListingOutputFormat, blank, format_address_for, format_deferred_line_with_template_for,
    format_line_with_template_for, logical_address_width
};
use super::shared::{
    ListingDeferredRender, ListingLineRender, ListingNotice, TextListingRenderer,
    global_prefix_for_symbol, is_identifier_char, qualify_local_symbol
};

impl TextListingRenderer {
    fn update_current_global_symbol(&mut self, token_kind: &TokenKind) {
        match token_kind {
            TokenKind::Label(symbol) | TokenKind::Set(symbol) => {
                if let Some(prefix) = global_prefix_for_symbol(symbol) {
                    self.current_global_symbol = Some(prefix.to_string());
                }
                else {
                    self.current_global_symbol = Some(symbol.clone());
                }
            },
            _ => {}
        }
    }

    fn qualify_locals_in_line(&self, text: &str) -> String {
        let mut output = String::new();
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            if is_identifier_char(ch) {
                let mut token = String::from(ch);
                while let Some(&next) = chars.peek() {
                    if !is_identifier_char(next) {
                        break;
                    }
                    token.push(next);
                    chars.next();
                }

                output.push_str(&qualify_local_symbol(
                    &token,
                    self.current_global_symbol.as_deref()
                ));
                continue;
            }

            output.push(ch);
        }

        output
    }

    pub(crate) fn render_notice(&mut self, writer: &mut dyn Write, notice: ListingNotice<'_>) {
        let line = match notice {
            ListingNotice::RawLine(line) => line.to_string(),
            ListingNotice::ContextHeader { file_index, fname } => {
                format!("Context [{file_index}]: {fname}")
            },
            ListingNotice::FileMapHeader { file_index, fname } => {
                format!("Source file map:\n  [{file_index}] {fname}")
            },
            ListingNotice::FileMapEntry { file_index, fname } => format!("  [{file_index}] {fname}")
        };

        writeln!(writer, "{line}").unwrap();
    }

    pub(crate) fn render_deferred(
        &mut self,
        writer: &mut dyn Write,
        format: &ListingOutputFormat,
        bytes_per_line: usize,
        deferred: ListingDeferredRender<'_>
    ) {
        self.update_current_global_symbol(deferred.token_kind);
        let source_line_raw = self.qualify_locals_in_line(deferred.source_line_raw);
        let source_line_expanded = self.qualify_locals_in_line(deferred.source_line_expanded);

        for line in format_deferred_line_with_template_for(
            format,
            bytes_per_line,
            deferred.file_index,
            deferred.specific_content,
            deferred.line_number,
            &source_line_raw,
            &source_line_expanded
        ) {
            writeln!(writer, "{line}").unwrap();
        }
    }

    pub(crate) fn render_line(
        &mut self,
        writer: &mut dyn Write,
        format: &ListingOutputFormat,
        bytes_per_line: usize,
        line: ListingLineRender<'_>
    ) {
        self.update_current_global_symbol(line.token_kind);
        let source_line_raw = self.qualify_locals_in_line(line.source_line_raw);
        let source_line_expanded = self.qualify_locals_in_line(line.source_line_expanded);

        let rendered = format_line_with_template_for(
            format,
            bytes_per_line,
            line.file_index,
            line.logical_address,
            line.physical_address_repr,
            line.bytes,
            line.line_number,
            Some(&source_line_raw),
            Some(&source_line_expanded)
        );
        let rendered = if rendered.trim().is_empty() {
            format!(
                "{} {} {:bytes_width$} {}",
                line.logical_address
                    .map(|value| format_address_for(format, value, logical_address_width(format)))
                    .unwrap_or_else(|| blank(logical_address_width(format))),
                line.physical_address_repr,
                line.fallback_bytes,
                source_line_expanded,
                bytes_width = bytes_per_line * 3
            )
        }
        else {
            rendered
        };

        let needs_continuation_marker =
            line.is_multiline_continuation && !matches!(line.token_kind, TokenKind::Hidden);

        if needs_continuation_marker {
            let rendered = rendered.strip_prefix(' ').unwrap_or(&rendered);
            writeln!(writer, ">{rendered}").unwrap();
        }
        else {
            writeln!(writer, "{rendered}").unwrap();
        }
    }
}
