//! The effects table against instructions produced by the *real* basm
//! parser, not hand-built tokens.
//!
//! `effects.rs`'s own unit tests construct `Token::OpCode` directly, which
//! proves the table logic but not that it lines up with what the parser
//! actually emits for real source text - a mismatch there (an operand shape
//! the parser produces that no table row accepts) would make instructions
//! silently opaque, and every liveness constraint over them would fail
//! closed. That failure mode is safe but invisible, so it needs its own test.

use cpclib_asm::flatten::flatten_listing;
use cpclib_asm::parser::{LocatedToken, parse_z80_str};
use cpclib_asmoptim::analysis_op::AnalysisOp;
use cpclib_asmoptim::effects::effects_of;
use cpclib_asmoptim::regflag::{Flag, Reg};

/// Deliberately spans every operand shape the table distinguishes: plain and
/// indexed registers, both index-register halves, immediates, memory,
/// conditions, ports, block operations, exchanges, and the special `I`/`R`
/// registers.
const WIDE_SAMPLE: &str = "
    ld a, 0
    ld a, b
    ld ixh, ixl
    ld a, i
    ld a, r
    ld (0x4000), hl
    ld hl, (0x4000)
    xor a
    xor b
    add a, (ix+2)
    sbc hl, de
    dec a
    inc hl
    push bc
    pop de
    bit 3, a
    res 0, (hl)
    set 7, (iy+1)
    ldir
    cpir
    djnz $
    ret nz
    ret
    jp nz, $
    jr c, $
    call z, $
    out (c), a
    in a, (c)
    ex de, hl
    ex af, af'
    exx
    neg
    halt
    rst 0x18
    im 1
    rlca
    cpl
    scf
    nop
";

fn ops(source: &str) -> (cpclib_asm::parser::LocatedListing, ()) {
    (parse_z80_str(source).expect("sample must parse"), ())
}

#[test]
fn every_instruction_in_a_wide_real_sample_is_described() {
    let (listing, ()) = ops(WIDE_SAMPLE);
    let tokens: Vec<&LocatedToken> = flatten_listing(listing.iter()).collect();

    let mut checked = 0;
    let mut opaque = Vec::new();
    for token in tokens {
        let op = AnalysisOp::Real(token);
        if op.mnemonic().is_none() {
            continue;
        }
        if effects_of(&op).is_none() {
            opaque.push(token.to_string().trim().to_string());
        }
        checked += 1;
    }

    assert!(checked > 30, "expected a wide sample, only checked {checked}");
    assert!(
        opaque.is_empty(),
        "these real instructions have no table row: {opaque:#?}"
    );
}

/// The CPC-specific correction that motivated using this table rather than
/// the ZX-Next one: on a CPC, ports are 16-bit, so `out (c),a` really depends
/// on `B` as well - i.e. it is `out (bc),a`. Losing this would let a rule
/// clobber `B` before an `out`.
#[test]
fn out_c_a_depends_on_b_because_cpc_ports_are_sixteen_bit() {
    let (listing, ()) = ops("    out (c), a\n");
    let tokens: Vec<&LocatedToken> = flatten_listing(listing.iter()).collect();
    let op = AnalysisOp::Real(tokens[0]);
    let e = effects_of(&op).expect("out (c),a must be described");

    assert!(e.reads.contains(&Reg::A), "{e:?}");
    assert!(
        e.reads.contains(&Reg::Bc),
        "the whole point of the CPC table: {e:?}"
    );
    assert!(e.writes_port, "{e:?}");
}

/// Spot-check a handful of real instructions end to end, so a table or
/// matcher regression shows up as a concrete wrong answer rather than only as
/// a count.
#[test]
fn real_instructions_report_the_semantics_they_actually_have() {
    let (listing, ()) = ops(
        "    xor a\n    xor b\n    dec a\n    push bc\n    pop de\n    ld a, r\n    sbc hl, de\n"
    );
    let tokens: Vec<&LocatedToken> = flatten_listing(listing.iter()).collect();
    let e: Vec<_> = tokens
        .iter()
        .map(|t| effects_of(&AnalysisOp::Real(*t)).expect("described"))
        .collect();

    // xor a - the self-clearing idiom reads nothing at all.
    assert!(e[0].reads.is_empty(), "xor a: {:?}", e[0]);
    // xor b - ...but the general form reads both.
    assert_eq!(e[1].reads, vec![Reg::A, Reg::B]);
    // dec a - read-modify-write.
    assert_eq!(e[2].reads, vec![Reg::A]);
    assert_eq!(e[2].writes, vec![Reg::A]);
    // push reads the pair; pop does not.
    assert!(e[3].reads.contains(&Reg::Bc));
    assert!(!e[4].reads.contains(&Reg::De));
    assert!(e[4].writes.contains(&Reg::De));
    // `R` the refresh register, not `r` the placeholder.
    assert_eq!(e[5].reads, vec![Reg::R]);
    // sbc consumes the carry it also rewrites.
    assert!(e[6].reads_flags.contains(&Flag::C), "{:?}", e[6]);
}
