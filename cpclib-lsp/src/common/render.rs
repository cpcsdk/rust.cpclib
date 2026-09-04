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
    let hex = format_hex_cpc(value);
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

/// Parses a bare numeric-literal string (already isolated, e.g. by
/// `basm::hover::extract_number_at_position`, or a firmware EQU's own
/// `#BB5A`-shaped value text) into its `i64` value. Handles `$`/`&`/`#`
/// (hex), `%` (binary), `0x`/`0b`/`0o`, and plain decimal — the same CPC/
/// basm numeric-literal conventions `format_number_hover` displays.
///
/// Delegates to `cpclib_common::parse_value`, the parser `basm` itself uses
/// for source-level numeric literals (also `_`-separated digit groups,
/// which this narrower reimplementation used to reject) - rather than a
/// second, independent implementation of the same conventions. That parser
/// returns `u32`; this still returns `i64` for its own callers, but neither
/// this function nor any current caller ever handled a *signed* literal (no
/// branch here recognised a leading `-`, and both callers only ever isolate
/// a token starting at a digit or a prefix), so the narrowing carries
/// nothing to preserve.
pub fn parse_numeric_literal_str(num_str: &str) -> Option<i64> {
    use cpclib_common::winnow::Parser;
    use cpclib_common::winnow::error::ContextError;
    use cpclib_common::winnow::stream::AsBStr;
    cpclib_common::parse_value::<_, ContextError>
        .parse(num_str.trim().as_bstr())
        .ok()
        .map(i64::from)
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

/// Format an i64 as hex, sized to 8 or 16 bits (2 or 4 digits) using the same
/// width rule as `format_binary_cpc`, so the two columns of a hover table
/// stay consistent — e.g. a negative literal renders as `&FF`/`&FFFF`
/// (two's complement), not a full 16-digit 64-bit value.
pub fn format_hex_cpc(value: i64) -> String {
    let bits: u32 = if (0..=0xFF).contains(&value) { 8 } else { 16 };
    let digits = (bits / 4) as usize;
    let mask = (1u64 << bits) - 1;
    format!("&{:0digits$X}", value as u64 & mask, digits = digits)
}

/// Format an i64 as binary with `_` every 4 bits, sized to 8 or 16 bits.
pub fn format_binary_cpc(value: i64) -> String {
    let bits: u32 = if (0..=0xFF).contains(&value) { 8 } else { 16 };
    let mut s = String::with_capacity(bits as usize + bits as usize / 4);
    for i in (0..bits).rev() {
        if i < bits - 1 && i % 4 == 3 {
            s.push('_');
        }
        s.push(if value & (1 << i) != 0 { '1' } else { '0' });
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_hex_cpc_sizes_small_positive_values_as_8_bit() {
        assert_eq!(format_hex_cpc(0x2A), "&2A");
    }

    #[test]
    fn format_hex_cpc_sizes_larger_values_as_16_bit() {
        assert_eq!(format_hex_cpc(0x1234), "&1234");
    }

    #[test]
    fn format_hex_cpc_sizes_negative_values_by_two_s_complement_not_64_bit() {
        // -1 fits in 16 bits two's complement (&FFFF) - it must not render
        // as a 16-digit 64-bit hex value (the previous no-op-bitmask bug).
        assert_eq!(format_hex_cpc(-1), "&FFFF");
    }

    #[test]
    fn format_number_hover_hex_column_matches_binary_column_width() {
        let md = format_number_hover("x", -1);
        assert!(md.contains("&FFFF"), "{md}");
        assert!(!md.contains("&FFFFFFFFFFFFFFFF"), "{md}");
    }

    /// Regression coverage for delegating to `cpclib_common::parse_value`:
    /// every prefix/suffix convention the narrower reimplementation used to
    /// hand-parse must still resolve to the same value, plus the
    /// underscore-separated groups the old implementation used to reject.
    #[test]
    fn parse_numeric_literal_str_covers_every_cpc_convention() {
        assert_eq!(parse_numeric_literal_str("42"), Some(42));
        assert_eq!(parse_numeric_literal_str("&BB5A"), Some(0xBB5A));
        assert_eq!(parse_numeric_literal_str("$BB5A"), Some(0xBB5A));
        assert_eq!(parse_numeric_literal_str("#BB5A"), Some(0xBB5A));
        assert_eq!(parse_numeric_literal_str("0xBB5A"), Some(0xBB5A));
        assert_eq!(parse_numeric_literal_str("%1010"), Some(0b1010));
        assert_eq!(parse_numeric_literal_str("0b1010"), Some(0b1010));
        assert_eq!(parse_numeric_literal_str("0o17"), Some(0o17));
        // Underscore-separated digit groups - rejected by the old,
        // narrower implementation, now handled the same way `basm` itself
        // parses a source-level literal.
        assert_eq!(parse_numeric_literal_str("&BB_5A"), Some(0xBB5A));
        assert_eq!(parse_numeric_literal_str("1_000"), Some(1000));
        assert_eq!(parse_numeric_literal_str("not a number"), None);
    }
}
