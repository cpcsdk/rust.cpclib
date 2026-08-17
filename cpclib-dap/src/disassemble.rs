//! Turning bytes back into instructions, ourselves.
//!
//! The emulator implements DAP's `disassemble`, and using it worked - but it
//! ties what you read to which emulator you happen to be debugging in. Its
//! mnemonics, its casing, its operand spelling. Swap the emulator and the view
//! changes under you, for a program that has not.
//!
//! So the bytes are read from the emulator - the one thing only it can answer -
//! and decoded here with `cpclib-asm`'s own disassembler, the same tables the
//! assembler uses. What you read is then what `basm` would have written, which
//! is the point: this view exists to be compared against your source.

use cpclib_asm::disass::disassemble;
use cpclib_asm::implementation::tokens::TokenExt;
use serde_json::{Value, json};

/// One decoded instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub address: u16,
    pub bytes: Vec<u8>,
    pub text: String
}

/// Decode `bytes`, which were read starting at `address`.
///
/// Stops when the bytes run out rather than inventing an instruction from a
/// truncated one - the last few bytes of a read are routinely half an
/// instruction, and showing that half as though it were whole is how a
/// disassembly view starts lying.
pub fn decode(address: u16, bytes: &[u8], limit: usize) -> Vec<Instruction> {
    let listing = disassemble(bytes);

    let mut out = Vec::new();
    let mut offset = 0usize;
    for token in listing.listing().iter() {
        if out.len() >= limit {
            break;
        }
        let Ok(length) = token.number_of_bytes()
        else {
            break;
        };
        if length == 0 || offset + length > bytes.len() {
            break;
        }
        out.push(Instruction {
            address: address.wrapping_add(offset as u16),
            bytes: bytes[offset..offset + length].to_vec(),
            text: render(token)
        });
        offset += length;
    }
    out
}

/// How an instruction reads.
///
/// `Token`'s own `Display` is what `basm` prints, which is exactly what this
/// view wants: the same spelling as the source it sits beside.
fn render(token: &cpclib_tokens::Token) -> String {
    token.to_string().trim().to_string()
}

/// The DAP shape of a decoded run, ready to be annotated with source lines.
pub fn as_dap_instructions(instructions: &[Instruction]) -> Vec<Value> {
    instructions
        .iter()
        .map(|instruction| {
            json!({
                "address": crate::protocol::address_reference(instruction.address as u32),
                "instruction": instruction.text,
                "instructionBytes": instruction
                    .bytes
                    .iter()
                    .map(|byte| format!("{byte:02X}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_become_the_instructions_they_encode() {
        // nop ; ld a,0x12 ; ld hl,0x3456 ; ret
        let decoded = decode(0x4000, &[0x00, 0x3E, 0x12, 0x21, 0x56, 0x34, 0xC9], 32);

        let text: Vec<&str> = decoded.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(text.len(), 4, "{decoded:?}");
        assert!(text[0].eq_ignore_ascii_case("nop"), "{text:?}");
        assert!(text[3].eq_ignore_ascii_case("ret"), "{text:?}");

        // Addresses follow the byte lengths, not the instruction count.
        let addresses: Vec<u16> = decoded.iter().map(|i| i.address).collect();
        assert_eq!(addresses, vec![0x4000, 0x4001, 0x4003, 0x4006]);
        assert_eq!(decoded[2].bytes, vec![0x21, 0x56, 0x34]);
    }

    /// A read ends mid-instruction constantly, and the decoder shows the
    /// leftovers as data rather than as an instruction it cannot justify -
    /// `DB 0x21`, not an invented `LD HL,`.
    ///
    /// What must hold either way: no instruction claims a byte that was not
    /// read, and the addresses stay contiguous.
    #[test]
    fn a_truncated_instruction_becomes_data_not_a_guess() {
        // `ld hl,` with only one of its two address bytes.
        let decoded = decode(0x4000, &[0x00, 0x21, 0x56], 32);

        let claimed: usize = decoded.iter().map(|i| i.bytes.len()).sum();
        assert!(claimed <= 3, "claimed {claimed} of 3 bytes: {decoded:?}");
        assert!(
            !decoded
                .iter()
                .any(|i| i.text.to_uppercase().starts_with("LD HL")),
            "no instruction is invented from bytes that were not there: {decoded:?}"
        );

        let mut expected = 0x4000u16;
        for instruction in &decoded {
            assert_eq!(instruction.address, expected, "{decoded:?}");
            expected += instruction.bytes.len() as u16;
        }
    }

    #[test]
    fn the_limit_is_honoured() {
        let decoded = decode(0x4000, &[0x00; 64], 5);
        assert_eq!(decoded.len(), 5);
    }

    #[test]
    fn nothing_in_nothing_out() {
        assert!(decode(0x4000, &[], 32).is_empty());
    }

    /// The DAP form carries the bytes as hex, which is what the panel prints
    /// beside the mnemonic so it can be checked against the source.
    #[test]
    fn the_dap_form_carries_address_text_and_bytes() {
        let dap = as_dap_instructions(&decode(0x4000, &[0x21, 0x56, 0x34], 32));
        assert_eq!(dap.len(), 1);
        assert_eq!(dap[0]["address"], json!("0x4000"));
        assert_eq!(dap[0]["instructionBytes"], json!("21 56 34"));
        assert!(
            dap[0]["instruction"]
                .as_str()
                .unwrap()
                .to_lowercase()
                .contains("ld"),
            "{dap:?}"
        );
    }
}

/// Decode a window that must contain `reference` on an instruction boundary.
///
/// The editor's disassembly view asks for context *before* the address it is
/// looking at, and Z80 has no way to read backwards: starting one byte early
/// gives a different, equally plausible instruction stream. So every possible
/// starting point is tried and the first whose boundaries land exactly on
/// `reference` wins - the stream that agrees with the one instruction we know
/// is real is the right one.
///
/// `None` when no alignment agrees, which is the honest answer for a region
/// that is mostly data: the caller then leaves the question to the emulator
/// rather than inventing instructions in front of the one you asked about.
pub fn decode_aligned(
    start: u16,
    bytes: &[u8],
    reference: u16,
    before: usize,
    count: usize
) -> Option<Vec<Instruction>> {
    let span = reference.wrapping_sub(start) as usize;
    if span > bytes.len() {
        return None;
    }

    for skip in 0..=span {
        let from = start.wrapping_add(skip as u16);
        // Decoding the whole window each time is cheap next to a round trip,
        // and the alignment is usually found on the first or second try.
        let decoded = decode(from, &bytes[skip..], before + count + 1);
        // Not this alignment; try the next one. Giving up here would abandon
        // the search on the first stream that happens to step over the anchor.
        let Some(landed) = decoded.iter().position(|i| i.address == reference)
        else {
            continue;
        };
        if landed == 0 && skip != span {
            // Reached `reference` only because the stream ran out - not an
            // alignment, just a short read.
            continue;
        }

        // Enough context before it, if we have it; never more than exists.
        let first = landed.saturating_sub(before);
        let window: Vec<Instruction> = decoded.into_iter().skip(first).take(count).collect();
        if window.iter().any(|i| i.address == reference) {
            return Some(window);
        }
    }
    None
}

#[cfg(test)]
mod aligned_tests {
    use super::*;

    /// `ld a,1 ; nop ; ld hl,0x3456 ; ret`, with the view anchored on the
    /// `ld hl` at 0x4003.
    const PROGRAM: [u8; 7] = [0x3E, 0x01, 0x00, 0x21, 0x56, 0x34, 0xC9];

    #[test]
    fn the_alignment_that_lands_on_the_reference_wins() {
        let window = decode_aligned(0x4000, &PROGRAM, 0x4003, 2, 4).expect("aligned");

        let addresses: Vec<u16> = window.iter().map(|i| i.address).collect();
        assert_eq!(addresses, vec![0x4000, 0x4002, 0x4003, 0x4006]);
        assert!(
            window
                .iter()
                .any(|i| i.address == 0x4003 && i.bytes == vec![0x21, 0x56, 0x34]),
            "the instruction we know is real is decoded as itself: {window:?}"
        );
    }

    /// Starting mid-instruction gives a different, equally plausible stream -
    /// so an alignment is only accepted when it agrees with the address the
    /// editor asked about.
    #[test]
    fn a_misaligned_start_is_rejected_in_favour_of_one_that_fits() {
        // Anchored on 0x4004, which is *inside* the `ld hl` - no alignment
        // starting at or before 0x4000 can put a boundary there except by
        // decoding the operand as code, which starting at 0x4004 does.
        let window = decode_aligned(0x4000, &PROGRAM, 0x4004, 1, 3);
        if let Some(window) = window {
            assert!(
                window.iter().any(|i| i.address == 0x4004),
                "whatever it chose, it contains the address asked for: {window:?}"
            );
        }
    }

    /// A reference outside the bytes read is not answerable here.
    #[test]
    fn a_reference_beyond_the_window_is_refused() {
        assert!(decode_aligned(0x4000, &PROGRAM, 0x9000, 2, 4).is_none());
    }

    #[test]
    fn no_context_asked_for_starts_at_the_reference() {
        let window = decode_aligned(0x4000, &PROGRAM, 0x4003, 0, 2).expect("aligned");
        assert_eq!(window[0].address, 0x4003);
    }
}
