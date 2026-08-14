//! What each instruction reads and writes.
//!
//! This is the per-instruction semantics the forward-liveness constraints
//! (`regsNotUsedAfter`, `flagsNotUsedAfter`) are built on, and it comes from
//! upstream's own vendored table (`vendor/z80cpc-instruction-set.tsv`) rather
//! than being hand-authored. That table is CPC-specific (its header documents
//! real CPC-only corrections, e.g. *"out (c),r instructions in the Amstrad CPC
//! actually also depend on B, as ports are 16 addresses"*), and it encodes
//! distinctions that are genuinely easy to get wrong by hand:
//!
//! | Instruction | Reads | Why it's subtle |
//! |---|---|---|
//! | `XOR A` | *nothing* | the self-clearing idiom really doesn't read `A` |
//! | `XOR B` | `A`, `B` | ...whereas the general form does |
//! | `DEC A` | `A` (and writes it) | read-modify-write, not a plain overwrite |
//! | `DJNZ o` | `B`, no flags | tests `B`, not the zero flag |
//! | `POP BC` | `SP`, memory | notably *not* the old `BC` |
//!
//! ## How a row is matched to a real instruction
//!
//! The table uses placeholders, and the rule for spotting one is exactly
//! upstream's `CPUOpSpec::isPrimitiveReg`: **an all-uppercase token is a
//! literal register, anything else is a placeholder**. So `LD A,r` covers
//! `LD A,B` through `LD A,L`, and `R` (the refresh register) is a completely
//! different thing from `r` (any 8-bit register) - a case-insensitive
//! comparison anywhere in here would silently conflate them.
//!
//! A placeholder in the input/output columns is resolved *positionally*: find
//! the argument slot whose own spec is that same token, then read the concrete
//! register out of the real instruction's operand at that index.
//!
//! ## Which direction the table's errors point
//!
//! The table is vendored verbatim and is not perfect - `LD A,R` is listed as
//! writing no flags, where real hardware sets `S`/`Z`/`H`/`P/V`/`N` exactly as
//! `LD A,I` does (which the table *does* record). That gap is worth knowing
//! about, and it is worth knowing that it is harmless here:
//!
//! - An **under-reported write** means a liveness walk fails to notice a flag
//!   was clobbered, so it keeps looking for a later read and is more likely to
//!   report "still used" - i.e. it declines an optimization it could have
//!   allowed. Safe.
//! - An **under-reported read** would be the dangerous direction: it would let
//!   the walk conclude a value is dead when something still consumes it.
//!
//! So the table is trusted as-is rather than patched (keeping re-vendoring a
//! plain file copy), because its known imprecision costs opportunities, never
//! correctness.

use std::collections::HashMap;
use std::sync::LazyLock;

use cpclib_tokens::{DataAccess, DataAccessElem, FlagTest, ListingElement, Mnemonic};

use crate::analysis_op::AnalysisOp;
use crate::regflag::{Flag, Reg};

/// The vendored table, compiled in.
const TABLE_TSV: &str = include_str!("vendor/z80cpc-instruction-set.tsv");

/// What one argument slot of a table row accepts.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ArgSpec {
    /// A literal register name (`A`, `HL`, `IXH`, `I`, `R`, ...).
    Reg(Reg),
    /// The shadow accumulator/flags pair, `AF'`.
    AfAlt,
    /// Indirection through a register: `(HL)`, `(BC)`, `(DE)`, `(SP)`,
    /// `(IX)`, `(IY)`.
    MemReg(Reg),
    /// The `(C)` I/O port.
    PortC,
    /// `(IX+o)` / `(IY+o)`.
    Indexed(Reg),
    /// Memory at an immediate address: `(n)` / `(nn)`.
    MemImm,
    /// One *specific* condition code. Deliberately not a catch-all "any
    /// condition": each form reads a different flag (`NZ`/`Z` read `Z`,
    /// `NC`/`C` read `C`, `PO`/`PE` read `P/V`, `P`/`M` read `S`), so a row
    /// matching the wrong one would report the wrong flag as read - a silent
    ///, wrong "safe to optimize" answer.
    Cond(FlagTest),
    /// Any immediate: `n`, `nn`, `o`, `b`, or a literal number (`RST 8H`,
    /// `IM 1`).
    Imm,
    /// `r` - any of the seven plain 8-bit registers.
    AnyReg8,
    /// `p` / `IXp` - either half of `IX`.
    IxHalf,
    /// `q` / `IYq` - either half of `IY`.
    IyHalf
}

/// One argument slot: the exact token as the table spells it, plus what it
/// accepts. The raw text is kept because placeholder resolution matches on it
/// literally (upstream compares `specArg.reg` to the dependency name).
#[derive(Debug, Clone)]
struct ArgPattern {
    raw: String,
    spec: ArgSpec
}

/// One row of the table: an instruction form and everything it touches.
#[derive(Debug, Clone)]
struct OpRow {
    mnemonic: Mnemonic,
    args: Vec<ArgPattern>,
    /// Raw (unresolved) register tokens - resolved against a real
    /// instruction by [`resolve_regs`].
    input_regs: Vec<String>,
    output_regs: Vec<String>,
    input_flags: Vec<Flag>,
    output_flags: Vec<Flag>,
    reads_memory: bool,
    writes_memory: bool,
    reads_port: bool,
    writes_port: bool
}

/// Everything one concrete instruction reads and writes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Effects {
    pub reads: Vec<Reg>,
    pub writes: Vec<Reg>,
    pub reads_flags: Vec<Flag>,
    pub writes_flags: Vec<Flag>,
    pub reads_memory: bool,
    pub writes_memory: bool,
    pub reads_port: bool,
    pub writes_port: bool
}

impl Effects {
    /// Nothing at all - `NOP`, and basm's `NOP2` breakpoint marker.
    fn none() -> Self {
        Self::default()
    }
}

/// The parsed table, keyed by mnemonic. Several rows share a mnemonic (one
/// per operand form), tried in table order with literal forms preferred over
/// placeholder ones - so `XOR A` matches its own row rather than `XOR r`.
static TABLE: LazyLock<HashMap<Mnemonic, Vec<OpRow>>> = LazyLock::new(|| {
    let mut table: HashMap<Mnemonic, Vec<OpRow>> = HashMap::new();
    for line in TABLE_TSV.lines() {
        if line.starts_with(';') || line.trim().is_empty() {
            continue;
        }
        if let Some(row) = parse_row(line) {
            table.entry(row.mnemonic).or_default().push(row);
        }
    }
    // A row with no placeholders is more specific than one with, so it must
    // win: `XOR A` reads nothing while `XOR r` reads `A` and the operand.
    for rows in table.values_mut() {
        rows.sort_by_key(|r| r.args.iter().filter(|a| is_placeholder(&a.raw)).count());
    }
    table
});

/// Upstream's `isPrimitiveReg`, inverted: a token is a placeholder unless it
/// is entirely uppercase. This is the whole disambiguation rule, and it is
/// why nothing in this module compares register tokens case-insensitively.
fn is_placeholder(token: &str) -> bool {
    token != token.to_ascii_uppercase()
}

fn parse_row(line: &str) -> Option<OpRow> {
    let cols: Vec<&str> = line.split('\t').collect();
    // Instruction, Timing, Opcode, Size, InRegs, InFlags, InPorts, InMem,
    // OutRegs, OutFlags, OutPorts, OutMem, Official
    if cols.len() < 12 {
        return None;
    }

    let instruction = cols[0].trim();
    let (mnemonic_text, operands_text) = match instruction.split_once(char::is_whitespace) {
        Some((m, rest)) => (m, rest.trim()),
        None => (instruction, "")
    };
    // `EX` is one mnemonic in the table but three in our enum, distinguished
    // only by its operands - so it has to be resolved before the generic
    // lookup, which deliberately refuses it.
    let mnemonic = if mnemonic_text.eq_ignore_ascii_case("EX") {
        ex_variant(operands_text)?
    }
    else {
        parse_mnemonic(mnemonic_text)?
    };

    // Our `EX` variants carry the operands in the mnemonic itself
    // (`ExHlDe` *is* `EX DE,HL`), so the row must expect no argument slots -
    // otherwise nothing would ever match it.
    if matches!(
        mnemonic,
        Mnemonic::ExAf | Mnemonic::ExHlDe | Mnemonic::ExMemSp
    ) {
        return Some(OpRow {
            mnemonic,
            args: Vec::new(),
            input_regs: split_tokens(cols[4]),
            output_regs: split_tokens(cols[8]),
            input_flags: parse_flags(cols[5]),
            output_flags: parse_flags(cols[9]),
            reads_port: !cols[6].trim().is_empty(),
            reads_memory: !cols[7].trim().is_empty(),
            writes_port: !cols[10].trim().is_empty(),
            writes_memory: !cols[11].trim().is_empty()
        });
    }

    let args = if operands_text.is_empty() {
        Vec::new()
    }
    else {
        operands_text
            .split(',')
            .map(|tok| {
                let raw = tok.trim().to_string();
                parse_arg_spec(&raw, mnemonic).map(|spec| ArgPattern { raw, spec })
            })
            .collect::<Option<Vec<_>>>()?
    };

    Some(OpRow {
        mnemonic,
        args,
        input_regs: split_tokens(cols[4]),
        input_flags: parse_flags(cols[5]),
        output_regs: split_tokens(cols[8]),
        output_flags: parse_flags(cols[9]),
        reads_port: !cols[6].trim().is_empty(),
        reads_memory: !cols[7].trim().is_empty(),
        writes_port: !cols[10].trim().is_empty(),
        writes_memory: !cols[11].trim().is_empty()
    })
}

fn split_tokens(col: &str) -> Vec<String> {
    col.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_flags(col: &str) -> Vec<Flag> {
    split_tokens(col)
        .iter()
        .filter_map(|t| Flag::parse(t))
        .collect()
}

/// Map the table's mnemonic spelling onto ours. Mostly identical; the
/// exceptions are the ones where our enum is *more* specific than the table
/// (`EX` is three separate variants for us) or spells a shift differently.
fn parse_mnemonic(text: &str) -> Option<Mnemonic> {
    Some(match text.to_ascii_uppercase().as_str() {
        "ADC" => Mnemonic::Adc,
        "ADD" => Mnemonic::Add,
        "AND" => Mnemonic::And,
        "BIT" => Mnemonic::Bit,
        "CALL" => Mnemonic::Call,
        "CCF" => Mnemonic::Ccf,
        "CP" => Mnemonic::Cp,
        "CPD" => Mnemonic::Cpd,
        "CPDR" => Mnemonic::Cpdr,
        "CPI" => Mnemonic::Cpi,
        "CPIR" => Mnemonic::Cpir,
        "CPL" => Mnemonic::Cpl,
        "DAA" => Mnemonic::Daa,
        "DEC" => Mnemonic::Dec,
        "DI" => Mnemonic::Di,
        "DJNZ" => Mnemonic::Djnz,
        "EI" => Mnemonic::Ei,
        "EXX" => Mnemonic::Exx,
        "HALT" => Mnemonic::Halt,
        "IM" => Mnemonic::Im,
        "IN" => Mnemonic::In,
        "INC" => Mnemonic::Inc,
        "IND" => Mnemonic::Ind,
        "INDR" => Mnemonic::Indr,
        "INI" => Mnemonic::Ini,
        "INIR" => Mnemonic::Inir,
        "JP" => Mnemonic::Jp,
        "JR" => Mnemonic::Jr,
        "LD" => Mnemonic::Ld,
        "LDD" => Mnemonic::Ldd,
        "LDDR" => Mnemonic::Lddr,
        "LDI" => Mnemonic::Ldi,
        "LDIR" => Mnemonic::Ldir,
        "NEG" => Mnemonic::Neg,
        "NOP" => Mnemonic::Nop,
        "OR" => Mnemonic::Or,
        "OTDR" => Mnemonic::Otdr,
        "OTIR" => Mnemonic::Otir,
        "OUT" => Mnemonic::Out,
        "OUTD" => Mnemonic::Outd,
        "OUTI" => Mnemonic::Outi,
        "POP" => Mnemonic::Pop,
        "PUSH" => Mnemonic::Push,
        "RES" => Mnemonic::Res,
        "RET" => Mnemonic::Ret,
        "RETI" => Mnemonic::Reti,
        "RETN" => Mnemonic::Retn,
        "RL" => Mnemonic::Rl,
        "RLA" => Mnemonic::Rla,
        "RLC" => Mnemonic::Rlc,
        "RLCA" => Mnemonic::Rlca,
        "RLD" => Mnemonic::Rld,
        "RR" => Mnemonic::Rr,
        "RRA" => Mnemonic::Rra,
        "RRC" => Mnemonic::Rrc,
        "RRCA" => Mnemonic::Rrca,
        "RRD" => Mnemonic::Rrd,
        "RST" => Mnemonic::Rst,
        "SBC" => Mnemonic::Sbc,
        "SCF" => Mnemonic::Scf,
        "SET" => Mnemonic::Set,
        "SLA" => Mnemonic::Sla,
        // The table spells the undocumented "shift left, set bit 0" three
        // ways; we have one variant for it.
        "SL1" | "SLI" | "SLL" => Mnemonic::Sl1,
        "SRA" => Mnemonic::Sra,
        "SRL" => Mnemonic::Srl,
        "SUB" => Mnemonic::Sub,
        "XOR" => Mnemonic::Xor,
        // `EX` is one mnemonic in the table and three in our enum; the row's
        // *operands* decide which, so it's handled by `parse_row`'s caller
        // via `ex_variant` below.
        "EX" => return None,
        _ => return None
    })
}

/// `EX` needs its operands to pick our variant, so it's resolved separately
/// from the plain mnemonic table above.
fn ex_variant(operands: &str) -> Option<Mnemonic> {
    let normalized: String = operands
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();
    Some(match normalized.as_str() {
        "AF,AF'" | "AF,AF" => Mnemonic::ExAf,
        "DE,HL" | "HL,DE" => Mnemonic::ExHlDe,
        s if s.starts_with("(SP),") => Mnemonic::ExMemSp,
        _ => return None
    })
}

fn parse_arg_spec(raw: &str, mnemonic: Mnemonic) -> Option<ArgSpec> {
    // Conditions only ever appear as the first operand of a branch, which is
    // what disambiguates `C` the carry condition from `C` the register.
    let branches = matches!(
        mnemonic,
        Mnemonic::Jp | Mnemonic::Jr | Mnemonic::Call | Mnemonic::Ret
    );
    if branches {
        let cond = match raw {
            "NZ" => Some(FlagTest::NZ),
            "Z" => Some(FlagTest::Z),
            "NC" => Some(FlagTest::NC),
            "C" => Some(FlagTest::C),
            "PO" => Some(FlagTest::PO),
            "PE" => Some(FlagTest::PE),
            "P" => Some(FlagTest::P),
            "M" => Some(FlagTest::M),
            _ => None
        };
        if let Some(cond) = cond {
            return Some(ArgSpec::Cond(cond));
        }
    }

    Some(match raw {
        "r" => ArgSpec::AnyReg8,
        "p" | "IXp" => ArgSpec::IxHalf,
        "q" | "IYq" => ArgSpec::IyHalf,
        "n" | "nn" | "o" | "b" => ArgSpec::Imm,
        "(n)" | "(nn)" => ArgSpec::MemImm,
        "(C)" => ArgSpec::PortC,
        "AF'" => ArgSpec::AfAlt,
        "(IX+o)" => ArgSpec::Indexed(Reg::Ix),
        "(IY+o)" => ArgSpec::Indexed(Reg::Iy),
        _ => {
            if let Some(inner) = raw.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
                ArgSpec::MemReg(Reg::parse(inner)?)
            }
            else if raw.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                // `RST 8H`, `IM 1`, and bit indices written literally.
                ArgSpec::Imm
            }
            else if is_placeholder(raw) {
                // An unknown lowercase token - refuse the row rather than
                // guess, so a future table revision fails loudly.
                return None;
            }
            else {
                ArgSpec::Reg(Reg::parse(raw)?)
            }
        }
    })
}

/// Whether a real operand satisfies one argument slot.
fn arg_matches(spec: &ArgSpec, operand: &DataAccess) -> bool {
    match spec {
        ArgSpec::Reg(expected) => concrete_reg(operand) == Some(*expected),
        ArgSpec::AfAlt => matches!(operand, DataAccess::Register16(cpclib_tokens::Register16::Af)),
        ArgSpec::MemReg(expected) => {
            match operand {
                DataAccess::MemoryRegister16(r) => Reg::from(*r) == *expected,
                DataAccess::MemoryIndexRegister16(r) => Reg::from(*r) == *expected,
                _ => false
            }
        },
        ArgSpec::PortC => matches!(operand, DataAccess::PortC),
        ArgSpec::Indexed(expected) => {
            matches!(operand, DataAccess::IndexRegister16WithIndex(r, ..) if Reg::from(*r) == *expected)
        },
        ArgSpec::MemImm => matches!(operand, DataAccess::Memory(_) | DataAccess::PortN(_)),
        ArgSpec::Cond(expected) => operand.get_flag_test() == Some(*expected),
        ArgSpec::Imm => matches!(operand, DataAccess::Expression(_)),
        ArgSpec::AnyReg8 => matches!(operand, DataAccess::Register8(_)),
        ArgSpec::IxHalf => {
            matches!(operand, DataAccess::IndexRegister8(r)
                if matches!(r, cpclib_tokens::IndexRegister8::Ixh | cpclib_tokens::IndexRegister8::Ixl))
        },
        ArgSpec::IyHalf => {
            matches!(operand, DataAccess::IndexRegister8(r)
                if matches!(r, cpclib_tokens::IndexRegister8::Iyh | cpclib_tokens::IndexRegister8::Iyl))
        }
    }
}

/// The plain register an operand names, if it names one directly.
fn concrete_reg(operand: &DataAccess) -> Option<Reg> {
    Some(match operand {
        DataAccess::Register8(r) => Reg::from(*r),
        DataAccess::Register16(r) => Reg::from(*r),
        DataAccess::IndexRegister8(r) => Reg::from(*r),
        DataAccess::IndexRegister16(r) => Reg::from(*r),
        DataAccess::SpecialRegisterI => Reg::I,
        DataAccess::SpecialRegisterR => Reg::R,
        _ => return None
    })
}

/// What `op` reads and writes, or `None` if no table row describes it - in
/// which case the caller must treat the instruction as opaque and fail
/// closed, never as "touches nothing".
pub fn effects_of<T>(op: &AnalysisOp<'_, T>) -> Option<Effects>
where T: ListingElement {
    let mnemonic = op.mnemonic()?;

    // basm's WinAPE-breakpoint marker assembles to `ED FF`, an undocumented
    // two-byte no-op. It has no table row (it isn't a real Z80 instruction),
    // but its effects are exactly `NOP`'s: nothing.
    if mnemonic == Mnemonic::Nop2 {
        return Some(Effects::none());
    }

    let operands: Vec<DataAccess> = [op.arg1(), op.arg2()]
        .into_iter()
        .flatten()
        .map(|c| c.into_owned())
        .collect();

    let rows = TABLE.get(&mnemonic)?;
    let row = rows.iter().find(|row| {
        row.args.len() == operands.len()
            && row
                .args
                .iter()
                .zip(&operands)
                .all(|(spec, operand)| arg_matches(&spec.spec, operand))
    })?;

    let mut effects = Effects {
        reads: resolve_regs(&row.input_regs, row, &operands),
        writes: resolve_regs(&row.output_regs, row, &operands),
        reads_flags: row.input_flags.clone(),
        writes_flags: row.output_flags.clone(),
        reads_memory: row.reads_memory,
        writes_memory: row.writes_memory,
        reads_port: row.reads_port,
        writes_port: row.writes_port
    };

    // The undocumented third operand (`SLA (IX+d), B`) additionally receives
    // the result. The table has no row for these forms, but when our parser
    // produces one the extra destination must not be lost.
    if let Some(extra) = op.arg3() {
        let extra = Reg::from(extra);
        if !effects.writes.contains(&extra) {
            effects.writes.push(extra);
        }
    }

    Some(effects)
}

/// Turn a row's raw register tokens into concrete registers, resolving
/// placeholders positionally against the real operands.
fn resolve_regs(raw: &[String], row: &OpRow, operands: &[DataAccess]) -> Vec<Reg> {
    let mut out = Vec::with_capacity(raw.len());
    for name in raw {
        if is_placeholder(name) {
            // Find the argument slot this placeholder refers to, then read
            // the real register out of that operand.
            if let Some(idx) = row.args.iter().position(|a| &a.raw == name)
                && let Some(reg) = operands.get(idx).and_then(concrete_reg)
            {
                out.push(reg);
            }
        }
        else if let Some(reg) = Reg::parse(name) {
            out.push(reg);
        }
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use cpclib_tokens::{DataAccess, Register8, Register16, Token};

    use super::*;

    fn op(mnemonic: Mnemonic, arg1: Option<DataAccess>, arg2: Option<DataAccess>) -> Token {
        Token::OpCode(mnemonic, arg1, arg2, None)
    }

    fn effects(token: &Token) -> Effects {
        effects_of(&AnalysisOp::Real(token)).unwrap_or_else(|| panic!("no row for {token:?}"))
    }

    fn r8(r: Register8) -> Option<DataAccess> {
        Some(DataAccess::Register8(r))
    }

    #[test]
    fn the_table_loads_and_covers_the_whole_instruction_set() {
        assert!(TABLE.len() > 50, "only {} mnemonics parsed", TABLE.len());
        // A few spot checks that rows really landed under the right key.
        assert!(TABLE.contains_key(&Mnemonic::Ld));
        assert!(TABLE.contains_key(&Mnemonic::Djnz));
        assert!(TABLE.contains_key(&Mnemonic::Ldir));
        // `EX` is one table mnemonic and three of ours - all three must have
        // landed, or every exchange would be opaque to the analysis.
        assert!(TABLE.contains_key(&Mnemonic::ExAf), "EX AF,AF' missing");
        assert!(TABLE.contains_key(&Mnemonic::ExHlDe), "EX DE,HL missing");
        assert!(TABLE.contains_key(&Mnemonic::ExMemSp), "EX (SP),HL missing");
    }

    /// Exhaustiveness guard, not an allowlist: every mnemonic our parser can
    /// produce must be describable, so nothing silently falls into the
    /// "opaque, fail closed" path just because a table row was missed.
    ///
    /// The three genuine exceptions are all resolved *before* this module
    /// ever sees them, and are listed here so a future change that breaks one
    /// of those upstream steps fails loudly here:
    ///  - `Jq` is rewritten to the concrete `JP`/`JR` it assembles to;
    ///  - `Srl8` is a fake instruction, expanded into real ones;
    ///  - `Nop2` has no table row and is answered directly above.
    #[test]
    fn every_mnemonic_we_can_parse_is_described_by_the_table() {
        let resolved_before_this_module = [Mnemonic::Jq, Mnemonic::Srl8];
        let missing: Vec<Mnemonic> = ALL_MNEMONICS
            .iter()
            .copied()
            .filter(|m| {
                *m != Mnemonic::Nop2
                    && !resolved_before_this_module.contains(m)
                    && !TABLE.contains_key(m)
            })
            .collect();
        assert!(missing.is_empty(), "no table rows for: {missing:?}");
    }

    /// Kept next to the test above so the two can't drift: this is every
    /// variant of `cpclib_tokens::Mnemonic`.
    const ALL_MNEMONICS: &[Mnemonic] = &[
        Mnemonic::Adc,
        Mnemonic::Add,
        Mnemonic::And,
        Mnemonic::Bit,
        Mnemonic::Call,
        Mnemonic::Ccf,
        Mnemonic::Cp,
        Mnemonic::Cpd,
        Mnemonic::Cpdr,
        Mnemonic::Cpi,
        Mnemonic::Cpir,
        Mnemonic::Cpl,
        Mnemonic::Daa,
        Mnemonic::Dec,
        Mnemonic::Di,
        Mnemonic::Djnz,
        Mnemonic::Ei,
        Mnemonic::ExAf,
        Mnemonic::ExHlDe,
        Mnemonic::ExMemSp,
        Mnemonic::Exx,
        Mnemonic::Halt,
        Mnemonic::Im,
        Mnemonic::In,
        Mnemonic::Inc,
        Mnemonic::Ind,
        Mnemonic::Indr,
        Mnemonic::Ini,
        Mnemonic::Inir,
        Mnemonic::Jp,
        Mnemonic::Jq,
        Mnemonic::Jr,
        Mnemonic::Ld,
        Mnemonic::Ldd,
        Mnemonic::Lddr,
        Mnemonic::Ldi,
        Mnemonic::Ldir,
        Mnemonic::Neg,
        Mnemonic::Nop,
        Mnemonic::Nop2,
        Mnemonic::Or,
        Mnemonic::Otdr,
        Mnemonic::Otir,
        Mnemonic::Out,
        Mnemonic::Outd,
        Mnemonic::Outi,
        Mnemonic::Pop,
        Mnemonic::Push,
        Mnemonic::Res,
        Mnemonic::Ret,
        Mnemonic::Reti,
        Mnemonic::Retn,
        Mnemonic::Rl,
        Mnemonic::Rla,
        Mnemonic::Rlc,
        Mnemonic::Rlca,
        Mnemonic::Rld,
        Mnemonic::Rr,
        Mnemonic::Rra,
        Mnemonic::Rrc,
        Mnemonic::Rrca,
        Mnemonic::Rrd,
        Mnemonic::Rst,
        Mnemonic::Sbc,
        Mnemonic::Scf,
        Mnemonic::Set,
        Mnemonic::Sl1,
        Mnemonic::Sla,
        Mnemonic::Sra,
        Mnemonic::Srl,
        Mnemonic::Srl8,
        Mnemonic::Sub,
        Mnemonic::Xor
    ];

    /// The single most important pair in the whole table: the self-clearing
    /// idiom reads nothing, while the general form reads both operands.
    /// Hand-authoring these semantics is exactly where this would go wrong.
    #[test]
    fn xor_a_reads_nothing_but_xor_b_reads_a_and_b() {
        let xor_a = effects(&op(Mnemonic::Xor, r8(Register8::A), None));
        assert!(xor_a.reads.is_empty(), "{xor_a:?}");
        assert_eq!(xor_a.writes, vec![Reg::A]);
        assert_eq!(xor_a.writes_flags.len(), 6);

        let xor_b = effects(&op(Mnemonic::Xor, r8(Register8::B), None));
        assert_eq!(xor_b.reads, vec![Reg::A, Reg::B]);
        assert_eq!(xor_b.writes, vec![Reg::A]);
    }

    /// Read-modify-write: `A` is both an input and an output, so a liveness
    /// walk must see the read and not treat it as a plain overwrite.
    #[test]
    fn dec_a_both_reads_and_writes_a() {
        let e = effects(&op(Mnemonic::Dec, r8(Register8::A), None));
        assert_eq!(e.reads, vec![Reg::A]);
        assert_eq!(e.writes, vec![Reg::A]);
    }

    /// `DJNZ` tests its counter register, *not* the zero flag - easy to get
    /// backwards, and getting it wrong would let a rule clobber `B`.
    #[test]
    fn djnz_reads_b_and_no_flag_at_all() {
        let e = effects(&op(Mnemonic::Djnz, Some(DataAccess::Expression(0.into())), None));
        assert_eq!(e.reads, vec![Reg::B]);
        assert!(e.reads_flags.is_empty(), "{e:?}");
        assert!(e.writes.contains(&Reg::B));
    }

    /// `POP` takes its value from the stack, so the old register contents are
    /// dead - it must not be reported as reading them.
    #[test]
    fn push_reads_the_pair_but_pop_does_not() {
        let push = effects(&op(Mnemonic::Push, Some(DataAccess::Register16(Register16::Bc)), None));
        assert!(push.reads.contains(&Reg::Bc), "{push:?}");

        let pop = effects(&op(Mnemonic::Pop, Some(DataAccess::Register16(Register16::Bc)), None));
        assert!(!pop.reads.contains(&Reg::Bc), "{pop:?}");
        assert!(pop.writes.contains(&Reg::Bc), "{pop:?}");
    }

    /// A placeholder row (`LD A,r`) must resolve `r` to the operand actually
    /// written, not to some fixed register.
    #[test]
    fn a_placeholder_row_resolves_to_the_real_operand() {
        let e = effects(&op(Mnemonic::Ld, r8(Register8::A), r8(Register8::D)));
        assert_eq!(e.reads, vec![Reg::D], "{e:?}");
        assert_eq!(e.writes, vec![Reg::A], "{e:?}");

        // ...and a different source resolves differently.
        let e = effects(&op(Mnemonic::Ld, r8(Register8::A), r8(Register8::L)));
        assert_eq!(e.reads, vec![Reg::L], "{e:?}");
    }

    /// `LD A,n` has no register input at all - the value is immediate.
    #[test]
    fn an_immediate_load_reads_no_register() {
        let e = effects(&op(
            Mnemonic::Ld,
            r8(Register8::A),
            Some(DataAccess::Expression(5.into()))
        ));
        assert!(e.reads.is_empty(), "{e:?}");
        assert_eq!(e.writes, vec![Reg::A]);
    }

    /// `R` (refresh register) and `r` (any 8-bit register) are different
    /// things that differ only by case - the exact conflation this module's
    /// `is_placeholder` rule exists to prevent.
    #[test]
    fn the_refresh_register_is_not_the_any_register_placeholder() {
        assert!(is_placeholder("r"));
        assert!(!is_placeholder("R"));
        assert!(is_placeholder("IXp"));
        assert!(!is_placeholder("IXH"));

        let e = effects(&op(Mnemonic::Ld, r8(Register8::A), Some(DataAccess::SpecialRegisterR)));
        assert_eq!(e.reads, vec![Reg::R], "{e:?}");
    }

    #[test]
    fn block_operations_report_their_whole_dependency_set() {
        let e = effects(&op(Mnemonic::Ldir, None, None));
        assert!(e.reads.contains(&Reg::Bc), "{e:?}");
        assert!(e.reads.contains(&Reg::De), "{e:?}");
        assert!(e.reads.contains(&Reg::Hl), "{e:?}");
        assert!(e.reads_memory && e.writes_memory, "{e:?}");
    }

    /// basm's breakpoint marker isn't in the table; it must still answer, and
    /// answer "nothing", rather than falling through to the opaque path.
    #[test]
    fn the_winape_breakpoint_marker_behaves_as_a_nop() {
        let e = effects(&op(Mnemonic::Nop2, None, None));
        assert_eq!(e, Effects::none());
        assert_eq!(e, effects(&op(Mnemonic::Nop, None, None)));
    }

    /// Each conditional form reads a *different* flag. Matching "any
    /// condition" instead of the specific one silently reports the wrong flag
    /// as read - which would let a rule clobber a flag a later branch still
    /// depends on. Caught for real by this test during development.
    #[test]
    fn every_condition_family_reads_its_own_flag() {
        for (cond, expected) in [
            (FlagTest::NZ, Flag::Z),
            (FlagTest::Z, Flag::Z),
            (FlagTest::NC, Flag::C),
            (FlagTest::C, Flag::C),
            (FlagTest::PO, Flag::PV),
            (FlagTest::PE, Flag::PV),
            (FlagTest::P, Flag::S),
            (FlagTest::M, Flag::S)
        ] {
            let e = effects(&op(Mnemonic::Ret, Some(DataAccess::FlagTest(cond)), None));
            assert_eq!(e.reads_flags, vec![expected], "RET {cond:?} -> {e:?}");

            let e = effects(&op(
                Mnemonic::Jp,
                Some(DataAccess::FlagTest(cond)),
                Some(DataAccess::Expression(0.into()))
            ));
            assert_eq!(e.reads_flags, vec![expected], "JP {cond:?} -> {e:?}");
        }
    }

    /// `JR` only supports the four simple conditions; `JP` also supports the
    /// parity and sign ones. A `JR` row must never be matched by `PO`/`P`.
    #[test]
    fn jr_supports_fewer_conditions_than_jp() {
        for cond in [FlagTest::NZ, FlagTest::Z, FlagTest::NC, FlagTest::C] {
            let token = op(
                Mnemonic::Jr,
                Some(DataAccess::FlagTest(cond)),
                Some(DataAccess::Expression(0.into()))
            );
            assert!(
                effects_of(&AnalysisOp::Real(&token)).is_some(),
                "JR {cond:?} should have a row"
            );
        }
        for cond in [FlagTest::PO, FlagTest::PE, FlagTest::P, FlagTest::M] {
            let token = op(
                Mnemonic::Jr,
                Some(DataAccess::FlagTest(cond)),
                Some(DataAccess::Expression(0.into()))
            );
            assert!(
                effects_of(&AnalysisOp::Real(&token)).is_none(),
                "JR {cond:?} is not a real instruction and must not match a row"
            );
        }
    }
}
