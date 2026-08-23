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

/// Pointer to the end of the tokenised program - the same address
/// [`PTR_VARIABLES_START`] holds, but tracked as its own field rather than
/// re-derived from it. On a fresh boot this coincides with variables/arrays
/// start (an empty program), which made it easy to miss: a launch snapshot
/// that patches the program in and updates [`PTR_VARIABLES_START`]/
/// [`PTR_ARRAYS_START`] but leaves this one at its stale "empty program"
/// value causes real corruption the moment BASIC creates its first
/// variable, since some internal bookkeeping trusts this field over
/// re-scanning the program. Confirmed independently by AMSpiriT Lite's own
/// BASIC injector, whose documentation calls out updating "BASIC
/// end-of-program pointers at 0xAE66" as a required step, not this crate's
/// own reverse-engineering.
pub const PTR_PROGRAM_END: u16 = 0xAE66;

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

/// Where a tokenised BASIC program lives once loaded, on a freshly booted
/// machine with default memory configuration. Confirmed empirically, not
/// folklore: `cpclib_sna::Snapshot::new_6128()`'s own
/// [`PTR_END_OF_RESERVED_AREA`] reads &016F, i.e. this value + 1, on a
/// fresh boot with no program loaded - this is what `build_launch_snapshot`
/// builds on directly rather than re-deriving it from that pointer on every
/// launch.
pub const PROGRAM_START: u16 = 0x170;

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

/// Builds a boot snapshot with `program_bytes` (a tokenised program's own
/// `as_bytes()`) already sitting at [`PROGRAM_START`], as if `LOAD`ed - the
/// machine is otherwise left exactly as a fresh boot leaves it, idle at the
/// Ready prompt, not mid-`RUN`.
///
/// Deliberately not further than that: getting a Z80 debugger to boot
/// straight into "the program is already running" only needs setting `PC`
/// to the program's own entry point, because that is a value *we* chose
/// when assembling it. BASIC's "RUN" has no such fixed entry point to jump
/// to - the ROM's own `RUN_from_HL` routine expects a stack already primed
/// by its caller (`ex (sp),hl` right at its start) and itself calls three
/// more setup routines (`reset_variable_data`, `reset_exec_data`,
/// `GRA_DEFAULT`) before ever reaching a line of the program, so jumping
/// `PC` there cold corrupts the stack rather than replicating what a real
/// `RUN` does. Actually running the program is [`crate::basic_session::BasicSession`]'s
/// job instead, once attached: it types `RUN` through the emulator's own
/// keyboard - `_poc_key` on 1984js, `POST /api/keytype` on AMSpiriT Lite -
/// the same real input path a person at the keyboard uses, on a peer that
/// offers one.
///
/// The firmware pointers a freshly `LOAD`ed program actually needs updated
/// are touched: [`PTR_PROGRAM_END`], [`PTR_VARIABLES_START`] and
/// [`PTR_ARRAYS_START`], all three to right after `program_bytes` (matching
/// a real `LOAD`'s effect: no variables exist yet, so all three point to the
/// same place). Missing [`PTR_PROGRAM_END`] here previously left it at its
/// stale "empty program" value from the base snapshot - harmless for a
/// program that never creates a variable, but real corruption of the
/// program's own bytes the moment one did, since some internal bookkeeping
/// reads this field directly rather than re-scanning the program to find
/// its end. [`PTR_END_OF_RESERVED_AREA`] does *not* change - it marks the
/// fixed workspace boundary the program area starts after, not the
/// program's own end, so it already reads correctly on the base snapshot
/// regardless of what gets loaded on top.
pub fn build_launch_snapshot(program_bytes: &[u8]) -> Result<cpclib_sna::Snapshot, String> {
    let mut sna = cpclib_sna::Snapshot::new_6128()?;
    sna.unwrap_memory_chunks();
    sna.add_data(program_bytes, PROGRAM_START as usize)
        .map_err(|e| format!("{e:?}"))?;

    let variables_start = PROGRAM_START.wrapping_add(program_bytes.len() as u16);
    let pointer_bytes = variables_start.to_le_bytes();
    sna.add_data(&pointer_bytes, PTR_PROGRAM_END as usize)
        .map_err(|e| format!("{e:?}"))?;
    sna.add_data(&pointer_bytes, PTR_VARIABLES_START as usize)
        .map_err(|e| format!("{e:?}"))?;
    sna.add_data(&pointer_bytes, PTR_ARRAYS_START as usize)
        .map_err(|e| format!("{e:?}"))?;

    Ok(sna)
}

#[cfg(test)]
mod tests {
    use cpclib_basic::BasicProgram;

    use super::*;

    fn peek16(sna: &cpclib_sna::Snapshot, address: u16) -> u16 {
        u16::from_le_bytes([sna.get_byte(address as u32), sna.get_byte(address as u32 + 1)])
    }

    #[test]
    fn a_fresh_boot_snapshot_has_no_program_loaded() {
        // The base every launch snapshot builds on: confirms PROGRAM_START
        // and the "no program" state build_launch_snapshot expects to
        // override are what this crate assumes they are, so a future
        // cpclib-sna update changing the embedded base snapshot fails this
        // test rather than silently breaking BASIC launches.
        let mut sna = cpclib_sna::Snapshot::new_6128().unwrap();
        sna.unwrap_memory_chunks();

        assert_eq!(peek16(&sna, PTR_END_OF_RESERVED_AREA), PROGRAM_START - 1);
        // An empty program: PTR_PROGRAM_END/PTR_VARIABLES_START/
        // PTR_ARRAYS_START all coincide right after PROGRAM_START, which is
        // exactly what made a missing PTR_PROGRAM_END update so easy to
        // miss - it silently reads as "correct" until a real, non-empty
        // program is loaded and this field is not moved along with it.
        assert_eq!(peek16(&sna, PTR_PROGRAM_END), PROGRAM_START + 2);
        assert_eq!(peek16(&sna, PTR_CURRENT_LINE_NUMBER_FIELD), 0);
        assert_eq!(peek16(&sna, VARIABLE_CHAIN_HEADS), 0);
    }

    #[test]
    fn build_launch_snapshot_moves_program_end_past_a_non_empty_program() {
        // Regression test: a real bug, not a hypothetical. build_launch_snapshot
        // used to update only PTR_VARIABLES_START/PTR_ARRAYS_START, leaving
        // PTR_PROGRAM_END at the base snapshot's "empty program" value
        // (PROGRAM_START + 2). A program that never creates a variable never
        // notices; the moment one does, BASIC's variable-creation code wrote
        // through this stale pointer - landing inside the freshly loaded
        // program's own bytes rather than past its end - and destroyed it.
        // Confirmed against AMSpiriT Lite's own documented BASIC injector,
        // which calls out updating "end-of-program pointers at 0xAE66" as a
        // required step.
        let prog = BasicProgram::parse("10 x=1\n20 x=x+1\n30 goto 20\n").unwrap();
        let bytes = prog.as_bytes();
        let sna = build_launch_snapshot(&bytes).unwrap();

        let expected_variables_start = PROGRAM_START + bytes.len() as u16;
        assert_eq!(peek16(&sna, PTR_PROGRAM_END), expected_variables_start);
        assert_ne!(
            peek16(&sna, PTR_PROGRAM_END),
            PROGRAM_START + 2,
            "left at the base snapshot's stale empty-program value"
        );
    }

    #[test]
    fn build_launch_snapshot_places_the_program_and_updates_variable_pointers() {
        let prog = BasicProgram::parse("10 PRINT \"HI\"\n20 GOTO 10\n").unwrap();
        let bytes = prog.as_bytes();

        let sna = build_launch_snapshot(&bytes).unwrap();

        for (i, &b) in bytes.iter().enumerate() {
            assert_eq!(
                sna.get_byte(PROGRAM_START as u32 + i as u32),
                b,
                "byte {i} of the program"
            );
        }

        let expected_variables_start = PROGRAM_START + bytes.len() as u16;
        assert_eq!(peek16(&sna, PTR_PROGRAM_END), expected_variables_start);
        assert_eq!(peek16(&sna, PTR_VARIABLES_START), expected_variables_start);
        assert_eq!(peek16(&sna, PTR_ARRAYS_START), expected_variables_start);

        // Untouched: the workspace boundary, not the program's own end.
        assert_eq!(peek16(&sna, PTR_END_OF_RESERVED_AREA), PROGRAM_START - 1);
        // Still idle - this snapshot loads the program but does not run it.
        assert_eq!(peek16(&sna, PTR_CURRENT_LINE_NUMBER_FIELD), 0);
    }

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
