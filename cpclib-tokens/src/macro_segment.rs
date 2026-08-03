use std::ops::Deref;

use cpclib_common::smallvec::SmallVec;
use memchr::memchr;

/// Tokenize a macro body into MacroSegments.
///
/// `has_variadic` gates the two variadic-macro-only forms - `{N}` (a plain
/// 0-based index, referencing the Nth argument actually passed at a call,
/// continuing past the named params) and `{#}` (the total argument count at
/// a call). Both are only recognized when the macro declared a trailing
/// `...` (`MACRO foo(a, b, ...)`) - gating on this keeps every
/// non-variadic macro's existing behavior byte-for-byte unchanged (an
/// unmatched `{key}` still falls through to a literal `Lit` segment either
/// way, so this is purely additive).
pub fn tokenize_macro_body<'l, 'p>(
    listing: &'l str,
    params: &'p [impl AsRef<str> + 'p],
    has_variadic: bool
) -> TokenizedMacroContent {
    let mut segments: SmallVec<[MacroSegment; 8]> = SmallVec::with_capacity(listing.len() / 8);
    let mut cursor = 0;
    let param_names: std::collections::HashMap<&'p str, usize> = params
        .iter()
        .enumerate()
        .map(|(idx, p)| {
            let s: &'p str = p.as_ref();
            let key = if let Some(stripped) = s.strip_prefix("r#") {
                stripped
            }
            else {
                s
            };
            (key, idx)
        })
        .collect();
    let bytes = listing.as_bytes();
    while let Some(rel_open) = memchr(b'{', &bytes[cursor..]) {
        let open = cursor + rel_open;
        if open > cursor {
            segments.push(MacroSegment::Lit {
                start: cursor,
                end: open
            });
        }
        let after_open = open + 1;
        // `{{paramname}` — the outer `{` is a literal (e.g. opening an
        // expression like `{value + 7}`); the inner `{paramname}` is a
        // normal parameter substitution.  Emit the first `{` as a literal
        // and restart processing from the second `{`.
        if bytes.get(after_open) == Some(&b'{') {
            segments.push(MacroSegment::Lit {
                start: open,
                end: after_open
            });
            cursor = after_open;
            continue;
        }
        if let Some(rel_close) = memchr(b'}', &bytes[after_open..]) {
            let close = after_open + rel_close;
            let key = &listing[after_open..close];
            if let Some(&idx) = param_names.get(key) {
                segments.push(MacroSegment::Arg { index: idx });
                cursor = close + 1;
                continue;
            }
            if has_variadic {
                if key == "#" {
                    segments.push(MacroSegment::ArgCount);
                    cursor = close + 1;
                    continue;
                }
                if let Ok(idx) = key.parse::<usize>() {
                    segments.push(MacroSegment::Arg { index: idx });
                    cursor = close + 1;
                    continue;
                }
            }
            segments.push(MacroSegment::Lit {
                start: open,
                end: close + 1
            });
            cursor = close + 1;
        }
        else {
            segments.push(MacroSegment::Lit {
                start: open,
                end: listing.len()
            });
            cursor = listing.len();
        }
    }
    if cursor < listing.len() {
        segments.push(MacroSegment::Lit {
            start: cursor,
            end: listing.len()
        });
    }

    TokenizedMacroContent {
        segments: segments.into_vec()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MacroSegment {
    Lit { start: usize, end: usize },
    Arg { index: usize },
    /// `{#}` in a variadic macro's body - the total number of arguments
    /// actually passed at a given call site.
    ArgCount
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct TokenizedMacroContent {
    pub segments: Vec<MacroSegment>
}

impl Deref for TokenizedMacroContent {
    type Target = [MacroSegment];

    fn deref(&self) -> &Self::Target {
        &self.segments
    }
}

#[cfg(test)]
mod tokenize_macro_body_tests {
    use super::*;

    fn args(segments: &TokenizedMacroContent) -> Vec<MacroSegment> {
        segments.iter().copied().collect()
    }

    #[test]
    fn named_params_are_still_resolved_first_when_variadic() {
        // `a`/`b` are declared params - even in a variadic macro, a `{a}`/
        // `{b}` reference must resolve to the *named* param's index, not be
        // reinterpreted as anything positional.
        let tokenized = tokenize_macro_body("{a}-{b}", &["a", "b"], true);
        assert_eq!(
            args(&tokenized),
            vec![
                MacroSegment::Arg { index: 0 },
                MacroSegment::Lit { start: 3, end: 4 },
                MacroSegment::Arg { index: 1 }
            ]
        );
    }

    #[test]
    fn numeric_placeholder_is_a_positional_arg_only_when_variadic() {
        let variadic = tokenize_macro_body("{2}", &["a", "b"], true);
        assert_eq!(args(&variadic), vec![MacroSegment::Arg { index: 2 }]);

        // Same body, non-variadic macro: `{2}` isn't a declared param name,
        // and the variadic-only numeric fallback must not kick in - stays
        // literal text, exactly like any other unmatched `{key}` always has.
        let non_variadic = tokenize_macro_body("{2}", &["a", "b"], false);
        assert_eq!(
            args(&non_variadic),
            vec![MacroSegment::Lit { start: 0, end: 3 }]
        );
    }

    #[test]
    fn hash_placeholder_is_arg_count_only_when_variadic() {
        let variadic = tokenize_macro_body("{#}", &["a"], true);
        assert_eq!(args(&variadic), vec![MacroSegment::ArgCount]);

        let non_variadic = tokenize_macro_body("{#}", &["a"], false);
        assert_eq!(
            args(&non_variadic),
            vec![MacroSegment::Lit { start: 0, end: 3 }]
        );
    }

    #[test]
    fn a_param_named_hash_wins_over_the_arg_count_special_form() {
        // Pathological but legal today (param names aren't validated
        // through the label parser) - an explicit declared `#` param must
        // still take priority over the variadic `{#}` special case, since
        // named-param resolution happens first.
        let tokenized = tokenize_macro_body("{#}", &["#"], true);
        assert_eq!(args(&tokenized), vec![MacroSegment::Arg { index: 0 }]);
    }

    #[test]
    fn mixed_named_and_positional_references_in_one_body() {
        let tokenized = tokenize_macro_body("{a} {2} {#} {b}", &["a", "b"], true);
        assert_eq!(
            args(&tokenized),
            vec![
                MacroSegment::Arg { index: 0 },
                MacroSegment::Lit { start: 3, end: 4 },
                MacroSegment::Arg { index: 2 },
                MacroSegment::Lit { start: 7, end: 8 },
                MacroSegment::ArgCount,
                MacroSegment::Lit { start: 11, end: 12 },
                MacroSegment::Arg { index: 1 }
            ]
        );
    }

    #[test]
    fn non_numeric_unknown_key_stays_literal_even_when_variadic() {
        let tokenized = tokenize_macro_body("{nope}", &["a"], true);
        assert_eq!(
            args(&tokenized),
            vec![MacroSegment::Lit { start: 0, end: 6 }]
        );
    }

    #[test]
    fn double_brace_escaping_still_works_when_variadic() {
        // `{{2}` - the outer `{` is literal, the inner `{2}` is a normal
        // (here: positional) substitution - unaffected by has_variadic.
        let tokenized = tokenize_macro_body("{{2}", &["a"], true);
        assert_eq!(
            args(&tokenized),
            vec![
                MacroSegment::Lit { start: 0, end: 1 },
                MacroSegment::Arg { index: 2 }
            ]
        );
    }

    #[test]
    fn a_variadic_macro_with_no_named_params_only_uses_positional_refs() {
        let tokenized = tokenize_macro_body("{0}, {1}, {#}", &[] as &[&str], true);
        assert_eq!(
            args(&tokenized),
            vec![
                MacroSegment::Arg { index: 0 },
                MacroSegment::Lit { start: 3, end: 5 },
                MacroSegment::Arg { index: 1 },
                MacroSegment::Lit { start: 8, end: 10 },
                MacroSegment::ArgCount
            ]
        );
    }

    #[test]
    fn negative_or_malformed_numeric_keys_stay_literal() {
        // `-1` doesn't parse as a `usize` - falls through to literal, same
        // as any other unmatched key.
        let tokenized = tokenize_macro_body("{-1}", &["a"], true);
        assert_eq!(
            args(&tokenized),
            vec![MacroSegment::Lit { start: 0, end: 4 }]
        );
    }
}
