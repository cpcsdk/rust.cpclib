//! Verification for shell-style `<`/`>`/`>>`/`|` on bndbuild task lines -
//! see `cpclib-bndbuild/src/shell_pipe.rs` and `crate::task::InnerTask::Pipe`.

use std::str::FromStr;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use cpclib_bndbuild::executor::execute;
use cpclib_bndbuild::task::InnerTask;
use cpclib_common::event::CapturingObserver;

fn run(line: &str, observer: &Arc<CapturingObserver>) -> Result<(), String> {
    let task = InnerTask::from_str(line).unwrap_or_else(|e| panic!("failed to parse {line}: {e}"));
    execute(&task, observer)
}

#[test]
fn stdout_redirection_truncates_the_target_file() {
    let dir = camino_tempfile::tempdir().unwrap();
    let out = dir.path().join("out.txt");

    let observer = Arc::new(CapturingObserver::new());
    let result = run(&format!("echo hello > {out}"), &observer);

    assert!(result.is_ok(), "{result:?}");
    let content = fs_err::read_to_string(&out).unwrap();
    assert_eq!(content, "hello\n");
    // The redirected stage's stdout must NOT also reach the outer observer.
    assert!(observer.stdout_joined().is_empty());
}

#[test]
fn stdout_redirection_append_adds_to_the_target_file() {
    let dir = camino_tempfile::tempdir().unwrap();
    let out = dir.path().join("out.txt");
    fs_err::write(&out, "line1\n").unwrap();

    let observer = Arc::new(CapturingObserver::new());
    let result = run(&format!("echo line2 >> {out}"), &observer);

    assert!(result.is_ok(), "{result:?}");
    let content = fs_err::read_to_string(&out).unwrap();
    assert_eq!(content, "line1\nline2\n");
}

#[test]
fn stdin_redirection_feeds_rm_a_file_list() {
    let dir = camino_tempfile::tempdir().unwrap();
    let victim1 = dir.path().join("victim1.txt");
    let victim2 = dir.path().join("victim2.txt");
    fs_err::write(&victim1, "a").unwrap();
    fs_err::write(&victim2, "b").unwrap();

    let filelist = dir.path().join("filelist.txt");
    fs_err::write(&filelist, format!("{victim1}\n{victim2}\n")).unwrap();

    assert!(victim1.exists());
    assert!(victim2.exists());

    let observer = Arc::new(CapturingObserver::new());
    let result = run(&format!("rm < {filelist}"), &observer);

    assert!(result.is_ok(), "{result:?}");
    assert!(!victim1.exists());
    assert!(!victim2.exists());
}

#[test]
fn a_delegated_tasks_stdout_redirects_while_its_stderr_still_reaches_the_observer() {
    let dir = camino_tempfile::tempdir().unwrap();
    let input = dir.path().join("input.txt");
    fs_err::write(&input, "delegated content\n").unwrap();
    let out = dir.path().join("out.txt");
    let missing = dir.path().join("does-not-exist.txt");

    let observer = Arc::new(CapturingObserver::new());
    // `cat` writes `input`'s content to stdout (redirected to `out`) and an
    // error about the missing file to stderr (never redirected/piped).
    let result = run(
        &format!("extern cat {input} {missing} > {out}"),
        &observer
    );

    assert!(result.is_err(), "cat should fail on the missing file");
    let content = fs_err::read_to_string(&out).unwrap();
    assert_eq!(content, "delegated content\n");
    assert!(
        !observer.get_stderr().is_empty(),
        "cat's error about the missing file should reach the observer's stderr"
    );
}

#[test]
fn piping_works_between_two_embedded_tasks() {
    let observer = Arc::new(CapturingObserver::new());
    let result = run("echo hello | echo", &observer);

    assert!(result.is_ok(), "{result:?}");
    assert!(observer.stdout_joined().contains("hello"));
}

#[test]
fn a_three_stage_pipeline_mixes_embedded_and_delegated_stages_embedded_first() {
    let observer = Arc::new(CapturingObserver::new());
    // embedded (echo) -> delegated (extern cat, reads stdin) -> embedded (echo, reads stdin)
    let result = run("echo hello-3-stage | extern cat | echo", &observer);

    assert!(result.is_ok(), "{result:?}");
    assert!(observer.stdout_joined().contains("hello-3-stage"));
}

#[test]
fn a_three_stage_pipeline_mixes_embedded_and_delegated_stages_delegated_first() {
    let dir = camino_tempfile::tempdir().unwrap();
    let input = dir.path().join("input.txt");
    fs_err::write(&input, "hello-other-order\n").unwrap();

    let observer = Arc::new(CapturingObserver::new());
    // delegated (extern cat file) -> embedded (echo, reads stdin) -> delegated (extern cat, reads stdin)
    let result = run(&format!("extern cat {input} | echo | extern cat"), &observer);

    assert!(result.is_ok(), "{result:?}");
    assert!(observer.stdout_joined().contains("hello-other-order"));
}

/// Regression guard for the real-world bug this feature was built to fix:
/// `gource ... -o - | ffmpeg ...` (gource's stdout is a raw PPM/pixel byte
/// stream, not text) lost almost all of its data, because a delegated
/// task's real stdout used to be forwarded through
/// `EventObserver::emit_stdout(&str)`, which decodes it as UTF-8 text one
/// character at a time - silently dropping every invalid byte sequence. A
/// small, deterministic stand-in with the same shape (every byte value,
/// plus a run that is invalid UTF-8 under any interpretation, plus embedded
/// NUL bytes and newlines) must survive a delegated-to-delegated pipe byte
/// for byte.
#[test]
fn binary_data_survives_a_delegated_to_delegated_pipe_byte_for_byte() {
    let dir = camino_tempfile::tempdir().unwrap();
    let input = dir.path().join("input.bin");
    let mut content: Vec<u8> = (0u16..=255).map(|b| b as u8).collect();
    content.extend_from_slice(&[0xFF, 0xFE, 0x80, 0x81, 0xC0, 0xC1]);
    fs_err::write(&input, &content).unwrap();

    let output = dir.path().join("output.bin");

    let observer = Arc::new(CapturingObserver::new());
    let result = run(
        &format!("extern cat {input} | extern cat > {output}"),
        &observer
    );

    assert!(result.is_ok(), "{result:?}");
    let produced = fs_err::read(&output).unwrap();
    assert_eq!(
        produced, content,
        "binary data must survive a delegated-to-delegated pipe byte for byte"
    );
}

/// The one test that would catch a naive sequential (non-`thread::scope`)
/// re-implementation of `execute_pipe`: a producer emitting more than the OS
/// pipe buffer (64KiB on Linux) in a single `emit_stdout` call, before its
/// consumer has had a chance to start reading. If stages ran to completion
/// one at a time instead of concurrently, the producer's write into the
/// unread pipe would block forever. Run under a wall-clock timeout so a
/// regression fails the test instead of hanging the suite.
#[test]
fn a_large_producer_write_does_not_deadlock_against_a_concurrent_consumer() {
    let big = "x".repeat(200_000); // well over a 64KiB pipe buffer
    let line = format!("echo {big} | echo");

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let observer = Arc::new(CapturingObserver::new());
        let result = run(&line, &observer);
        let _ = tx.send((result, observer.stdout_joined().len()));
    });

    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok((result, out_len)) => {
            assert!(result.is_ok(), "{result:?}");
            // +1 for echo's trailing newline.
            assert_eq!(out_len, big.len() + 1);
        },
        Err(_) => panic!("pipeline did not complete within 10s - likely a deadlock")
    }
}
