//! Backend-agnostic Locomotive BASIC 1.1 runtime introspection.
//!
//! Everything here decodes bytes that a plain `readMemory` request already
//! returns from either peer - there is no BASIC-specific wire protocol to
//! implement, because the addresses below are RAM *pointers* the ROM itself
//! keeps live-updated, not folklore constants (the common "&170" for program
//! start only holds under stock memory config; the real value lives at
//! [`PTR_END_OF_RESERVED_AREA`] and must be read, not assumed).
//!
//! Reverse-engineered from the annotated BASIC 1.1 ROM disassembly at
//! <https://github.com/Bread80/Amstrad-CPC-BASIC-Source>
//! (`Includes/MemoryBASIC.asm`/`.txt`), cross-checked against the
//! independent write-up at
//! <https://bread80.com/2021/11/20/variables-def-fn-definitions-and-arrays-storage-in-amstrad-cpc-locomotive-basic/>.
//! Both sources agree on the variable-storage addresses; the current-line
//! and program-start pointers come from the ROM disassembly's `Execution.asm`
//! and `LoadSaveRun.asm`, which is the only source of the two.

use cpclib_basic::binary_parser::line_or_end;
use cpclib_basic::tokens::BasicFloat;
use cpclib_basic::BasicLine;
use cpclib_common::winnow::Parser;

/// Pointer to the current statement (advances on every `:`-separated
/// statement, finer-grained than [`PTR_CURRENT_LINE_NUMBER_FIELD`]).
/// `&0000` in direct/immediate mode.
pub const PTR_CURRENT_STATEMENT: u16 = 0xAE1B;

/// Pointer to the current line's own 2-byte line-number field inside the
/// tokenised program. Dereference *that* address to get the line number
/// itself - `&0000` here means direct/immediate mode (no program running).
pub const PTR_CURRENT_LINE_NUMBER_FIELD: u16 = 0xAE1D;

/// Pointer to the byte immediately before the tokenised program.
/// `program_start = deref(PTR_END_OF_RESERVED_AREA) + 1`.
pub const PTR_END_OF_RESERVED_AREA: u16 = 0xAE64;

/// Pointer to the start of the variable storage area, immediately following
/// the tokenised program.
pub const PTR_VARIABLES_START: u16 = 0xAE68;

/// Pointer to the start of the array storage area, immediately following
/// variable storage.
pub const PTR_ARRAYS_START: u16 = 0xAE6A;

/// 26 little-endian word chain heads, one per initial letter A-Z. Each is a
/// relative offset from the byte before the address `PTR_VARIABLES_START`
/// dereferences to (`0` = empty chain).
pub const VARIABLE_CHAIN_HEADS: u16 = 0xADB7;
pub const VARIABLE_CHAIN_HEADS_COUNT: usize = 26;

/// Chain head for `DEF FN` definitions, same format as the A-Z table.
pub const DEF_FN_CHAIN_HEAD: u16 = 0xADEB;

/// ROM entry point called once per BASIC line about to execute (`HL` points
/// at the line's length field on entry) - the breakpoint target for
/// line-granularity stepping/breakpoints.
pub const EXECUTE_LINE_ENTRY: u16 = 0xDE77;

/// ROM entry point called once per statement - finer-grained than
/// [`EXECUTE_LINE_ENTRY`], fires on every `:`-separated statement too.
pub const EXECUTE_STATEMENT_ENTRY: u16 = 0xDE60;

/// Given the 2 bytes read from [`PTR_CURRENT_LINE_NUMBER_FIELD`], the
/// address to dereference next to get the actual line number - `None` if
/// the program is not running (direct/immediate mode).
pub fn current_line_number_field_address(ptr_bytes: [u8; 2]) -> Option<u16> {
    let addr = u16::from_le_bytes(ptr_bytes);
    (addr != 0).then_some(addr)
}

/// Given the 2 bytes read from the address
/// [`current_line_number_field_address`] returned, the current line number.
pub fn decode_line_number(bytes: [u8; 2]) -> u16 {
    u16::from_le_bytes(bytes)
}

#[derive(Debug, Clone)]
pub struct AddressedLine {
    pub address: u16,
    pub line: BasicLine
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum BasicRuntimeError {
    #[error("malformed BASIC program in memory: {0}")]
    Decode(String)
}

/// Decodes the tokenised program starting at `program_start`, pairing each
/// line with the RAM address it actually starts at.
///
/// Deliberately reuses [`line_or_end`] directly rather than
/// `BasicProgram::decode` + recomputing byte lengths from the decoded
/// tokens: the two can disagree. A line's on-RAM length field is exactly
/// the value [`line_or_end`] consumed to size its read, but a program using
/// the classic "hide_line" trick (a line whose *stored* length silently
/// swallows the line(s) after it, see `BasicProgram::hide_line`) has a
/// stored length that does not match what the decoded tokens alone would
/// recompute. Reading the true consumed-byte count as the parser itself
/// consumes it keeps the returned addresses correct even then.
pub fn program_with_addresses(
    bytes: &[u8],
    program_start: u16
) -> Result<Vec<AddressedLine>, BasicRuntimeError> {
    let mut input = bytes;
    let mut address = program_start;
    let mut lines = Vec::new();

    loop {
        let before = input.len();
        match line_or_end.parse_next(&mut input) {
            Ok(Some(line)) => {
                let consumed = (before - input.len()) as u16;
                lines.push(AddressedLine { address, line });
                address = address.wrapping_add(consumed);
            },
            Ok(None) => break,
            Err(e) => return Err(BasicRuntimeError::Decode(format!("{e:?}")))
        }
    }

    Ok(lines)
}

#[derive(Debug, Clone, PartialEq)]
pub enum BasicVariableValue {
    Integer(i16),
    Real(f64),
    /// The string's characters live elsewhere in RAM; decoding them is a
    /// second `readMemory(address, len)` the caller issues, not done here.
    StringRef { len: u8, address: u16 },
    /// `DEF FN` - opaque, not decoded.
    DefFn,
    Unknown(u8)
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicVariable {
    pub name: String,
    pub address: u16,
    pub value: BasicVariableValue
}

/// Walks all 27 variable chains (A-Z + `DEF FN`) against one already-read
/// buffer covering the variable storage area, starting at `variables_base`
/// (the address [`PTR_VARIABLES_START`] dereferenced to). One bulk read
/// upfront and a purely local walk - no further round-trips per variable,
/// mirroring how the reference `amspirit-basic` extension reads its whole
/// variable zone (capped) in a single request too.
pub fn decode_variable_chains(
    chain_heads: &[u16; VARIABLE_CHAIN_HEADS_COUNT],
    def_fn_head: u16,
    variables_base: u16,
    buffer: &[u8]
) -> Vec<BasicVariable> {
    let mut out = Vec::new();
    for &head in chain_heads.iter().chain(std::iter::once(&def_fn_head)) {
        walk_chain(head, variables_base, buffer, &mut out);
    }
    out
}

fn walk_chain(mut offset: u16, variables_base: u16, buffer: &[u8], out: &mut Vec<BasicVariable>) {
    // A chain link is a relative offset from the byte before
    // `variables_base` - buffer index 0 already *is* `variables_base`, so
    // offset `R` lands at buffer index `R - 1`.
    while offset != 0 {
        let idx = (offset - 1) as usize;
        let Some(next_bytes) = buffer.get(idx..idx + 2)
        else {
            break;
        };
        let next = u16::from_le_bytes([next_bytes[0], next_bytes[1]]);

        let Some((name, after_name)) = read_name(buffer, idx + 2)
        else {
            break;
        };
        let Some(&type_byte) = buffer.get(after_name)
        else {
            break;
        };
        let value_start = after_name + 1;

        let value = match type_byte {
            0x01 => {
                let Some(b) = buffer.get(value_start..value_start + 2)
                else {
                    break;
                };
                BasicVariableValue::Integer(i16::from_le_bytes([b[0], b[1]]))
            },
            0x02 => {
                let Some(b) = buffer.get(value_start..value_start + 3)
                else {
                    break;
                };
                BasicVariableValue::StringRef {
                    len: b[0],
                    address: u16::from_le_bytes([b[1], b[2]])
                }
            },
            0x04 => {
                let Some(b) = buffer.get(value_start..value_start + 5)
                else {
                    break;
                };
                let float = BasicFloat::from_bytes([b[0], b[1], b[2], b[3], b[4]]);
                BasicVariableValue::Real(float.to_f64())
            },
            0x05 => BasicVariableValue::DefFn,
            other => BasicVariableValue::Unknown(other)
        };

        out.push(BasicVariable {
            name,
            address: variables_base + idx as u16,
            value
        });
        offset = next;
    }
}

/// Reads a variable name starting at `idx`: 7-bit ASCII chars, the last one
/// flagged by bit 7. Returns the name and the index right after it.
fn read_name(buffer: &[u8], mut idx: usize) -> Option<(String, usize)> {
    let mut name = String::new();
    loop {
        let byte = *buffer.get(idx)?;
        name.push((byte & 0x7f) as char);
        idx += 1;
        if byte & 0x80 != 0 {
            break;
        }
    }
    Some((name, idx))
}

#[cfg(test)]
mod tests {
    use cpclib_basic::BasicProgram;

    use super::*;

    #[test]
    fn direct_mode_is_a_null_pointer() {
        assert_eq!(current_line_number_field_address([0, 0]), None);
    }

    #[test]
    fn a_running_program_gives_a_dereferenceable_pointer() {
        // &AE20, little-endian
        assert_eq!(
            current_line_number_field_address([0x20, 0xAE]),
            Some(0xAE20)
        );
        assert_eq!(decode_line_number([0x64, 0x00]), 100);
    }

    #[test]
    fn program_with_addresses_matches_manually_computed_offsets() {
        let prog = BasicProgram::parse("10 PRINT \"HI\"\n20 GOTO 10\n").unwrap();
        let bytes = prog.as_bytes();

        let lines = program_with_addresses(&bytes, 0x170).unwrap();

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].address, 0x170);
        assert_eq!(lines[0].line.line_number(), 10);

        let first_len = prog.lines()[0].complete_bytes_length();
        assert_eq!(lines[1].address, 0x170 + first_len);
        assert_eq!(lines[1].line.line_number(), 20);
    }

    #[test]
    fn program_with_addresses_stops_at_the_end_marker() {
        let prog = BasicProgram::parse("10 END\n").unwrap();
        let bytes = prog.as_bytes();
        let lines = program_with_addresses(&bytes, 0x170).unwrap();
        assert_eq!(lines.len(), 1);
    }

    fn node(next: u16, name: &str, type_byte: u8, value: &[u8]) -> Vec<u8> {
        let mut bytes = next.to_le_bytes().to_vec();
        let chars: Vec<u8> = name.bytes().collect();
        for (i, &c) in chars.iter().enumerate() {
            let last = i == chars.len() - 1;
            bytes.push(if last { c | 0x80 } else { c });
        }
        bytes.push(type_byte);
        bytes.extend_from_slice(value);
        bytes
    }

    #[test]
    fn decodes_an_integer_variable_chain_of_one() {
        // variables_base = 0xAE70 (arbitrary). Node stored at buffer index 0
        // -> relative offset 1 in the chain head.
        let mut buffer = node(0, "I", 0x01, &42i16.to_le_bytes());
        buffer.resize(buffer.len().max(64), 0);

        let mut heads = [0u16; VARIABLE_CHAIN_HEADS_COUNT];
        heads[8] = 1; // 'I' is the 9th letter

        let vars = decode_variable_chains(&heads, 0, 0xAE70, &buffer);
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "I");
        assert_eq!(vars[0].value, BasicVariableValue::Integer(42));
        assert_eq!(vars[0].address, 0xAE70);
    }

    #[test]
    fn decodes_a_string_variable_as_a_pending_reference() {
        let mut value = vec![5u8]; // length
        value.extend_from_slice(&0xC000u16.to_le_bytes()); // address
        let buffer = node(0, "A$", 0x02, &value);

        let mut heads = [0u16; VARIABLE_CHAIN_HEADS_COUNT];
        heads[0] = 1;

        let vars = decode_variable_chains(&heads, 0, 0x1000, &buffer);
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "A$");
        assert_eq!(
            vars[0].value,
            BasicVariableValue::StringRef {
                len: 5,
                address: 0xC000
            }
        );
    }

    #[test]
    fn decodes_a_real_variable_using_the_shared_float_format() {
        let float = BasicFloat::from_f64(3.5).as_bytes();
        let buffer = node(0, "F", 0x04, &float);

        let mut heads = [0u16; VARIABLE_CHAIN_HEADS_COUNT];
        heads[5] = 1;

        let vars = decode_variable_chains(&heads, 0, 0x1000, &buffer);
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].value, BasicVariableValue::Real(3.5));
    }

    #[test]
    fn walks_a_chain_of_more_than_one_variable() {
        // Second node (name "B") at relative offset 1, first node (name "A")
        // linked from it at relative offset 1 + len(second node).
        let second = node(0, "B", 0x01, &2i16.to_le_bytes());
        let second_len = second.len() as u16;
        let first_offset = 1 + second_len;
        let first = node(1, "A", 0x01, &1i16.to_le_bytes());

        let mut buffer = second;
        buffer.extend_from_slice(&first);

        let mut heads = [0u16; VARIABLE_CHAIN_HEADS_COUNT];
        heads[0] = first_offset;

        let vars = decode_variable_chains(&heads, 0, 0x1000, &buffer);
        assert_eq!(vars.len(), 2);
        assert_eq!(vars[0].name, "A");
        assert_eq!(vars[1].name, "B");
    }

    #[test]
    fn an_empty_chain_head_decodes_to_nothing() {
        let heads = [0u16; VARIABLE_CHAIN_HEADS_COUNT];
        let vars = decode_variable_chains(&heads, 0, 0x1000, &[]);
        assert!(vars.is_empty());
    }

    #[test]
    fn a_truncated_buffer_stops_cleanly_instead_of_panicking() {
        let mut heads = [0u16; VARIABLE_CHAIN_HEADS_COUNT];
        heads[0] = 1;
        // Buffer far too short to hold even the "next" pointer.
        let vars = decode_variable_chains(&heads, 0, 0x1000, &[0x00]);
        assert!(vars.is_empty());
    }
}
