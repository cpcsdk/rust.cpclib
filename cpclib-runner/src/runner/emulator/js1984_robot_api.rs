//! Talking to the robot bridge patched into a 1984js instance - see
//! `crate::web::robot_bridge`'s own doc comment for what that bridge is and
//! why it needs no DAP logic at all.
//!
//! One request in flight at a time, matching every other Robot backend:
//! send `{"id", "cmd", ...}` over the already-open [`ServerHandle`] channel,
//! then poll [`ServerHandle::try_recv`] for the `{"id", "ok", ...}` (or
//! `{"id", "ok": false, "error"}`) reply carrying the same id.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use base64::Engine;
use serde_json::{Value, json};

use crate::web::ServerHandle;

/// Send `body` (with a fresh `id` merged in) and wait up to `timeout` for the
/// matching reply.
fn call(server: &ServerHandle, mut body: Value, timeout: Duration) -> Result<Value, String> {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    body.as_object_mut()
        .expect("callers always pass a JSON object")
        .insert("id".to_owned(), json!(id));

    server.send(body.to_string()).map_err(|e| format!("cannot reach the robot bridge: {e}"))?;

    let deadline = Instant::now() + timeout;
    loop {
        if let Some(raw) = server.try_recv() {
            match serde_json::from_str::<Value>(&raw) {
                Ok(reply) if reply.get("id").and_then(Value::as_u64) == Some(id) => {
                    return match reply.get("ok").and_then(Value::as_bool) {
                        Some(true) => Ok(reply),
                        Some(false) => {
                            let error =
                                reply.get("error").and_then(Value::as_str).unwrap_or("unknown error");
                            Err(format!("robot bridge refused the request: {error}"))
                        },
                        None => Err(format!("robot bridge reply has no `ok` field: {reply}"))
                    };
                },
                // Not our reply (or not JSON at all) - keep waiting for ours.
                _ => continue
            }
        }
        if Instant::now() >= deadline {
            return Err("the robot bridge never answered (timed out)".to_owned());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// `{"cmd":"screenshot"}` - the reply carries `canvas.toDataURL("image/png")`
/// verbatim; this strips the `data:image/png;base64,` prefix and decodes it.
pub fn screenshot_png(server: &ServerHandle) -> Result<Vec<u8>, String> {
    let reply = call(server, json!({"cmd": "screenshot"}), Duration::from_secs(5))?;
    let data_url = reply
        .get("png")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("screenshot reply has no `png` field: {reply}"))?;
    let base64_part = data_url
        .split_once(',')
        .map(|(_, b64)| b64)
        .ok_or_else(|| format!("`png` is not a data URL: {data_url}"))?;
    base64::engine::general_purpose::STANDARD
        .decode(base64_part)
        .map_err(|e| format!("failed to base64-decode the screenshot: {e}"))
}

/// `{"cmd":"keytype","text":..}` - a generous timeout: each character takes
/// roughly the bridge's own 50ms press/release poll interval, so a long
/// string genuinely needs more than the 5s deadline the other calls use.
pub fn keytype(server: &ServerHandle, text: &str) -> Result<(), String> {
    call(server, json!({"cmd": "keytype", "text": text}), Duration::from_secs(30)).map(|_| ())
}

/// `{"cmd":"readMemory","address":..,"count":..}` - the reply carries
/// `bytes` as a plain JSON int array.
pub fn read_memory(server: &ServerHandle, address: u16, count: u16) -> Result<Vec<u8>, String> {
    let reply = call(
        server,
        json!({"cmd": "readMemory", "address": address, "count": count}),
        Duration::from_secs(5)
    )?;
    let bytes = reply
        .get("bytes")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("readMemory reply has no `bytes` field: {reply}"))?;
    bytes
        .iter()
        .map(|v| {
            v.as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .ok_or_else(|| format!("readMemory reply has a non-byte value: {v}"))
        })
        .collect()
}

/// `{"cmd":"writeMemory","address":..,"bytes":[..]}`.
pub fn write_memory(server: &ServerHandle, address: u16, data: &[u8]) -> Result<(), String> {
    call(
        server,
        json!({"cmd": "writeMemory", "address": address, "bytes": data}),
        Duration::from_secs(5)
    )
    .map(|_| ())
}
