//! Text helpers shared by the views and the adapter: numbers as people type them, base64 as
//! DAP carries bytes.

/// Bytes as DAP carries them: the emulator answers hex, DAP asks for base64.
pub(crate) fn encode_base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let block = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0)
        ];
        let packed = ((block[0] as u32) << 16) | ((block[1] as u32) << 8) | block[2] as u32;
        for index in 0..4 {
            if index <= chunk.len() {
                out.push(ALPHABET[((packed >> (18 - 6 * index)) & 0x3F) as usize] as char);
            }
            else {
                out.push('=');
            }
        }
    }
    out
}

/// `0x`/`&`/`#`/`$`-prefixed hex, `0o`/`@`-prefixed octal, `%`/`0b`-prefixed
/// binary, or plain decimal. Reuses `cpclib_common`'s own number parser (the
/// same one the assembler's own source-level number literals go through)
/// instead of this session's own narrower one, which used to only know
/// about hex/decimal - see `basic_session::parse_address`, which already
/// made this switch and is a strict superset of what this used to accept.
pub(crate) fn parse_number(text: &str) -> Option<u32> {
    use cpclib_common::winnow::Parser;
    use cpclib_common::winnow::error::ContextError;
    use cpclib_common::winnow::stream::AsBStr;
    cpclib_common::parse_value::<_, ContextError>
        .parse(text.trim().as_bstr())
        .ok()
}

#[cfg_attr(not(feature = "adapter"), allow(dead_code))]
pub(crate) fn decode_base64(encoded: &str) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let mut accumulator = 0u32;
    let mut bits = 0u32;
    for byte in encoded.bytes() {
        if byte == b'=' {
            break;
        }
        let Some(index) = ALPHABET.iter().position(|c| *c == byte)
        else {
            continue; // whitespace and anything else are not data
        };
        accumulator = (accumulator << 6) | index as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((accumulator >> bits) as u8);
        }
    }
    out
}
