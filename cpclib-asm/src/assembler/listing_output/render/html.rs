use std::io::Write;

use super::{
    escape_html, global_prefix_for_symbol, is_identifier_char, qualify_local_symbol,
    render_html_bytes_for_row, HtmlBlockKind,
    HtmlListingRenderer, ListingDeferredRender, ListingLineRender, ListingNotice,
    ListingTokenRender
};
use super::super::format::{format_address_for, hex_byte_for, logical_address_width, render_source_column, ListingOutputFormat};
use super::super::TokenKind;

impl HtmlListingRenderer {
    const BYTES_COLUMN_CHARS: usize = 23;

    fn update_current_global_symbol(&mut self, token_kind: &TokenKind) {
        match token_kind {
            TokenKind::Label(name) | TokenKind::Set(name) | TokenKind::MacroDefine(name)
                if !name.starts_with('.') && !name.starts_with('@')
                    && !name.contains('.') => {
                self.current_global_symbol = Some(name.clone());
            }
            _ => {}
        }
    }

    fn escape_js_string(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        for ch in text.chars() {
            match ch {
                '\\' => out.push_str("\\\\"),
                '\'' => out.push_str("\\'"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                _ => out.push(ch)
            }
        }
        out
    }

    fn normalize_symbol_key(text: &str) -> Option<String> {
        let trimmed = text.trim().trim_end_matches(':');
        if trimmed.is_empty() {
            return None;
        }

        let start = trimmed
            .char_indices()
            .find_map(|(idx, ch)| is_identifier_char(ch).then_some(idx))?;
        let suffix = &trimmed[start..];
        let end = suffix
            .char_indices()
            .find_map(|(idx, ch)| (!is_identifier_char(ch)).then_some(idx))
            .unwrap_or(suffix.len());

        if end == 0 {
            return None;
        }

        Some(suffix[..end].to_ascii_lowercase())
    }

    fn insert_symbol_target(&mut self, symbol: &str, row_id: usize, value: Option<&str>) {
        self.symbol_names_by_row.entry(row_id).or_insert_with(|| symbol.to_string());
        self.symbol_targets.entry(symbol.to_string()).or_insert(row_id);
        self.symbol_targets
            .entry(symbol.to_ascii_lowercase())
            .or_insert(row_id);
        if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
            self.symbol_values
                .entry(row_id)
                .or_insert_with(|| value.to_string());
        }
        if let Some(normalized) = Self::normalize_symbol_key(symbol) {
            self.symbol_targets.entry(normalized).or_insert(row_id);
        }

        if let Some(last_dot) = symbol.rfind('.') {
            if last_dot > 0 && last_dot + 1 < symbol.len() {
                let short_local = format!(".{}", &symbol[last_dot + 1..]);
                self.symbol_targets.entry(short_local.clone()).or_insert(row_id);
                self.symbol_targets
                    .entry(short_local.to_ascii_lowercase())
                    .or_insert(row_id);
            }
        }

        if let Some(prefix) = global_prefix_for_symbol(symbol) {
            self.symbol_targets
                .entry(prefix.to_string())
                .or_insert(row_id);
            self.symbol_targets
                .entry(prefix.to_ascii_lowercase())
                .or_insert(row_id);
        }
    }

    fn row_hover_attr_for_key(hover_key: &str) -> String {
        format!(" data-hover-row=\"{hover_key}\"")
    }

    fn template_shows_source_expanded(format: &ListingOutputFormat) -> bool {
        format.listing_line_template.contains("{S}")
    }

    fn template_shows_source_raw(format: &ListingOutputFormat) -> bool {
        format.listing_line_template.contains("{SR}")
    }

    fn row_hover_attr(row_id: usize) -> String {
        Self::row_hover_attr_for_key(&format!("row-{row_id}"))
    }

    fn token_hover_key(token_id: usize) -> String {
        format!("tok-{token_id}")
    }

    fn render_source_fragment_html(&self, text: &str, hover_key: &str) -> String {
        let mut output = String::new();
        let mut chars = text.chars().peekable();
        let keywords = [
            "org", "macro", "endm", "repeat", "endr", "for", "next", "while", "wend",
            "if", "else", "endif", "db", "dw", "ds", "include", "equ", "set", "assert"
        ];

        while let Some(ch) = chars.next() {
            if ch == ';' {
                let mut comment = String::from(ch);
                comment.extend(chars.by_ref());
                output.push_str(&format!("<span class=\"token comment\">{}</span>", escape_html(&comment).replace(' ', "&nbsp;")));
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
                output.push_str(&format!("<span class=\"token string\">{}</span>", escape_html(&string).replace(' ', "&nbsp;")));
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
                if keywords.iter().any(|kw| *kw == token_lower) {
                    output.push_str(&format!(
                        "<span class=\"token keyword\">{}</span>",
                        escape_html(&token)
                    ));
                }
                else if token.starts_with("0x") || token.starts_with('#') || token.chars().all(|c| c.is_ascii_digit()) {
                    output.push_str(&format!(
                        "<span class=\"token number\">{}</span>",
                        escape_html(&token)
                    ));
                }
                else {
                    output.push_str(&format!(
                        "<span class=\"token\" data-symbol-candidate=\"{}\">{}</span>",
                        escape_html(&token),
                        escape_html(&token)
                    ));
                }
                continue;
            }

            output.push_str(&escape_html(&ch.to_string()).replace(' ', "&nbsp;"));
        }

        let hover_attr = Self::row_hover_attr_for_key(hover_key);
        format!("<span class=\"token fragment\"{hover_attr}>{output}</span>")
    }

    fn highlight_source_html_precise(
        &self,
        text: &str,
        row_id: usize,
        tokens: &[ListingTokenRender<'_>],
        expanded: bool
    ) -> String {
        if tokens.is_empty() {
            return self.highlight_source_html(text, Some(row_id));
        }

        let mut out = String::new();
        let mut cursor = 0usize;
        let append_non_token_fragment = |dst: &mut String, fragment: &str| {
            if let Some(comment_start) = fragment.find(';') {
                dst.push_str(&escape_html(&fragment[..comment_start]));
                dst.push_str(&format!(
                    "<span class=\"token comment\">{}</span>",
                    escape_html(&fragment[comment_start..])
                ));
            }
            else {
                dst.push_str(&escape_html(fragment));
            }
        };

        for token in tokens.iter() {
            if token.raw_text.trim_start().starts_with(';')
                || token.expanded_text.trim_start().starts_with(';')
            {
                continue;
            }

            let token_text = if expanded {
                token.expanded_text.trim()
            }
            else {
                token.raw_text.trim()
            };

            if token_text.is_empty() {
                continue;
            }

            if let Some(pos) = text[cursor..].find(token_text) {
                append_non_token_fragment(&mut out, &text[cursor..cursor + pos]);
                let hover_key = Self::token_hover_key(token.token_id);
                out.push_str(&self.render_source_fragment_html(token_text, &hover_key));
                cursor += pos + token_text.len();
            }
        }

        if cursor < text.len() {
            append_non_token_fragment(&mut out, &text[cursor..]);
        }

        out
    }

    fn render_token_bytes_html(
        &self,
        format: &ListingOutputFormat,
        row_id: usize,
        line: &ListingLineRender<'_>
    ) -> String {
        if line.tokens.is_empty() {
            return render_html_bytes_for_row(format, line.bytes, Some(row_id));
        }

        let mut rendered = Vec::new();
        let mut consumed = 0usize;

        let token_with_bytes = line
            .tokens
            .iter()
            .filter(|token| !token.bytes.is_empty())
            .collect::<Vec<_>>();

        for (token_idx, token) in token_with_bytes.iter().enumerate() {

            let hover_key = Self::token_hover_key(token.token_id);
            let hover_attr = Self::row_hover_attr_for_key(&hover_key);
            for (idx, byte) in token.bytes.iter().enumerate() {
                rendered.push(format!(
                    "<span class=\"token byte\"{hover_attr}>{}</span>",
                    hex_byte_for(format, *byte)
                ));
                if idx + 1 < token.bytes.len() {
                    rendered.push(format!("<span class=\"byte-sep\"{hover_attr}>&nbsp;</span>"));
                }
                consumed += 1;
            }
            if token_idx + 1 < token_with_bytes.len() {
                rendered.push(format!("<span class=\"byte-sep\"{hover_attr}>&nbsp;</span>"));
            }
        }

        if consumed < line.bytes.len() {
            let hover_attr = Self::row_hover_attr(row_id);
            let remaining = &line.bytes[consumed..];
            for (idx, byte) in remaining.iter().enumerate() {
                rendered.push(format!(
                    "<span class=\"token byte\"{hover_attr}>{}</span>",
                    hex_byte_for(format, *byte)
                ));
                if idx + 1 < remaining.len() {
                    rendered.push(format!("<span class=\"byte-sep\"{hover_attr}>&nbsp;</span>"));
                }
            }
        }

        rendered.join("").trim_end().to_string()
    }

    pub(crate) fn start(&mut self, writer: &mut dyn Write) {
        writer.write_all(br#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>BASM Listing</title>
<style>
:root { --bg:#f7f3ea; --panel:#fffdfa; --ink:#1f1b18; --muted:#71665d; --line:#dccfbe; --accent:#c66a1b; --hover:#fff0c2; --mono:"SFMono-Regular",Menlo,Consolas,monospace; --bytes-col:23ch; }
body { margin:0; font-family:var(--mono); background:linear-gradient(180deg,#efe7d8 0%,#f7f3ea 100%); color:var(--ink); }
.listing { max-width: 1800px; margin: 0 auto; padding: 12px 0; }
.notice { margin: 0; padding: 0; background:transparent; white-space:pre-wrap; }
.rows { display:flex; flex-direction:column; gap:0; }
.row { display:grid; grid-template-columns: 2ch 7ch 8ch var(--bytes-col) 6ch minmax(0,1fr) minmax(0,1fr); gap:0; align-items:start; padding:0; margin:0; border:0; background:transparent; }
.row:hover, .row.is-hovered { background:var(--hover); }
.row.is-collapsed { opacity:.55; }
.row.block-start { cursor:pointer; }
.cell { white-space:pre-wrap; overflow-wrap:anywhere; padding:0; margin:0; }
.marker { grid-column: 1; }
.addr { grid-column: 2; }
.phys { grid-column: 3; }
.bytes { grid-column: 4; }
.line { grid-column: 5; padding-left: 1ch; }
.source-expanded { grid-column: 6; }
.source-raw { grid-column: 7; }
.bytes { white-space:pre; overflow-wrap:normal; min-width:var(--bytes-col); }
.marker, .addr, .phys, .line { color:var(--muted); }
.bytes { color:#0d5f8c; }
.source-expanded { color:#8b2e00; white-space:pre; overflow-wrap:normal; }
.source-raw { color:#4a4039; white-space:pre; overflow-wrap:normal; }
.specific { color:#5d2e8c; }
.token { border-radius:2px; padding:0 1px; }
.token.is-hovered, .token.byte.is-hovered { background:rgba(198,106,27,.16); }
.token:hover { background:rgba(198,106,27,.14); }
.token.keyword { color:#7a2e00; font-weight:700; }
.token.number { color:#0d5f8c; }
.token.string { color:#2f6f4e; }
.token.comment { color:#7b6f62; font-style:italic; }
.token.fragment { white-space:pre; }
.token.linked-symbol { color:#8b2e00; text-decoration:underline dotted; cursor:pointer; }
.token.label { color:#8b2e00; font-weight:700; }
.token.byte { color:#0d5f8c; display:inline-block; width:2ch; padding:0; border-radius:0; }
.byte-sep { display:inline-block; width:1ch; }
.byte-sep.is-hovered { background:rgba(198,106,27,.16); }
.token-pending { color:#b88a59; }
.toggle { cursor:pointer; user-select:none; color:var(--accent); padding:0 0.25rem; }
.hidden-by-collapse { display:none !important; }
.row.deferred .specific { grid-column: 6; }
.row.deferred .line { grid-column: 5; }
.row.deferred .source-expanded { grid-column: 6; }
.row.deferred .source-raw { grid-column: 7; }
.row.deferred .specific-bytes { grid-column: 4; color:#5d2e8c; white-space:pre; overflow-wrap:normal; }
.row.deferred .specific-bytes.specific-overflow { white-space:pre; overflow-wrap:normal; overflow:visible; grid-row: 1; }
.row.deferred .line.line-next-line { grid-row: 2; }
.row.deferred .source-expanded.source-next-line { grid-row: 2; }
.row.deferred .source-raw.source-next-line { grid-row: 2; }
.symbols-panel { margin:18px 0 8px; padding:12px; background:var(--panel); border-top:1px solid var(--line); }
.symbols-title { margin:0 0 8px; color:var(--ink); font-size:13px; }
.symbols-search { width:min(560px,100%); margin:0 0 10px; padding:6px 8px; border:1px solid var(--line); background:#fff; color:var(--ink); font-family:var(--mono); }
.symbols-table { width:100%; border-collapse:collapse; font-size:12px; }
.symbols-table thead th { text-align:left; color:var(--muted); border-bottom:1px solid var(--line); padding:4px 6px; }
.symbols-table tbody td { padding:3px 6px; border-bottom:1px solid #eee5d7; }
.symbols-row.is-hidden { display:none; }
.symbols-link { color:#8b2e00; text-decoration:underline dotted; }
@media (max-width: 1100px) { .row { grid-template-columns: 2ch 7ch 8ch minmax(0,1fr); } .line, .source-raw { display:none; } }
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

function normalizeSymbolKey(text) {
    const trimmed = (text || '').trim().replace(/:+$/, '');
    if (!trimmed) return null;
    const start = trimmed.search(/[A-Za-z0-9_.]/);
    if (start < 0) return null;
    const suffix = trimmed.slice(start);
    const match = suffix.match(/^[A-Za-z0-9_.]+/);
    if (!match) return null;
    return match[0].toLowerCase();
}

function resolveSymbolTarget(symbolTargets, raw) {
    if (!raw) return null;
    if (symbolTargets.has(raw)) return symbolTargets.get(raw);
    const lowered = raw.toLowerCase();
    if (symbolTargets.has(lowered)) return symbolTargets.get(lowered);
    const normalized = normalizeSymbolKey(raw);
    if (normalized && symbolTargets.has(normalized)) return symbolTargets.get(normalized);
    return null;
}

function attachSymbolLinks(symbolTargets) {
    document.querySelectorAll('[data-symbol-candidate]').forEach((node) => {
        if (!(node instanceof HTMLElement)) return;
        if (node.querySelector('a')) return;
        const raw = node.dataset.symbolCandidate || node.textContent || '';
        const target = resolveSymbolTarget(symbolTargets, raw);
        if (target === null || target === undefined) return;

        const link = document.createElement('a');
        link.href = `#row-${target}`;
        link.dataset.targetRow = `row-${target}`;
        link.textContent = node.textContent || raw;
        node.textContent = '';
        node.appendChild(link);
        node.classList.add('linked-symbol');
    });
}

function applySymbolFilter() {
    const input = document.getElementById('symbol-search');
    const query = (input ? input.value : '').trim().toLowerCase();
    document.querySelectorAll('.symbols-row').forEach((row) => {
        const name = (row.getAttribute('data-symbol-name') || '').toLowerCase();
        const value = (row.getAttribute('data-symbol-value') || '').toLowerCase();
        const visible = !query || name.includes(query) || value.includes(query);
        row.classList.toggle('is-hidden', !visible);
    });
}

function initializeSymbolFilter() {
    const input = document.getElementById('symbol-search');
    if (!input) return;
    input.addEventListener('input', applySymbolFilter);
    applySymbolFilter();
}

function initializeCollapsedBlocks() {
    document.querySelectorAll('.row.block-start').forEach((blockStart) => {
        blockStart.classList.add('is-collapsed');
        findRowsUntilEnd(blockStart).forEach((row) => {
            row.classList.add('hidden-by-collapse');
        });
    });
}

let activeHoverKey = null;

function setActiveHoverKey(nextKey) {
    if (activeHoverKey === nextKey) return;
    if (activeHoverKey) {
        setHoverGroup(activeHoverKey, false);
    }
    activeHoverKey = nextKey;
    if (activeHoverKey) {
        setHoverGroup(activeHoverKey, true);
    }
}

document.addEventListener('mousemove', (event) => {
    setActiveHoverKey(hoveredGroupFromEventTarget(event.target));
});

document.addEventListener('mouseleave', () => {
    setActiveHoverKey(null);
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

    fn highlight_source_html(&self, text: &str, row_id: Option<usize>) -> String {
        let mut output = String::new();
        let mut chars = text.chars().peekable();
        let hover_attr = row_id.map(Self::row_hover_attr).unwrap_or_default();
        let keywords = [
            "org", "macro", "endm", "repeat", "endr", "for", "next", "while", "wend",
            "if", "else", "endif", "db", "dw", "ds", "include", "equ", "set", "assert"
        ];

        while let Some(ch) = chars.next() {
            if ch == ';' {
                let mut comment = String::from(ch);
                comment.extend(chars.by_ref());
                output.push_str(&format!("<span class=\"token comment\">{}</span>", escape_html(&comment)));
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
                if keywords.iter().any(|kw| *kw == token_lower) {
                    output.push_str(&format!(
                        "<span class=\"token keyword\"{hover_attr}>{}</span>",
                        escape_html(&token)
                    ));
                }
                else if token.starts_with("0x") || token.starts_with('#') || token.chars().all(|c| c.is_ascii_digit()) {
                    output.push_str(&format!(
                        "<span class=\"token number\"{hover_attr}>{}</span>",
                        escape_html(&token)
                    ));
                }
                else {
                    let display_token = qualify_local_symbol(&token, self.current_global_symbol.as_deref());
                    output.push_str(&format!(
                        "<span class=\"token\"{hover_attr} data-symbol-candidate=\"{}\">{}</span>",
                        escape_html(&token),
                        escape_html(&display_token)
                    ));
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

    fn deferred_symbol_uses_bytes_column(token_kind: &TokenKind) -> bool {
        matches!(token_kind, TokenKind::Label(_) | TokenKind::Set(_))
    }

    fn split_deferred_specific_columns(token_kind: &TokenKind, specific: &str) -> (String, String, String) {
        if !matches!(token_kind, TokenKind::Label(_) | TokenKind::Set(_)) {
            return (String::new(), String::new(), specific.to_string());
        }

        let mut parts = specific.split_whitespace();
        let addr = parts.next().unwrap_or_default().to_string();
        let phys = parts.next().unwrap_or_default().to_string();
        let symbol = parts.collect::<Vec<_>>().join(" ");

        if addr.is_empty() || phys.is_empty() || symbol.is_empty() {
            return (String::new(), String::new(), specific.to_string());
        }

        (addr, phys, symbol)
    }

    pub(crate) fn render_deferred(&mut self, writer: &mut dyn Write, format: &ListingOutputFormat, deferred: ListingDeferredRender<'_>) {
        let row_id = self.next_row_id;
        self.next_row_id += 1;
        self.update_current_global_symbol(deferred.token_kind);
        let (deferred_addr, deferred_phys, deferred_specific) =
            Self::split_deferred_specific_columns(deferred.token_kind, deferred.specific_content);
        if !deferred.specific_content.is_empty() {
            if let Some(symbol) = Self::extract_definition_symbol(deferred.token_kind) {
                let value = if !deferred_addr.is_empty() {
                    Some(deferred_addr.as_str())
                }
                else if !deferred_phys.is_empty() {
                    Some(deferred_phys.as_str())
                }
                else {
                    None
                };
                self.insert_symbol_target(&symbol, row_id, value);
            }
        }
        let block_kind = self
            .classify_block(deferred.token_kind, deferred.source_line_expanded)
            .or_else(|| self.classify_block(deferred.token_kind, deferred.source_line_raw));
        let block_kind_name = block_kind.as_ref().map(|kind| match kind {
            HtmlBlockKind::MacroDefinition => "macro",
            HtmlBlockKind::Repeat => "repeat"
        }).unwrap_or("");
        let block_start_attrs = block_kind
            .as_ref()
            .map(|_| format!(" data-block-kind=\"{}\"", block_kind_name))
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
        let show_toggle = block_kind.is_some() && !deferred.specific_content.is_empty();
        let row_classes = if show_toggle { "row deferred block-start" } else { "row deferred" };
        let collapsible_toggle = if show_toggle { "<span class=\"toggle\">▸</span>" } else { "" };
        let specific_in_bytes = Self::deferred_symbol_uses_bytes_column(deferred.token_kind);
        let source_on_next_line = specific_in_bytes
            && deferred_specific.chars().count() > Self::BYTES_COLUMN_CHARS;
        let show_expanded = Self::template_shows_source_expanded(format);
        let show_raw = Self::template_shows_source_raw(format);
        let bytes_cell = if specific_in_bytes {
            format!(
                "<span class=\"cell bytes specific-bytes{}\">{}</span>",
                if source_on_next_line { " specific-overflow" } else { "" },
                escape_html(&deferred_specific)
            )
        }
        else {
            "<span class=\"cell bytes\"></span>".to_string()
        };
        let specific_cell = if specific_in_bytes {
            String::new()
        }
        else {
            format!("<span class=\"cell specific\">{}</span>", escape_html(&deferred_specific))
        };
        let source_extra_class = if source_on_next_line { " source-next-line" } else { "" };
        let line_extra_class = if source_on_next_line { " line-next-line" } else { "" };
        writeln!(
            writer,
            "<div id=\"row-{row_id}\" class=\"{row_classes}\" data-row-id=\"row-{row_id}\" data-kind=\"{}\"{block_start_attrs}{block_end_attrs}><span class=\"cell marker\">{collapsible_toggle}</span><span class=\"cell addr\">{}</span><span class=\"cell phys\">{}</span>{}{}<span class=\"cell line{line_extra_class}\">{}</span>{}{}</div>",
            Self::token_kind_name(deferred.token_kind),
            escape_html(&deferred_addr),
            escape_html(&deferred_phys),
            bytes_cell,
            specific_cell,
            deferred.line_number.map(|v| v.to_string()).unwrap_or_default(),
            if show_expanded {
                format!("<span class=\"cell source-expanded{source_extra_class}\">{}</span>", self.highlight_source_html(&render_source_column(Some(deferred.source_line_expanded)), None))
            } else { String::new() },
            if show_raw {
                format!("<span class=\"cell source-raw{source_extra_class}\">{}</span>", self.highlight_source_html(&render_source_column(Some(deferred.source_line_raw)), None))
            } else { String::new() }
        ).unwrap();
    }

    pub(crate) fn render_line(&mut self, writer: &mut dyn Write, format: &ListingOutputFormat, line: ListingLineRender<'_>) {
        let row_id = self.next_row_id;
        self.next_row_id += 1;
        self.update_current_global_symbol(line.token_kind);
        let marker = if line.is_multiline_continuation
            && !matches!(line.token_kind, TokenKind::Hidden)
        {
            "&gt;"
        }
        else {
            ""
        };
        let logical = line
            .logical_address
            .map(|value| format_address_for(format, value, logical_address_width(format)))
            .unwrap_or_default();
        if let Some(symbol) = Self::extract_definition_symbol(line.token_kind) {
            let value = if !logical.is_empty() {
                Some(logical.as_str())
            }
            else {
                Some(line.physical_address_repr)
            };
            self.insert_symbol_target(&symbol, row_id, value);
        }
        let block_kind = self.classify_block(line.token_kind, line.source_line_expanded)
            .or_else(|| self.classify_block(line.token_kind, line.source_line_raw));
        let block_kind_name = block_kind.as_ref().map(|kind| match kind {
            HtmlBlockKind::MacroDefinition => "macro",
            HtmlBlockKind::Repeat => "repeat"
        }).unwrap_or("");
        let block_start_attrs = block_kind
            .as_ref()
            .map(|_| format!(" data-block-kind=\"{}\"", block_kind_name))
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
        let show_toggle = block_kind.is_some() && !line.is_multiline_continuation;
        let has_byte_hover = !line.bytes.is_empty() || line.tokens.iter().any(|token| !token.bytes.is_empty());
        let bytes_class = if has_byte_hover { "cell bytes interactive" } else { "cell bytes" };
        let source_expanded_class = if has_byte_hover { "cell source-expanded interactive" } else { "cell source-expanded" };
        let source_raw_class = if has_byte_hover { "cell source-raw interactive" } else { "cell source-raw" };
        let hover_attr = if has_byte_hover { Self::row_hover_attr(row_id) } else { String::new() };
        let row_classes = if show_toggle { "row block-start" } else { "row" };
        let collapsible_toggle = if show_toggle { "<span class=\"toggle\">▸</span>" } else { "" };
        let show_expanded = Self::template_shows_source_expanded(format);
        let show_raw = Self::template_shows_source_raw(format);
        writeln!(
            writer,
            "<div id=\"row-{row_id}\" class=\"{row_classes}\" data-row-id=\"row-{row_id}\" data-kind=\"{}\"{block_start_attrs}{block_end_attrs}><span class=\"cell marker\">{marker}{collapsible_toggle}</span><span class=\"cell addr\">{}</span><span class=\"cell phys\">{}</span><span class=\"{bytes_class}\"{}>{}</span><span class=\"cell line\">{}</span>{}{}</div>",
            Self::token_kind_name(line.token_kind),
            escape_html(&logical),
            escape_html(line.physical_address_repr),
            &hover_attr,
            self.render_token_bytes_html(format, row_id, &line),
            line.line_number.map(|value| value.to_string()).unwrap_or_default(),
            if show_expanded {
                format!("<span class=\"{source_expanded_class}\"{}>{}</span>", &hover_attr, self.highlight_source_html_precise(&render_source_column(Some(line.source_line_expanded)), row_id, line.source_tokens, true))
            } else { String::new() },
            if show_raw {
                format!("<span class=\"{source_raw_class}\"{}>{}</span>", &hover_attr, self.highlight_source_html_precise(&render_source_column(Some(line.source_line_raw)), row_id, line.source_tokens, false))
            } else { String::new() }
        ).unwrap();
    }

    pub(crate) fn finish(&mut self, writer: &mut dyn Write) {
        let mut symbol_rows = self
            .symbol_names_by_row
            .iter()
            .map(|(row, symbol)| {
                let value = self
                    .symbol_values
                    .get(row)
                    .cloned()
                    .unwrap_or_default();
                (*row, symbol.clone(), value)
            })
            .collect::<Vec<_>>();
        symbol_rows.sort_by(|left, right| {
            left.1
                .cmp(&right.1)
                .then_with(|| left.0.cmp(&right.0))
        });

        writer.write_all(br#"</div>
<div class="symbols-panel">
<h2 class="symbols-title">Symbols</h2>
<input id="symbol-search" class="symbols-search" type="search" placeholder="Filter symbols by name or value" aria-label="Filter symbols">
<table class="symbols-table">
<thead><tr><th>Symbol</th><th>Value</th></tr></thead>
<tbody>
"#).unwrap();

        for (row, symbol, value) in symbol_rows {
            writeln!(
                writer,
                "<tr class=\"symbols-row\" data-symbol-name=\"{}\" data-symbol-value=\"{}\"><td><a class=\"symbols-link\" href=\"#row-{}\" data-target-row=\"row-{}\">{}</a></td><td>{}</td></tr>",
                escape_html(&symbol),
                escape_html(&value),
                row,
                row,
                escape_html(&symbol),
                escape_html(&value)
            ).unwrap();
        }

        writer.write_all(br#"</tbody>
</table>
</div>
"#).unwrap();

        let mut symbol_entries = self
            .symbol_targets
            .iter()
            .map(|(symbol, row)| (symbol.clone(), *row))
            .collect::<Vec<_>>();
        symbol_entries.sort_by(|(left, _), (right, _)| left.cmp(right));

        let mut symbol_map_script = String::from("<script>\nconst BASM_SYMBOL_TARGETS = new Map([\n");
        for (symbol, row) in symbol_entries {
            symbol_map_script.push_str(&format!(
                "['{}', {}],\n",
                Self::escape_js_string(&symbol),
                row
            ));
        }
        symbol_map_script.push_str("]);\nattachSymbolLinks(BASM_SYMBOL_TARGETS);\ninitializeCollapsedBlocks();\ninitializeSymbolFilter();\n</script>\n");
        writer.write_all(symbol_map_script.as_bytes()).unwrap();

        writer.write_all(br#"</div>
</div>
</body>
</html>
"#).unwrap();
    }
}
