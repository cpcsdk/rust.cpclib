//! `; fmt: off` / `; fmt: on` - an explicit escape hatch, the same idea every
//! mature formatter gives its users (rustfmt's `#[rustfmt::skip]`,
//! clang-format's `// clang-format off/on`, prettier's `// prettier-ignore`):
//! heuristics can't perfectly infer every author's intentional layout, so
//! rather than keep growing special cases to guess right more often, give
//! the author a way to say "leave this exactly as written" and stop
//! guessing at all for that region.
//!
//! Concretely for this crate: a hand-aligned data table, a `FONT_CREATE_CHAR`
//! call whose string-literal arguments are laid out to look like the glyph
//! they encode, or any block whose whitespace carries meaning a formatter has
//! no way to infer - wrap it in the two markers and it passes through
//! completely unchanged, comments and all, rather than hoping the rest of
//! this formatter's heuristics land on the same answer the author had in
//! mind.

/// A marker comment line, trimmed and normalized: `; fmt: off`, `;fmt:off`,
/// `; FMT: OFF`, `;   fmt   :   off` all match - anything else does not.
/// `None` when `line` isn't one of the two markers at all.
fn marker(line: &str) -> Option<bool> {
    let rest = line.trim().strip_prefix(';')?.trim();
    let rest = rest.strip_prefix("fmt").or_else(|| rest.strip_prefix("FMT"))?.trim_start();
    let rest = rest.strip_prefix(':')?.trim();
    match rest.to_ascii_lowercase().as_str() {
        "off" => Some(false),
        "on" => Some(true),
        _ => None
    }
}

/// Every 0-based, inclusive `(start, end)` line range in `source` between a
/// `; fmt: off` and its next `; fmt: on` (both marker lines are themselves
/// part of the range - they get passed through verbatim too, so the pair
/// stays visibly matched in the output rather than one of them being
/// reformatted away). An `off` with no matching `on` before end of file
/// disables formatting for the rest of the file - the same "you forgot to
/// turn it back on" behavior every other formatter with this feature has,
/// rather than silently ignoring an unclosed marker.  A stray `on` with no
/// preceding `off` is a no-op, not an error.
pub(super) fn disabled_ranges(source: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut open: Option<usize> = None;
    for (i, line) in source.lines().enumerate() {
        match marker(line) {
            // A second `off` before the matching `on` keeps the first (the
            // range covers from the earliest `off`, not the latest).
            Some(false) => {
                open.get_or_insert(i);
            },
            Some(true) => {
                if let Some(start) = open.take() {
                    ranges.push((start, i));
                }
            },
            None => {}
        }
    }
    if let Some(start) = open {
        let last_line = source.lines().count().saturating_sub(1);
        ranges.push((start, last_line));
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_the_marker_in_any_reasonable_spelling() {
        for off in [
            "; fmt: off",
            ";fmt:off",
            "; FMT: OFF",
            "  ; fmt : off  ",
            ";  fmt:off"
        ] {
            assert_eq!(marker(off), Some(false), "{off:?}");
        }
        for on in ["; fmt: on", ";fmt:on", "; FMT: ON"] {
            assert_eq!(marker(on), Some(true), "{on:?}");
        }
    }

    #[test]
    fn ignores_ordinary_comments_and_code() {
        assert_eq!(marker("; a normal comment"), None);
        assert_eq!(marker("; fmt: sideways"), None);
        assert_eq!(marker("    ld a,0"), None);
        assert_eq!(marker(""), None);
    }

    #[test]
    fn finds_one_matched_range() {
        let src = "a\n; fmt: off\nb\nc\n; fmt: on\nd\n";
        assert_eq!(disabled_ranges(src), vec![(1, 4)]);
    }

    #[test]
    fn an_unclosed_off_disables_to_end_of_file() {
        let src = "a\n; fmt: off\nb\nc\n";
        assert_eq!(disabled_ranges(src), vec![(1, 3)]);
    }

    #[test]
    fn a_stray_on_with_no_preceding_off_is_a_no_op() {
        let src = "a\n; fmt: on\nb\n";
        assert_eq!(disabled_ranges(src), vec![]);
    }

    #[test]
    fn two_separate_ranges_are_both_found() {
        let src = "; fmt: off\na\n; fmt: on\nb\n; fmt: off\nc\n; fmt: on\nd\n";
        assert_eq!(disabled_ranges(src), vec![(0, 2), (4, 6)]);
    }
}
