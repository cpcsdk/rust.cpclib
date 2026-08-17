//! DAP messages, kept as JSON.
//!
//! An adapter that sits *between* two peers has to round-trip messages it does
//! not model: unknown requests, unknown response bodies, fields added by a
//! future version of either side. A strongly-typed enum that silently drops
//! what it does not know would corrupt the conversation, so messages travel as
//! `serde_json::Value` and only the handful of bodies actually inspected get
//! typed accessors.

use serde_json::{Value, json};

/// Content-Length framing, the transport both DAP peers use.
pub fn encode(message: &Value) -> String {
    let body = serde_json::to_string(message).expect("a DAP message must serialise");
    format!("Content-Length: {}\r\n\r\n{}", body.len(), body)
}

/// Pull whole messages out of an accumulating buffer, leaving any partial tail.
///
/// Returns the messages decoded and drains exactly the bytes they occupied -
/// a socket hands over arbitrary chunks, so a message may arrive in pieces or
/// several may arrive at once.
pub fn decode(buffer: &mut Vec<u8>) -> Vec<Value> {
    let mut out = Vec::new();
    loop {
        let Some(header_end) = find(buffer, b"\r\n\r\n")
        else {
            break;
        };
        let header = String::from_utf8_lossy(&buffer[..header_end]).to_string();
        let Some(length) = content_length(&header)
        else {
            // A header we cannot read is not going to become readable; drop it
            // rather than spin on it forever.
            buffer.drain(..header_end + 4);
            continue;
        };
        let body_start = header_end + 4;
        if buffer.len() < body_start + length {
            break; // the body has not all arrived yet
        }
        let body = &buffer[body_start..body_start + length];
        if let Ok(value) = serde_json::from_slice::<Value>(body) {
            out.push(value);
        }
        buffer.drain(..body_start + length);
    }
    out
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn content_length(header: &str) -> Option<usize> {
    header
        .lines()
        .find_map(|line| line.strip_prefix("Content-Length:"))
        .and_then(|value| value.trim().parse().ok())
}

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
    fn a_message_round_trips_through_the_framing() {
        let message = json!({"seq": 1, "type": "request", "command": "initialize"});
        let mut buffer = encode(&message).into_bytes();
        let decoded = decode(&mut buffer);
        assert_eq!(decoded, vec![message]);
        assert!(buffer.is_empty(), "the buffer is drained");
    }

    /// A socket splits wherever it likes; a message must survive arriving one
    /// byte at a time.
    #[test]
    fn a_message_split_across_reads_is_reassembled() {
        let message = json!({"seq": 7, "type": "event", "event": "stopped"});
        let encoded = encode(&message).into_bytes();
        let mut buffer = Vec::new();
        for byte in &encoded[..encoded.len() - 1] {
            buffer.push(*byte);
            assert!(decode(&mut buffer).is_empty(), "not yet complete");
        }
        buffer.push(*encoded.last().unwrap());
        assert_eq!(decode(&mut buffer), vec![message]);
    }

    #[test]
    fn several_messages_in_one_read_all_come_out() {
        let a = json!({"seq": 1, "type": "request", "command": "a"});
        let b = json!({"seq": 2, "type": "request", "command": "b"});
        let mut buffer = format!("{}{}", encode(&a), encode(&b)).into_bytes();
        assert_eq!(decode(&mut buffer), vec![a, b]);
    }

    /// Unknown fields survive the trip - the whole reason messages stay JSON.
    #[test]
    fn unknown_fields_are_preserved() {
        let message = json!({
            "seq": 1, "type": "request", "command": "somethingNew",
            "arguments": {"aFieldWeDoNotModel": [1, 2, 3]}
        });
        let mut buffer = encode(&message).into_bytes();
        assert_eq!(decode(&mut buffer), vec![message]);
    }

    #[test]
    fn addresses_round_trip() {
        assert_eq!(address_reference(0x4000), "0x4000");
        assert_eq!(parse_address_reference("0x4000"), Some(0x4000));
        assert_eq!(parse_address_reference("&BB5A"), Some(0xBB5A));
        assert_eq!(parse_address_reference("16384"), Some(16384));
        assert_eq!(parse_address_reference("nonsense"), None);
    }
}
