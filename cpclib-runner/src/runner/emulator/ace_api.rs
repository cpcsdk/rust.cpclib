//! Talking to ACE's own embedded JSON-over-TCP web API (`-enable_webapi`).
//!
//! One-shot only, same model as `sugarbox_api.rs`: connects, sends exactly
//! one real command, reads its answer, and disconnects - confirmed live
//! against a running instance (`Xvfb` + `AceDL -enable_webapi -web_port
//! <port> -borderless`) that reusing one TCP connection for several
//! back-to-back commands is unreliable (occasional broken pipes, and the
//! connection was observed to close itself right after `{"cmd":"continue"}`
//! specifically), so unlike `sugarbox_api.rs` this isn't a stylistic choice
//! shared with a differently-lifetimed peer - it's the only connection
//! model that was actually observed to work.
//!
//! Two confirmed protocol differences from SugarBoxV2's debug server:
//! - `readMemory`/`writeMemory` require `memType`/`bank` fields (omitting
//!   `memType` answers `{"error":"Unsupported memType"}`) - `"ram"`/`-1` is
//!   the confirmed-working default for both.
//! - Connecting does NOT halt the CPU (unlike SugarBoxV2's `Break()` on
//!   first connect), so there is no need for `sugarbox_api.rs`'s trailing
//!   `{"cmd":"continue"}` courtesy-resume before disconnecting.
//!
//! Everything else about the wire format matches: newline-delimited JSON,
//! `{"cmd": ...}` requests, "the first non-event line is the answer".

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use serde_json::{Value, json};

/// Connects, sends `cmd`, returns the first non-`{"type":"event",...}`
/// response, and disconnects - see this module's own doc comment for why
/// there's no trailing courtesy command here, unlike `sugarbox_api.rs`'s
/// `send_command`.
///
/// Unlike SugarboxV2, ACE's real replies are pretty-printed across several
/// physical lines (confirmed live: a `readMemory` answer arrives as `"{\n"`
/// on one `read_line` call, the actual `"bytes":[...]"` content on the
/// next) - so a single `read_line` per message, as `sugarbox_api.rs` gets
/// away with, isn't enough here. Lines are accumulated into `buffer` and
/// re-parsed as a whole after each one; a parse failure only means "the
/// message isn't complete yet, keep reading" (`serde_json`'s own EOF error
/// for a still-open `{`), not a hard error - only a closed connection or a
/// read-timeout is.
fn send_command(port: u16, cmd: Value) -> Result<Value, String> {
    let addr = format!("127.0.0.1:{port}");
    let stream = TcpStream::connect(&addr)
        .map_err(|e| format!("Failed to connect to ACE's web API at {addr}: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
    let mut writer = stream.try_clone().map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream);

    writer
        .write_all(format!("{cmd}\n").as_bytes())
        .map_err(|e| format!("Failed to send {cmd} to ACE: {e}"))?;

    let mut buffer = String::new();
    let answer = loop {
        let mut line = String::new();
        let n = reader
            .read_line(&mut line)
            .map_err(|e| format!("Failed to read ACE's response to {cmd}: {e}"))?;
        if n == 0 {
            return Err(format!("ACE closed the web-API connection without answering {cmd}"));
        }
        buffer.push_str(&line);

        let Ok(value) = serde_json::from_str::<Value>(buffer.trim()) else {
            // Message not complete yet - keep accumulating lines.
            continue;
        };
        if value.get("type").and_then(|t| t.as_str()) == Some("event") {
            // A complete async event pushed ahead of the real answer -
            // keep reading, starting a fresh message.
            buffer.clear();
            continue;
        }
        break value;
    };

    Ok(answer)
}

/// `{"cmd":"readMemory","address":..,"size":..,"memType":"ram","bank":-1}` -
/// confirmed live: response is `{"bytes":[<u8>, ...]}` (a plain JSON int
/// array, not hex or base64). `memType`/`bank` are mandatory on ACE's side -
/// omitting `memType` answers `{"error":"Unsupported memType"}`.
pub fn read_memory(port: u16, address: u16, count: u16) -> Result<Vec<u8>, String> {
    let response = send_command(
        port,
        json!({
            "cmd": "readMemory",
            "address": address,
            "size": count,
            "memType": "ram",
            "bank": -1
        })
    )?;
    let bytes = response
        .get("bytes")
        .and_then(|b| b.as_array())
        .ok_or_else(|| format!("readMemory response has no `bytes` field: {response}"))?;
    bytes
        .iter()
        .map(|v| {
            v.as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .ok_or_else(|| format!("readMemory response has a non-byte value: {v}"))
        })
        .collect()
}

/// `{"cmd":"writeMemory","address":..,"bytes":[..],"memType":"ram","bank":-1}`
/// - confirmed live: answers `{"status":"ok"}`.
pub fn write_memory(port: u16, address: u16, data: &[u8]) -> Result<(), String> {
    let response = send_command(
        port,
        json!({
            "cmd": "writeMemory",
            "address": address,
            "bytes": data,
            "memType": "ram",
            "bank": -1
        })
    )?;
    if response.get("status").and_then(|s| s.as_str()) == Some("ok") {
        Ok(())
    }
    else {
        Err(format!("ACE writeMemory did not report ok: {response}"))
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    use super::*;

    /// Spins up a fake one-shot ACE server on a background thread: accepts
    /// one connection, reads one line, replies with `response`, closes.
    fn fake_server(response: &'static str) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                let _ = reader.read_line(&mut line);
                let mut writer = stream;
                let _ = writer.write_all(response.as_bytes());
            }
        });
        port
    }

    #[test]
    fn read_memory_request_carries_mem_type_and_bank() {
        let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests_clone = requests.clone();
        std::thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                let _ = reader.read_line(&mut line);
                requests_clone.lock().unwrap().push(line);
                let mut writer = stream;
                let _ = writer.write_all(b"{\"bytes\":[1,2,3]}\n");
            }
        });

        let bytes = read_memory(port, 0x4000, 3).unwrap();
        assert_eq!(bytes, vec![1, 2, 3]);

        let sent = requests.lock().unwrap();
        let sent_value: Value = serde_json::from_str(sent[0].trim()).unwrap();
        assert_eq!(sent_value["memType"], "ram");
        assert_eq!(sent_value["bank"], -1);
        assert_eq!(sent_value["address"], 0x4000);
        assert_eq!(sent_value["size"], 3);
    }

    #[test]
    fn write_memory_success_parses_ok_status() {
        let port = fake_server("{\"status\":\"ok\"}\n");
        write_memory(port, 0x4000, &[1, 2, 3]).unwrap();
    }

    #[test]
    fn write_memory_failure_surfaces_as_error() {
        let port = fake_server("{\"status\":\"error\",\"message\":\"nope\"}\n");
        assert!(write_memory(port, 0x4000, &[1, 2, 3]).is_err());
    }

    /// The memory-API equivalent of `sugarbox.rs`'s own
    /// `v2_1_1_installs_and_runs_without_the_appimage_libpthread_crash`:
    /// install the real ACE distribution if not already cached, launch it
    /// for real with `-enable_webapi -web_port <port> -borderless`, and
    /// prove `write_memory`/`read_memory` round-trip against the real
    /// binary - not just a fake socket. `#[ignore]`d like this crate's
    /// other real-download/real-process tests: downloads the distribution
    /// and spawns a real emulator process, too heavy for the default
    /// `cargo test` run.
    ///
    /// Run with: `cargo test -p cpclib-runner --features screenshot --lib
    /// runner::emulator::ace_api::tests::write_then_read_memory_round_trips_against_a_real_ace_instance
    /// -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn write_then_read_memory_round_trips_against_a_real_ace_instance() {
        use std::net::TcpStream;
        use std::time::{Duration, Instant};

        use cpclib_common::event::CapturingObserver;

        use crate::delegated::InternetDynamicCompiledApplication;
        use crate::runner::emulator::ace::AceVersion;

        let version = AceVersion::default();
        let conf = version.configuration::<CapturingObserver>();
        if !conf.is_cached() {
            let observer = CapturingObserver::new();
            conf.install(&observer).expect("installing ACE failed");
        }

        let port = 18765u16;
        let mut child = std::process::Command::new(conf.exec_fname())
            .current_dir(conf.cache_folder())
            .args(["-enable_webapi", "-web_port", &port.to_string(), "-borderless"])
            .spawn()
            .expect("failed to spawn ACE");

        // No `wait_until_listening` helper in this crate (that's
        // `cpclib-dap/src/sugarbox.rs`'s, a downstream crate this one
        // can't depend on) - poll a plain TCP connect instead.
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if TcpStream::connect_timeout(
                &format!("127.0.0.1:{port}").parse().unwrap(),
                Duration::from_millis(200)
            )
            .is_ok()
            {
                break;
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                panic!("ACE's web API never started listening on port {port}");
            }
            std::thread::sleep(Duration::from_millis(200));
        }

        let result = std::panic::catch_unwind(|| {
            write_memory(port, 0x4000, &[1, 2, 3, 4]).expect("writeMemory failed");
            let bytes = read_memory(port, 0x4000, 4).expect("readMemory failed");
            assert_eq!(bytes, vec![1, 2, 3, 4]);
        });

        let _ = child.kill();
        let _ = child.wait();
        result.unwrap();
    }

    #[test]
    fn an_interleaved_event_line_ahead_of_the_real_answer_is_skipped() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                let _ = reader.read_line(&mut line);
                let mut writer = stream;
                let _ = writer.write_all(b"{\"type\":\"event\",\"event\":\"stopped\"}\n");
                let _ = writer.write_all(b"{\"bytes\":[42]}\n");
            }
        });

        let bytes = read_memory(port, 0, 1).unwrap();
        assert_eq!(bytes, vec![42]);
    }
}
