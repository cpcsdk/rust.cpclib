use super::*;

impl TextListingRenderer {
    pub(crate) fn render_notice(&mut self, writer: &mut dyn Write, notice: ListingNotice<'_>) {
        let line = match notice {
            ListingNotice::RawLine(line) => line.to_string(),
            ListingNotice::ContextHeader { file_index, fname } => format!("Context [{file_index}]: {fname}"),
            ListingNotice::FileMapHeader { file_index, fname } => format!("Source file map:\n  [{file_index}] {fname}"),
            ListingNotice::FileMapEntry { file_index, fname } => format!("  [{file_index}] {fname}")
        };

        writeln!(writer, "{line}").unwrap();
    }

    pub(crate) fn render_deferred(&mut self, writer: &mut dyn Write, format: &ListingOutputFormat, bytes_per_line: usize, deferred: ListingDeferredRender<'_>) {
        for line in format_deferred_line_with_template_for(
            format,
            bytes_per_line,
            deferred.file_index,
            deferred.specific_content,
            deferred.line_number,
            deferred.source_line_raw,
            deferred.source_line_expanded
        ) {
            writeln!(writer, "{line}").unwrap();
        }
    }

    pub(crate) fn render_line(&mut self, writer: &mut dyn Write, format: &ListingOutputFormat, bytes_per_line: usize, line: ListingLineRender<'_>) {
        let rendered = format_line_with_template_for(
            format,
            bytes_per_line,
            line.file_index,
            line.logical_address,
            line.physical_address_repr,
            line.bytes,
            line.line_number,
            Some(line.source_line_raw),
            Some(line.source_line_expanded)
        );
        let rendered = if rendered.trim().is_empty() {
            format!(
                "{} {} {:bytes_width$} {}",
                line.logical_address
                    .map(|value| format_address_for(format, value, logical_address_width(format)))
                    .unwrap_or_else(|| blank(logical_address_width(format))),
                line.physical_address_repr,
                line.fallback_bytes,
                line.source_line_expanded,
                bytes_width = bytes_per_line * 3
            )
        }
        else {
            rendered
        };

        if line.is_multiline_continuation {
            let rendered = rendered.strip_prefix(' ').unwrap_or(&rendered);
            writeln!(writer, ">{rendered}").unwrap();
        }
        else {
            writeln!(writer, "{rendered}").unwrap();
        }
    }
}
