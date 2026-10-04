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

/// A DAP `output` event with `category: "stderr"` - "something went wrong,
/// tell the user via the Debug Console" is built this exact way, by hand,
/// at every one of its ~11 call sites across `session.rs`/`basic_session.rs`.
/// `seq` is still the caller's own to supply - some sites use `self.next_seq()`,
/// others a fixed `1` (an existing, unexplained difference between the two
/// session types, left exactly as each already had it: fixing *that* is a
/// correctness question, not something a reuse consolidation should change
/// on its own).
pub fn stderr_output_event(message: impl Into<String>, seq: i64) -> Value {
    event(
        "output",
        json!({ "category": "stderr", "output": message.into() }),
        seq
    )
}

pub fn request(command: &str, arguments: Value, seq: i64) -> Value {
    json!({ "seq": seq, "type": "request", "command": command, "arguments": arguments })
}

/// `0x` + four hex digits, the form the emulator's DAP uses for addresses.
pub fn address_reference(address: u32) -> String {
    format!("0x{address:04x}")
}

/// DAP `variables` entries for whichever of `names` are present in `state`
/// (a flat register-name-keyed object, as SugarboxV2's and ACE's own
/// `getStatus`/`readRegisters` replies both already are), each formatted as
/// 4-digit hex. Shared by `sugarbox`/`ace` (previously identical,
/// byte-for-byte, differing only in their own `names` list); AMSpiriT
/// Lite's own `registers_of` is deliberately not folded in here - its state
/// shape needs a real per-register value lookup (`register(state, name)`,
/// not a plain `state.get(name)`) and variable-width formatting (2 hex
/// digits for 8-bit registers, 4 for 16-bit), not just a different name
/// list.
pub fn registers_of(state: &Value, names: &[&str]) -> Vec<Value> {
    names
        .iter()
        .filter_map(|name| {
            state.get(*name).and_then(Value::as_u64).map(|value| {
                json!({
                    "name": name,
                    "value": format!("0x{value:04X}"),
                    "variablesReference": 0
                })
            })
        })
        .collect()
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
