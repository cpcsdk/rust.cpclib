//! Classifier for `label+*:` ("smart" SMC offset) labels - see
//! `cpclib_tokens::SmcOffset` and `Env::pending_smc_label`.
//!
//! Determines the tail-immediate width (or, for the DDCB/FDCB exception,
//! the fixed displacement offset) of an already-assembled instruction, from
//! its operand *shapes* alone. The instruction's real encoded length comes
//! from the caller (`Env::visit_opcode`, via `assemble_opcode_impl`'s own
//! `bytes.len()`), so this can never drift out of sync with the real
//! encoder - it only ever classifies *width*, never re-derives length.
//!
//! Deliberately narrow: covers the common immediate/address-operand shapes
//! (`LD` in its various forms, `JP`/`CALL`/`JR`/`DJNZ`, `IN`/`OUT (n)`,
//! `AND`/`OR`/`XOR`/`CP n`, and DDCB/FDCB indexed bit-ops), and returns a
//! clear error for anything else rather than guessing - matching this
//! project's established philosophy of declining rather than silently
//! producing a wrong answer.
//!
//! `ADD`/`ADC`/`SUB`/`SBC` and the `RST`/`JQ` fake-instruction families are
//! deliberately left out entirely (the *whole* mnemonic, including its
//! perfectly ordinary single-instruction 8-bit-immediate forms like
//! `ADD A,n` - not just the ambiguous cases), and not just because nobody
//! got around to them: `ADD`/`ADC`/`SUB`/`SBC` share their mnemonic with a
//! *fake* 16-bit form (`ADD DE,BC`, `SBC HL,rr`, ...) that expands into a
//! whole `Listing` of several real instructions
//! (`Env::assemble_fake_listing`, dispatched from e.g. `Env::assemble_sub`
//! whenever `arg1` is `DE`/`HL`), and `RST`/`JQ` are themselves nothing but
//! such fake, multi-instruction expansions
//! (`assemble_rst_fake`/`assemble_jq`). This classifier's whole model -
//! "the immediate sits in the tail of *one* already-assembled instruction,
//! offset = that instruction's `bytes_len` minus the immediate's width" -
//! has no meaning once `bytes_len` might span several real instructions
//! instead of one; there's no single well-defined "the instruction" left to
//! take a tail-offset of. Excluding the whole family rather than just the
//! `DE`/`HL`-first shape is the simple, safe choice for now - teasing the
//! two apart (mirroring `AND`/`OR`/`XOR`/`CP n`'s own arg1-is-`A`-or-real-
//! operand dispatch, gated on arg1 *not* being `DE`/`HL`/an index register)
//! would recover `ADD A,n`/`ADC A,n`/`SUB n`/`SBC A,n` support cheaply, but
//! hasn't been done - a real, scoped follow-up, not something this
//! comment's "why" should be read as ruling out.

use cpclib_tokens::{DataAccessElem, Mnemonic, Register8};

/// See the module doc comment (including for why `ADD`/`ADC`/`SUB`/`SBC`/
/// `RST`/`JQ` aren't supported). `bytes_len` is the real, already-encoded
/// instruction length (`assemble_opcode_impl`'s own `Bytes::len()`).
pub(crate) fn smart_smc_offset<D: DataAccessElem>(
    mnemonic: Mnemonic,
    arg1: Option<&D>,
    arg2: Option<&D>,
    _arg3: Option<&Register8>,
    bytes_len: usize
) -> Result<usize, String> {
    // DDCB/FDCB indexed bit-ops (`BIT n,(IX+d)`, `RES n,(IX+d)`, `RLC
    // (IX+d)`, ...): layout is `prefix, 0xCB, delta, opcode` - the
    // patchable displacement sits at a *fixed* offset 2, followed by a
    // trailing, non-patchable opcode byte. Not "the tail", so this must be
    // checked before the general tail-width rule below.
    if matches!(
        mnemonic,
        Mnemonic::Bit
            | Mnemonic::Res
            | Mnemonic::Set
            | Mnemonic::Rlc
            | Mnemonic::Rrc
            | Mnemonic::Rl
            | Mnemonic::Rr
            | Mnemonic::Sla
            | Mnemonic::Sra
            | Mnemonic::Srl
            | Mnemonic::Sl1
    ) && (arg1.is_some_and(|a| a.is_indexregister_with_index())
        || arg2.is_some_and(|a| a.is_indexregister_with_index()))
    {
        return Ok(2);
    }

    // General case: the immediate/address operand, when there is one, is
    // always the last byte(s) pushed by every `assemble_*` encoder that
    // isn't the DDCB/FDCB exception above - so `offset = bytes_len -
    // immediate_width`, and only `immediate_width` needs classifying here.
    let immediate_width = match mnemonic {
        Mnemonic::Ld => ld_immediate_width(arg1, arg2)?,

        // `JR`/`JP`/`CALL` all put the (optional) condition flag in arg1
        // and the target in arg2 - see `Env::assemble_call_jr_or_jp`.
        // `JP (HL)`/`JP (IX)` have no address operand at all, so this must
        // be gated on the target actually being an expression, not assumed
        // from the mnemonic alone.
        Mnemonic::Jp | Mnemonic::Call if arg2.is_some_and(|a| a.is_expression()) => 2,
        Mnemonic::Jr if arg2.is_some_and(|a| a.is_expression()) => 1,

        // `DJNZ`'s single relative-target expression is passed as arg1 -
        // see `Env::assemble_djnz`.
        Mnemonic::Djnz if arg1.is_some_and(|a| a.is_expression()) => 1,

        // `IN r,(n)` / `OUT (n),r` - `Env::assemble_in`/`assemble_out`
        // distinguish the `(n)` port-immediate form from `(C)` via
        // `is_port_n()`.
        Mnemonic::In if arg2.is_some_and(|a| a.is_port_n()) => 1,
        Mnemonic::Out if arg1.is_some_and(|a| a.is_port_n()) => 1,

        // `AND`/`OR`/`XOR`/`CP n` - the parser always populates `arg2` with
        // the operand, falling back to `arg1` only for the optional
        // explicit `A,` prefix form - see `Env::assemble_opcode_impl`'s own
        // dispatch for these mnemonics.
        Mnemonic::And | Mnemonic::Or | Mnemonic::Xor | Mnemonic::Cp
            if arg2.or(arg1).is_some_and(|a| a.is_expression()) =>
        {
            1
        },

        _ => {
            return Err(format!("smart SMC offset not supported for `{mnemonic}`"));
        }
    };

    bytes_len.checked_sub(immediate_width).ok_or_else(|| {
        format!(
            "instruction too short for `{mnemonic}` (expected at least {immediate_width} bytes, \
             got {bytes_len})"
        )
    })
}

/// Mirrors `Env::assemble_ld`'s own shape dispatch (`cpclib-asm/src/
/// assembler/mod.rs`) for exactly the shapes that genuinely end with a
/// patchable immediate/address byte - not re-derived from scratch.
fn ld_immediate_width<D: DataAccessElem>(
    arg1: Option<&D>,
    arg2: Option<&D>
) -> Result<usize, String> {
    let (Some(arg1), Some(arg2)) = (arg1, arg2)
    else {
        return Err("smart SMC offset not supported for this LD shape".to_string());
    };

    // `LD (nn),x` - the destination address is always the trailing 2 bytes,
    // regardless of what `x` is.
    if arg1.is_memory() {
        return Ok(2);
    }

    if arg1.is_register8() {
        if arg2.is_expression() {
            return Ok(1); // LD r,n
        }
        if arg2.is_memory() {
            return Ok(2); // LD A,(nn)
        }
        if arg2.is_indexregister_with_index() {
            return Ok(1); // LD r,(IX+d) - delta is the tail
        }
        return Err("no patchable immediate operand for this LD r,<shape>".to_string());
    }

    if arg1.is_register16() {
        if arg2.is_expression() || arg2.is_memory() {
            return Ok(2); // LD rr,nn / LD rr,(nn)
        }
        return Err("no patchable immediate operand for this LD rr,<shape>".to_string());
    }

    if arg1.is_indexregister8() {
        if arg2.is_expression() {
            return Ok(1); // LD IXh/IXl,n
        }
        return Err("no patchable immediate operand for this LD IXh/IXl,<shape>".to_string());
    }

    if arg1.is_indexregister16() {
        if arg2.is_expression() || arg2.is_memory() {
            return Ok(2); // LD IX,nn / LD IX,(nn)
        }
        return Err("no patchable immediate operand for this LD IX,<shape>".to_string());
    }

    if arg1.is_address_in_register16() {
        if arg2.is_expression() {
            return Ok(1); // LD (HL),n
        }
        return Err("no patchable immediate operand for this LD (rr),<shape>".to_string());
    }

    if arg1.is_address_in_indexregister16() {
        if arg2.is_expression() {
            return Ok(1); // LD (IX),n
        }
        return Err("no patchable immediate operand for this LD (IX),<shape>".to_string());
    }

    if arg1.is_indexregister_with_index() {
        // Either `LD (IX+d),n` (the value is the tail) or `LD (IX+d),r`
        // (the delta is the tail, no value byte at all) - both width 1.
        return Ok(1);
    }

    Err("smart SMC offset not supported for this LD operand shape".to_string())
}
