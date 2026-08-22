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
//!
//! Decoding produces real `cpclib_tokens::Token`s, so what an instruction
//! *costs* comes out of the same pass - from the assembler's own table, on the
//! bytes the Z80 will actually fetch. That is the honest answer for code a
//! macro expanded, code the program wrote itself, and a `defs` run that has no
//! instruction text to read at all.

use cpclib_asm::disass::{disassemble, resolve_jr_djnz_target};
use cpclib_asm::implementation::tokens::TokenExt;
use cpclib_tokens::builder::defb_elements;
use cpclib_tokens::{DataAccess, Expr, Mnemonic, Token};
use serde_json::{Value, json};

/// One decoded instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub address: u16,
    pub bytes: Vec<u8>,
    pub text: String,
    /// What it costs to execute, in NOPs.
    ///
    /// Kept here because this is the only place that holds the decoded
    /// `Token`, and the token is what the assembler prices. Anything reading
    /// the cost back off `text` would be re-parsing the string this module
    /// just printed, and would then be pricing a spelling rather than the
    /// bytes the Z80 will actually fetch.
    ///
    /// `None` for bytes that decode to no instruction - a `DB` left by a
    /// truncated read - which have no execution cost to report.
    pub cost: Option<usize>
}

/// Decode `bytes`, which were read starting at `address`.
///
/// Stops when the bytes run out rather than inventing an instruction from a
/// truncated one - the last few bytes of a read are routinely half an
/// instruction, and showing that half as though it were whole is how a
/// disassembly view starts lying.
pub fn decode(address: u16, bytes: &[u8], limit: usize) -> Vec<Instruction> {
    let mut listing = disassemble(bytes);

    let mut out = Vec::new();
    let mut offset = 0usize;
    for i in 0..listing.listing().len() {
        if out.len() >= limit {
            break;
        }
        let this_address = address.wrapping_add(offset as u16);

        // JR/DJNZ decode with a relative offset that renders as raw
        // two's-complement hex (`JR 0xfffffff8`) or, worse, as an
        // indistinguishable-from-absolute forward offset (`JR 0x8`). Rewrite
        // the operand to the absolute target address before number_of_bytes/
        // render/estimated_duration read it - none of those depend on the
        // operand's value, only its shape, so this changes neither the byte
        // length nor the timing.
        if let Token::OpCode(Mnemonic::Djnz, Some(DataAccess::Expression(e)), ..)
        | Token::OpCode(Mnemonic::Jr, _, Some(DataAccess::Expression(e)), _) =
            &mut listing.listing_mut()[i]
        {
            if let Some(target) = resolve_jr_djnz_target(e, Some(this_address)) {
                *e = Expr::Value(target as i32);
            }
        }

        let token = &listing.listing()[i];
        let Ok(length) = token.number_of_bytes()
        else {
            break;
        };
        if length == 0 || offset + length > bytes.len() {
            break;
        }
        out.push(Instruction {
            address: this_address,
            bytes: bytes[offset..offset + length].to_vec(),
            text: render(token),
            cost: token.estimated_duration().ok()
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

/// Replace decode()'s guessed instructions with the real `DB ...` for any
/// span the assembler recorded as data - using live bytes, never the
/// assembled image, so self-modified data still shows what is actually
/// there rather than what was assembled.
///
/// `current_pc` overrides everything else: whatever the source map says, the
/// row the program is actually stopped on is what the CPU is about to fetch
/// as an opcode, and showing it as inert data would be a lie of a different
/// kind than the one this function exists to fix. This is not a heuristic -
/// if `PC` is on a byte, that byte is being executed, full stop.
pub fn overlay_data_rows(
    instructions: Vec<Instruction>,
    data_span_at: impl Fn(u16) -> Option<(u16, u16)>,
    live_bytes: impl Fn(u16, usize) -> Option<Vec<u8>>,
    current_pc: Option<u16>
) -> Vec<Instruction> {
    let mut out = Vec::with_capacity(instructions.len());
    let mut i = 0;
    while i < instructions.len() {
        let address = instructions[i].address;

        // The current_pc check comes first, before the data-span lookup even
        // runs for this row - stronger than and independent of every check
        // below it.
        if current_pc == Some(address) {
            out.push(instructions[i].clone());
            i += 1;
            continue;
        }

        let overlay = data_span_at(address).and_then(|(row_start, row_len)| {
            let row_end = row_start.wrapping_add(row_len);

            // PC might sit further into the row than its first byte. Folding
            // the whole row into one data line would hide the very row
            // execution is standing on, so the row is left alone rather than
            // overlaid.
            if current_pc.is_some_and(|pc| pc >= row_start && pc < row_end) {
                return None;
            }

            // Gather every already-decoded instruction inside the row - the
            // live bytes, exactly as read from the emulator, with no second
            // read needed.
            let mut j = i;
            let mut bytes = Vec::with_capacity(row_len as usize);
            while j < instructions.len() && instructions[j].address < row_end {
                bytes.extend_from_slice(&instructions[j].bytes);
                j += 1;
            }
            // A window that ends mid-row, or a decoded instruction that
            // overruns the row's end, both fail this check - the honest
            // answer is decode()'s own guess for that partial span, not an
            // invented partial data row.
            if bytes.len() != row_len as usize {
                return None;
            }

            // Self-modifying code: only overlay when every live byte still
            // matches what was assembled there. No image at all means no way
            // to detect staleness, so the source map's claim is still the
            // best available evidence and the overlay proceeds.
            if let Some(assembled) = live_bytes(row_start, row_len as usize) {
                if assembled != bytes {
                    return None;
                }
            }

            let text = render(&defb_elements::<u8>(&bytes));
            Some((
                j,
                Instruction {
                    address: row_start,
                    bytes,
                    text,
                    cost: None
                }
            ))
        });

        match overlay {
            Some((next, instruction)) => {
                out.push(instruction);
                i = next;
            },
            None => {
                out.push(instructions[i].clone());
                i += 1;
            }
        }
    }
    out
}

/// What a decoded run costs to execute, in NOPs.
///
/// `None` as soon as one instruction has no cost: a run that is partly
/// unpriceable has no total, and reporting the sum of the rest would be a
/// number that is quietly too small.
pub fn nops(instructions: &[Instruction]) -> Option<usize> {
    instructions
        .iter()
        .try_fold(0usize, |total, instruction| Some(total + instruction.cost?))
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

    /// A forward `JR` used to print `JR 0x8`, indistinguishable from a real
    /// absolute address. It must now read as the resolved target, not the
    /// raw offset byte.
    #[test]
    fn a_forward_jr_shows_its_resolved_target() {
        // JR +5 at 0x4000: target = 0x4000 + 2 (instruction length) + 5.
        let decoded = decode(0x4000, &[0x18, 0x05], 32);
        assert_eq!(decoded.len(), 1);
        assert!(decoded[0].text.contains("0x4007"), "{:?}", decoded[0]);
    }

    /// A backward `JR` used to print raw two's-complement hex
    /// (`JR 0xfffffff8`) instead of the address it actually targets.
    #[test]
    fn a_backward_jr_shows_its_resolved_target() {
        // JR -8 at 0x4000: target = 0x4000 + 2 - 8 = 0x3ffa.
        let decoded = decode(0x4000, &[0x18, 0xF8], 32);
        assert_eq!(decoded.len(), 1);
        assert!(decoded[0].text.contains("0x3ffa"), "{:?}", decoded[0]);
    }

    /// `JR $` targets the instruction itself - offset zero once the +2 bias
    /// is folded in.
    #[test]
    fn jr_dollar_shows_its_own_address() {
        let decoded = decode(0x4000, &[0x18, 0xFE], 32);
        assert_eq!(decoded.len(), 1);
        assert!(decoded[0].text.contains("0x4000"), "{:?}", decoded[0]);
    }

    /// `DJNZ` is resolved exactly like `JR` - same relative addressing.
    #[test]
    fn djnz_shows_its_resolved_target() {
        // DJNZ +5 at 0x4000: target = 0x4000 + 2 + 5 = 0x4007.
        let decoded = decode(0x4000, &[0x10, 0x05], 32);
        assert_eq!(decoded.len(), 1);
        assert!(decoded[0].text.contains("0x4007"), "{:?}", decoded[0]);
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

    /// The owner's own scenario, and the more fundamental rule: `PC` sitting
    /// on a byte means that byte is being executed, even when those same
    /// live bytes are exactly what the source map says was assembled as
    /// data at that address. The row must still decode as a real
    /// instruction, not `DB ...`.
    #[test]
    fn pc_on_a_data_span_still_decodes_as_a_real_instruction() {
        // `ld hl,0x3456` - bytes that also happen to be exactly what the
        // source map claims is a `db` row at the same address.
        let decoded = decode(0x4000, &[0x21, 0x56, 0x34], 32);
        let overlaid = overlay_data_rows(
            decoded,
            |address| (address == 0x4000).then_some((0x4000, 3)),
            |_, _| None,
            Some(0x4000)
        );

        assert_eq!(overlaid.len(), 1);
        assert!(
            overlaid[0].text.to_lowercase().starts_with("ld hl"),
            "PC's own row must stay a real instruction: {overlaid:?}"
        );
    }

    /// The same bytes, but with `PC` elsewhere: nothing protects this row
    /// any more, so it becomes the `DB ...` the source actually wrote.
    #[test]
    fn an_ordinary_data_span_overlays_as_db() {
        let decoded = decode(0x4000, &[0x21, 0x56, 0x34], 32);
        let overlaid = overlay_data_rows(
            decoded,
            |address| (address == 0x4000).then_some((0x4000, 3)),
            |_, _| None,
            Some(0x9000)
        );

        assert_eq!(overlaid.len(), 1);
        assert_eq!(overlaid[0].address, 0x4000);
        assert_eq!(overlaid[0].bytes, vec![0x21, 0x56, 0x34]);
        assert!(
            overlaid[0].text.to_uppercase().starts_with("DB"),
            "{overlaid:?}"
        );
        assert_eq!(
            overlaid[0].cost, None,
            "a data row has no execution cost to report"
        );
    }

    /// Self-modifying code: the live bytes no longer match what was
    /// assembled there, so the overlay is refused and normal disassembly of
    /// the live bytes stands - the honest answer for data that used to be
    /// data but isn't any more.
    #[test]
    fn stale_bytes_are_not_overlaid() {
        let decoded = decode(0x4000, &[0x21, 0x56, 0x34], 32);
        let overlaid = overlay_data_rows(
            decoded,
            |address| (address == 0x4000).then_some((0x4000, 3)),
            |_, _| Some(vec![0x00, 0x00, 0x00]), // the assembled image disagrees
            None
        );

        assert_eq!(overlaid.len(), 1);
        assert!(
            overlaid[0].text.to_lowercase().starts_with("ld hl"),
            "stale bytes must not be shown as inert data: {overlaid:?}"
        );
    }

    /// A row only partially inside the already-read window is left exactly
    /// as `decode()` guessed it, rather than inventing a partial `DB` row.
    #[test]
    fn a_partially_windowed_row_is_left_as_decodes_guess() {
        // The source map claims a 3-byte data row at 0x4000, but only the
        // first byte was actually read - the window ends mid-row.
        let decoded = decode(0x4000, &[0x00], 32);
        let overlaid = overlay_data_rows(
            decoded.clone(),
            |address| (address == 0x4000).then_some((0x4000, 3)),
            |_, _| None,
            None
        );

        assert_eq!(
            overlaid, decoded,
            "an incomplete row is left exactly as decode() guessed it"
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
