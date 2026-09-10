//! DAP messages, kept as JSON.
//!
//! An adapter that sits *between* two peers has to round-trip messages it does
//! not model: unknown requests, unknown response bodies, fields added by a
//! future version of either side. A strongly-typed enum that silently drops
//! what it does not know would corrupt the conversation, so messages travel as
//! `serde_json::Value` and only the handful of bodies actually inspected get
//! typed accessors.
//!
//! Message *framing* (Content-Length, the transport both DAP peers use) is
//! not DAP-specific business logic - it's the same scheme LSP uses - and
//! lives in `cpclib_runner::web::server` instead, next to the transport
//! that actually carries framed bytes in (`dap.js`'s own SSE output). This
//! module re-exports it under its own names so existing call sites here
//! don't care where it lives.

use serde_json::{Value, json};

pub use cpclib_runner::web::{
    decode_content_length_messages as decode, encode_content_length_message as encode
};

/// A response to `request`.
pub fn response(request: &Value, body: Value, seq: i64) -> Value {
    json!({
        "seq": seq,
        "type": "response",
        "request_seq": request.get("seq").and_then(Value::as_i64).unwrap_or(0),
        "success": true,
        "command": request.get("command").and_then(Value::as_str).unwrap_or(""),
        "body": body
    })
}

/// A failed response to `request`.
pub fn failure(request: &Value, message: &str, seq: i64) -> Value {
    json!({
        "seq": seq,
        "type": "response",
        "request_seq": request.get("seq").and_then(Value::as_i64).unwrap_or(0),
        "success": false,
        "command": request.get("command").and_then(Value::as_str).unwrap_or(""),
        "message": message
    })
}

pub fn event(name: &str, body: Value, seq: i64) -> Value {
    json!({ "seq": seq, "type": "event", "event": name, "body": body })
}

pub fn request(command: &str, arguments: Value, seq: i64) -> Value {
    json!({ "seq": seq, "type": "request", "command": command, "arguments": arguments })
}

/// `0x` + four hex digits, the form the emulator's DAP uses for addresses.
pub fn address_reference(address: u32) -> String {
    format!("0x{address:04x}")
}

pub fn parse_address_reference(reference: &str) -> Option<u32> {
    let trimmed = reference.trim();
    let hex = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .or_else(|| trimmed.strip_prefix('&'));
    match hex {
        Some(digits) => u32::from_str_radix(digits, 16).ok(),
        None => trimmed.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_round_trip() {
        assert_eq!(address_reference(0x4000), "0x4000");
        assert_eq!(parse_address_reference("0x4000"), Some(0x4000));
        assert_eq!(parse_address_reference("&BB5A"), Some(0xBB5A));
        assert_eq!(parse_address_reference("16384"), Some(16384));
        assert_eq!(parse_address_reference("nonsense"), None);
    }

    /// The framing itself is tested where it lives
    /// (`cpclib_runner::web::server`) - this just confirms the re-export
    /// under this module's own names actually round-trips.
    #[test]
    fn the_reexported_framing_still_round_trips() {
        let message = json!({"seq": 1, "type": "request", "command": "initialize"});
        let mut buffer = encode(&message).into_bytes();
        assert_eq!(decode(&mut buffer), vec![message]);
        assert!(buffer.is_empty());
    }
}
