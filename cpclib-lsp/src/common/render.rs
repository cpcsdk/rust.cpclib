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
