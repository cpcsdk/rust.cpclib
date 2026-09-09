//! Talking to AMSpiriT Lite's own embedded HTTP debug server.
//!
//! Shared transport for two very different callers: `cpclib-dap` (a live
//! debug session, `cpclib-dap/src/amspiritlite.rs`) and this crate's own
//! `Robot` automation (`emucontrol.rs`, screenshots/keytype/memory peeking
//! outside a debug session). Both used to speak this exact same HTTP API
//! with two separate hand-rolled clients - this module is the single one
//! both now use, so the request/response shapes are verified once, in one
//! place, against a live emulator (see this crate's own tests and
//! `cpclib-dap`'s `live_tests` module).
//!
//! Raw HTTP over a plain `TcpStream` rather than a client crate: this is
//! loopback-only, the requests are a handful of fixed shapes, and pulling
//! in a general-purpose HTTP client for that would be a heavier dependency
//! than the protocol it's replacing.
//!
//! Every shape here (`/api/ram`'s query params and response fields,
//! `/api/screenshot`'s params, `/api/keytype`'s body) was cross-checked
//! against a running 1.14.3 instance's own `GET /api/doc`/`GET
//! /api/doc/<name>` - the emulator's authoritative, self-generated
//! documentation - not against the older, occasionally-inaccurate published
//! docs (see `cpclib-dap/src/amspiritlite.rs`'s own doc comment for the
//! history there).

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use serde_json::json;

/// The base a session talks to. The emulator's own out-of-the-box default.
pub const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:8765";

/// One HTTP call, described rather than made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub method: Method,
    pub path: &'static str,
    /// `key=value` pairs, already in the order the emulator expects.
    pub query: Vec<(String, String)>,
    pub body: Option<String>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post
}

impl Call {
    pub fn get(path: &'static str) -> Self {
        Self {
            method: Method::Get,
            path,
            query: Vec::new(),
            body: None
        }
    }

    pub fn post(path: &'static str) -> Self {
        Self {
            method: Method::Post,
            path,
            query: Vec::new(),
            body: None
        }
    }

    pub fn query(mut self, key: &str, value: impl std::fmt::Display) -> Self {
        self.query.push((key.to_string(), value.to_string()));
        self
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    fn path_with_query(&self) -> String {
        let mut path = self.path.to_string();
        if !self.query.is_empty() {
            let query: Vec<String> =
                self.query.iter().map(|(key, value)| format!("{key}={value}")).collect();
            path.push('?');
            path.push_str(&query.join("&"));
        }
        path
    }

    fn request_bytes(&self, host: &str) -> Vec<u8> {
        let path = self.path_with_query();
        match (&self.method, &self.body) {
            (Method::Get, _) => {
                format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n")
                    .into_bytes()
            },
            (Method::Post, body) => {
                let body = body.clone().unwrap_or_default();
                format!(
                    "POST {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\
                     Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                )
                .into_bytes()
            }
        }
    }
}

/// `http://127.0.0.1:8765` into `127.0.0.1:8765`.
pub fn host_of(endpoint: &str) -> std::io::Result<String> {
    let rest = endpoint
        .trim_end_matches('/')
        .strip_prefix("http://")
        .ok_or_else(|| std::io::Error::other(format!("{endpoint} is not an http:// address")))?;
    Ok(rest.to_string())
}

fn connect_and_send(endpoint: &str, call: &Call) -> std::io::Result<TcpStream> {
    let host = host_of(endpoint)?;
    let mut stream = TcpStream::connect(&host)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.write_all(&call.request_bytes(&host))?;
    Ok(stream)
}

/// Make one call and return its body as text - JSON/plain-text responses
/// (every endpoint except `/api/screenshot`).
pub fn perform(endpoint: &str, call: &Call) -> std::io::Result<String> {
    let mut stream = connect_and_send(endpoint, call)?;
    let mut raw = String::new();
    stream.read_to_string(&mut raw)?;
    Ok(body_of(&raw).to_string())
}

/// Make one call and return its body as raw bytes - for `/api/screenshot`,
/// whose PNG response isn't valid UTF-8 (so `perform`'s `read_to_string`
/// would fail on it).
pub fn perform_bytes(endpoint: &str, call: &Call) -> std::io::Result<Vec<u8>> {
    let mut stream = connect_and_send(endpoint, call)?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;
    Ok(bytes_body_of(&raw).to_vec())
}

/// Everything after the blank line that ends the headers.
pub fn body_of(response: &str) -> &str {
    match response.find("\r\n\r\n") {
        Some(at) => &response[at + 4..],
        None => ""
    }
}

/// Byte-oriented twin of `body_of`, for a response whose body isn't valid
/// UTF-8.
fn bytes_body_of(response: &[u8]) -> &[u8] {
    match response.windows(4).position(|w| w == b"\r\n\r\n") {
        Some(at) => &response[at + 4..],
        None => &[]
    }
}

/// `"3E 00 C9"` or `"3E00C9"` into bytes.
pub fn bytes_from_hex(hex: &str) -> Vec<u8> {
    let digits: Vec<u8> = hex.bytes().filter(|b| b.is_ascii_hexdigit()).collect();
    digits
        .chunks(2)
        .filter(|pair| pair.len() == 2)
        .filter_map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

/// The inverse of `bytes_from_hex` - what `/api/ram`'s own `data` field
/// (writes) and this module's own read parsing (`hex` field) both expect.
pub fn hex_from_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

/// `GET /api/ram?addr=..&len=..&view=cpu` - `view=cpu` (the Z80's own
/// mapped view, ROM overlays and paging applied) rather than the endpoint's
/// own default of `view=raw` (physical bank, no paging at all): confirmed
/// live that reading with the default silently returns physical bank 0
/// regardless of what's actually paged in, which only happens to match
/// under the *default*, unbanked RAM configuration - the CPC's normal,
/// widely-used banked RAM configs and ROM paging otherwise return bytes
/// that never match what the Z80 really sees. Response shape confirmed
/// live: `{"addr":..,"len":..,"view":"cpu","hex":".."}`.
pub fn read_memory(endpoint: &str, address: u16, count: u16) -> Result<Vec<u8>, String> {
    let call = Call::get("/api/ram")
        .query("addr", address)
        .query("len", count)
        .query("view", "cpu");
    let response = perform(endpoint, &call)
        .map_err(|e| format!("AMSpiriT Lite readMemory request failed: {e}"))?;
    let value: serde_json::Value = serde_json::from_str(&response)
        .map_err(|e| format!("Malformed JSON from AMSpiriT Lite's /api/ram: {e} ({response:?})"))?;
    let hex = value
        .get("hex")
        .and_then(|h| h.as_str())
        .ok_or_else(|| format!("/api/ram response has no `hex` field: {value}"))?;
    Ok(bytes_from_hex(hex))
}

/// `POST /api/ram {"addr":..,"data":"<hex>"}` - confirmed live that the
/// write is *queued*, applied at the emulator's next main-loop iteration
/// rather than synchronously (its own `/api/doc/ram` says so explicitly):
/// a caller reading straight back after writing needs to allow at least
/// one frame to pass first.
pub fn write_memory(endpoint: &str, address: u16, data: &[u8]) -> Result<(), String> {
    let call = Call::post("/api/ram")
        .body(json!({ "addr": address, "data": hex_from_bytes(data) }).to_string());
    perform(endpoint, &call)
        .map(|_| ())
        .map_err(|e| format!("AMSpiriT Lite writeMemory request failed: {e}"))
}

/// `GET /api/screenshot?crop=1&full=1` - `crop=1` asks for just the visible
/// screen area (not the full emulated frame's border/overscan), `full=1`
/// asks for a plain settled frame rather than a partial in-progress
/// composite. Returns the raw PNG bytes - decoding them into an image is
/// left to the caller, so this module needs no image-decoding dependency
/// of its own.
pub fn get_screenshot_png(endpoint: &str) -> Result<Vec<u8>, String> {
    let call = Call::get("/api/screenshot").query("crop", 1).query("full", 1);
    perform_bytes(endpoint, &call)
        .map_err(|e| format!("AMSpiriT Lite screenshot request failed: {e}"))
}

/// `POST /api/keytype {"text": ".."}` - queues a string of characters to be
/// auto-typed on the emulated keyboard, no window focus needed. Confirmed
/// live: posting `PRINT "HELLO"\n` typed and executed it exactly as real
/// keystrokes would.
pub fn keytype(endpoint: &str, text: &str) -> Result<(), String> {
    let call = Call::post("/api/keytype").body(json!({ "text": text }).to_string());
    perform(endpoint, &call).map(|_| ()).map_err(|e| format!("AMSpiriT Lite keytype request failed: {e}"))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn call_builds_query_and_body() {
        let call = Call::get("/api/ram").query("addr", 40000).query("len", 5);
        assert_eq!(call.path, "/api/ram");
        assert_eq!(
            call.query,
            vec![("addr".to_string(), "40000".to_string()), ("len".to_string(), "5".to_string())]
        );
        assert_eq!(call.path_with_query(), "/api/ram?addr=40000&len=5");

        let call = Call::post("/api/keytype").body("{}");
        assert_eq!(call.body.as_deref(), Some("{}"));
    }

    #[test]
    fn bytes_from_hex_tolerates_spaces_and_case() {
        assert_eq!(bytes_from_hex("3E 00 c9"), vec![0x3E, 0x00, 0xC9]);
        assert_eq!(bytes_from_hex("3E00C9"), vec![0x3E, 0x00, 0xC9]);
        assert_eq!(bytes_from_hex(""), Vec::<u8>::new());
    }

    #[test]
    fn hex_from_bytes_round_trips_through_bytes_from_hex() {
        let data = [0x01, 0xAB, 0xFF, 0x00];
        assert_eq!(bytes_from_hex(&hex_from_bytes(&data)), data);
    }

    #[test]
    fn body_of_splits_on_the_blank_line() {
        assert_eq!(body_of("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"), "ok");
        assert_eq!(body_of("no headers here"), "");
    }

    #[test]
    fn bytes_body_of_splits_on_the_blank_line() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\n\r\n\x89PNG\r\n";
        assert_eq!(bytes_body_of(response), b"\x89PNG\r\n");
        assert_eq!(bytes_body_of(b"no headers here"), b"");
    }
}
