//! Detect macro/function parameters and REPEAT/ITERATE/FOR loop counters
//! that are declared but never referenced in their own body. The detection
//! logic lives here (not in `cpclib-lsp`) so any consumer - currently the
//! LSP, potentially `basm` later - can reuse it without recoding it.

use std::collections::HashSet;

use cpclib_tokens::ListingElement;
use cpclib_tokens::macro_segment::{MacroSegment, tokenize_macro_body};

use crate::parser::obtained::MayHaveSpan;

/// What kind of declared-but-unused binding this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnusedBindingKind {
    MacroParameter,
    FunctionParameter,
    RepeatCounter,
    IterateCounter,
    ForCounter
}

/// One declared-but-never-referenced macro/function parameter or
/// REPEAT/ITERATE/FOR loop counter.
#[derive(Debug, Clone)]
pub struct UnusedBinding {
    pub name: String,
    pub kind: UnusedBindingKind,
    /// 1-based line/column of the *owning definition's own* span (matching
    /// `Z80Span::relative_line_and_column()`'s convention), and that span's
    /// byte length - anchors a diagnostic on the whole definition, not a
    /// precise per-parameter column within its header (a possible later
    /// refinement, not required for a correct first pass).
    pub line: usize,
    pub column: usize,
    pub len: usize,
    /// The removable `", countername"` clause's own `(line, column, len)`
    /// (same convention as this struct's own fields above) - only ever
    /// `Some` for a REPEAT's *optional* counter with no explicit start/step
    /// value, the one case removing it is both syntactically safe (REPEAT
    /// with no counter at all is valid) and confined to a single, easily
    /// located span. `None` for every other kind: ITERATE/FOR require a
    /// counter syntactically (nothing to remove); a REPEAT counter with an
    /// explicit start/step is left for a later pass rather than risk
    /// mis-locating a multi-clause removal; MACRO/FUNCTION parameters can't
    /// be removed without also rewriting every call site, a separate,
    /// larger feature (tracked, not forgotten - see the LSP roadmap).
    pub removable_clause: Option<(usize, usize, usize)>
}

/// Recursively find every unused binding in `listing`, including inside
/// nested blocks (IF/MODULE/WHILE/RORG/FOR/ITERATE/SWITCH/CONFINED/crunched
/// sections/assembler-control, and FUNCTION/REPEAT/ITERATE bodies
/// themselves, since these can nest). A MACRO's own body is a leaf here: it
/// is raw, unparsed text (see `check_macro`), so nothing can be *discovered*
/// nested inside one by this token-level walk.
pub fn find_unused_bindings<'a, T>(listing: impl IntoIterator<Item = &'a T>) -> Vec<UnusedBinding>
where T: MayHaveSpan + ListingElement + 'a {
    let mut out = Vec::new();
    for token in listing {
        walk(token, &mut out);
    }
    out
}

fn walk<T>(token: &T, out: &mut Vec<UnusedBinding>)
where T: MayHaveSpan + ListingElement {
    if token.is_macro_definition() {
        check_macro(token, out);
        return;
    }
    if token.is_function_definition() {
        check_function(token, out);
        for inner in token.function_definition_inner() {
            walk(inner, out);
        }
        return;
    }
    if token.is_repeat() {
        check_repeat(token, out);
        for inner in token.repeat_listing() {
            walk(inner, out);
        }
        return;
    }
    if token.is_repeat_until() {
        for inner in token.repeat_until_listing() {
            walk(inner, out);
        }
        return;
    }
    if token.is_iterate() {
        check_iterate(token, out);
        for inner in token.iterate_listing() {
            walk(inner, out);
        }
        return;
    }
    if token.is_for() {
        check_for(token, out);
        for inner in token.for_listing() {
            walk(inner, out);
        }
        return;
    }
    if token.is_module() {
        for inner in token.module_listing() {
            walk(inner, out);
        }
        return;
    }
    if token.is_if() {
        for i in 0..token.if_nb_tests() {
            for inner in token.if_test(i).1 {
                walk(inner, out);
            }
        }
        if let Some(else_listing) = token.if_else() {
            for inner in else_listing {
                walk(inner, out);
            }
        }
        return;
    }
    if token.is_while() {
        for inner in token.while_listing() {
            walk(inner, out);
        }
        return;
    }
    if token.is_rorg() {
        for inner in token.rorg_listing() {
            walk(inner, out);
        }
        return;
    }
    if token.is_switch() {
        let cases: Vec<_> = token.switch_cases().collect();
        for (_, listing, _) in cases {
            for inner in listing {
                walk(inner, out);
            }
        }
        if let Some(default_listing) = token.switch_default() {
            for inner in default_listing {
                walk(inner, out);
            }
        }
        return;
    }
    if token.is_confined() {
        for inner in token.confined_listing() {
            walk(inner, out);
        }
        return;
    }
    if token.is_crunched_section() {
        for inner in token.crunched_section_listing() {
            walk(inner, out);
        }
        return;
    }
    if token.is_assembler_control() {
        for inner in token.assembler_control_get_listing() {
            walk(inner, out);
        }
    }
}

/// `(line, column, len)` for the definition's own *header line only* - the
/// span itself covers the whole construct (header through its
/// `ENDM`/`ENDR`/etc., often many lines: `into_located_token_between` builds
/// it that way, see this module's own doc comment), so `len` here is
/// deliberately the first line's own length, not the whole span's - a
/// caller turning this into a single-line diagnostic range needs a length
/// that actually fits on `line`.
fn definition_location<T: MayHaveSpan>(token: &T) -> (usize, usize, usize) {
    let span = token.span();
    let (line, column) = span.relative_line_and_column();
    let text: &str = span.as_ref();
    let header_len = text.lines().next().map(str::len).unwrap_or(0);
    (line, column, header_len)
}

/// Does `text` reference `name` the way a macro/function parameter or loop
/// counter is referenced - a literal `{name}` occurrence, per basm's own
/// substitution convention (see this module's own doc comment). Reuses the
/// parser's own tokenizer rather than a hand-rolled substring search, so
/// this can never drift from what basm itself actually recognizes as a
/// reference.
fn references(text: &str, name: &str) -> bool {
    tokenize_macro_body(text, &[name])
        .iter()
        .any(|segment| matches!(segment, MacroSegment::Arg { .. }))
}

/// Which of `params`' indices are never referenced (as a literal
/// `{paramname}`) in `tokenized`. The shared core both this crate's own
/// static per-listing walk (`check_macro` below, backing
/// `find_unused_bindings`, used by e.g. the LSP) and the real assembler's
/// own real-time warning (`Env::visit_macro_definition`, which already
/// computes a `TokenizedMacroContent` for every macro definition it sees)
/// call - so the two can never disagree about what counts as "used": they
/// run the exact same check, not two independently-written ones that
/// merely intend to agree.
pub fn unused_macro_parameter_indices(
    params: &[impl AsRef<str>],
    tokenized: &cpclib_tokens::macro_segment::TokenizedMacroContent
) -> Vec<usize> {
    let used: HashSet<usize> = tokenized
        .iter()
        .filter_map(|segment| {
            match segment {
                MacroSegment::Arg { index } => Some(*index),
                _ => None
            }
        })
        .collect();
    (0..params.len())
        .filter(|index| !used.contains(index))
        .collect()
}

fn check_macro<T: MayHaveSpan + ListingElement>(token: &T, out: &mut Vec<UnusedBinding>) {
    let params = token.macro_definition_arguments();
    if params.is_empty() {
        return;
    }
    let tokenized = tokenize_macro_body(token.macro_definition_code(), &params);
    let (line, column, len) = definition_location(token);
    for index in unused_macro_parameter_indices(&params, &tokenized) {
        out.push(UnusedBinding {
            name: params[index].to_string(),
            kind: UnusedBindingKind::MacroParameter,
            line,
            column,
            len,
            removable_clause: None
        });
    }
}

fn check_function<T: MayHaveSpan + ListingElement>(token: &T, out: &mut Vec<UnusedBinding>) {
    let params = token.function_definition_params();
    if params.is_empty() {
        return;
    }
    let text: &str = token.span().as_ref();
    let (line, column, len) = definition_location(token);
    for name in params {
        if !references(text, name) {
            out.push(UnusedBinding {
                name: name.to_string(),
                kind: UnusedBindingKind::FunctionParameter,
                line,
                column,
                len,
                removable_clause: None
            });
        }
    }
}

fn check_iterate<T: MayHaveSpan + ListingElement>(token: &T, out: &mut Vec<UnusedBinding>) {
    let name = token.iterate_counter_name();
    let text: &str = token.span().as_ref();
    if references(text, name) {
        return;
    }
    let (line, column, len) = definition_location(token);
    out.push(UnusedBinding {
        name: name.to_string(),
        kind: UnusedBindingKind::IterateCounter,
        line,
        column,
        len,
        removable_clause: None
    });
}

fn check_for<T: MayHaveSpan + ListingElement>(token: &T, out: &mut Vec<UnusedBinding>) {
    let name = token.for_label();
    let text: &str = token.span().as_ref();
    if references(text, name) {
        return;
    }
    let (line, column, len) = definition_location(token);
    out.push(UnusedBinding {
        name: name.to_string(),
        kind: UnusedBindingKind::ForCounter,
        line,
        column,
        len,
        removable_clause: None
    });
}

fn check_repeat<T: MayHaveSpan + ListingElement>(token: &T, out: &mut Vec<UnusedBinding>) {
    let Some(name) = token.repeat_counter_name()
    else {
        return; // no counter declared at all - nothing to check
    };
    let text: &str = token.span().as_ref();
    if references(text, name) {
        return;
    }
    let (line, column, len) = definition_location(token);
    // Only offer removal for the simplest, single-clause, same-line shape
    // (no explicit start/step value) - see `removable_clause`'s own doc
    // comment for why the other shapes are left warning-only.
    let removable_clause = if token.repeat_counter_start().is_none() {
        repeat_counter_clause_location(text, name, line, column)
    }
    else {
        None
    };
    out.push(UnusedBinding {
        name: name.to_string(),
        kind: UnusedBindingKind::RepeatCounter,
        line,
        column,
        len,
        removable_clause
    });
}

/// Locate the removable `", countername"` clause on `full_text`'s first
/// line (the REPEAT construct's own header, where its counter is declared -
/// the clause is always on that same line as `def_line`/`def_column`, the
/// construct's own start position, since a REPEAT header is realistically
/// never itself split across lines).
///
/// `start` is a *byte* offset from `str::find`, added directly onto
/// `def_column` (a 1-based *character* count) - a byte/character mismatch
/// is only possible when the header text before the match contains a
/// non-ASCII character (e.g. in the count expression), an edge case
/// deliberately not handled here; `Some` is still correct for the
/// overwhelmingly common ASCII case.
fn repeat_counter_clause_location(
    full_text: &str,
    name: &str,
    def_line: usize,
    def_column: usize
) -> Option<(usize, usize, usize)> {
    let header = full_text.lines().next()?;
    let with_space = format!(", {name}");
    let no_space = format!(",{name}");
    let (start, matched_len) = header
        .find(&with_space)
        .map(|i| (i, with_space.len()))
        .or_else(|| header.find(&no_space).map(|i| (i, no_space.len())))?;
    Some((def_line, def_column + start, matched_len))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unused_in(code: &str) -> Vec<UnusedBinding> {
        let listing = crate::parser::common::parse_z80_str(code).expect("should parse");
        find_unused_bindings(listing.iter())
    }

    #[test]
    fn macro_parameter_used_in_body_is_not_reported() {
        let found = unused_in("MACRO foo, a\n    ld a, {a}\nENDM\n");
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn macro_parameter_never_referenced_is_reported() {
        let found = unused_in("MACRO foo, a, b, c\n    ld a, {a}\n    ld b, {b}\nENDM\n");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].name, "c");
        assert_eq!(found[0].kind, UnusedBindingKind::MacroParameter);
        assert!(found[0].removable_clause.is_none());
    }

    /// The construct's own `.span()` covers the *whole* multi-line body
    /// (header through `ENDM`), not just the header - `len` must stay the
    /// header line's own length, or a caller building a single-line
    /// diagnostic range from `(line, column, len)` would compute an `end`
    /// character offset far past that line's actual length.
    #[test]
    fn reported_len_fits_the_header_line_not_the_whole_multi_line_body() {
        let header = "MACRO foo, a, b, c";
        let code = format!("{header}\n    ld a, {{a}}\n    ld b, {{b}}\nENDM\n");
        let found = unused_in(&code);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].name, "c");
        assert!(
            found[0].len <= header.len(),
            "len {} should fit within the header line ({} chars), not the whole macro body: {found:?}",
            found[0].len,
            header.len()
        );
    }

    #[test]
    fn function_parameter_never_referenced_is_reported() {
        let found = unused_in(
            "FUNCTION f, a, b\n    IF {a} > 0\n        RETURN 1\n    ENDIF\n    RETURN 0\nENDFUNCTION\n"
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].name, "b");
        assert_eq!(found[0].kind, UnusedBindingKind::FunctionParameter);
    }

    #[test]
    fn function_with_every_parameter_used_reports_nothing() {
        let found = unused_in("FUNCTION f, a, b\n    RETURN {a} + {b}\nENDFUNCTION\n");
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn repeat_with_an_unused_optional_counter_offers_a_removable_clause() {
        let found = unused_in("REPEAT 5, i\n    nop\nENDR\n");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].name, "i");
        assert_eq!(found[0].kind, UnusedBindingKind::RepeatCounter);
        let (line, column, len) = found[0].removable_clause.expect("removable clause");
        assert_eq!(line, 1);
        // "REPEAT 5, i" - the removable clause is ", i" starting right after "REPEAT 5".
        let header = "REPEAT 5, i";
        assert_eq!(&header[column - 1..column - 1 + len], ", i");
    }

    #[test]
    fn repeat_with_a_used_counter_reports_nothing() {
        let found = unused_in("REPEAT 5, i\n    db {i}\nENDR\n");
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn repeat_with_no_counter_at_all_is_not_a_false_positive() {
        let found = unused_in("REPEAT 5\n    nop\nENDR\n");
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn repeat_with_an_unused_counter_and_an_explicit_start_has_no_removable_clause() {
        let found = unused_in("REPEAT 5, i, 10\n    nop\nENDR\n");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].name, "i");
        assert!(
            found[0].removable_clause.is_none(),
            "a start value makes the clause shape more than this pass handles - must stay None, not a wrong range"
        );
    }

    #[test]
    fn iterate_with_an_unused_mandatory_counter_is_reported_but_not_removable() {
        let found = unused_in("ITERATE i, [1, 2, 3]\n    nop\nENDITERATE\n");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].name, "i");
        assert_eq!(found[0].kind, UnusedBindingKind::IterateCounter);
        assert!(found[0].removable_clause.is_none());
    }

    #[test]
    fn for_with_an_unused_mandatory_counter_is_reported_but_not_removable() {
        let found = unused_in("FOR i, 0, 5, 1\n    nop\nENDFOR\n");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].name, "i");
        assert_eq!(found[0].kind, UnusedBindingKind::ForCounter);
        assert!(found[0].removable_clause.is_none());
    }

    #[test]
    fn a_repeat_nested_inside_a_module_still_has_its_own_counter_checked() {
        // FUNCTION bodies parse in a restricted state that doesn't allow a
        // nested REPEAT at all - MODULE bodies aren't restricted this way,
        // so this exercises the same "recursion reaches nested constructs"
        // property `walk` needs, without an invalid fixture.
        let found = unused_in("MODULE m\n    REPEAT 5, i\n        nop\n    ENDR\nENDMODULE\n");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].name, "i");
        assert_eq!(found[0].kind, UnusedBindingKind::RepeatCounter);
    }

    /// Documents a deliberate, accepted imprecision (not a bug to "fix"
    /// later without noticing): `references` is a plain, non-context-aware
    /// text scan (the same one `tokenize_macro_body` already uses for real
    /// macro expansion), so a name that only appears inside a string
    /// literal or comment still counts as "used" - biasing towards no
    /// false-positive "unused" warning rather than a context-aware (and
    /// much more involved) real reference check.
    #[test]
    fn a_name_appearing_only_inside_a_comment_is_still_treated_as_used() {
        let found = unused_in("MACRO foo, a\n    nop ; mentions {a} only in a comment\nENDM\n");
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn nothing_is_reported_for_a_document_with_no_macro_function_or_loop_at_all() {
        let found = unused_in("org 0x8000\nmain:\n    ld a, 1\n    ret\n");
        assert!(found.is_empty(), "{found:?}");
    }

    // ── Cross-validation against the real assembler's own real-time check ──
    //
    // MACRO/REPEAT/ITERATE/FOR are also checked *during a real assemble*
    // (`Env::visit_macro_definition`/`warn_if_counter_unused`, wired into
    // `visit_repeat`/`visit_iterate`/`visit_for`) - a second, independent
    // way to reach the same answer, using the assembler's own `is_used`
    // symbol tracking rather than this module's static text scan. FUNCTION
    // parameters are *not* covered by the real-time method (a separate,
    // frame-based mechanism that doesn't scope `is_used` the same way -
    // tracked as a known follow-on, not silently dropped). These tests
    // drive a real assemble and confirm both methods agree.

    /// Runs both detection methods against the exact same `code` and
    /// returns `(names the static method reports, warning messages the
    /// real-time method produces)`.
    fn both_methods(code: &str) -> (Vec<String>, Vec<String>) {
        let listing = crate::parser::common::parse_z80_str(code).expect("should parse (static)");
        let static_names: Vec<String> = find_unused_bindings(listing.iter())
            .into_iter()
            .map(|b| b.name)
            .collect();

        let tokens = crate::parser::parse_z80_str(code).expect("should parse (real-time)");
        let env = match crate::assembler::visit_tokens_all_passes_with_options(
            &tokens,
            crate::assembler::EnvOptions::default()
        ) {
            Ok((_tok, env)) => env,
            Err((_tok, _env, e)) => panic!("assembling should not fail: {e}")
        };
        let real_time_messages: Vec<String> =
            env.warnings().iter().map(|w| w.to_string()).collect();

        (static_names, real_time_messages)
    }

    #[test]
    fn static_and_real_time_methods_agree_for_an_unused_macro_parameter() {
        let (static_names, real_time_messages) =
            both_methods("MACRO foo, a, b, c\n    ld a, {a}\n    ld b, {b}\nENDM\nfoo(1, 2, 3)\n");
        assert_eq!(static_names, vec!["c"], "{static_names:?}");
        assert!(
            real_time_messages
                .iter()
                .any(|m| m.contains("'c' is never used")),
            "{real_time_messages:?}"
        );
    }

    #[test]
    fn static_and_real_time_methods_agree_for_an_unused_repeat_counter() {
        let (static_names, real_time_messages) = both_methods("REPEAT 3, i\n    nop\nENDR\n");
        assert_eq!(static_names, vec!["i"], "{static_names:?}");
        assert!(
            real_time_messages
                .iter()
                .any(|m| m.contains("'i' is never used")),
            "{real_time_messages:?}"
        );
    }

    #[test]
    fn static_and_real_time_methods_agree_for_an_unused_iterate_counter() {
        let (static_names, real_time_messages) =
            both_methods("ITERATE i, [1, 2, 3]\n    nop\nENDITERATE\n");
        assert_eq!(static_names, vec!["i"], "{static_names:?}");
        assert!(
            real_time_messages
                .iter()
                .any(|m| m.contains("'i' is never used")),
            "{real_time_messages:?}"
        );
    }

    #[test]
    fn static_and_real_time_methods_agree_for_an_unused_for_counter() {
        let (static_names, real_time_messages) = both_methods("FOR i, 0, 5, 1\n    nop\nENDFOR\n");
        assert_eq!(static_names, vec!["i"], "{static_names:?}");
        assert!(
            real_time_messages
                .iter()
                .any(|m| m.contains("'i' is never used")),
            "{real_time_messages:?}"
        );
    }

    #[test]
    fn static_and_real_time_methods_agree_when_nothing_is_unused() {
        let (static_names, real_time_messages) = both_methods(
            "MACRO foo, a\n    ld a, {a}\nENDM\nfoo(1)\nREPEAT 3, i\n    db {i}\nENDR\n"
        );
        assert!(static_names.is_empty(), "{static_names:?}");
        assert!(
            !real_time_messages
                .iter()
                .any(|m| m.contains("is never used")),
            "{real_time_messages:?}"
        );
    }

    #[test]
    fn a_reused_counter_name_across_separate_loops_is_still_correctly_scoped() {
        // Regression test for the `remove_symbol` fix (cpclib-tokens):
        // `used_symbols` used to leak across separate loops sharing a
        // counter name - loop 1 using `i` must not make loop 2's own,
        // genuinely-unused `i` look "used" too.
        let code = "REPEAT 3, i\n    db {i}\nENDR\nREPEAT 3, i\n    nop\nENDR\n";
        let (static_names, real_time_messages) = both_methods(code);
        assert_eq!(static_names, vec!["i"], "{static_names:?}"); // only the 2nd loop
        let unused_count = real_time_messages
            .iter()
            .filter(|m| m.contains("'i' is never used"))
            .count();
        assert_eq!(unused_count, 1, "{real_time_messages:?}");
    }
}
