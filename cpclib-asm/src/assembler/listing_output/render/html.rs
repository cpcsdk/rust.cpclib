use super::*;

impl HtmlListingRenderer {
    fn template_shows_source_expanded(format: &ListingOutputFormat) -> bool {
        format.listing_line_template.contains("{S}")
    }

    fn template_shows_source_raw(format: &ListingOutputFormat) -> bool {
        format.listing_line_template.contains("{SR}")
    }

    fn row_hover_attr(row_id: usize) -> String {
        format!(" data-hover-row=\"row-{row_id}\"")
    }

    pub(crate) fn start(&mut self, writer: &mut dyn Write) {
        writer.write_all(br#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>BASM Listing</title>
<style>
:root { --bg:#f7f3ea; --panel:#fffdfa; --ink:#1f1b18; --muted:#71665d; --line:#dccfbe; --accent:#c66a1b; --hover:#fff0c2; --mono:"SFMono-Regular",Menlo,Consolas,monospace; }
body { margin:0; font-family:var(--mono); background:linear-gradient(180deg,#efe7d8 0%,#f7f3ea 100%); color:var(--ink); }
.listing { max-width: 1800px; margin: 0 auto; padding: 12px 0; }
.notice { margin: 0; padding: 0; background:transparent; white-space:pre-wrap; }
.rows { display:flex; flex-direction:column; gap:0; }
.row { display:grid; grid-template-columns: 2ch 7ch 8ch 2fr 6ch 4fr 4fr; gap:0; align-items:start; padding:0; margin:0; border:0; background:transparent; }
.row:hover, .row.is-hovered { background:var(--hover); }
.row.is-collapsed { opacity:.55; }
.row.block-start { cursor:pointer; }
.cell { white-space:pre-wrap; overflow-wrap:anywhere; padding:0; margin:0; }
.marker, .addr, .phys, .line { color:var(--muted); }
.bytes { color:#0d5f8c; }
.source-expanded { color:#8b2e00; }
.source-raw { color:#4a4039; }
.specific { color:#5d2e8c; }
.token { border-radius:2px; padding:0 1px; }
.token.is-hovered, .token.byte.is-hovered { background:rgba(198,106,27,.16); }
.token:hover { background:rgba(198,106,27,.14); }
.token.keyword { color:#7a2e00; font-weight:700; }
.token.number { color:#0d5f8c; }
.token.string { color:#2f6f4e; }
.token.comment { color:#7b6f62; font-style:italic; }
.token.symbol { color:#8b2e00; text-decoration:underline dotted; cursor:pointer; }
.token.label { color:#8b2e00; font-weight:700; }
.token.byte { color:#0d5f8c; }
.token-pending { color:#b88a59; }
.toggle { cursor:pointer; user-select:none; color:var(--accent); padding:0 0.25rem; }
.hidden-by-collapse { display:none !important; }
@media (max-width: 1100px) { .row { grid-template-columns: 2ch 7ch 8ch 1fr; } .line, .source-raw { display:none; } }
</style>
</head>
<body>
<div class="listing">
<div class="rows">
<script>
function findRowsUntilEnd(startRow) {
    const rows = [];
    let current = startRow.nextElementSibling;
    while (current) {
        rows.push(current);
        if (current.dataset.blockEnd === startRow.dataset.blockKind) break;
        current = current.nextElementSibling;
    }
    return rows;
}

function setRowHover(row, hovered) {
    row.classList.toggle('is-hovered', hovered);
}

function setHoverGroup(rowId, hovered) {
    document.querySelectorAll(`[data-hover-row="${rowId}"]`).forEach((element) => {
        element.classList.toggle('is-hovered', hovered);
    });
    const row = document.getElementById(rowId);
    if (row) {
        setRowHover(row, hovered);
    }
}

function hoveredGroupFromEventTarget(target) {
    if (!(target instanceof Element)) return null;
    const hoverTarget = target.closest('[data-hover-row]');
    return hoverTarget ? hoverTarget.dataset.hoverRow : null;
}

document.addEventListener('mouseover', (event) => {
    const rowId = hoveredGroupFromEventTarget(event.target);
    if (!rowId) return;
    const relatedRowId = hoveredGroupFromEventTarget(event.relatedTarget);
    if (relatedRowId === rowId) return;
    setHoverGroup(rowId, true);
});

document.addEventListener('mouseout', (event) => {
    const rowId = hoveredGroupFromEventTarget(event.target);
    if (!rowId) return;
    const relatedRowId = hoveredGroupFromEventTarget(event.relatedTarget);
    if (relatedRowId === rowId) return;
    setHoverGroup(rowId, false);
});

document.addEventListener('click', (event) => {
    const target = event.target.closest('[data-target-row]');
    if (target) {
        const id = target.dataset.targetRow;
        const row = document.getElementById(id);
        if (row) {
            row.scrollIntoView({ block: 'center', behavior: 'smooth' });
            row.classList.add('is-hovered');
            window.setTimeout(() => row.classList.remove('is-hovered'), 800);
        }
        return;
    }

    const blockStart = event.target.closest('.row.block-start');
    if (!blockStart) return;

    const collapsed = !blockStart.classList.contains('is-collapsed');
    blockStart.classList.toggle('is-collapsed', collapsed);
    findRowsUntilEnd(blockStart).forEach((row) => {
        row.classList.toggle('hidden-by-collapse', collapsed);
    });
});
</script>
"#).unwrap();
    }

    pub(crate) fn render_notice(&mut self, writer: &mut dyn Write, notice: ListingNotice<'_>) {
        let (class_name, content) = match notice {
            ListingNotice::RawLine(line) => ("notice raw", line.to_string()),
            ListingNotice::ContextHeader { file_index, fname } => (
                "notice context",
                format!("Context [{file_index}]: {fname}")
            ),
            ListingNotice::FileMapHeader { file_index, fname } => (
                "notice filemap",
                format!("Source file map\n[{file_index}] {fname}")
            ),
            ListingNotice::FileMapEntry { file_index, fname } => (
                "notice filemap-entry",
                format!("[{file_index}] {fname}")
            )
        };

        writeln!(writer, "<div class=\"{class_name}\">{}</div>", escape_html(&content)).unwrap();
    }

    fn highlight_source_html(&self, text: &str, row_id: usize) -> String {
        let mut output = String::new();
        let mut chars = text.chars().peekable();
        let hover_attr = Self::row_hover_attr(row_id);
        let keywords = [
            "org", "macro", "endm", "repeat", "endr", "for", "next", "while", "wend",
            "if", "else", "endif", "db", "dw", "ds", "include", "equ", "set", "assert"
        ];

        while let Some(ch) = chars.next() {
            if ch == ';' {
                let mut comment = String::from(ch);
                comment.extend(chars.by_ref());
                output.push_str(&format!("<span class=\"token comment\"{hover_attr}>{}</span>", escape_html(&comment)));
                break;
            }

            if ch == '"' || ch == '\'' {
                let quote = ch;
                let mut string = String::from(ch);
                while let Some(next) = chars.next() {
                    string.push(next);
                    if next == quote {
                        break;
                    }
                }
                output.push_str(&format!("<span class=\"token string\"{hover_attr}>{}</span>", escape_html(&string)));
                continue;
            }

            if is_identifier_char(ch) {
                let mut token = String::from(ch);
                while let Some(&next) = chars.peek() {
                    if !is_identifier_char(next) {
                        break;
                    }
                    token.push(next);
                    chars.next();
                }

                let token_lower = token.to_ascii_lowercase();
                let (class_name, rendered_token) = if keywords.iter().any(|kw| *kw == token_lower) {
                    ("keyword", escape_html(&token))
                }
                else if token.starts_with("0x") || token.starts_with('#') || token.chars().all(|c| c.is_ascii_digit()) {
                    ("number", escape_html(&token))
                }
                else if let Some(target) = self.symbol_targets.get(&token) {
                    (
                        "symbol",
                        format!("<a href=\"#row-{target}\" data-target-row=\"row-{target}\">{}</a>", escape_html(&token))
                    )
                }
                else {
                    ("", escape_html(&token))
                };

                if class_name.is_empty() {
                    output.push_str(&rendered_token);
                }
                else {
                    output.push_str(&format!("<span class=\"token {class_name}\"{hover_attr}>{rendered_token}</span>"));
                }
                continue;
            }

            output.push_str(&escape_html(&ch.to_string()));
        }

        output
    }

    fn classify_block(&self, token_kind: &TokenKind, source: &str) -> Option<HtmlBlockKind> {
        let normalized = source.trim_start().to_ascii_lowercase();
        if matches!(token_kind, TokenKind::MacroDefine(_)) || normalized.starts_with("macro ") {
            Some(HtmlBlockKind::MacroDefinition)
        }
        else if normalized.starts_with("repeat ") || normalized.starts_with("for ") || normalized.starts_with("while ") {
            Some(HtmlBlockKind::Repeat)
        }
        else {
            None
        }
    }

    fn block_end_marker(kind: &HtmlBlockKind) -> &'static str {
        match kind {
            HtmlBlockKind::MacroDefinition => "endm",
            HtmlBlockKind::Repeat => "endr"
        }
    }

    fn token_kind_name(token_kind: &TokenKind) -> &'static str {
        match token_kind {
            TokenKind::Hidden => "hidden",
            TokenKind::Label(_) => "label",
            TokenKind::Set(_) => "set",
            TokenKind::MacroCall => "macro-call",
            TokenKind::MacroDefine(_) => "macro-define",
            TokenKind::Displayable => "displayable"
        }
    }

    fn extract_definition_symbol(token_kind: &TokenKind) -> Option<String> {
        match token_kind {
            TokenKind::Label(name) | TokenKind::Set(name) | TokenKind::MacroDefine(name) => Some(name.clone()),
            _ => None
        }
    }

    pub(crate) fn render_deferred(&mut self, writer: &mut dyn Write, format: &ListingOutputFormat, deferred: ListingDeferredRender<'_>) {
        let row_id = self.next_row_id;
        self.next_row_id += 1;
        let block_kind = self
            .classify_block(deferred.token_kind, deferred.source_line_expanded)
            .or_else(|| self.classify_block(deferred.token_kind, deferred.source_line_raw));
        let block_kind_name = block_kind.as_ref().map(|kind| match kind {
            HtmlBlockKind::MacroDefinition => "macro",
            HtmlBlockKind::Repeat => "repeat"
        }).unwrap_or("");
        let block_start_attrs = block_kind
            .as_ref()
            .map(|kind| format!(" data-block-kind=\"{}\"", block_kind_name))
            .unwrap_or_default();
        let block_end_attrs = if matches!(deferred.source_line_expanded.trim_start().to_ascii_lowercase().as_str(), "endm" | "endr") {
            match deferred.source_line_expanded.trim_start().to_ascii_lowercase().as_str() {
                "endm" => " data-block-end=\"macro\"",
                _ => " data-block-end=\"repeat\""
            }
        }
        else {
            ""
        };
        let row_classes = if block_kind.is_some() { "row deferred block-start" } else { "row deferred" };
        let collapsible_toggle = if block_kind.is_some() { "<span class=\"toggle\">▸</span>" } else { "" };
        let show_expanded = Self::template_shows_source_expanded(format);
        let show_raw = Self::template_shows_source_raw(format);
        writeln!(
            writer,
            "<div id=\"row-{row_id}\" class=\"{row_classes}\" data-row-id=\"row-{row_id}\" data-kind=\"{}\"{block_start_attrs}{block_end_attrs}><span class=\"cell marker\">{collapsible_toggle}</span><span class=\"cell addr\"></span><span class=\"cell phys\"></span><span class=\"cell bytes\"></span><span class=\"cell line\">{}</span><span class=\"cell specific\"{}>{}</span>{}{}</div>",
            Self::token_kind_name(deferred.token_kind),
            deferred.line_number.map(|v| v.to_string()).unwrap_or_default(),
            Self::row_hover_attr(row_id),
            escape_html(deferred.specific_content),
            if show_expanded {
                format!("<span class=\"cell source-expanded\"{}>{}</span>", Self::row_hover_attr(row_id), self.highlight_source_html(&render_source_column(Some(deferred.source_line_expanded)), row_id))
            } else { String::new() },
            if show_raw {
                format!("<span class=\"cell source-raw\"{}>{}</span>", Self::row_hover_attr(row_id), self.highlight_source_html(&render_source_column(Some(deferred.source_line_raw)), row_id))
            } else { String::new() }
        ).unwrap();
    }

    pub(crate) fn render_line(&mut self, writer: &mut dyn Write, format: &ListingOutputFormat, line: ListingLineRender<'_>) {
        let row_id = self.next_row_id;
        self.next_row_id += 1;
        let marker = if line.is_multiline_continuation { "&gt;" } else { "" };
        let logical = line
            .logical_address
            .map(|value| format_address_for(format, value, logical_address_width(format)))
            .unwrap_or_default();
        if let Some(symbol) = Self::extract_definition_symbol(line.token_kind) {
            self.symbol_targets.insert(symbol, row_id);
        }
        let block_kind = self.classify_block(line.token_kind, line.source_line_expanded)
            .or_else(|| self.classify_block(line.token_kind, line.source_line_raw));
        let block_kind_name = block_kind.as_ref().map(|kind| match kind {
            HtmlBlockKind::MacroDefinition => "macro",
            HtmlBlockKind::Repeat => "repeat"
        }).unwrap_or("");
        let block_start_attrs = block_kind
            .as_ref()
            .map(|kind| format!(" data-block-kind=\"{}\"", block_kind_name))
            .unwrap_or_default();
        let block_end_attrs = if matches!(line.source_line_expanded.trim_start().to_ascii_lowercase().as_str(), "endm" | "endr") {
            match line.source_line_expanded.trim_start().to_ascii_lowercase().as_str() {
                "endm" => " data-block-end=\"macro\"",
                _ => " data-block-end=\"repeat\""
            }
        }
        else {
            ""
        };
        let row_classes = if block_kind.is_some() { "row block-start" } else { "row" };
        let collapsible_toggle = if block_kind.is_some() { "<span class=\"toggle\">▸</span>" } else { "" };
        let show_expanded = Self::template_shows_source_expanded(format);
        let show_raw = Self::template_shows_source_raw(format);
        writeln!(
            writer,
            "<div id=\"row-{row_id}\" class=\"{row_classes}\" data-row-id=\"row-{row_id}\" data-kind=\"{}\"{block_start_attrs}{block_end_attrs}><span class=\"cell marker\">{marker}{collapsible_toggle}</span><span class=\"cell addr\">{}</span><span class=\"cell phys\">{}</span><span class=\"cell bytes interactive\"{}>{}</span><span class=\"cell line\">{}</span>{}{}</div>",
            Self::token_kind_name(line.token_kind),
            escape_html(&logical),
            escape_html(line.physical_address_repr),
            Self::row_hover_attr(row_id),
            render_html_bytes_for_row(format, line.bytes, Some(row_id)),
            line.line_number.map(|value| value.to_string()).unwrap_or_default(),
            if show_expanded {
                format!("<span class=\"cell source-expanded interactive\"{}>{}</span>", Self::row_hover_attr(row_id), self.highlight_source_html(&render_source_column(Some(line.source_line_expanded)), row_id))
            } else { String::new() },
            if show_raw {
                format!("<span class=\"cell source-raw interactive\"{}>{}</span>", Self::row_hover_attr(row_id), self.highlight_source_html(&render_source_column(Some(line.source_line_raw)), row_id))
            } else { String::new() }
        ).unwrap();
    }

    pub(crate) fn finish(&mut self, writer: &mut dyn Write) {
        writer.write_all(br#"</div>
</div>
</body>
</html>
"#).unwrap();
    }
}
