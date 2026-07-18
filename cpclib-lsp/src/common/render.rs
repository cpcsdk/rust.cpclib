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
