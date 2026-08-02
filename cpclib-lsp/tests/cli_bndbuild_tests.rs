//! `cpclib-lsp bndbuild ...`: the installed LSP binary can also act as
//! `bndbuild` itself, so editor integrations only need one binary on PATH.
//! Spawns the real, compiled `cpclib-lsp` binary as a subprocess (required
//! since `run_as_bndbuild` calls `std::process::exit`, unsafe to invoke
//! in-process from a test) against a real temp project.

use std::process::Command;

fn cpclib_lsp_bin() -> &'static str {
    env!("CARGO_BIN_EXE_cpclib-lsp")
}

#[test]
fn bndbuild_mode_builds_a_real_target() {
    let tmp = camino_tempfile::tempdir().unwrap();
    let build_file = tmp.path().join("build.bnd");
    std::fs::write(
        build_file.as_std_path(),
        "- tgt: out.txt\n  phony: true\n  cmd: extern touch out.txt\n"
    )
    .unwrap();

    let output = Command::new(cpclib_lsp_bin())
        .args(["bndbuild", "-f", "build.bnd", "out.txt"])
        .current_dir(tmp.path())
        .output()
        .expect("failed to spawn cpclib-lsp bndbuild");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        tmp.path().join("out.txt").as_std_path().exists(),
        "target file was not actually built"
    );
}

#[test]
fn bndbuild_mode_reports_failure_with_a_non_zero_exit_code() {
    let tmp = camino_tempfile::tempdir().unwrap();
    let build_file = tmp.path().join("build.bnd");
    std::fs::write(
        build_file.as_std_path(),
        "- tgt: nope.txt\n  phony: true\n  cmd: extern false\n"
    )
    .unwrap();

    let output = Command::new(cpclib_lsp_bin())
        .args(["bndbuild", "-f", "build.bnd", "nope.txt"])
        .current_dir(tmp.path())
        .output()
        .expect("failed to spawn cpclib-lsp bndbuild");

    assert!(
        !output.status.success(),
        "a failing rule must exit non-zero: stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The LSP server itself must still start normally when `bndbuild` isn't
/// the given subcommand - only that exact subcommand switches modes.
#[test]
fn without_the_subcommand_the_binary_does_not_try_to_act_as_bndbuild() {
    let output = Command::new(cpclib_lsp_bin())
        .args(["--init-config", "/nonexistent-dir-for-this-test-only"])
        .output()
        .expect("failed to spawn cpclib-lsp");

    // Reaches the ordinary `--init-config` path (fails because the
    // directory doesn't exist), never bndbuild's own arg parser - which
    // would reject `--init-config` as an unknown flag with a different
    // error message.
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("failed to write") || stderr.contains("No such file"),
        "stderr: {stderr}"
    );
}
