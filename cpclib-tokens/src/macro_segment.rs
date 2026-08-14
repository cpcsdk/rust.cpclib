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
            let raw = &listing[after_open..close];
            // `{key:=default}` - the default is used only when the call did
            // not supply that argument. `:=` cannot collide with anything
            // that works today: a key containing it matches no parameter name
            // and parses as no number, so such a `{...}` already fell through
            // to a literal.
            let (key, default) = match raw.find(":=") {
                Some(at) => {
                    (
                        &raw[..at],
                        Some((after_open + at + 2, close))
                    )
                },
                None => (raw, None)
            };
            if let Some(&idx) = param_names.get(key) {
                if let Some((start, end)) = default {
                    segments.push(MacroSegment::ArgOr {
                        index: idx,
                        start,
                        end
                    });
                    cursor = close + 1;
                    continue;
                }
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
                    segments.push(match default {
                        Some((start, end)) => {
                            MacroSegment::ArgOr {
                                index: idx,
                                start,
                                end
                            }
                        },
                        None => MacroSegment::Arg { index: idx }
                    });
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
    /// `{N:=text}` - argument `index`, or `text` when this call did not
    /// supply one.
    ///
    /// A variadic macro's body legitimately references arguments that only
    /// *some* calls pass, and the reference is often inside a branch those
    /// calls never take:
    ///
    /// ```text
    /// switch {kind}
    ///     case EVENT_CHANGE_PALETTE
    ///         dw {3}          ; only this branch uses a 3rd argument
    /// ```
    ///
    /// Expansion happens before the Z80 parser runs and cannot know which
    /// branch will be taken, so `{3}` alone is an error for every call that
    /// passes two arguments. `{3:=0}` says what to put there instead.
    ///
    /// `start`/`end` bound the default text inside the macro body, like
    /// [`MacroSegment::Lit`]; it is emitted verbatim, never re-expanded.
    ArgOr {
        index: usize,
        start: usize,
        end: usize
    },
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

    /// `{N:=default}` records where the default text lives, so expansion can
    /// use it when the call supplies no such argument.
    #[test]
    fn a_positional_reference_can_carry_a_default() {
        let body = "dw {3:=0}";
        let tokenized = tokenize_macro_body(body, &["a", "b"], true);
        let segments = args(&tokenized);
        assert_eq!(segments.len(), 2, "{segments:?}");
        let MacroSegment::ArgOr { index, start, end } = segments[1]
        else {
            panic!("expected an ArgOr, got {:?}", segments[1]);
        };
        assert_eq!(index, 3);
        assert_eq!(&body[start..end], "0");
    }

    /// The default may be any text, including an expression with spaces - it
    /// is spliced into the body verbatim.
    #[test]
    fn a_default_may_be_an_arbitrary_expression() {
        let body = "dw {3:=SOME_LABEL + 2}";
        let tokenized = tokenize_macro_body(body, &[] as &[&str], true);
        let MacroSegment::ArgOr { start, end, .. } = args(&tokenized)[1]
        else {
            panic!("expected an ArgOr")
        };
        assert_eq!(&body[start..end], "SOME_LABEL + 2");
    }

    /// Named parameters take a default the same way.
    #[test]
    fn a_named_reference_can_carry_a_default_too() {
        let body = "dw {b:=7}";
        let tokenized = tokenize_macro_body(body, &["a", "b"], false);
        let MacroSegment::ArgOr { index, start, end } = args(&tokenized)[1]
        else {
            panic!("expected an ArgOr")
        };
        assert_eq!(index, 1);
        assert_eq!(&body[start..end], "7");
    }

    /// Without a default the segment is unchanged - every macro that works
    /// today keeps tokenizing exactly as it did.
    #[test]
    fn a_reference_without_a_default_is_untouched() {
        let tokenized = tokenize_macro_body("dw {3}", &[] as &[&str], true);
        assert_eq!(args(&tokenized)[1], MacroSegment::Arg { index: 3 });
    }

    /// `:=` inside a `{...}` that names nothing is still a literal, as it was
    /// before this existed - so no macro can change meaning by accident.
    #[test]
    fn a_default_on_an_unknown_key_stays_literal() {
        let body = "dw {nope:=0}";
        let tokenized = tokenize_macro_body(body, &["a"], false);
        assert!(
            args(&tokenized)
                .iter()
                .all(|s| matches!(s, MacroSegment::Lit { .. })),
            "{:?}",
            args(&tokenized)
        );
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
