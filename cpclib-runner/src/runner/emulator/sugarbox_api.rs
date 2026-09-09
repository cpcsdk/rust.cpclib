//! Talking to SugarBoxV2's own embedded JSON-over-TCP debug server
//! (`Sugarbox/debugers/DebugServer.cpp`).
//!
//! One-shot only: connects, sends exactly one real command, reads its
//! answer, and disconnects. `cpclib-dap`'s own `SugarBoxPeer`
//! (`cpclib-dap/src/sugarbox.rs`) keeps ONE persistent connection open for
//! an entire live debug session instead, since it needs to receive
//! breakpoint/stop events asynchronously while stepping - a materially
//! different connection-lifetime model this module deliberately does not
//! try to unify with (forcing a debug session through one-shot connections
//! would break its event-ordering guarantees, and this Robot-driven
//! instance always runs on its own port anyway - see
//! `SUGARBOX_ROBOT_DEBUG_SERVER_PORT` in `emucontrol.rs`). What genuinely
//! is shared with it: the wire format itself (newline-delimited JSON,
//! `{"cmd": ...}` requests, the no-request-id "the first non-event line is
//! the answer" rule documented there) - the request/response shapes below
//! were cross-checked against the very same protocol, live, against a
//! running 2.1.1 instance.
//!
//! Important, confirmed live: the *first* debug-server connection after
//! launch halts the emulated CPU immediately (`Break()`) and it does not
//! auto-resume when that connection closes - it stays halted until
//! something sends `{"cmd":"continue"}`. Every function here does that as
//! its last step before disconnecting, so a Robot-driven SugarBoxV2
//! instance keeps running normally between occasional screenshot/memory
//! peeks instead of silently freezing the first time one is taken.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use base64::Engine;
use serde_json::{Value, json};

/// Connects, sends `cmd`, returns the first non-`{"type":"event",...}`
/// response, sends `{"cmd":"continue"}`, and disconnects - see this
/// module's own doc comment for why the trailing continue matters.
fn send_command(port: u16, cmd: Value) -> Result<Value, String> {
    let addr = format!("127.0.0.1:{port}");
    let stream = TcpStream::connect(&addr)
        .map_err(|e| format!("Failed to connect to SugarBoxV2's debug server at {addr}: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
    let mut writer = stream.try_clone().map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream);

    writer
        .write_all(format!("{cmd}\n").as_bytes())
        .map_err(|e| format!("Failed to send {cmd} to SugarBoxV2: {e}"))?;

    let answer = loop {
        let mut line = String::new();
        let n = reader
            .read_line(&mut line)
            .map_err(|e| format!("Failed to read SugarBoxV2's response to {cmd}: {e}"))?;
        if n == 0 {
            return Err(format!(
                "SugarBoxV2 closed the debug-server connection without answering {cmd}"
            ));
        }
        let value: Value = serde_json::from_str(line.trim())
            .map_err(|e| format!("Malformed JSON from SugarBoxV2: {e} (got {line:?})"))?;
        if value.get("type").and_then(|t| t.as_str()) == Some("event") {
            // An async event pushed ahead of the real answer - keep reading.
            continue;
        }
        break value;
    };

    // Leave the machine running - see this module's doc comment.
    let _ = writer.write_all(b"{\"cmd\":\"continue\"}\n");

    Ok(answer)
}

/// `{"cmd":"getScreen"}` - confirmed live against a running 2.1.1 instance:
/// the response is `{"data": "<base64 PNG>", "format": "png", "width",
/// "height"}`. Returns the raw PNG bytes; decoding them into an image is
/// left to the caller.
pub fn get_screenshot_png(port: u16) -> Result<Vec<u8>, String> {
    let response = send_command(port, json!({"cmd": "getScreen"}))?;
    let data = response
        .get("data")
        .and_then(|d| d.as_str())
        .ok_or_else(|| format!("getScreen response has no `data` field: {response}"))?;
    base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|e| format!("Failed to base64-decode SugarBoxV2's screenshot: {e}"))
}

/// `{"cmd":"readMemory","address":..,"size":..}` - confirmed live: response
/// is `{"bytes":[<u8>, ...]}` (a plain JSON int array, not hex or base64).
pub fn read_memory(port: u16, address: u16, count: u16) -> Result<Vec<u8>, String> {
    let response = send_command(port, json!({"cmd": "readMemory", "address": address, "size": count}))?;
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

/// `{"cmd":"writeMemory","address":..,"bytes":[..]}` - confirmed live:
/// applies synchronously (unlike AMSpiriT Lite's queued `/api/ram` write -
/// a readback right after this returns already sees the new bytes), and
/// answers `{"status":"ok","written":<count>}`.
pub fn write_memory(port: u16, address: u16, data: &[u8]) -> Result<(), String> {
    let response =
        send_command(port, json!({"cmd": "writeMemory", "address": address, "bytes": data}))?;
    if response.get("status").and_then(|s| s.as_str()) == Some("ok") {
        Ok(())
    }
    else {
        Err(format!("SugarBoxV2 writeMemory did not report ok: {response}"))
    }
}
