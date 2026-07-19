//! The Amstrad CPC's fixed 27-ink palette (+5 firmware duplicates), used to
//! render `textDocument/documentColor` swatches for ink/palette values in
//! basm (`SNASET GA_PAL:n, value`) and Locomotive BASIC (`INK`/`BORDER`).
//!
//! These two tables mirror `cpclib-image/src/ink.rs`'s `INKS_RGB_VALUES` and
//! `INKS_GA_VALUE` exactly (duplicated here, as plain tuples, rather than
//! depending on the `cpclib-image` crate — which pulls in `image`/`gif`/
//! `rayon` for what is otherwise two small constant lookup tables). Keep in
//! sync with that file if the CPC palette ever changes (it won't).

/// RGB value of each of the 27 inks, plus 5 firmware-numbering duplicates
/// (indices 27-31), indexed by ink number (0-31).
pub const INK_RGB: [(u8, u8, u8); 32] = [
    (0x00, 0x00, 0x00), // 0
    (0x00, 0x00, 0x80), // 1
    (0x00, 0x00, 0xFF), // 2
    (0x80, 0x00, 0x00), // 3
    (0x80, 0x00, 0x80), // 4
    (0x80, 0x00, 0xFF), // 5
    (0xFF, 0x00, 0x00), // 6
    (0xFF, 0x00, 0x80), // 7
    (0xFF, 0x00, 0xFF), // 8
    (0x00, 0x80, 0x00), // 9
    (0x00, 0x80, 0x80), // 10
    (0x00, 0x80, 0xFF), // 11
    (0x80, 0x80, 0x00), // 12
    (0x80, 0x80, 0x80), // 13
    (0x80, 0x80, 0xFF), // 14
    (0xFF, 0x80, 0x00), // 15
    (0xFF, 0x80, 0x80), // 16
    (0xFF, 0x80, 0xFF), // 17
    (0x00, 0xFF, 0x00), // 18
    (0x00, 0xFF, 0x80), // 19
    (0x00, 0xFF, 0xFF), // 20
    (0x80, 0xFF, 0x00), // 21
    (0x80, 0xFF, 0x80), // 22
    (0x80, 0xFF, 0xFF), // 23
    (0xFF, 0xFF, 0x00), // 24
    (0xFF, 0xFF, 0x80), // 25
    (0xFF, 0xFF, 0xFF), // 26
    (0x80, 0x80, 0x80), // 27 => ink 13
    (0xFF, 0x00, 0x80), // 28 => ink 7
    (0xFF, 0xFF, 0x80), // 29 => ink 25
    (0x00, 0x00, 0x80), // 30 => ink 1
    (0x00, 0xFF, 0x80)  // 31 => ink 19
];

/// The Gate Array 8-bit value that selects each ink, indexed by ink number
/// (0-31) — the inverse of this table (`ink_index_from_ga_value`) maps a
/// `SNASET GA_PAL:n, value`-style byte back to an ink index.
pub const INK_GA_VALUE: [u8; 32] = [
    0x54, 0x44, 0x55, 0x5C, 0x58, 0x5D, 0x4C, 0x45, 0x4D, 0x56, 0x46, 0x57, 0x5E, 0x40, 0x5F, 0x4E,
    0x47, 0x4F, 0x52, 0x42, 0x53, 0x5A, 0x59, 0x5B, 0x4A, 0x43, 0x4B, 0x41, 0x48, 0x49, 0x50, 0x51
];

/// Reverse lookup: Gate Array byte value → ink index (0-31), or `None` if
/// `value` is not a valid GA ink-select byte.
pub fn ink_index_from_ga_value(value: u8) -> Option<usize> {
    INK_GA_VALUE.iter().position(|&v| v == value)
}

/// RGB for ink index `idx` (0-31), or `None` if out of range.
pub fn ink_rgb(idx: usize) -> Option<(u8, u8, u8)> {
    INK_RGB.get(idx).copied()
}

/// Convert an 8-bit RGB triple to an LSP `Color` (components in `0.0..=1.0`).
pub fn to_lsp_color((r, g, b): (u8, u8, u8)) -> tower_lsp::lsp_types::Color {
    tower_lsp::lsp_types::Color {
        red: r as f32 / 255.0,
        green: g as f32 / 255.0,
        blue: b as f32 / 255.0,
        alpha: 1.0
    }
}

/// Convert an LSP `Color` (components in `0.0..=1.0`) to an 8-bit RGB triple.
pub fn from_lsp_color(c: tower_lsp::lsp_types::Color) -> (u8, u8, u8) {
    (
        (c.red.clamp(0.0, 1.0) * 255.0).round() as u8,
        (c.green.clamp(0.0, 1.0) * 255.0).round() as u8,
        (c.blue.clamp(0.0, 1.0) * 255.0).round() as u8
    )
}

/// The 27 canonical CPC ink indices (0-26 — the 27-31 range are firmware
/// duplicates of an earlier index's *color*, though each is still a
/// distinct GA byte, so excluded here), sorted by RGB distance to `target`,
/// closest first. Used to turn a client's continuous color picker into a
/// discrete "which CPC ink did you mean" list via
/// `textDocument/colorPresentation`, for a language (Locomotive BASIC's
/// `INK`/`BORDER`) whose value only ever accepts 0-26.
pub fn inks_by_distance(target: (u8, u8, u8)) -> Vec<usize> {
    inks_by_distance_in(target, 0..27)
}

/// Same as [`inks_by_distance`], but over all 32 GA byte encodings
/// (0-31, duplicates included) — for basm, where each of the 32 is a
/// distinct, individually valid `SNASET GA_PAL:n` value even when a few
/// happen to select the same RGB color as an earlier index.
pub fn inks_by_distance_including_duplicates(target: (u8, u8, u8)) -> Vec<usize> {
    inks_by_distance_in(target, 0..32)
}

fn inks_by_distance_in(target: (u8, u8, u8), range: std::ops::Range<usize>) -> Vec<usize> {
    let mut idxs: Vec<usize> = range.collect();
    idxs.sort_by_key(|&i| {
        let (r, g, b) = INK_RGB[i];
        let dr = r as i32 - target.0 as i32;
        let dg = g as i32 - target.1 as i32;
        let db = b as i32 - target.2 as i32;
        dr * dr + dg * dg + db * db
    });
    idxs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ga_value_reverse_lookup_matches_the_forward_table() {
        for (idx, &ga) in INK_GA_VALUE.iter().enumerate() {
            assert_eq!(ink_index_from_ga_value(ga), Some(idx));
        }
    }

    #[test]
    fn unknown_ga_value_resolves_to_nothing() {
        assert_eq!(ink_index_from_ga_value(0x00), None);
    }

    #[test]
    fn duplicated_firmware_indices_match_their_canonical_ink() {
        assert_eq!(INK_RGB[27], INK_RGB[13]);
        assert_eq!(INK_RGB[28], INK_RGB[7]);
        assert_eq!(INK_RGB[29], INK_RGB[25]);
        assert_eq!(INK_RGB[30], INK_RGB[1]);
        assert_eq!(INK_RGB[31], INK_RGB[19]);
    }

    #[test]
    fn inks_by_distance_puts_the_exact_match_first() {
        let order = inks_by_distance(INK_RGB[5]);
        assert_eq!(order[0], 5);
    }

    #[test]
    fn inks_by_distance_only_covers_the_27_canonical_inks() {
        let order = inks_by_distance((0, 0, 0));
        assert_eq!(order.len(), 27);
        assert!(order.iter().all(|&i| i < 27));
    }

    #[test]
    fn lsp_color_round_trips_through_from_and_to() {
        let rgb = (0x80, 0x40, 0xFF);
        let back = from_lsp_color(to_lsp_color(rgb));
        assert_eq!(back, rgb);
    }
}
