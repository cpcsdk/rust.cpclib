//! End-to-end tests for `apply_fixes_in_place_project`: real `.asm` files on
//! disk, under a real temp directory, run through the actual project-wide
//! walk + apply loop.

use camino::Utf8PathBuf;
use cpclib_basmopt::{OptimizationGoal, Options, apply_fixes_in_place_project};

fn write(dir: &camino_tempfile::Utf8TempDir, name: &str, content: &str) -> Utf8PathBuf {
    let path = dir.path().join(name);
    if let Some(parent) = path.parent() {
        fs_err::create_dir_all(parent).unwrap();
    }
    fs_err::write(&path, content).unwrap();
    path
}

#[test]
fn touches_every_asm_file_under_root() {
    let dir = camino_tempfile::tempdir().unwrap();
    for name in ["a.asm", "b.asm", "c.asm"] {
        write(&dir, name, "start:\n    ld b, b\n    ret\n");
    }

    let outcome = apply_fixes_in_place_project(dir.path(), &Options::default());
    assert_eq!(outcome.files_touched, 3, "{outcome:?}");
    assert_eq!(outcome.total_applied, 3, "{outcome:?}");
    assert_eq!(outcome.files_with_errors, 0, "{outcome:?}");

    for name in ["a.asm", "b.asm", "c.asm"] {
        let fixed = fs_err::read_to_string(dir.path().join(name)).unwrap();
        assert!(!fixed.contains("ld b, b"), "{name}: {fixed}");
    }
}

/// The load-bearing regression test: a project-wide run must never apply
/// `jp2jr` (address-aware), even under a goal that would normally include
/// it and even though the single-file `apply_fixes_in_place` *would* rewrite
/// the very same file - proving the classification/exclusion actually
/// excludes, not just documents, the unsafe path.
#[test]
fn never_applies_jp2jr_even_when_the_goal_would_normally_include_it() {
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

    // Sanity: the single-file path *does* rewrite it under this goal - so
    // the project-wide test below is a real exclusion, not a fixture that
    // never would have matched at all.
    let single_file = cpclib_basmopt::apply_fixes_in_place(&path, &options).unwrap();
    assert_eq!(single_file.total_applied, 1, "{single_file:?}");
    fs_err::write(&path, "start:\n    jp target\ntarget:\n    ret\n").unwrap();

    let outcome = apply_fixes_in_place_project(dir.path(), &options);
    assert_eq!(outcome.total_applied, 0, "{outcome:?}");
    assert_eq!(outcome.total_address_aware_skipped, 1, "{outcome:?}");
    let source = fs_err::read_to_string(&path).unwrap();
    assert!(source.contains("jp target"), "{source}");
    assert!(!source.contains("jr target"), "{source}");
}

#[test]
fn respects_gitignore() {
    let dir = camino_tempfile::tempdir().unwrap();
    write(&dir, ".gitignore", "ignored.asm\n");
    write(&dir, "ignored.asm", "start:\n    ld b, b\n    ret\n");
    write(&dir, "kept.asm", "start:\n    ld b, b\n    ret\n");

    let outcome = apply_fixes_in_place_project(dir.path(), &Options::default());
    assert_eq!(outcome.files_touched, 1, "{outcome:?}");
    let ignored = fs_err::read_to_string(dir.path().join("ignored.asm")).unwrap();
    assert!(ignored.contains("ld b, b"), "{ignored}");
    let kept = fs_err::read_to_string(dir.path().join("kept.asm")).unwrap();
    assert!(!kept.contains("ld b, b"), "{kept}");
}

/// A dead-output-deletion match (empty replacement + a liveness constraint)
/// must be skipped for review, same as the single-file path, never applied
/// unreviewed.
#[test]
fn skips_bulk_unsafe_matches_same_as_single_file_mode() {
    let dir = camino_tempfile::tempdir().unwrap();
    let rules_path = write(
        &dir,
        "dead_output.txt",
        "pattern: Remove unused ld ?reg, ?any\nname: unused-ld-any\n0: ld ?reg,?any\n\
         replacement:\nconstraints:\nregsNotUsedAfter(0,?reg)\n"
    );
    write(&dir, "test.asm", "start:\n    ld a, 1\n    ld a, 2\n    ret\n");

    let options = Options {
        no_builtin: true,
        extra_rule_files: vec![rules_path],
        ..Options::default()
    };

    let outcome = apply_fixes_in_place_project(dir.path(), &options);
    assert_eq!(outcome.total_applied, 0, "{outcome:?}");
    assert_eq!(outcome.total_skipped_for_review, 1, "{outcome:?}");
    let source = fs_err::read_to_string(dir.path().join("test.asm")).unwrap();
    assert!(source.contains("ld a, 1"), "{source}");
}

/// One malformed file among otherwise-valid ones - the others must still be
/// processed, not aborted on the first failure.
#[test]
fn continues_past_a_file_that_fails_to_parse() {
    let dir = camino_tempfile::tempdir().unwrap();
    write(&dir, "broken.asm", "@#$ garbage @#$\n");
    write(&dir, "good.asm", "start:\n    ld b, b\n    ret\n");

    let outcome = apply_fixes_in_place_project(dir.path(), &Options::default());
    assert_eq!(outcome.files_with_errors, 1, "{outcome:?}");
    assert_eq!(outcome.files_touched, 1, "{outcome:?}");
    assert_eq!(outcome.total_applied, 1, "{outcome:?}");
    let good = fs_err::read_to_string(dir.path().join("good.asm")).unwrap();
    assert!(!good.contains("ld b, b"), "{good}");
}

/// Composition with the peephole engine's own `; noopt` veto (landed in the
/// same session, before this item): a bulk-safe-shaped match immediately
/// adjacent to `; noopt` must survive project-wide apply untouched. No extra
/// code in `apply_fixes_in_place_project` makes this true - the engine
/// vetoes the match before it is ever constructed, so it never reaches this
/// function's apply/skip logic at all.
#[test]
fn noopt_marked_instructions_survive_project_wide_apply() {
    let dir = camino_tempfile::tempdir().unwrap();
    write(
        &dir,
        "test.asm",
        "start:\n    ld b, b ; noopt\n    ret\n"
    );

    let outcome = apply_fixes_in_place_project(dir.path(), &Options::default());
    assert_eq!(outcome.total_applied, 0, "{outcome:?}");
    let source = fs_err::read_to_string(dir.path().join("test.asm")).unwrap();
    assert!(source.contains("ld b, b"), "{source}");
}
