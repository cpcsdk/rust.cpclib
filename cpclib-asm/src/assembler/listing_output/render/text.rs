use std::io::Write;

use super::super::TokenKind;
use super::super::format::{
    ListingOutputFormat, blank, format_address_for, format_deferred_line_with_template_for,
    format_line_with_template_for, logical_address_width
};
use super::shared::{
    ListingDeferredRender, ListingLineRender, ListingNotice, TextListingRenderer,
    global_prefix_for_symbol, is_identifier_char
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

    /// Walks `text` byte-slicing out each identifier-like run instead of
    /// accumulating it char-by-char into a fresh `String` (DHAT profiling of
    /// a real project's listing generation found the old char-by-char
    /// `String::from(ch)` + `qualify_local_symbol`'s own `to_string()` was
    /// the single largest allocation source in the whole build - ~5M
    /// allocations, one per identifier character per line, doubled since
    /// both the raw and expanded source line are qualified). A non-local
    /// token (the overwhelming majority) is now pushed straight from the
    /// original `&str` with no allocation of its own; only a `.`-prefixed
    /// local symbol pays for the global-prefix push, same as before.
    fn qualify_locals_in_line(&self, text: &str) -> String {
        let mut output = String::with_capacity(text.len());
        let mut chars = text.char_indices().peekable();

        while let Some((start, ch)) = chars.next() {
            if is_identifier_char(ch) {
                let mut end = start + ch.len_utf8();
                while let Some(&(next_idx, next_ch)) = chars.peek() {
                    if !is_identifier_char(next_ch) {
                        break;
                    }
                    end = next_idx + next_ch.len_utf8();
                    chars.next();
                }

                let token = &text[start..end];
                if token.starts_with('.')
                    && let Some(global) = self.current_global_symbol.as_deref()
                {
                    output.push_str(global);
                }
                output.push_str(token);
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
