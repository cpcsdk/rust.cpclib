use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

/// Wrap an already-rendered markdown string into an LSP `Hover`.
pub fn make_hover(md: String) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: md
        }),
        range: None
    }
}

/// Build the Markdown hover table for a numeric literal, using CPC conventions:
///   decimal — no prefix, hex — `&`, binary — `%` with `_` every 4 bits.
/// The value column is right-aligned.
pub fn format_number_hover(label: &str, value: i64) -> String {
    let hex = format!("&{:X}", value as u64 & 0xFFFF_FFFF_FFFF_FFFF);
    let bin = format_binary_cpc(value);
    format!(
        "**`{label}`**\n\n\
        | Base    | Value |\n\
        |---------|------:|\n\
        | Decimal | `{value}` |\n\
        | Hex     | `{hex}` |\n\
        | Binary  | `%{bin}` |"
    )
}

/// Extract a short one-line summary from a keyword/directive's full markdown
/// documentation, for use as a `CompletionItem.detail` (the compact
/// completion-menu line, as opposed to `.documentation`'s full popup).
///
/// Doc strings in this crate are generated as either `"**NAME** sig\n\ndesc"`
/// or `"**NAME**\n\n**Synopsis:**\n```..```\n\ndesc"` — in both cases the
/// actual description is the text after the *last* `\n\n`. A few short
/// entries have no `\n\n` at all (e.g. `"**THEN** — keyword used after..."`),
/// in which case the whole string is used, with the leading `**NAME**`
/// marker stripped.
pub fn first_doc_line(doc: &str) -> String {
    let body = doc.rsplit("\n\n").next().unwrap_or(doc);
    let line = body.lines().next().unwrap_or(body).trim();

    let line = match line
        .strip_prefix("**")
        .and_then(|rest| rest.find("**").map(|end| &rest[end + 2..]))
    {
        Some(rest) => rest.trim_start_matches(['—', '-', ' ']).trim(),
        None => line
    };

    const MAX_CHARS: usize = 100;
    if line.chars().count() > MAX_CHARS {
        let truncated: String = line.chars().take(MAX_CHARS).collect();
        format!("{}…", truncated.trim_end())
    }
    else {
        line.to_string()
    }
}

/// Render one `(label, bytes)` pair per row as a Markdown table, bytes
/// column first — e.g. for splitting an encoded BASIC line into its
/// header/token/terminator byte groups. A real table (rather than a padded
/// fixed-width block) is what guarantees every row's byte and label column
/// start at the same x position regardless of how their contents vary in
/// width. Bytes are rendered as inline code, which Markdown renderers
/// typically show in a visually distinct, monospaced style from the
/// plain-text label column — hover popups don't allow arbitrary CSS/colors,
/// so this is the closest available visual separation between the two.
pub fn format_labeled_bytes(groups: &[(&str, &[u8])]) -> String {
    let mut md = String::from("| Bytes | Token |\n|---|---|\n");
    for (label, bytes) in groups {
        let hex = bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        md.push_str(&format!("| `{hex}` | {} |\n", label.replace('|', "\\|")));
    }
    md
}

/// Format an i64 as binary with `_` every 4 bits, sized to 8 or 16 bits.
pub fn format_binary_cpc(value: i64) -> String {
    let bits: u32 = if value >= 0 && value <= 0xFF { 8 } else { 16 };
    let mut s = String::with_capacity(bits as usize + bits as usize / 4);
    for i in (0..bits).rev() {
        if i < bits - 1 && i % 4 == 3 {
            s.push('_');
        }
        s.push(if value & (1 << i) != 0 { '1' } else { '0' });
    }
    s
}
