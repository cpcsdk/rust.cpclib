use std::ops::Deref;

use cpclib_common::smallvec::SmallVec;
use memchr::memchr;

/// Find the `]` that closes the `[` already consumed right before `bytes`
/// starts (i.e. `bytes[0]` is the first byte *after* that `[`) - tracking
/// `[`/`]` depth so a nested bracket (e.g. a literal list-of-indices,
/// `{*[[0, 2]]}`) doesn't prematurely end the outer one. Returns the
/// position of the matching `]`, relative to `bytes`, or `None` if it is
/// never closed.
fn find_matching_closing_bracket(bytes: &[u8]) -> Option<usize> {
    let mut depth = 1i32;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            },
            _ => {}
        }
    }
    None
}

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
        // `{*}` / `{*[indexes]}` - handled before the generic scan below,
        // since `indexes` may itself contain nested `{...}` placeholders
        // (e.g. `{*[{#}-1]}`) whose own `}` would otherwise be mistaken by
        // a flat `memchr(b'}', ...)` for the end of *this* placeholder.
        if bytes.get(after_open) == Some(&b'*') {
            let after_star = after_open + 1;
            if bytes.get(after_star) == Some(&b'}') {
                segments.push(MacroSegment::AllArgs {
                    followed_by_bracket: bytes.get(after_star + 1) == Some(&b'[')
                });
                cursor = after_star + 1;
                continue;
            }
            if bytes.get(after_star) == Some(&b'[') {
                let index_start = after_star + 1;
                if let Some(rel_index_end) =
                    find_matching_closing_bracket(&bytes[index_start..])
                {
                    let index_end = index_start + rel_index_end;
                    if bytes.get(index_end + 1) == Some(&b'}') {
                        segments.push(MacroSegment::SelectedArgs {
                            start: index_start,
                            end: index_end,
                            followed_by_bracket: bytes.get(index_end + 2) == Some(&b'[')
                        });
                        cursor = index_end + 2;
                        continue;
                    }
                }
                // Malformed (`]` never balanced, or not immediately
                // followed by `}`) - fall through to the generic scan
                // below, same treatment as any other unrecognized `{...}`.
            }
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
            let followed_by_bracket = bytes.get(close + 1) == Some(&b'[');
            if let Some(&idx) = param_names.get(key) {
                if let Some((start, end)) = default {
                    segments.push(MacroSegment::ArgOr {
                        index: idx,
                        start,
                        end,
                        followed_by_bracket
                    });
                    cursor = close + 1;
                    continue;
                }
                segments.push(MacroSegment::Arg {
                    index: idx,
                    followed_by_bracket
                });
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
                                end,
                                followed_by_bracket
                            }
                        },
                        None => {
                            MacroSegment::Arg {
                                index: idx,
                                followed_by_bracket
                            }
                        }
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
    /// `followed_by_bracket` - the macro body's own next byte after this
    /// placeholder's closing `}` is a literal `[` (e.g. `{l}[{idx}]`),
    /// computed once here from the body text alone, independent of any
    /// particular call. A call whose argument at `index` is itself a list
    /// (`GET([10,20,30], 1)`, as opposed to a plain identifier/expression
    /// that merely evaluates to one) needs its substitution re-wrapped in
    /// `[...]` for the following `[...]` to index into it, rather than the
    /// flattened, bracket-free comma list substitution normally used to
    /// spread a list argument across a `DB`/`DW` line - see
    /// `expand_param`/`finish_expand_for_basm` in
    /// `cpclib-asm/src/assembler/macro.rs`, which decide the actual
    /// wrapping (this flag alone is not sufficient - it also needs to know
    /// whether the call's argument is a list, a call-time property this
    /// body-only tokenizer has no access to).
    Arg { index: usize, followed_by_bracket: bool },
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
    /// `followed_by_bracket` - see [`MacroSegment::Arg`].
    ArgOr {
        index: usize,
        start: usize,
        end: usize,
        followed_by_bracket: bool
    },
    /// `{#}` in a variadic macro's body - the total number of arguments
    /// actually passed at a given call site.
    ArgCount,
    /// `{*}` - every argument actually passed at a given call site,
    /// expanded and joined with `,` (like `DB {list_arg}`'s own flat
    /// spreading, but for *all* of the call's arguments rather than one
    /// list-valued one). Unlike `{N}`/`{#}`, available unconditionally
    /// (not gated on `has_variadic`) - `*` can never collide with a
    /// declared parameter name.
    ///
    /// `followed_by_bracket` - as [`MacroSegment::Arg`], but simpler:
    /// `{*}`/`{*[indexes]}` always produce a list-shaped (possibly
    /// single-element) comma list, unlike a plain `{name}` which might be
    /// a scalar - so unlike `Arg`/`ArgOr`, wrapping the substitution in
    /// `[...]` when this is set is unconditional, no further "is the
    /// argument actually a list" check needed at expansion time.
    AllArgs { followed_by_bracket: bool },
    /// `{*[indexes]}` - a *subset* of the call's arguments, selected by
    /// `indexes` (a range or a list of indices), expanded and joined with
    /// `,` in the order given. `start`/`end` bound the raw index-expression
    /// text inside the brackets (like `ArgOr`'s default text) - it is
    /// **not** evaluated here: it may itself reference other placeholders
    /// (`{#}`, a named parameter, ...), which only have values once a
    /// specific call is being expanded, so evaluation is deferred to
    /// `cpclib-asm/src/assembler/macro.rs`'s expansion code, not decided by
    /// this body-only tokenizer. `followed_by_bracket` - see [`AllArgs`](Self::AllArgs).
    SelectedArgs {
        start: usize,
        end: usize,
        followed_by_bracket: bool
    }
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
        let MacroSegment::ArgOr { index, start, end, .. } = segments[1]
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
        let MacroSegment::ArgOr { index, start, end, .. } = args(&tokenized)[1]
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
        assert_eq!(
            args(&tokenized)[1],
            MacroSegment::Arg {
                index: 3,
                followed_by_bracket: false
            }
        );
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
                MacroSegment::Arg {
                    index: 0,
                    followed_by_bracket: false
                },
                MacroSegment::Lit { start: 3, end: 4 },
                MacroSegment::Arg {
                    index: 1,
                    followed_by_bracket: false
                }
            ]
        );
    }

    #[test]
    fn numeric_placeholder_is_a_positional_arg_only_when_variadic() {
        let variadic = tokenize_macro_body("{2}", &["a", "b"], true);
        assert_eq!(
            args(&variadic),
            vec![MacroSegment::Arg {
                index: 2,
                followed_by_bracket: false
            }]
        );

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
        assert_eq!(
            args(&tokenized),
            vec![MacroSegment::Arg {
                index: 0,
                followed_by_bracket: false
            }]
        );
    }

    #[test]
    fn mixed_named_and_positional_references_in_one_body() {
        let tokenized = tokenize_macro_body("{a} {2} {#} {b}", &["a", "b"], true);
        assert_eq!(
            args(&tokenized),
            vec![
                MacroSegment::Arg {
                    index: 0,
                    followed_by_bracket: false
                },
                MacroSegment::Lit { start: 3, end: 4 },
                MacroSegment::Arg {
                    index: 2,
                    followed_by_bracket: false
                },
                MacroSegment::Lit { start: 7, end: 8 },
                MacroSegment::ArgCount,
                MacroSegment::Lit { start: 11, end: 12 },
                MacroSegment::Arg {
                    index: 1,
                    followed_by_bracket: false
                }
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
                MacroSegment::Arg {
                    index: 2,
                    followed_by_bracket: false
                }
            ]
        );
    }

    #[test]
    fn a_variadic_macro_with_no_named_params_only_uses_positional_refs() {
        let tokenized = tokenize_macro_body("{0}, {1}, {#}", &[] as &[&str], true);
        assert_eq!(
            args(&tokenized),
            vec![
                MacroSegment::Arg {
                    index: 0,
                    followed_by_bracket: false
                },
                MacroSegment::Lit { start: 3, end: 5 },
                MacroSegment::Arg {
                    index: 1,
                    followed_by_bracket: false
                },
                MacroSegment::Lit { start: 8, end: 10 },
                MacroSegment::ArgCount
            ]
        );
    }

    /// The new, actual feature: `{l}[{idx}]` - a list-valued argument
    /// followed immediately by `[` needs re-wrapping in brackets at
    /// expansion time (see `cpclib-asm/src/assembler/macro.rs`) so the
    /// following `[...]` indexes into it rather than parsing as a comma
    /// list spliced into the surrounding statement. That decision needs to
    /// know the body's own next byte, computed here once regardless of
    /// what any particular call passes.
    #[test]
    fn a_reference_immediately_followed_by_a_bracket_is_flagged() {
        let tokenized = tokenize_macro_body("db {l}[{idx}]", &["l", "idx"], false);
        assert_eq!(
            args(&tokenized),
            vec![
                MacroSegment::Lit { start: 0, end: 3 },
                MacroSegment::Arg {
                    index: 0,
                    followed_by_bracket: true
                },
                MacroSegment::Lit { start: 6, end: 7 },
                MacroSegment::Arg {
                    index: 1,
                    followed_by_bracket: false
                },
                MacroSegment::Lit { start: 12, end: 13 }
            ]
        );
    }

    /// A default-carrying reference (`ArgOr`) gets the same flag.
    #[test]
    fn a_default_reference_immediately_followed_by_a_bracket_is_flagged() {
        let tokenized = tokenize_macro_body("db {l:=[]}[0]", &["l"], false);
        let MacroSegment::ArgOr {
            followed_by_bracket,
            ..
        } = args(&tokenized)[1]
        else {
            panic!("expected an ArgOr, got {:?}", args(&tokenized)[1]);
        };
        assert!(followed_by_bracket);
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

    /// `{*}` - all arguments, joined with `,`. Unlike `{N}`/`{#}`, available
    /// even when the macro is not variadic - `*` cannot collide with a
    /// declared parameter name.
    #[test]
    fn star_alone_is_all_args_non_variadic_too() {
        let tokenized = tokenize_macro_body("db {*}", &["a"], false);
        assert_eq!(
            args(&tokenized),
            vec![
                MacroSegment::Lit { start: 0, end: 3 },
                MacroSegment::AllArgs {
                    followed_by_bracket: false
                }
            ]
        );
    }

    /// `{*}[i]` - indexing straight into the spread of every argument.
    #[test]
    fn star_alone_immediately_followed_by_a_bracket_is_flagged() {
        let tokenized = tokenize_macro_body("db {*}[0]", &[] as &[&str], false);
        assert_eq!(
            args(&tokenized)[1],
            MacroSegment::AllArgs {
                followed_by_bracket: true
            }
        );
    }

    /// `{*[indexes]}` - the raw text between the brackets is captured, not
    /// evaluated (see `MacroSegment::SelectedArgs`'s own doc comment).
    #[test]
    fn star_with_a_range_selector_captures_the_raw_index_text() {
        let body = "db {*[0..2]}";
        let tokenized = tokenize_macro_body(body, &[] as &[&str], false);
        assert_eq!(
            args(&tokenized),
            vec![
                MacroSegment::Lit { start: 0, end: 3 },
                MacroSegment::SelectedArgs {
                    start: 6,
                    end: 10,
                    followed_by_bracket: false
                }
            ]
        );
        let MacroSegment::SelectedArgs { start, end, .. } = args(&tokenized)[1]
        else {
            unreachable!()
        };
        assert_eq!(&body[start..end], "0..2");
    }

    /// `{*[indexes]}[i]` - indexing straight into the selected subset.
    #[test]
    fn star_selector_immediately_followed_by_a_bracket_is_flagged() {
        let tokenized = tokenize_macro_body("db {*[0..2]}[0]", &[] as &[&str], false);
        let MacroSegment::SelectedArgs {
            followed_by_bracket,
            ..
        } = args(&tokenized)[1]
        else {
            panic!("expected a SelectedArgs, got {:?}", args(&tokenized)[1]);
        };
        assert!(followed_by_bracket);
    }

    /// A literal list-of-indices selector (`[[0, 2]]`) nests brackets inside
    /// the outer `{*[...]}` - the matching-`]` scan must track that depth,
    /// not stop at the first `]` it sees.
    #[test]
    fn star_with_a_nested_bracket_list_selector_tracks_bracket_depth() {
        let body = "db {*[[0, 2]]}";
        let tokenized = tokenize_macro_body(body, &[] as &[&str], false);
        let MacroSegment::SelectedArgs { start, end, .. } = args(&tokenized)[1]
        else {
            panic!("expected a SelectedArgs, got {:?}", args(&tokenized)[1]);
        };
        assert_eq!(&body[start..end], "[0, 2]");
    }

    /// The index expression can itself contain a nested `{...}` placeholder
    /// (e.g. `{#}`) - its own `}` must not be mistaken for the end of the
    /// outer `{*[...]}`.
    #[test]
    fn star_selector_can_contain_a_nested_placeholder() {
        let body = "db {*[{#}-1]}";
        let tokenized = tokenize_macro_body(body, &[] as &[&str], true);
        let MacroSegment::SelectedArgs { start, end, .. } = args(&tokenized)[1]
        else {
            panic!("expected a SelectedArgs, got {:?}", args(&tokenized)[1]);
        };
        assert_eq!(&body[start..end], "{#}-1");
    }

    /// An unclosed `{*[...` (no matching `]`) is not a valid selector -
    /// falls through to a literal, same treatment as any other malformed
    /// `{...}`.
    #[test]
    fn an_unclosed_star_selector_stays_literal() {
        let tokenized = tokenize_macro_body("{*[abc}", &[] as &[&str], false);
        assert_eq!(
            args(&tokenized),
            vec![MacroSegment::Lit { start: 0, end: 7 }]
        );
    }
}
