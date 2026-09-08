//! A real bug, not a hypothetical: `img2cpc`, run as an embedded (in-process)
//! bndbuild task inside `cpclib-lsp`, once wrote raw `println!` debug output
//! (a "Loading palette" line followed by an ANSI truecolor swatch dump) straight
//! to the process's real stdout - which, for the LSP server, *is* the
//! `Content-Length`-framed JSON-RPC transport to the editor. That corrupted the
//! stream and disconnected the client the moment a build rule doing an image
//! conversion ran (e.g. from the "Run in emulator" CodeLens, once it started
//! resolving a real build rule instead of a bare direct assemble).
//!
//! Every legitimate message this conversion emits is supposed to go through
//! the `EventObserver` passed into `process_img2cpc`, never straight to
//! `println!`/`eprintln!` - a caller embedding the conversion in-process (the
//! LSP/DAP servers) relies on that to route it safely; only a real subprocess
//! caller (a plain `bndbuild` CLI run, a terminal task) can afford raw stdout,
//! and even then a stray `println!` would just be noise mixed into otherwise
//! machine-parseable output. This spawns the real `img2cpc` binary (not an
//! in-process call, which would only prove the observer *received* something,
//! not that nothing *also* leaked to the real stdout) and checks its actual
//! OS-level stdout is clean.
use std::process::Command;

#[test]
fn converting_an_image_writes_nothing_to_stdout() {
    let out = std::env::temp_dir().join("cpclib_no_stdout_leak_test.sna");
    let _ = std::fs::remove_file(&out);

    let output = Command::new(env!("CARGO_BIN_EXE_img2cpc"))
        .args([
            "--mode",
            "0",
            "tests/plus_sprite.png",
            "sna",
            out.to_str().unwrap()
        ])
        .output()
        .expect("failed to spawn img2cpc");

    assert!(
        output.status.success(),
        "the conversion itself must succeed, or this test proves nothing: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "img2cpc wrote to its own real stdout - fine for a standalone CLI run, \
         but exactly what corrupts an embedding process's own stdout (e.g. the \
         LSP server's JSON-RPC transport) when this conversion runs in-process \
         as a bndbuild task instead. Captured stdout: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );

    let _ = std::fs::remove_file(&out);
}
