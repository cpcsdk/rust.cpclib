//! Static register/flag value tracking for a sequence of already-parsed Z80
//! instructions.
//!
//! Unlike [`crate::z80::Z80`]/`emul.rs` (a real, PC-following interpreter
//! over concrete values, used to actually simulate a listing's execution),
//! this never follows jumps and every value is `Option`: absence means "not
//! statically known", never a guess. It's driven externally, one token at a
//! time, by a caller that already knows which tokens to apply and in which
//! order (e.g. cpclib-lsp's register-hover feature, walking forward from a
//! control-flow boundary to a hover position). It operates directly on
//! `cpclib_asm`'s parsed `LocatedToken`/`LocatedDataAccess` — no
//! assembled-bytes/memory-bus step needed.

use std::collections::HashMap;

use cpclib_asm::assembler::Env;
use cpclib_asm::implementation::expression::ExprEvaluationExt;
use cpclib_asm::parser::obtained::{LocatedDataAccess, LocatedToken};
use cpclib_tokens::{
    IndexRegister8, IndexRegister16, ListingElement, Mnemonic, Register8, Register16
};

/// Tracked register/flag state. Every field's absence means "not statically
/// known" - never a guessed value. 8-bit halves are stored independently
/// (not as a composed 16-bit value) so e.g. `LD IXH,n` can update just one
/// half while leaving the other's knowledge (or lack of it) untouched,
/// mirroring how `main`/`shadow` already work for B/C/D/E/H/L.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TrackedState {
    main: HashMap<Register8, u8>,
    shadow: HashMap<Register8, u8>,
    index: HashMap<IndexRegister8, u8>,
    sp: Option<u16>,
    flag_z: Option<bool>,
    flag_c: Option<bool>,
    shadow_flag_z: Option<bool>,
    shadow_flag_c: Option<bool>
}

impl TrackedState {
    pub fn get8(&self, r: Register8) -> Option<u8> {
        self.main.get(&r).copied()
    }

    fn set8(&mut self, r: Register8, v: u8) {
        self.main.insert(r, v);
    }

    fn clear8(&mut self, r: Register8) {
        self.main.remove(&r);
    }

    /// `Sp` reads `self.sp` directly; `Bc`/`De`/`Hl` compose both halves
    /// (`None` unless both are known); `Af` always answers `None` (flags
    /// aren't modeled as part of a composed 16-bit value).
    pub fn get16(&self, r: Register16) -> Option<i32> {
        if r == Register16::Sp {
            return self.sp();
        }
        let (hi, lo) = halves(r)?;
        Some(((self.get8(hi)? as i32) << 8) | self.get8(lo)? as i32)
    }

    fn set16(&mut self, r: Register16, value: i32) {
        if r == Register16::Sp {
            self.sp = Some(value as u16);
            return;
        }
        if let Some((hi, lo)) = halves(r) {
            self.set8(hi, ((value >> 8) & 0xFF) as u8);
            self.set8(lo, (value & 0xFF) as u8);
        }
    }

    fn clear16(&mut self, r: Register16) {
        if r == Register16::Sp {
            self.sp = None;
            return;
        }
        if let Some((hi, lo)) = halves(r) {
            self.clear8(hi);
            self.clear8(lo);
        }
    }

    pub fn get_index8(&self, r: IndexRegister8) -> Option<u8> {
        self.index.get(&r).copied()
    }

    fn set_index8(&mut self, r: IndexRegister8, v: u8) {
        self.index.insert(r, v);
    }

    fn clear_index8(&mut self, r: IndexRegister8) {
        self.index.remove(&r);
    }

    pub fn get_index16(&self, r: IndexRegister16) -> Option<i32> {
        let hi = self.get_index8(r.high())?;
        let lo = self.get_index8(r.low())?;
        Some(((hi as i32) << 8) | lo as i32)
    }

    fn set_index16(&mut self, r: IndexRegister16, value: i32) {
        self.set_index8(r.high(), ((value >> 8) & 0xFF) as u8);
        self.set_index8(r.low(), (value & 0xFF) as u8);
    }

    fn clear_index16(&mut self, r: IndexRegister16) {
        self.clear_index8(r.high());
        self.clear_index8(r.low());
    }

    pub fn sp(&self) -> Option<i32> {
        self.sp.map(|v| v as i32)
    }

    pub fn flag_z(&self) -> Option<bool> {
        self.flag_z
    }

    pub fn flag_c(&self) -> Option<bool> {
        self.flag_c
    }

    fn invalidate_all(&mut self) {
        self.main.clear();
        self.shadow.clear();
        self.index.clear();
        self.sp = None;
        self.flag_z = None;
        self.flag_c = None;
        self.shadow_flag_z = None;
        self.shadow_flag_c = None;
    }

    /// `EXX`: swaps `main`/`shadow` for B,C,D,E,H,L only - independent of
    /// `ex_af` (real Z80 has two separate exchange groups).
    fn exx(&mut self) {
        for r in [
            Register8::B,
            Register8::C,
            Register8::D,
            Register8::E,
            Register8::H,
            Register8::L
        ] {
            let m = self.main.remove(&r);
            let s = self.shadow.remove(&r);
            if let Some(v) = s {
                self.main.insert(r, v);
            }
            if let Some(v) = m {
                self.shadow.insert(r, v);
            }
        }
    }

    /// `EX AF,AF'`: swaps A and the flags (the whole AF pair) - independent
    /// of `exx`.
    fn ex_af(&mut self) {
        let m = self.main.remove(&Register8::A);
        let s = self.shadow.remove(&Register8::A);
        if let Some(v) = s {
            self.main.insert(Register8::A, v);
        }
        if let Some(v) = m {
            self.shadow.insert(Register8::A, v);
        }
        std::mem::swap(&mut self.flag_z, &mut self.shadow_flag_z);
        std::mem::swap(&mut self.flag_c, &mut self.shadow_flag_c);
    }

    /// `EX DE,HL`: swaps D<->H, E<->L within `main` only - no shadow
    /// interaction (the case most likely to get accidentally coupled to
    /// `exx` if implemented carelessly).
    fn ex_de_hl(&mut self) {
        for (a, b) in [(Register8::D, Register8::H), (Register8::E, Register8::L)] {
            let av = self.main.remove(&a);
            let bv = self.main.remove(&b);
            if let Some(v) = bv {
                self.main.insert(a, v);
            }
            if let Some(v) = av {
                self.main.insert(b, v);
            }
        }
    }
}

fn halves(r: Register16) -> Option<(Register8, Register8)> {
    match r {
        Register16::Bc => Some((Register8::B, Register8::C)),
        Register16::De => Some((Register8::D, Register8::E)),
        Register16::Hl => Some((Register8::H, Register8::L)),
        Register16::Af | Register16::Sp => None
    }
}

// ─── operand resolution ─────────────────────────────────────────────────────

fn resolve8(state: &TrackedState, access: &LocatedDataAccess, env: &mut Env) -> Option<u8> {
    match access {
        LocatedDataAccess::Register8(r, _) => state.get8(*r),
        LocatedDataAccess::IndexRegister8(r, _) => state.get_index8(*r),
        LocatedDataAccess::Expression(expr) => {
            expr.resolve(env)
                .ok()
                .and_then(|v| v.int().ok())
                .map(|v| (v & 0xFF) as u8)
        },
        _ => None
    }
}

fn resolve16(state: &TrackedState, access: &LocatedDataAccess, env: &mut Env) -> Option<i32> {
    match access {
        LocatedDataAccess::Register16(r, _) => state.get16(*r),
        LocatedDataAccess::IndexRegister16(r, _) => state.get_index16(*r),
        LocatedDataAccess::Expression(expr) => expr.resolve(env).ok().and_then(|v| v.int().ok()),
        _ => None
    }
}

fn write8(state: &mut TrackedState, dst: &LocatedDataAccess, value: Option<u8>) {
    match dst {
        LocatedDataAccess::Register8(r, _) => {
            match value {
                Some(v) => state.set8(*r, v),
                None => state.clear8(*r)
            }
        },
        LocatedDataAccess::IndexRegister8(r, _) => {
            match value {
                Some(v) => state.set_index8(*r, v),
                None => state.clear_index8(*r)
            }
        },
        _ => {}
    }
}

fn write16(state: &mut TrackedState, dst: &LocatedDataAccess, value: Option<i32>) {
    match dst {
        LocatedDataAccess::Register16(r, _) => {
            match value {
                Some(v) => state.set16(*r, v),
                None => state.clear16(*r)
            }
        },
        LocatedDataAccess::IndexRegister16(r, _) => {
            match value {
                Some(v) => state.set_index16(*r, v),
                None => state.clear_index16(*r)
            }
        },
        _ => {}
    }
}

/// Invalidate whatever `dst` writes (a `Register8`/`Register16`/index
/// register, whole or half) - the generic, safe fallback used by
/// instructions whose precise effect isn't otherwise modeled.
fn invalidate_write_target(state: &mut TrackedState, dst: Option<&LocatedDataAccess>) {
    let Some(dst) = dst
    else {
        return;
    };
    write8(state, dst, None);
    write16(state, dst, None);
}

// ─── instruction interpretation ────────────────────────────────────────────

/// Apply one instruction's effect to `state`. Unwraps a warning-wrapped
/// token (overflow warning, fake-instruction wrapper) via `warning_token()`
/// and recurses once rather than silently skipping it - an unhandled
/// warning must never leave a stale value behind.
pub fn apply(state: &mut TrackedState, token: &LocatedToken, env: &mut Env) {
    if token.is_warning() {
        apply(state, token.warning_token(), env);
        return;
    }
    if token.is_fake_instruction() {
        // A fake instruction (e.g. `ld hl,sp`, assembled as several real
        // opcodes) doesn't have a single well-defined effect here - the
        // safe fallback is to invalidate whatever it appears to write,
        // same as any other not-precisely-modeled instruction.
        invalidate_write_target(state, token.mnemonic_arg1());
        return;
    }
    let Some(mnemonic) = token.mnemonic()
    else {
        return;
    };
    let arg1 = token.mnemonic_arg1();
    let arg2 = token.mnemonic_arg2();

    match mnemonic {
        Mnemonic::Exx => state.exx(),
        Mnemonic::ExAf => state.ex_af(),
        Mnemonic::ExHlDe => state.ex_de_hl(),

        Mnemonic::Ld => apply_ld(state, arg1, arg2, env),

        Mnemonic::Inc | Mnemonic::Dec => apply_inc_dec(state, *mnemonic, arg1),

        Mnemonic::Sub | Mnemonic::And | Mnemonic::Or | Mnemonic::Xor => {
            apply_accumulator_single_op(state, *mnemonic, arg1, env)
        },

        Mnemonic::Cp => apply_cp(state, arg1, env),

        Mnemonic::Add | Mnemonic::Adc | Mnemonic::Sbc => {
            apply_accumulator_two_op(state, *mnemonic, arg1, arg2, env)
        },

        Mnemonic::Bit => apply_bit(state, arg1, arg2, env),

        Mnemonic::Push => apply_push(state, arg1),
        Mnemonic::Pop => apply_pop(state, arg1),

        Mnemonic::Cpl
        | Mnemonic::Daa
        | Mnemonic::Neg
        | Mnemonic::Rla
        | Mnemonic::Rlca
        | Mnemonic::Rra
        | Mnemonic::Rrca
        | Mnemonic::Rld
        | Mnemonic::Rrd => {
            state.clear8(Register8::A);
            state.flag_z = None;
            state.flag_c = None;
        },

        Mnemonic::Ldi
        | Mnemonic::Ldd
        | Mnemonic::Ldir
        | Mnemonic::Lddr
        | Mnemonic::Cpi
        | Mnemonic::Cpd
        | Mnemonic::Cpir
        | Mnemonic::Cpdr
        | Mnemonic::Ini
        | Mnemonic::Ind
        | Mnemonic::Inir
        | Mnemonic::Indr
        | Mnemonic::Outi
        | Mnemonic::Outd
        | Mnemonic::Otir
        | Mnemonic::Otdr => {
            for r in [
                Register8::B,
                Register8::C,
                Register8::D,
                Register8::E,
                Register8::H,
                Register8::L
            ] {
                state.clear8(r);
            }
            state.flag_z = None;
            state.flag_c = None;
        },

        Mnemonic::Res | Mnemonic::Set => {
            // arg1 is the bit-index expression, arg2 the register - never
            // arg1, a mismatch that would silently no-op every RES/SET.
            invalidate_write_target(state, arg2);
        },

        Mnemonic::Rlc
        | Mnemonic::Rrc
        | Mnemonic::Rl
        | Mnemonic::Rr
        | Mnemonic::Sla
        | Mnemonic::Sra
        | Mnemonic::Srl
        | Mnemonic::Sl1
        | Mnemonic::Srl8 => apply_rotate_shift(state, *mnemonic, arg1),

        Mnemonic::In | Mnemonic::ExMemSp => invalidate_write_target(state, arg1),

        Mnemonic::Ccf => state.flag_c = state.flag_c.map(|c| !c),
        Mnemonic::Scf => state.flag_c = Some(true),

        // IX/IY/SP-specific instructions (separate types from
        // Register8/Register16 in cpclib_tokens).
        Mnemonic::Call | Mnemonic::Ret | Mnemonic::Rst => {
            // These are boundary mnemonics for the caller's own walk (see
            // `registers.rs`) and should never reach `apply` mid-block -
            // but if one somehow does, the safe behavior is a full reset.
            state.invalidate_all();
        },
        Mnemonic::Jp | Mnemonic::Jr | Mnemonic::Djnz | Mnemonic::Reti | Mnemonic::Retn => {
            state.invalidate_all();
        },

        // No tracked-register/flag effect.
        Mnemonic::Out => {},
        Mnemonic::Di
        | Mnemonic::Ei
        | Mnemonic::Halt
        | Mnemonic::Im
        | Mnemonic::Nop
        | Mnemonic::Nop2 => {}
    }
}

fn apply_ld(
    state: &mut TrackedState,
    arg1: Option<&LocatedDataAccess>,
    arg2: Option<&LocatedDataAccess>,
    env: &mut Env
) {
    let (Some(dst), Some(src)) = (arg1, arg2)
    else {
        return;
    };
    match dst {
        LocatedDataAccess::Register8(..) | LocatedDataAccess::IndexRegister8(..) => {
            write8(state, dst, resolve8(state, src, env));
        },
        LocatedDataAccess::Register16(..) | LocatedDataAccess::IndexRegister16(..) => {
            write16(state, dst, resolve16(state, src, env));
        },
        _ => {} // memory destination - no tracked-register effect
    }
}

fn apply_inc_dec(state: &mut TrackedState, mnemonic: Mnemonic, arg1: Option<&LocatedDataAccess>) {
    let delta: i32 = if mnemonic == Mnemonic::Inc { 1 } else { -1 };
    let Some(arg1) = arg1
    else {
        return;
    };
    match arg1 {
        LocatedDataAccess::Register8(r, _) => {
            match state.get8(*r) {
                Some(v) => {
                    let new_v = (v as i32 + delta).rem_euclid(256) as u8;
                    state.set8(*r, new_v);
                    // INC/DEC never affect the carry flag - a classic Z80
                    // quirk, deliberately left untouched here.
                    state.flag_z = Some(new_v == 0);
                },
                None => state.flag_z = None
            }
        },
        LocatedDataAccess::IndexRegister8(r, _) => {
            match state.get_index8(*r) {
                Some(v) => {
                    let new_v = (v as i32 + delta).rem_euclid(256) as u8;
                    state.set_index8(*r, new_v);
                    state.flag_z = Some(new_v == 0);
                },
                None => state.flag_z = None
            }
        },
        LocatedDataAccess::Register16(r, _) => {
            match state.get16(*r) {
                Some(v) => state.set16(*r, (v + delta).rem_euclid(65536)),
                None => state.clear16(*r)
            }
        },
        LocatedDataAccess::IndexRegister16(r, _) => {
            match state.get_index16(*r) {
                Some(v) => state.set_index16(*r, (v + delta).rem_euclid(65536)),
                None => state.clear_index16(*r)
            }
        },
        _ => {}
    }
}

/// `SUB`/`AND`/`OR`/`XOR` - single operand, `A` implied as both destination
/// and the other operand.
fn apply_accumulator_single_op(
    state: &mut TrackedState,
    mnemonic: Mnemonic,
    arg1: Option<&LocatedDataAccess>,
    env: &mut Env
) {
    let Some(arg1) = arg1
    else {
        return;
    };
    let a = state.get8(Register8::A);
    let b = resolve8(state, arg1, env);
    match mnemonic {
        Mnemonic::And | Mnemonic::Or | Mnemonic::Xor => {
            // C is always forced false by these, independent of the values.
            state.flag_c = Some(false);
            match (a, b) {
                (Some(a), Some(b)) => {
                    let result = match mnemonic {
                        Mnemonic::And => a & b,
                        Mnemonic::Or => a | b,
                        _ => a ^ b
                    };
                    state.set8(Register8::A, result);
                    state.flag_z = Some(result == 0);
                },
                _ => {
                    state.clear8(Register8::A);
                    state.flag_z = None;
                }
            }
        },
        _ => {
            // Sub
            match (a, b) {
                (Some(a), Some(b)) => {
                    let result = (a as i32 - b as i32).rem_euclid(256) as u8;
                    state.set8(Register8::A, result);
                    state.flag_z = Some(result == 0);
                    state.flag_c = Some(a < b);
                },
                _ => {
                    state.clear8(Register8::A);
                    state.flag_z = None;
                    state.flag_c = None;
                }
            }
        }
    }
}

/// `CP` - like `SUB` but never writes `A`, only `Z`/`C`.
fn apply_cp(state: &mut TrackedState, arg1: Option<&LocatedDataAccess>, env: &mut Env) {
    let Some(arg1) = arg1
    else {
        return;
    };
    let a = state.get8(Register8::A);
    let b = resolve8(state, arg1, env);
    match (a, b) {
        (Some(a), Some(b)) => {
            state.flag_z = Some(a == b);
            state.flag_c = Some(a < b);
        },
        _ => {
            state.flag_z = None;
            state.flag_c = None;
        }
    }
}

/// `ADD`/`ADC`/`SBC` - always 2-operand. The 8-bit `A`-destination form is
/// precisely modeled (including `ADC`/`SBC`'s carry-in, when the tracked
/// carry flag is itself known); a 16-bit/index-register destination (the
/// `ADD/ADC/SBC HL,rr` form) isn't precisely modeled - falls through to the
/// generic invalidation, safely clearing the pair rather than guessing.
fn apply_accumulator_two_op(
    state: &mut TrackedState,
    mnemonic: Mnemonic,
    arg1: Option<&LocatedDataAccess>,
    arg2: Option<&LocatedDataAccess>,
    env: &mut Env
) {
    let (Some(arg1), Some(arg2)) = (arg1, arg2)
    else {
        return;
    };
    if !matches!(arg1, LocatedDataAccess::Register8(Register8::A, _)) {
        // 16-bit form (`ADD HL,rr` etc.) - not precisely modeled.
        invalidate_write_target(state, Some(arg1));
        return;
    }

    let a = state.get8(Register8::A);
    let b = resolve8(state, arg2, env);
    let carry_in = if mnemonic == Mnemonic::Add {
        Some(false)
    }
    else {
        state.flag_c
    };

    match (a, b, carry_in) {
        (Some(a), Some(b), Some(carry_in)) => {
            let carry_in = carry_in as i32;
            let (result, carry_out) = if mnemonic == Mnemonic::Sbc {
                let raw = a as i32 - b as i32 - carry_in;
                (raw.rem_euclid(256) as u8, raw < 0)
            }
            else {
                let raw = a as i32 + b as i32 + carry_in;
                (raw.rem_euclid(256) as u8, raw > 255)
            };
            state.set8(Register8::A, result);
            state.flag_z = Some(result == 0);
            state.flag_c = Some(carry_out);
        },
        _ => {
            state.clear8(Register8::A);
            state.flag_z = None;
            state.flag_c = None;
        }
    }
}

/// `BIT b,r` - tests a bit, sets `Z` (bit was 0) when `r` is known; never
/// touches `C`; the tested register itself is never written.
fn apply_bit(
    state: &mut TrackedState,
    arg1: Option<&LocatedDataAccess>,
    arg2: Option<&LocatedDataAccess>,
    env: &mut Env
) {
    let (Some(bit_expr), Some(reg)) = (arg1, arg2)
    else {
        return;
    };
    let bit = resolve8(state, bit_expr, env);
    let value = resolve8(state, reg, env);
    state.flag_z = match (bit, value) {
        (Some(bit), Some(value)) if bit < 8 => Some((value & (1 << bit)) == 0),
        _ => None
    };
}

fn apply_push(state: &mut TrackedState, arg1: Option<&LocatedDataAccess>) {
    // PUSH only reads its operand - never invalidate it - but does adjust a
    // known SP by -2.
    if let Some(sp) = state.sp {
        state.sp = Some(sp.wrapping_sub(2));
    }
    let _ = arg1;
}

fn apply_pop(state: &mut TrackedState, arg1: Option<&LocatedDataAccess>) {
    // The popped value comes from the (untracked) stack - always unknown -
    // plus SP advances by +2 when known.
    invalidate_write_target(state, arg1);
    if let Some(sp) = state.sp {
        state.sp = Some(sp.wrapping_add(2));
    }
}

/// `RLC`/`RRC`/`RL`/`RR`/`SLA`/`SRA`/`SRL`/`SL1`/`SRL8` - precise
/// value+flag computation when the input (and, for `RL`/`RR`, the current
/// carry-in) is known. Only `Register8`/`IndexRegister8` targets are
/// tracked; a memory/indexed-displacement target (`(HL)`/`(IX+d)`) has no
/// tracked-register effect, but its flags are still real and unknown here.
fn apply_rotate_shift(
    state: &mut TrackedState,
    mnemonic: Mnemonic,
    arg1: Option<&LocatedDataAccess>
) {
    let Some(arg1) = arg1
    else {
        return;
    };
    let input = match arg1 {
        LocatedDataAccess::Register8(r, _) => state.get8(*r),
        LocatedDataAccess::IndexRegister8(r, _) => state.get_index8(*r),
        _ => {
            state.flag_z = None;
            state.flag_c = None;
            return;
        }
    };

    let carry_in = state.flag_c;
    let (new_value, carry_out) = match (input, mnemonic) {
        (Some(v), Mnemonic::Rlc) => (Some(v.rotate_left(1)), Some((v & 0x80) != 0)),
        (Some(v), Mnemonic::Rrc) => (Some(v.rotate_right(1)), Some((v & 0x01) != 0)),
        (Some(v), Mnemonic::Rl) => {
            match carry_in {
                Some(c) => (Some((v << 1) | (c as u8)), Some((v & 0x80) != 0)),
                None => (None, None)
            }
        },
        (Some(v), Mnemonic::Rr) => {
            match carry_in {
                Some(c) => (Some((v >> 1) | ((c as u8) << 7)), Some((v & 0x01) != 0)),
                None => (None, None)
            }
        },
        (Some(v), Mnemonic::Sla) => (Some(v << 1), Some((v & 0x80) != 0)),
        (Some(v), Mnemonic::Sra) => (Some((v >> 1) | (v & 0x80)), Some((v & 0x01) != 0)),
        (Some(v), Mnemonic::Srl | Mnemonic::Srl8) => (Some(v >> 1), Some((v & 0x01) != 0)),
        (Some(v), Mnemonic::Sl1) => (Some((v << 1) | 1), Some((v & 0x80) != 0)),
        _ => (None, None)
    };

    match arg1 {
        LocatedDataAccess::Register8(r, _) => {
            match new_value {
                Some(v) => state.set8(*r, v),
                None => state.clear8(*r)
            }
        },
        LocatedDataAccess::IndexRegister8(r, _) => {
            match new_value {
                Some(v) => state.set_index8(*r, v),
                None => state.clear_index8(*r)
            }
        },
        _ => {}
    }
    state.flag_z = new_value.map(|v| v == 0);
    state.flag_c = carry_out;
}

#[cfg(test)]
mod tests {
    use cpclib_asm::assembler::Env;
    use cpclib_asm::parser::context::ParserContextBuilder;
    use cpclib_asm::parser::obtained::LocatedListing;

    use super::*;

    fn apply_all(text: &str) -> TrackedState {
        // `LocatedToken::clone()` is deliberately `unimplemented!()` -
        // iterate borrowed tokens straight from the still-alive listing,
        // never collect owned copies.
        let builder = ParserContextBuilder::default().set_quiet(true);
        let listing = LocatedListing::new_complete_source(text, builder)
            .unwrap_or_else(|_| panic!("expected {text:?} to parse cleanly"));
        let mut state = TrackedState::default();
        let mut env = Env::default();
        for token in listing.iter() {
            apply(&mut state, token, &mut env);
        }
        state
    }

    #[test]
    fn ld_immediate_chain_tracks_forward() {
        let s = apply_all("ld a,5\nld b,a\nld c,10\n");
        assert_eq!(s.get8(Register8::A), Some(5));
        assert_eq!(s.get8(Register8::B), Some(5));
        assert_eq!(s.get8(Register8::C), Some(10));
    }

    #[test]
    fn ld_16bit_immediate_sets_both_halves() {
        let s = apply_all("ld bc,0x1234\n");
        assert_eq!(s.get8(Register8::B), Some(0x12));
        assert_eq!(s.get8(Register8::C), Some(0x34));
        assert_eq!(s.get16(Register16::Bc), Some(0x1234));
    }

    #[test]
    fn inc_dec_8bit_adjusts_known_value_and_sets_z_not_c() {
        let s = apply_all("ld a,5\ninc a\ninc a\n");
        assert_eq!(s.get8(Register8::A), Some(7));
        assert_eq!(s.flag_z(), Some(false));

        let s = apply_all("ld a,0xff\ninc a\n");
        assert_eq!(s.get8(Register8::A), Some(0));
        assert_eq!(s.flag_z(), Some(true));
        // INC never affects carry - it must stay unknown here (never
        // guessed at some value), since nothing else set it.
        assert_eq!(s.flag_c(), None);
    }

    #[test]
    fn inc_alone_with_unknown_register_stays_unknown() {
        let s = apply_all("inc a\n");
        assert_eq!(s.get8(Register8::A), None);
    }

    #[test]
    fn inc_dec_16bit_pair_requires_both_halves_known() {
        let s = apply_all("ld h,0x40\ninc hl\n");
        assert_eq!(s.get8(Register8::H), None);
        assert_eq!(s.get8(Register8::L), None);

        let s = apply_all("ld hl,0x40ff\ninc hl\n");
        assert_eq!(s.get16(Register16::Hl), Some(0x4100));
    }

    #[test]
    fn exx_swaps_and_swaps_back() {
        let s = apply_all("ld b,1\nld c,2\nexx\nld b,3\nld c,4\nexx\n");
        assert_eq!(s.get8(Register8::B), Some(1));
        assert_eq!(s.get8(Register8::C), Some(2));
    }

    #[test]
    fn ex_af_is_independent_of_exx() {
        let s = apply_all("ld a,1\nexx\n");
        // EXX must never touch A.
        assert_eq!(s.get8(Register8::A), Some(1));

        let s = apply_all("ld a,1\nex af,af'\n");
        // EX AF,AF' swaps A away to the shadow set - now unknown in main.
        assert_eq!(s.get8(Register8::A), None);
    }

    #[test]
    fn ex_de_hl_swaps_only_main() {
        let s = apply_all("ld d,1\nld e,2\nld h,3\nld l,4\nex de,hl\n");
        assert_eq!(s.get8(Register8::D), Some(3));
        assert_eq!(s.get8(Register8::E), Some(4));
        assert_eq!(s.get8(Register8::H), Some(1));
        assert_eq!(s.get8(Register8::L), Some(2));
    }

    #[test]
    fn accumulator_arithmetic_both_known_computes_result_and_flags() {
        let s = apply_all("ld a,3\nld b,4\nadd a,b\n");
        assert_eq!(s.get8(Register8::A), Some(7));
        assert_eq!(s.flag_z(), Some(false));
        assert_eq!(s.flag_c(), Some(false));

        let s = apply_all("ld a,0xff\nld b,1\nadd a,b\n");
        assert_eq!(s.get8(Register8::A), Some(0));
        assert_eq!(s.flag_z(), Some(true));
        assert_eq!(s.flag_c(), Some(true));
    }

    #[test]
    fn accumulator_arithmetic_unknown_operand_reports_unknown_not_a_guess() {
        let s = apply_all("ld a,3\nadd a,b\n");
        assert_eq!(s.get8(Register8::A), None);
        assert_eq!(s.flag_z(), None);
    }

    #[test]
    fn cp_never_writes_a_only_flags() {
        let s = apply_all("ld a,5\nld b,5\ncp b\n");
        assert_eq!(s.get8(Register8::A), Some(5));
        assert_eq!(s.flag_z(), Some(true));
        assert_eq!(s.flag_c(), Some(false));

        let s = apply_all("ld a,3\nld b,5\ncp b\n");
        assert_eq!(s.flag_z(), Some(false));
        assert_eq!(s.flag_c(), Some(true));
    }

    #[test]
    fn adc_sbc_computed_precisely_when_carry_known() {
        let s = apply_all("ld a,3\nld b,4\nand a\nadc a,b\n");
        // `and a` forces C=false and A=3 (3&3), so ADC with known carry=0
        // should compute exactly like ADD.
        assert_eq!(s.get8(Register8::A), Some(7));
    }

    #[test]
    fn adc_sbc_invalidated_when_carry_unknown() {
        let s = apply_all("ld a,3\nld b,4\nadc a,b\n");
        assert_eq!(s.get8(Register8::A), None);
        assert_eq!(s.flag_z(), None);
        assert_eq!(s.flag_c(), None);
    }

    #[test]
    fn neg_and_cpl_invalidate_accumulator_not_stale() {
        let s = apply_all("ld a,5\nneg\n");
        assert_eq!(s.get8(Register8::A), None);

        let s = apply_all("ld a,5\ncpl\n");
        assert_eq!(s.get8(Register8::A), None);
    }

    #[test]
    fn ldir_invalidates_the_block_not_stale() {
        let s = apply_all("ld bc,5\nld de,0x8000\nld hl,0x9000\nldir\n");
        assert_eq!(s.get8(Register8::B), None);
        assert_eq!(s.get8(Register8::C), None);
        assert_eq!(s.get8(Register8::D), None);
        assert_eq!(s.get8(Register8::E), None);
        assert_eq!(s.get8(Register8::H), None);
        assert_eq!(s.get8(Register8::L), None);
    }

    #[test]
    fn res_set_invalidate_arg2_not_arg1() {
        // Regression test: RES/SET's register is arg2, not arg1 (arg1 is
        // the bit-index expression) - asserted against the specific
        // pre-fix stale value (5) to make this a real regression guard.
        let s = apply_all("ld b,5\nres 0,b\n");
        assert_eq!(
            s.get8(Register8::B),
            None,
            "should be invalidated, not stale 5"
        );

        let s = apply_all("ld b,5\nset 0,b\n");
        assert_eq!(
            s.get8(Register8::B),
            None,
            "should be invalidated, not stale 5"
        );
    }

    #[test]
    fn push_does_not_invalidate_the_pushed_register_but_adjusts_a_known_sp() {
        let s = apply_all("ld bc,5\nld sp,0xc000\npush bc\n");
        assert_eq!(s.get16(Register16::Bc), Some(5));
        assert_eq!(s.sp(), Some(0xBFFE));
    }

    #[test]
    fn pop_invalidates_the_popped_register_and_adjusts_sp() {
        let s = apply_all("ld bc,5\nld sp,0xc000\npop bc\n");
        assert_eq!(s.get16(Register16::Bc), None);
        assert_eq!(s.sp(), Some(0xC002));
    }

    #[test]
    fn bit_sets_z_precisely() {
        let s = apply_all("ld a,0b00000010\nbit 1,a\n");
        assert_eq!(s.flag_z(), Some(false));

        let s = apply_all("ld a,0b00000000\nbit 1,a\n");
        assert_eq!(s.flag_z(), Some(true));
    }

    #[test]
    fn rotate_shift_sets_z_and_c_from_known_input() {
        let s = apply_all("ld a,0b10000001\nrlc a\n");
        assert_eq!(s.get8(Register8::A), Some(0b00000011));
        assert_eq!(s.flag_c(), Some(true));
        assert_eq!(s.flag_z(), Some(false));
    }

    #[test]
    fn ix_iy_halves_tracked_like_h_and_l() {
        let s = apply_all("ld ixh,0x12\nld ixl,0x34\n");
        assert_eq!(s.get_index16(IndexRegister16::Ix), Some(0x1234));
    }

    #[test]
    fn ix_iy_immediate_and_inc() {
        let s = apply_all("ld ix,0x1000\ninc ix\n");
        assert_eq!(s.get_index16(IndexRegister16::Ix), Some(0x1001));
    }

    #[test]
    fn a_warning_wrapped_token_still_gets_tracked_not_skipped() {
        // `ld a, 300` triggers an overflow warning - the wrapped token must
        // still be applied (A becomes the truncated value, not silently
        // skipped, which would leave A permanently unknown for the rest of
        // the block).
        let s = apply_all("ld a, 300\n");
        assert_eq!(s.get8(Register8::A), Some((300i32 & 0xFF) as u8));
    }
}
