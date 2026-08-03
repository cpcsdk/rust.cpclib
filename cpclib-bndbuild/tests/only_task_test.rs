//! `--only-task RULE:INDEX`: runs one task of one rule directly, bypassing
//! dependency resolution and up-to-date checks entirely - it must run even
//! when the rule's own target already exists on disk and would otherwise be
//! considered up to date.

use assert_cmd::Command;

#[test]
fn runs_even_when_the_target_already_exists_and_is_up_to_date() {
    let tmp = camino_tempfile::tempdir().unwrap();
    let build_file = tmp.path().join("build.bnd");
    std::fs::write(
        build_file.as_std_path(),
        "- tgt: out.txt\n  cmd: extern touch out.txt\n"
    )
    .unwrap();

    // The target already exists on disk, with no dependency newer than it -
    // a normal `bndbuild -f build.bnd out.txt` would consider this rule
    // already up to date and skip it entirely.
    let out_path = tmp.path().join("out.txt");
    std::fs::write(out_path.as_std_path(), "stale content").unwrap();
    let before = std::fs::metadata(out_path.as_std_path())
        .unwrap()
        .modified()
        .unwrap();

    // Ensure the filesystem's mtime resolution can actually observe a
    // difference before re-running the task.
    std::thread::sleep(std::time::Duration::from_millis(1100));

    let mut cmd = Command::cargo_bin("bndbuild").unwrap();
    cmd.current_dir(tmp.path());
    cmd.arg("-f")
        .arg("build.bnd")
        .arg("--only-task")
        .arg("out.txt:0");
    cmd.assert().success();

    let after = std::fs::metadata(out_path.as_std_path())
        .unwrap()
        .modified()
        .unwrap();
    assert!(
        after > before,
        "the task must have actually re-run (touch updates mtime) even though the target already existed"
    );
}

#[test]
fn fails_clearly_for_an_unknown_rule() {
    let tmp = camino_tempfile::tempdir().unwrap();
    let build_file = tmp.path().join("build.bnd");
    std::fs::write(
        build_file.as_std_path(),
        "- tgt: out.txt\n  cmd: extern touch out.txt\n"
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("bndbuild").unwrap();
    cmd.current_dir(tmp.path());
    cmd.arg("-f")
        .arg("build.bnd")
        .arg("--only-task")
        .arg("nope.txt:0");
    cmd.assert()
        .failure()
        .stderr(predicates::str::contains("no rule named 'nope.txt'"));
}

#[test]
fn fails_clearly_for_an_out_of_range_task_index() {
    let tmp = camino_tempfile::tempdir().unwrap();
    let build_file = tmp.path().join("build.bnd");
    std::fs::write(
        build_file.as_std_path(),
        "- tgt: out.txt\n  cmd: extern touch out.txt\n"
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("bndbuild").unwrap();
    cmd.current_dir(tmp.path());
    cmd.arg("-f")
        .arg("build.bnd")
        .arg("--only-task")
        .arg("out.txt:5");
    cmd.assert()
        .failure()
        .stderr(predicates::str::contains("has no task #6"));
}
