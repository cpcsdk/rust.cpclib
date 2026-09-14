//! End-to-end tests through the real public API: write a real `.asm` file to
//! disk, call `analyze_file`/`apply_fixes` exactly the way `main.rs` and the
//! future `cpclib-bndbuild` runner do, and check the result.

use camino::Utf8PathBuf;
use cpclib_basmopt::{
    BasmOptError, OptimizationGoal, Options, Suggestion, analyze_file, apply_fixes,
    apply_fixes_in_place
};

fn write(dir: &camino_tempfile::Utf8TempDir, name: &str, content: &str) -> Utf8PathBuf {
    let path = dir.path().join(name);
    fs_err::write(&path, content).unwrap();
    path
}

/// `analyze_file`, unwrapped down to the `(source, suggestions)` shape most
/// tests only care about - the ones that also care about
/// `assemble_warning` call `analyze_file` directly instead.
fn analyze(path: &camino::Utf8Path, options: &Options) -> (String, Vec<Suggestion>) {
    let outcome = analyze_file(path, options).unwrap();
    assert!(
        outcome.assemble_warning.is_none(),
        "unexpected assemble warning: {:?}",
        outcome.assemble_warning
    );
    (outcome.source, outcome.suggestions)
}

#[test]
fn finds_real_optimisations_in_ordinary_source() {
    let dir = camino_tempfile::tempdir().unwrap();
    let path = write(
        &dir,
        "test.asm",
        "start:\n    ld b, b\n    inc hl\n    ret\n"
    );

    let (_source, suggestions) = analyze(&path, &Options::default());
    assert_eq!(suggestions.len(), 1, "{suggestions:?}");
    assert_eq!(
        suggestions[0].rule_name.as_deref(),
        Some("unnecessary-ld-to-itself")
    );
    assert_eq!(suggestions[0].line, 2);
}

#[test]
fn already_optimal_source_reports_nothing() {
    let dir = camino_tempfile::tempdir().unwrap();
    let path = write(
        &dir,
        "test.asm",
        "start:\n    xor a\n    inc hl\n    ld (hl), a\n    ret\n"
    );

    let (_source, suggestions) = analyze(&path, &Options::default());
    assert!(suggestions.is_empty(), "{suggestions:?}");
}

#[test]
fn apply_fixes_round_trips_through_a_real_file() {
    let dir = camino_tempfile::tempdir().unwrap();
    let path = write(
        &dir,
        "test.asm",
        "start:\n    ld a, 0\n    cp 0\n    ld b, b\n    push hl\n    pop de\n    ret\n"
    );

    let (source, suggestions) = analyze(&path, &Options::default());
    assert!(!suggestions.is_empty());
    let fixed = apply_fixes(&source, &suggestions);

    assert!(fixed.contains("ld d, h"), "{fixed}");
    assert!(fixed.contains("ld e, l"), "{fixed}");
    assert!(!fixed.contains("ld b, b"), "{fixed}");
    assert!(!fixed.contains("push hl"), "{fixed}");
    // Deleting `ld b, b` must not leave a blank line behind.
    assert!(!fixed.contains("\n\n"), "{fixed}");

    // Writing the fixed source back and re-analyzing must find nothing left
    // to fix - the whole point of applying a fix is that it actually fixes.
    fs_err::write(&path, &fixed).unwrap();
    let (_source2, remaining) = analyze(&path, &Options::default());
    assert!(remaining.is_empty(), "still found: {remaining:?}");
}

#[test]
fn an_extra_rule_file_is_picked_up_and_can_be_disabled() {
    let dir = camino_tempfile::tempdir().unwrap();
    // Deliberately `xor` rather than `and`/`or` - the built-in `redundant-op`
    // rule already covers those two (see `cpclib-asmoptim`'s own tests), so
    // reusing them here would prove nothing about the *extra* rule file
    // actually being the thing that made the match happen.
    // `inc a` twice, deliberately: the second one *reads* A, so no built-in
    // liveness rule can claim the span first and preempt the extra rule under
    // test. (This used to be `xor a` twice, where the first is genuinely dead
    // and the built-in `unused-op-1arg2` rightly takes it.)
    let source_path = write(&dir, "test.asm", "start:\n    inc a\n    inc a\n    ret\n");
    let rules_path = write(
        &dir,
        "extra.txt",
        "pattern: Remove redundant ?op ?any\n\
         name: my-redundant-op\n\
         0: ?op ?any\n\
         1: ?op ?any\n\
         replacement:\n\
         0: ?op ?any\n\
         constraints:\n\
         in(?op,inc)\n"
    );

    // Not found by the built-in set alone - nothing built-in carries this
    // name, so its presence below can only come from the extra file.
    let (_source, without) = analyze(&source_path, &Options::default());
    assert!(
        !without
            .iter()
            .any(|s| s.rule_name.as_deref() == Some("my-redundant-op")),
        "{without:?}"
    );

    // Supplying the extra rule file makes it fire.
    let with_extra = Options {
        extra_rule_files: vec![rules_path.clone()],
        ..Options::default()
    };
    let (_source, found) = analyze(&source_path, &with_extra);
    assert!(
        found
            .iter()
            .any(|s| s.rule_name.as_deref() == Some("my-redundant-op")),
        "{found:?}"
    );

    // Disabling it by name silences it again, even though it's supplied.
    let disabled = Options {
        extra_rule_files: vec![rules_path],
        disabled_rules: vec!["my-redundant-op".to_string()],
        ..Options::default()
    };
    let (_source, none) = analyze(&source_path, &disabled);
    assert!(
        !none
            .iter()
            .any(|s| s.rule_name.as_deref() == Some("my-redundant-op")),
        "{none:?}"
    );
}

#[test]
fn disabling_a_builtin_rule_by_name_silences_it() {
    let dir = camino_tempfile::tempdir().unwrap();
    let path = write(&dir, "test.asm", "start:\n    ld b, b\n    ret\n");

    let options = Options {
        disabled_rules: vec!["unnecessary-ld-to-itself".to_string()],
        ..Options::default()
    };
    let (_source, suggestions) = analyze(&path, &options);
    assert!(suggestions.is_empty(), "{suggestions:?}");
}

#[test]
fn no_builtin_uses_only_the_supplied_rules() {
    let dir = camino_tempfile::tempdir().unwrap();
    let path = write(&dir, "test.asm", "start:\n    ld b, b\n    cp 0\n    ret\n");

    let options = Options {
        no_builtin: true,
        ..Options::default()
    };
    let (_source, suggestions) = analyze(&path, &options);
    assert!(
        suggestions.is_empty(),
        "no rules at all should be active: {suggestions:?}"
    );
}

/// The real point of the whole `Env`-backed address recorder from Session D:
/// this only fires when the source actually gets assembled with real
/// addresses, which `analyze_file` must set up correctly on its own (a
/// consumer of the public API never touches `Env` directly).
#[test]
fn jp_to_a_reachable_label_becomes_jr_under_the_size_goal() {
    let dir = camino_tempfile::tempdir().unwrap();
    let path = write(
        &dir,
        "test.asm",
        "start:\n    jp target\ntarget:\n    ret\n"
    );

    let options = Options {
        goal: OptimizationGoal::Size,
        ..Options::default()
    };
    let (_source, suggestions) = analyze(&path, &options);
    let hit = suggestions
        .iter()
        .find(|s| s.rule_name.as_deref() == Some("jp2jr"))
        .unwrap_or_else(|| panic!("expected jp2jr to fire: {suggestions:?}"));
    assert_eq!(hit.replacement, vec!["jr target".to_string()]);
}

/// The bug this test guards against: a real project's source references an
/// `INCLUDE` that basmopt has no way to resolve (wrong cwd, no -I given).
/// The active rule set (even the default, `Neutral`, goal - it includes
/// `jp2jr`, which is `reachableByJr`-gated) genuinely needs a real assemble,
/// so one is attempted and does fail - but that must not take the whole
/// analysis down with it: every rule that doesn't need addresses is still
/// checked normally, and the failure is reported as a warning, not an error.
#[test]
fn an_unresolvable_include_degrades_gracefully_instead_of_failing_the_whole_analysis() {
    let dir = camino_tempfile::tempdir().unwrap();
    let path = write(
        &dir,
        "test.asm",
        "include \"this/file/does/not/exist.asm\"\nstart:\n    ld b, b\n    ret\n"
    );

    let outcome = analyze_file(&path, &Options::default()).unwrap();
    assert!(
        outcome.assemble_warning.is_some(),
        "expected an assemble_warning when the include cannot be resolved"
    );
    assert!(
        outcome
            .suggestions
            .iter()
            .any(|s| s.rule_name.as_deref() == Some("unnecessary-ld-to-itself")),
        "{:?}",
        outcome.suggestions
    );
    // `jp2jr` itself is address-aware and could not be evaluated this run -
    // it must simply be absent, not cause a panic or a wrong answer.
    assert!(
        !outcome
            .suggestions
            .iter()
            .any(|s| s.rule_name.as_deref() == Some("jp2jr")),
        "{:?}",
        outcome.suggestions
    );
}

/// `include_dirs` (`-I`/`--include`, matching `basm`'s own flag) is how to
/// avoid the degraded fallback above: point it at the real directory and the
/// assemble succeeds, so address-aware rules like `jp2jr` can fire too.
#[test]
fn include_dirs_resolves_an_include_so_address_aware_rules_can_fire() {
    let dir = camino_tempfile::tempdir().unwrap();
    let included_dir = dir.path().join("shared");
    fs_err::create_dir(&included_dir).unwrap();
    write(&dir, "shared/macros.asm", "macro nop_twice\n    nop\n    nop\nendmacro\n");

    let path = write(
        &dir,
        "test.asm",
        "include \"macros.asm\"\nstart:\n    jp target\ntarget:\n    ret\n"
    );

    // Without a search path pointing at `shared/`, the include cannot be
    // found, so this degrades gracefully (see the test above) rather than
    // failing outright - but `jp2jr` specifically can't fire without it.
    let without_include_dir = Options {
        goal: OptimizationGoal::Size,
        ..Options::default()
    };
    let without = analyze_file(&path, &without_include_dir).unwrap();
    assert!(without.assemble_warning.is_some(), "{without:?}");
    assert!(
        !without
            .suggestions
            .iter()
            .any(|s| s.rule_name.as_deref() == Some("jp2jr")),
        "{:?}",
        without.suggestions
    );

    // Pointing -I at the right directory fixes it: no warning, and jp2jr
    // fires.
    let with_include_dir = Options {
        goal: OptimizationGoal::Size,
        include_dirs: vec![included_dir],
        ..Options::default()
    };
    let (_source, suggestions) = analyze(&path, &with_include_dir);
    assert!(
        suggestions
            .iter()
            .any(|s| s.rule_name.as_deref() == Some("jp2jr")),
        "{suggestions:?}"
    );
}

#[test]
fn a_nonexistent_file_reports_an_io_error() {
    let dir = camino_tempfile::tempdir().unwrap();
    let path = dir.path().join("does-not-exist.asm");
    let err = analyze_file(&path, &Options::default()).unwrap_err();
    assert!(matches!(err, BasmOptError::Io { .. }), "{err:?}");
}

#[test]
fn a_syntax_error_reports_a_parse_error() {
    let dir = camino_tempfile::tempdir().unwrap();
    let path = write(&dir, "bad.asm", "ld a, ,\n");
    let err = analyze_file(&path, &Options::default()).unwrap_err();
    assert!(matches!(err, BasmOptError::Parse { .. }), "{err:?}");
}

/// Applying a fix must leave a file that still assembles. This is the property
/// that matters most for `-i`, and the one nothing checked until preserved
/// regions started carrying comments and multiple instructions per suggestion.
fn assert_applies_cleanly(source: &str) -> String {
    let dir = camino_tempfile::tempdir().unwrap();
    let path = write(&dir, "test.asm", source);
    let (text, suggestions) = analyze(&path, &Options::default());
    let fixed = apply_fixes(&text, &suggestions);

    cpclib_asm::parser::parse_z80_str(&fixed)
        .unwrap_or_else(|e| panic!("fixed source no longer parses: {e}\n---\n{fixed}\n---"));

    // ... and re-analysing the result must not loop: a fix that keeps
    // re-suggesting itself would make `--fix` never converge.
    let again = write(&dir, "again.asm", &fixed);
    let (_, remaining) = analyze(&again, &Options::default());
    assert!(
        remaining.len() <= suggestions.len(),
        "applying fixes increased the number of suggestions:\n{fixed}"
    );
    fixed
}

#[test]
fn applying_a_fix_that_preserves_a_gap_keeps_the_users_own_text() {
    // `unnecessary-push-pop` removes the push/pop and writes the gap back out.
    // That gap is code the rule never intended to touch, so it has to come
    // through the rewrite unchanged - comment, hex spelling and case included.
    let fixed = assert_applies_cleanly(
        "start:\n    PUSH HL\n    LD BC, #7F10   ; set the gate array\n\
             OUT (C), C\n    POP HL\n    ret\n"
    );

    assert!(
        fixed.contains("#7F10"),
        "the number's base must survive the rewrite:\n{fixed}"
    );
    assert!(
        fixed.contains("set the gate array"),
        "the comment must survive the rewrite:\n{fixed}"
    );
    assert!(
        !fixed.to_uppercase().contains("PUSH HL"),
        "the push should be gone:\n{fixed}"
    );
    assert!(
        fixed.to_uppercase().contains("OUT (C), C"),
        "the instruction after the comment must not be swallowed by it:\n{fixed}"
    );
}

#[test]
fn applying_fixes_to_a_variety_of_sources_always_yields_assemblable_code() {
    for source in [
        // A plain deletion.
        "start:\n    ld b, b\n    inc hl\n    ret\n",
        // A rewrite with a captured symbol.
        "start:\n    jp target\ntarget:\n    ret\n",
        // A multi-instruction replacement.
        "start:\n    push hl\n    pop de\n    ret\n",
        // Several independent fixes in one file - the ordering path.
        "start:\n    ld b, b\n    nop\n    ld c, c\n    nop\n    ld d, d\n    ret\n",
        // A fix whose line carries a trailing comment.
        "start:\n    ld b, b   ; redundant\n    inc hl\n    ret\n",
        // A labelled instruction, to be sure the label is not swallowed.
        "start:\nhere:\n    ld b, b\n    inc hl\n    ret\n"
    ] {
        assert_applies_cleanly(source);
    }
}

/// basm allows several instructions on a line, and a fix must touch only the
/// matched ones. Replacing whole lines instead silently deleted the rest -
/// on real CPC palette code (`ld bc,#7F01 : out (c),c : ld a,#44 : out (c),a`)
/// three instructions per line disappeared, and the file still assembled, so
/// nothing short of counting instructions would have noticed.
#[test]
fn a_fix_on_a_shared_line_leaves_the_other_instructions_alone() {
    let dir = camino_tempfile::tempdir().unwrap();
    let source = "start:\n\
         ld bc,#7F00 : out (c),c : ld a,#54 : out (c),a\n\
         ld bc,#7F01 : out (c),c : ld a,#44 : out (c),a\n\
         ret\n";
    let path = write(&dir, "test.asm", source);

    let (text, suggestions) = analyze(&path, &Options::default());
    assert!(!suggestions.is_empty(), "expected the inc-bc rule to fire");
    let fixed = apply_fixes(&text, &suggestions);

    // Every instruction that was not part of a match must survive.
    assert_eq!(
        fixed.matches("out (c),c").count(),
        2,
        "both `out (c),c` must survive:\n{fixed}"
    );
    assert_eq!(
        fixed.matches("ld a,#44").count(),
        1,
        "`ld a,#44` must survive:\n{fixed}"
    );
    assert_eq!(
        fixed.matches("out (c),a").count(),
        2,
        "both `out (c),a` must survive:\n{fixed}"
    );

    // ... and the result is still assemblable, with the separator intact
    // rather than glued to the new instruction (`inc bc: out` would read
    // `bc:` as a label).
    cpclib_asm::parser::parse_z80_str(&fixed)
        .unwrap_or_else(|e| panic!("fixed source no longer parses: {e}\n{fixed}"));
    assert!(
        !fixed.contains("bc: out"),
        "the statement separator must stay separated:\n{fixed}"
    );
}

/// A deletion on a shared line has the opposite need: it must take a `:` with
/// it, or a dangling separator is left behind.
#[test]
fn a_deletion_on_a_shared_line_takes_its_separator() {
    let dir = camino_tempfile::tempdir().unwrap();
    let path = write(&dir, "test.asm", "start:\n    nop : ld b, b : inc hl\n    ret\n");

    let (text, suggestions) = analyze(&path, &Options::default());
    assert!(!suggestions.is_empty(), "expected ld b,b to be flagged");
    let fixed = apply_fixes(&text, &suggestions);

    assert!(fixed.contains("nop"), "{fixed}");
    assert!(fixed.contains("inc hl"), "{fixed}");
    assert!(!fixed.contains("ld b, b"), "the ld should be gone:\n{fixed}");
    cpclib_asm::parser::parse_z80_str(&fixed)
        .unwrap_or_else(|e| panic!("fixed source no longer parses: {e}\n{fixed}"));
}

/// A pass-2-only opportunity: `r2-collapse-ld-a` only matches `ld a,1`
/// immediately followed by `ld a,2`, with nothing between them. In the
/// original source a `nop` sits in between, so pass 1 can only fire
/// `r1-remove-nop` - but that deletion makes the two `ld a` instructions
/// adjacent, which is exactly the shape `r2-collapse-ld-a` needs. This is
/// `apply_fixes_in_place`'s whole reason to exist: a single `analyze_file` +
/// `apply_fixes` call (one pass) cannot find `r2-collapse-ld-a` at all, since
/// it genuinely does not match the original text.
#[test]
fn a_second_pass_catches_an_opportunity_the_first_pass_exposed() {
    let dir = camino_tempfile::tempdir().unwrap();
    let source_path = write(
        &dir,
        "test.asm",
        "start:\n    ld a, 1\n    nop\n    ld a, 2\n    ret\n"
    );
    let rules_path = write(
        &dir,
        "extra.txt",
        "pattern: Remove standalone nop\n\
         name: r1-remove-nop\n\
         0: nop\n\
         replacement:\n\
         \n\
         pattern: Collapse redundant ld a\n\
         name: r2-collapse-ld-a\n\
         0: ld a,1\n\
         1: ld a,2\n\
         replacement:\n\
         1: ld a,2\n"
    );
    let options = Options {
        no_builtin: true,
        extra_rule_files: vec![rules_path],
        ..Options::default()
    };

    // A single pass finds and applies only r1 - r2 genuinely does not match
    // the original source (the nop is still in the way).
    let (source, first_pass_suggestions) = {
        let outcome = analyze_file(&source_path, &options).unwrap();
        (outcome.source, outcome.suggestions)
    };
    assert_eq!(
        first_pass_suggestions
            .iter()
            .map(|s| s.rule_name.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("r1-remove-nop")],
        "{first_pass_suggestions:?}"
    );
    let single_pass_result = apply_fixes(&source, &first_pass_suggestions);
    assert!(
        single_pass_result.contains("ld a, 1"),
        "a single pass must not have collapsed the ld a's yet:\n{single_pass_result}"
    );

    // The multi-pass entry point catches both, within its 2-pass cap.
    let outcome = apply_fixes_in_place(&source_path, &options).unwrap();
    assert_eq!(outcome.passes_run, 2, "{outcome:?}");
    assert_eq!(outcome.total_applied, 2, "{outcome:?}");
    assert_eq!(outcome.remaining_skipped, 0, "{outcome:?}");
    assert!(!outcome.final_source.contains("nop"), "{}", outcome.final_source);
    assert!(
        !outcome.final_source.contains("ld a, 1"),
        "the second pass should have collapsed this: {}",
        outcome.final_source
    );
    assert!(outcome.final_source.contains("ld a, 2"), "{}", outcome.final_source);
    cpclib_asm::parser::parse_z80_str(&outcome.final_source)
        .unwrap_or_else(|e| panic!("final source no longer parses: {e}\n{}", outcome.final_source));
}
