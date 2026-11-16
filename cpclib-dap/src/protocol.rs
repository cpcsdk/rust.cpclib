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

#[cfg(feature = "adapter")]
pub use cpclib_runner::web::{
    decode_content_length_messages as decode, encode_content_length_message as encode
};
use serde_json::{Value, json};

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

/// The RAM configuration and page of `paging`, when they are all it chooses (with the ROMs
/// turned off at most): a RAM view, which an emulator that pages nothing else (AMSpiriT Lite) can
/// be read through, bank by bank.
pub fn ram_view(paging: &cpclib_bemu_api::Paging) -> Option<(u8, Option<u8>)> {
    let rest = cpclib_bemu_api::Paging {
        ram_config: None,
        page: None,
        lower_rom: paging.lower_rom.filter(|paged| *paged),
        upper_rom: paging.upper_rom.filter(|paged| *paged),
        ..*paging
    };
    if !rest.is_cpu() {
        return None;
    }
    Some((paging.ram_config?, paging.page))
}

/// Where a view shows a location in a space, and the paging that puts the space there - so its
/// addresses, labels and disassembly read as where it runs: an upper ROM at `&C000`, a CPC+
/// cartridge page there too (as upper ROM 128 + n, the way a CPC+ reaches any of them), the lower
/// ROM at `&0000`, the ASIC's registers at `&4000`, a RAM bank of the main 64K in its own block
/// (with the ROM over it turned off), an extension's at `&4000` (`C4`-`C7` and its page). `None`
/// for a CPU address, which has no space to show.
pub fn view_of_location(location: cpclib_bemu_api::Location) -> Option<(u32, cpclib_bemu_api::Paging)> {
    use cpclib_bemu_api::{Location, Paging, Space};
    let Location::In { space, offset } = location
    else {
        return None;
    };
    let paging = Paging::default();
    let (base, paging) = match space {
        Space::Ram(bank @ 0..=3) => (
            u32::from(bank) * 0x4000,
            Paging {
                ram_config: Some(0),
                lower_rom: (bank == 0).then_some(false),
                upper_rom: (bank == 3).then_some(false),
                ..paging
            }
        ),
        Space::Ram(bank) => (
            0x4000,
            Paging {
                ram_config: Some(4 + ((bank - 4) % 4) as u8),
                page: Some(u8::try_from((bank - 4) / 4).ok()?),
                ..paging
            }
        ),
        Space::Lower => (
            0,
            Paging {
                lower_rom: Some(true),
                lower_block: Some(0),
                ..paging
            }
        ),
        Space::Upper(slot) => (
            0xC000,
            Paging {
                upper_rom: Some(true),
                upper_number: Some(slot),
                ..paging
            }
        ),
        Space::Cartridge(page) => (
            0xC000,
            Paging {
                upper_rom: Some(true),
                upper_number: Some(128u8.checked_add(page)?),
                ..paging
            }
        ),
        Space::Asic => (
            0x4000,
            Paging {
                registers: Some(true),
                ..paging
            }
        )
    };
    Some(((base + offset) & 0xFFFF, paging))
}

/// [`view_of_location`] of a location as written (`UPPER07:0000`); `None` for anything else,
/// a CPU address included.
pub fn view_of_location_text(text: &str) -> Option<(u32, cpclib_bemu_api::Paging)> {
    view_of_location(cpclib_bemu_api::Location::parse(text)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_location_is_shown_where_it_runs() {
        let view = |text: &str| view_of_location_text(text).map(|(address, paging)| (address, paging.to_string()));
        assert_eq!(view("UPPER07:0010"), Some((0xC010, "U7".to_owned())));
        assert_eq!(view("CART03:0000"), Some((0xC000, "U131".to_owned())));
        assert_eq!(view("LOWER:0038"), Some((0x0038, "L,LB0".to_owned())));
        assert_eq!(view("ASIC:0020"), Some((0x4020, "R".to_owned())));
        assert_eq!(view("B00:0100"), Some((0x0100, "C0,-L".to_owned())));
        assert_eq!(view("B01:0100"), Some((0x4100, "C0".to_owned())));
        assert_eq!(view("B03:0000"), Some((0xC000, "C0,-U".to_owned())));
        // an extension's bank: C4-C7 put it at &4000, with its page
        assert_eq!(view("B04:0010"), Some((0x4010, "C4,P0".to_owned())));
        assert_eq!(view("P1B2:0000"), Some((0x4000, "C6,P1".to_owned())));
        assert_eq!(view("0x4000"), None, "a CPU address has no space to show");
        assert_eq!(view("HL"), None);
    }

    #[test]
    fn a_ram_view_is_a_configuration_and_a_page_at_most_with_the_roms_off() {
        let ram_view = |text: &str| ram_view(&cpclib_bemu_api::Paging::parse(text).unwrap());
        assert_eq!(ram_view("C4"), Some((4, None)));
        assert_eq!(ram_view("C4,P2,-L,-U"), Some((4, Some(2))));
        assert_eq!(ram_view("C4,U7"), None, "an upper ROM is not RAM");
        assert_eq!(ram_view("C4,W"), None);
        assert_eq!(ram_view("-L"), None, "no configuration to read through");
    }

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
    #[cfg(feature = "adapter")]
    fn the_reexported_framing_still_round_trips() {
        let message = json!({"seq": 1, "type": "request", "command": "initialize"});
        let mut buffer = encode(&message).into_bytes();
        assert_eq!(decode(&mut buffer), vec![message]);
        assert!(buffer.is_empty());
    }
}
