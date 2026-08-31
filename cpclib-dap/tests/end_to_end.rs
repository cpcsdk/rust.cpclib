//! The adapter, driven as a real process over stdio.
//!
//! Everything else tests the translation with a peer in memory; this checks
//! the binary an editor actually launches: that it frames correctly, answers
//! `initialize`, and assembles a real program on `launch`.

use std::io::{BufReader, Read, Write};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

fn frame(message: &Value) -> String {
    let body = serde_json::to_string(message).unwrap();
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

/// Read messages until `wanted` responses/events have arrived, or the pipe dies.
fn read_messages(reader: &mut impl Read, wanted: usize) -> Vec<Value> {
    let mut buffer = Vec::new();
    let mut out = Vec::new();
    let mut chunk = [0u8; 4096];
    while out.len() < wanted {
        match reader.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(read) => buffer.extend_from_slice(&chunk[..read])
        }
        while let Some(position) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
            let header = String::from_utf8_lossy(&buffer[..position]).to_string();
            let Some(length) = header
                .lines()
                .find_map(|l| l.strip_prefix("Content-Length:"))
                .and_then(|v| v.trim().parse::<usize>().ok())
            else {
                buffer.drain(..position + 4);
                continue;
            };
            if buffer.len() < position + 4 + length {
                break;
            }
            let body = buffer[position + 4..position + 4 + length].to_vec();
            buffer.drain(..position + 4 + length);
            if let Ok(value) = serde_json::from_slice::<Value>(&body) {
                out.push(value);
            }
        }
    }
    out
}

fn adapter() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cpclib-dap"))
}

#[test]
fn it_answers_initialize_with_its_capabilities() {
    let mut child = adapter()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the adapter binary must run");

    let mut stdin = child.stdin.take().unwrap();
    stdin
        .write_all(frame(&json!({"seq": 1, "type": "request", "command": "initialize"})).as_bytes())
        .unwrap();
    stdin.flush().unwrap();

    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let messages = read_messages(&mut reader, 1);
    let _ = child.kill();

    assert_eq!(messages.len(), 1, "{messages:?}");
    assert_eq!(messages[0]["command"], json!("initialize"));
    assert_eq!(messages[0]["success"], json!(true));
    // False, not merely absent: `initialize` runs before a peer is chosen,
    // so this cannot yet know whether an emulator that really reverses
    // execution will be the one answering (see the capabilities() doc
    // comment).
    assert_eq!(messages[0]["body"]["supportsStepBack"], json!(false));
}

/// A request before any session is refused with a reason, not silence.
#[test]
fn a_request_without_a_session_is_refused_clearly() {
    let mut child = adapter()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    stdin
        .write_all(frame(&json!({"seq": 1, "type": "request", "command": "continue"})).as_bytes())
        .unwrap();
    stdin.flush().unwrap();

    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let messages = read_messages(&mut reader, 1);
    let _ = child.kill();

    assert_eq!(messages[0]["success"], json!(false));
    assert!(
        messages[0]["message"]
            .as_str()
            .unwrap()
            .contains("no debug session"),
        "{:?}",
        messages[0]
    );
}

/// `launch` on a program that does not exist fails with the path in the
/// message rather than hanging or panicking.
#[test]
fn a_launch_of_a_missing_program_fails_with_a_reason() {
    let mut child = adapter()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    stdin
        .write_all(
            frame(&json!({
                "seq": 1, "type": "request", "command": "launch",
                "arguments": {"program": "/nonexistent/nowhere.asm"}
            }))
            .as_bytes()
        )
        .unwrap();
    stdin.flush().unwrap();

    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let messages = read_messages(&mut reader, 1);
    let _ = child.kill();

    assert_eq!(messages[0]["success"], json!(false));
    assert!(
        messages[0]["message"]
            .as_str()
            .unwrap()
            .contains("nowhere.asm"),
        "the path is named: {:?}",
        messages[0]
    );
}

/// `progressStart`/`progressEnd` bracket a `launch`, but only for a client
/// that said in its own `initialize` arguments it accepts them - the DAP
/// spec gates these two events on that, unlike every other event this
/// adapter sends.
#[test]
fn a_client_that_accepts_progress_gets_it_around_launch() {
    let mut child = adapter()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    stdin
        .write_all(
            frame(&json!({
                "seq": 1, "type": "request", "command": "initialize",
                "arguments": {"supportsProgressReporting": true}
            }))
            .as_bytes()
        )
        .unwrap();
    stdin
        .write_all(
            frame(&json!({
                "seq": 2, "type": "request", "command": "launch",
                "arguments": {"program": "/nonexistent/nowhere.asm"}
            }))
            .as_bytes()
        )
        .unwrap();
    stdin.flush().unwrap();

    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let messages = read_messages(&mut reader, 4);
    let _ = child.kill();

    assert_eq!(messages[0]["command"], json!("initialize"));
    assert_eq!(messages[1]["event"], json!("progressStart"));
    assert_eq!(messages[1]["body"]["progressId"], json!("launch"));
    assert_eq!(messages[2]["event"], json!("progressEnd"));
    assert_eq!(messages[2]["body"]["progressId"], json!("launch"));
    assert_eq!(
        messages[3]["success"], json!(false),
        "the launch response itself still follows: {:?}",
        messages[3]
    );
}

/// A client that never said it accepts progress events gets none - sending
/// them anyway would be a protocol violation, not just noise.
#[test]
fn a_client_that_never_said_so_gets_no_progress_events() {
    let mut child = adapter()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    stdin
        .write_all(
            frame(&json!({"seq": 1, "type": "request", "command": "initialize"})).as_bytes()
        )
        .unwrap();
    stdin
        .write_all(
            frame(&json!({
                "seq": 2, "type": "request", "command": "launch",
                "arguments": {"program": "/nonexistent/nowhere.asm"}
            }))
            .as_bytes()
        )
        .unwrap();
    stdin.flush().unwrap();

    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let messages = read_messages(&mut reader, 2);
    let _ = child.kill();

    assert_eq!(messages[0]["command"], json!("initialize"));
    assert_eq!(
        messages[1]["success"], json!(false),
        "no progressStart/progressEnd in between: {:?}",
        messages
    );
}

/// What the map really records for a contested address, on a real project.
///
/// Run with a copy of birthtro at /tmp/bt:
/// `cargo test -p cpclib-dap --test end_to_end map_at -- --ignored --nocapture`
#[test]
#[ignore]
fn map_at_the_contested_address() {
    let entry = std::path::Path::new("/tmp/bt/src/sna.asm");
    if !entry.exists() {
        eprintln!("no /tmp/bt; copy the project there first");
        return;
    }
    let config = cpclib_project::config::load_config(Some(std::path::Path::new("/tmp/bt")))
        .config
        .asm;
    let built = match cpclib_dap::launch::assemble_for_debug(entry, &config) {
        Ok(built) => built,
        Err(problem) => {
            eprintln!("assembling failed: {problem}");
            return;
        }
    };

    for address in [0x79F3u32, 0x5C44] {
        let candidates = built.source_map.candidates_at(address);
        eprintln!("--- 0x{address:04X}: {} candidate(s)", candidates.len());
        for (page, location) in &candidates {
            eprintln!(
                "    page {page}: {}:{}",
                location.file.display(),
                location.line
            );
        }
    }
}

/// Where a real project's `BREAKPOINT` directives are written.
///
/// The one that matters is written inside `macros.asm`'s `DEBUG` macro and
/// arms an address in whichever file used the macro - the stop the user
/// reported as "the emulator stopped at random locations". Run with a copy of
/// birthtro at /tmp/bt, its `DEBUG` macro holding a `breakpoint`:
/// `cargo test -p cpclib-dap --test end_to_end directives_ -- --ignored --nocapture`
#[test]
#[ignore]
fn directives_say_where_they_are_written() {
    let entry = std::path::Path::new("/tmp/bt/src/sna.asm");
    if !entry.exists() {
        eprintln!("no /tmp/bt; copy the project there first");
        return;
    }
    let config = cpclib_project::config::load_config(Some(std::path::Path::new("/tmp/bt")))
        .config
        .asm;
    let built = match cpclib_dap::launch::assemble_for_debug(entry, &config) {
        Ok(built) => built,
        Err(problem) => {
            eprintln!("assembling failed: {problem}");
            return;
        }
    };

    for breakpoint in built.breakpoints.iter().take(20) {
        let written = breakpoint
            .written_at
            .as_ref()
            .map(|at| format!("{}:{}:{}", at.file, at.line, at.column))
            .unwrap_or_else(|| "unknown".to_string());
        let stops_at = built
            .source_map
            .location_at(breakpoint.address as u32)
            .map(|at| format!("{}:{}", at.file.display(), at.line))
            .unwrap_or_else(|| "nowhere".to_string());
        eprintln!(
            "0x{:04X}: written {written}, stops at {stops_at}",
            breakpoint.address
        );
    }
    assert!(
        built.breakpoints.iter().all(|b| b.written_at.is_some()),
        "every directive knows where it was written"
    );
}
