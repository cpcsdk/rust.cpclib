//! `opcode_duration` against every instruction the LSP's own timing table
//! knows, on both token types.
//!
//! Two things are checked, and the second is why this file exists:
//!
//! 1. The duration rules give the same answer for a `LocatedToken` as for the
//!    `Token` it converts to. They are now generic over `DataAccessElem`
//!    rather than matching concrete `DataAccess` variants, and a classifier
//!    that mapped one shape wrongly would show up here as a disagreement
//!    between the two - which no test inside `cpclib-z80flow` could catch,
//!    since it has no parser.
//! 2. Nothing regressed against `cpclib-asm`'s `estimated_duration`, which is
//!    where these rules lived before and which still answers basm's own
//!    `duration()` operator.

use cpclib_asm::implementation::tokens::TokenExt;
use cpclib_asm::parser::parse_z80_str;
use cpclib_tokens::{ListingElement, ToSimpleToken};
use cpclib_z80flow::cost::opcode_duration;

/// A broad spread of real instructions: every operand shape the rules
/// distinguish, including the ones whose timing depends on *which* register.
const SAMPLES: &[&str] = &[
    "nop", "halt", "di", "ei", "ccf", "scf", "cpl", "daa", "neg", "exx",
    "ld a,b", "ld h,d", "ld b,5", "ld a,(hl)", "ld (hl),a", "ld a,(bc)",
    "ld a,(de)", "ld (bc),a", "ld hl,0x4000", "ld sp,hl", "ld a,(0x4000)",
    "ld (0x4000),a", "ld (0x4000),hl", "ld a,i", "ld a,r", "ld i,a", "ld r,a",
    "ld a,(ix+5)", "ld (ix+5),a", "ld ixl,3", "ld ixh,ixl", "ld ix,0x4000",
    "inc a", "inc hl", "inc ix", "inc (hl)", "inc (ix+2)", "dec b", "dec de",
    "add a,b", "add a,5", "add a,(hl)", "add a,(ix+1)", "add hl,de",
    "add ix,de", "adc a,c", "adc hl,bc", "sbc a,d", "sbc hl,de",
    "and b", "or (hl)", "xor 7", "cp (ix+3)", "sub e",
    "push bc", "push ix", "pop de", "pop iy",
    "jp 0x4000", "jp nz,0x4000", "jp (hl)", "jp (ix)", "jr 0x10", "jr z,0x10",
    "djnz 0x10", "call 0x4000", "call nc,0x4000", "ret", "ret po", "reti",
    "retn", "rst 0x38",
    "ex de,hl", "ex af,af'", "ex (sp),hl", "ex (sp),ix",
    "ldi", "ldd", "ldir", "lddr", "cpi", "cpd", "cpir", "cpdr",
    "rlca", "rrca", "rla", "rra", "rld", "rrd",
    "rlc b", "rrc (hl)", "rl (ix+1)", "sla c", "sra d", "srl e",
    "bit 3,a", "bit 0,(hl)", "set 7,b", "res 2,(ix+4)",
    "in a,(0x10)", "in b,(c)", "out (0x10),a", "out (c),d",
    "im 1",
];

/// Not opcodes at all: basm accepts them and assembles each to several real
/// instructions.
const FAKE: &[&str] = &["ld hl,de", "ld bc,hl", "ld de,bc"];

/// The duration of `src`, read three ways: through the generic rules on a
/// `LocatedToken`, through them on the equivalent `Token`, and through
/// `cpclib-asm`'s `estimated_duration`.
fn three_ways(src: &str) -> (Option<u32>, Option<u32>, Option<u32>) {
    let text = format!("    {src}\n");
    let listing = parse_z80_str(&text).unwrap_or_else(|e| panic!("{src:?} must parse: {e}"));
    let located = listing.iter().next().expect("one token");
    let simple = located.as_simple_token().into_owned();

    let via_located = located.mnemonic().and_then(|m| {
        opcode_duration(m, located.mnemonic_arg1(), located.mnemonic_arg2())
    });
    let via_token = simple
        .mnemonic()
        .and_then(|m| opcode_duration(m, simple.mnemonic_arg1(), simple.mnemonic_arg2()));
    let via_asm = simple.estimated_duration().ok().map(|d| d as u32);

    (via_located, via_token, via_asm)
}

/// The point of making the rules generic: a `LocatedToken` no longer has to be
/// cloned into a `Token` to be priced, and must not get a different answer for
/// having skipped that.
#[test]
fn a_located_token_and_a_plain_token_cost_the_same() {
    let mut disagreements = Vec::new();
    for src in SAMPLES {
        let (located, token, _) = three_ways(src);
        if located != token {
            disagreements.push(format!("{src}: located={located:?} token={token:?}"));
        }
    }
    assert!(
        disagreements.is_empty(),
        "the operand classifier reads these two token types differently:\n  {}",
        disagreements.join("\n  ")
    );
}

/// And the rules still agree with the assembler they were extracted from.
#[test]
fn the_shared_rules_agree_with_the_assemblers_own_duration() {
    let mut disagreements = Vec::new();
    for src in SAMPLES {
        let (_, token, asm) = three_ways(src);
        // `estimated_duration` handles a few non-opcode shapes this does not;
        // where both have an answer, the answers must match.
        if let (Some(t), Some(a)) = (token, asm)
            && t != a
        {
            disagreements.push(format!("{src}: shared={t} asm={a}"));
        }
    }
    assert!(
        disagreements.is_empty(),
        "the shared rules and `estimated_duration` disagree:\n  {}",
        disagreements.join("\n  ")
    );
}

/// Every sample must actually be priced - a test comparing two `None`s would
/// pass while measuring nothing.
#[test]
fn every_sample_gets_a_real_duration() {
    let unpriced: Vec<&str> = SAMPLES
        .iter()
        .copied()
        .filter(|src| three_ways(src).1.is_none())
        .collect();
    assert!(
        unpriced.is_empty(),
        "these have no duration, so the comparisons above say nothing about them: {unpriced:?}"
    );
}

/// A fake instruction has no opcode duration, because it has no opcode.
///
/// This is the boundary between the two halves of pricing: `opcode_duration`
/// answers for real opcodes only, and `cost::instruction_cost` is what expands
/// a fake instruction and sums the opcodes it becomes. Asserting the `None`
/// here keeps someone from "fixing" it by inventing a number.
#[test]
fn a_fake_instruction_has_no_opcode_duration_of_its_own() {
    for src in FAKE {
        let (located, token, _) = three_ways(src);
        assert_eq!(token, None, "{src} is not an opcode");
        assert_eq!(located, None, "{src} is not an opcode");
    }
}
