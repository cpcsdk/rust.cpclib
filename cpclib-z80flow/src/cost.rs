//! What an instruction costs — the one question both timing analyses ask, and
//! the one place that knows an instruction is not always what it looks like.
//!
//! Two shapes of instruction do not cost what a table lookup on their own text
//! says, and both are ordinary in basm source:
//!
//! * **Fake instructions.** `ld hl, de` is not a Z80 opcode; basm assembles it
//!   to `ld h, d` / `ld l, e`. A cost source keyed on the text of what the user
//!   wrote finds nothing, so before this module the whole instruction
//!   contributed **zero** to a cycle count and merely bumped
//!   `unrecognized_count`. The real corpus has 29 of them across 15 files.
//! * **`JQ`**, basm's "assembler picks `JR` or `JP`" form. Also absent from any
//!   opcode table, for the same reason.
//!
//! Both are handled by asking the caller's cost source about the *real* opcodes
//! involved instead of the written form.

use cpclib_tokens::{
    DataAccess, DataAccessElem, Expr, ListingElement, Mnemonic, OperandKind, Register8,
    Register16, Token
};

/// A cost source the algorithm queries once per token - kept fully
/// decoupled from any specific timing-data representation (see
/// [`crate::branch_balance`]'s module doc comment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionCost {
    /// A plain instruction's single cost.
    Fixed(u32),
    /// A conditional `JR`/`JP`'s own two costs. Mirrors `timing::
    /// format_hover`'s "taken/not taken" convention in `cpclib-lsp`.
    Conditional { taken: u32, not_taken: u32 },
    /// The cost source doesn't recognize this instruction.
    Unknown
}

/// Prices instructions.
///
/// Two questions, because they are genuinely different:
///
/// * [`cost`](CostModel::cost) is asked about a token the caller supplied, and
///   may legitimately answer [`InstructionCost::Unknown`] - a macro
///   invocation, a directive, something the caller's own table has no entry
///   for.
/// * [`mnemonic_cost`](CostModel::mnemonic_cost) is asked about a **real Z80
///   opcode**, and there is no such thing as an opcode whose duration is
///   unknowable. It has no default for exactly that reason: a default
///   returning `Unknown` would be a lie, and it silently made every fake
///   instruction cost nothing at all for as long as it existed.
///
pub trait CostModel<T> {
    /// The cost of a token the caller supplied.
    fn cost(&self, token: &T) -> InstructionCost;

    /// The cost of one real opcode: a step of a fake instruction's expansion,
    /// or a candidate a `JQ` might assemble to.
    ///
    /// Defaults to [`opcode_duration`], this workspace's single source of
    /// truth for how long an instruction takes - which is a *real answer*, not
    /// a placeholder. Override it only to price opcodes differently from the
    /// assembler itself, which almost nothing should want to do.
    fn mnemonic_cost(&self, op: &Token) -> InstructionCost {
        let Token::OpCode(mnemonic, arg1, arg2, _) = op
        else {
            return InstructionCost::Unknown;
        };
        match opcode_duration(mnemonic, arg1.as_ref(), arg2.as_ref()) {
            Some(nops) => InstructionCost::Fixed(nops),
            None => InstructionCost::Unknown
        }
    }
}

/// A plain `Fn(&T) -> InstructionCost` is a complete cost model: it answers for
/// the caller's own tokens, and the trait's default answers for real opcodes
/// out of the shared duration rules.
impl<T, F> CostModel<T> for F
where F: Fn(&T) -> InstructionCost
{
    fn cost(&self, token: &T) -> InstructionCost {
        self(token)
    }
}

/// Sum a fake instruction's expansion, or `Unknown` if any part is unknown.
///
/// All-or-nothing on purpose: a partial sum is a number that looks right and
/// is short, which for a cycle-exact routine is worse than admitting the gap.
fn sum_expansion<T, C: CostModel<T>>(
    ops: &[(Mnemonic, Option<DataAccess>, Option<DataAccess>)],
    cost: &C
) -> InstructionCost {
    let mut total = 0;
    for (mnemonic, arg1, arg2) in ops {
        let op = Token::OpCode(*mnemonic, arg1.clone(), arg2.clone(), None);
        match cost.mnemonic_cost(&op) {
            InstructionCost::Fixed(n) => total += n,
            // An expansion is a run of plain opcodes; nothing in one branches,
            // so a conditional cost here means the cost source disagrees with
            // that assumption and the honest answer is that we do not know.
            InstructionCost::Conditional { .. } | InstructionCost::Unknown => {
                return InstructionCost::Unknown;
            }
        }
    }
    InstructionCost::Fixed(total)
}

/// `token`'s real cost, seeing through the two forms that are not opcodes.
///
/// Falls back to `cost.cost(token)` for everything else, so an ordinary
/// instruction takes exactly the path it always did.
pub(crate) fn instruction_cost<T, C>(token: &T, cost: &C) -> InstructionCost
where
    T: ListingElement,
    T::DataAccess: cpclib_tokens::DataAccessElem,
    C: CostModel<T>
{
    let Some(mnemonic) = token.mnemonic().copied()
    else {
        return cost.cost(token);
    };
    let (arg1, arg2, arg3) = (
        token.mnemonic_arg1(),
        token.mnemonic_arg2(),
        token.mnemonic_arg3()
    );

    // A fake instruction costs what it assembles to. `cpclib-tokens` owns both
    // the "is it fake" test and the expansion, so this stays in step with the
    // assembler by construction rather than by a list kept here.
    if T::is_fake_instruction_from_access(mnemonic, arg1, arg2, arg3)
        && let Some(expansion) = T::fake_to_listing_from_access(mnemonic, arg1, arg2, arg3)
    {
        return sum_expansion(&expansion, cost);
    }

    if mnemonic == Mnemonic::Jq {
        return jq_cost(token, cost);
    }

    cost.cost(token)
}

/// What a `JQ` costs, without knowing which instruction basm will pick.
///
/// The trick is that it usually does not matter: on the CPC an unconditional
/// `JR` and `JP` are both 3 NOPs, so the answer is the same either way. Rather
/// than assume that from the timing table, this *asks* for both and only
/// answers when they agree - so if the data ever said otherwise, the result
/// would become `Unknown` instead of quietly wrong.
///
/// A conditional `JQ` genuinely differs (`jr cc` is "3 or 2", `jp cc` is always
/// 3), so it stays unknown: which one it is depends on a distance only a real
/// assemble knows.
fn jq_cost<T, C>(token: &T, cost: &C) -> InstructionCost
where
    T: ListingElement,
    T::DataAccess: cpclib_tokens::DataAccessElem,
    C: CostModel<T>
{
    use cpclib_tokens::DataAccessElem;

    let arg1 = token.mnemonic_arg1().map(|a| a.to_data_access().into_owned());
    let arg2 = token.mnemonic_arg2().map(|a| a.to_data_access().into_owned());

    let conditional = arg1.as_ref().is_some_and(DataAccess::is_flag_test);
    if conditional {
        return InstructionCost::Unknown;
    }

    let as_op = |mnemonic| Token::OpCode(mnemonic, arg1.clone(), arg2.clone(), None);
    let as_jr = cost.mnemonic_cost(&as_op(Mnemonic::Jr));
    let as_jp = cost.mnemonic_cost(&as_op(Mnemonic::Jp));
    if as_jr == as_jp {
        as_jr
    }
    else {
        InstructionCost::Unknown
    }
}


/// How many NOPs one real Z80 opcode takes on a CPC.
///
/// **The single source of truth for instruction duration in this workspace.**
/// `cpclib-asm`'s `TokenExt::estimated_duration` - which answers basm's own
/// `duration()` operator for user source - delegates here, and so does every
/// [`CostModel`] that has nothing better. Two independent statements of the
/// same fact can drift; one cannot.
///
/// `None` rather than a panic for an operand shape it has no entry for. The
/// code this grew from panicked (`"Duration not set for ..."`, 35 such arms),
/// which was survivable inside an assembler that had already validated its
/// input and is not survivable inside an editor asking about half-typed code.
/// Every caller already had to handle "unknown", so nothing lost a guarantee.
///
/// Reports a *single* duration, so a conditional instruction's two states are
/// not distinguished here - the answer is the one this table happens to carry.
/// A caller needing taken/not-taken (the LSP's cycle counter does) reads it
/// from `data/timings.txt` instead; the two agree wherever both have an entry,
/// checked over 22 common instructions and every fake-instruction expansion in
/// the real corpus.
pub fn opcode_duration<D: DataAccessElem>(
    mnemonic: &Mnemonic,
    arg1: Option<&D>,
    arg2: Option<&D>
) -> Option<u32> {
    // Classified once, then matched exactly as the concrete rules were
    // written - so this is a change of *what* is matched, never of the values.
    let arg1 = arg1.map(DataAccessElem::kind);
    let arg2 = arg2.map(DataAccessElem::kind);
    let duration = match mnemonic {
    &Mnemonic::Add => {
        match arg1 {
            None | Some(OperandKind::Reg8(_)) => {
                match arg2 {
                    Some(OperandKind::Reg8(_)) => 1,
                    Some(OperandKind::Indexed) => 5,
                    _ => 2
                }
            },
            Some(OperandKind::Reg16(_)) => 3,
            Some(OperandKind::IndexReg16(_)) => 4,
            _ => return None
        }
    },

    &Mnemonic::Adc => {
        match arg1 {
            Some(OperandKind::Reg8(_)) => {
                match arg2 {
                    Some(OperandKind::Reg8(_)) => 1,
                    Some(OperandKind::Indexed) => 5,
                    _ => 2
                }
            },
            Some(OperandKind::Reg16(Register16::Hl)) => 4,
            _ => return None
        }
    },

    // `arg1` is the optional explicit `A,` prefix and never
    // affects the encoded size; `arg2` is the real operand.
    &Mnemonic::And | &Mnemonic::Or | &Mnemonic::Xor => {
        match arg2.as_ref().or(arg1.as_ref()) {
            Some(OperandKind::Reg8(_)) => 1,
            Some(OperandKind::IndexReg8) => 2,
            Some(OperandKind::Expression) => 2,
            Some(OperandKind::MemReg16(_)) => 2,
            Some(OperandKind::Indexed) => 5,
            _ => return None
        }
    },

    Mnemonic::Call => {
        match (arg1, arg2) {
            (Some(OperandKind::FlagTest), Some(OperandKind::Expression)) => 3,
            (None, Some(OperandKind::Expression)) => 5,
            _ => return None
        }
    },

    // `arg1` is the optional explicit `A,` prefix (`CP A,r`
    // vs bare `CP r` - see `parse_cp` in cpclib-asm's
    // parser) and never affects the encoded size; `arg2` is
    // the compared value that does. `parse_cp` always
    // populates `arg2`, but fall back to `arg1` defensively
    // rather than panicking if some other construction path
    // ever produces the pre-fix shape (compared value in
    // `arg1`, `arg2` empty).
    &Mnemonic::Cp => {
        match arg2.as_ref().or(arg1.as_ref()) {
            Some(OperandKind::Reg8(_)) => 1,
            Some(OperandKind::IndexReg8) => 2,
            Some(OperandKind::Expression) => 2,
            Some(OperandKind::MemReg16(Register16::Hl)) => 2,
            Some(OperandKind::Indexed) => 5,
            _ => return None
        }
    },

    // XXX Not stable timing
    &Mnemonic::Djnz => 3, // or 4

    &Mnemonic::ExAf => 1,

    &Mnemonic::Inc | &Mnemonic::Dec => {
        match arg1 {
            Some(OperandKind::Reg8(_)) => 1,
            Some(OperandKind::Reg16(_)) => 2,
            Some(OperandKind::IndexReg16(_)) => 3,
            Some(OperandKind::MemReg16(Register16::Hl)) => 3,
            Some(OperandKind::Indexed) => 6,
            _ => return None
        }
    },

    &Mnemonic::Jp => {
        match arg1 {
            None => {
                match arg2 {
                    Some(OperandKind::Expression) => 3,
                    Some(OperandKind::MemReg16(Register16::Hl)) => 1,
                    Some(OperandKind::MemIndexReg16) => 2,
                    _ => {
                        return None
                    }
                }
            },

            Some(OperandKind::FlagTest) => {
                match arg2 {
                    Some(OperandKind::Expression) => 3,
                    _ => {
                        return None
                    }
                }
            },

            _ => return None
        }
    },

    // Always give the fastest
    &Mnemonic::Jr => {
        match arg1 {
            None => {
                match arg2 {
                    Some(OperandKind::Expression) => 3,
                    _ => {
                        return None
                    }
                }
            },

            Some(OperandKind::FlagTest) => {
                match arg2 {
                    Some(OperandKind::Expression) => 2, // or 3
                    _ => {
                        return None
                    }
                }
            },

            _ => return None
        }
    },

    &Mnemonic::Ld => {
        match arg1 {
            // Dest in memory pointed by register
            Some(OperandKind::MemReg16(_)) => {
                match arg2 {
                    Some(OperandKind::Reg8(_)) => 2,
                    Some(OperandKind::Expression) => 3, // XXX Valid only for HL
                    _ => {
                        return None
                    }
                }
            },

            // Dest in indexed memory pointed by IX/IY + displacement
            Some(OperandKind::Indexed) => {
                match arg2 {
                    Some(OperandKind::Reg8(_)) => 5,
                    Some(OperandKind::Expression) => 5,
                    _ => {
                        return None
                    }
                }
            },

            // Dest in 8bits reg
            Some(OperandKind::Reg8(_dst)) => {
                match arg2 {
                    Some(OperandKind::Reg8(_)) => 1,
                    Some(OperandKind::MemReg16(Register16::Hl)) => 2,
                    Some(OperandKind::MemReg16(Register16::Bc)) => 2,
                    Some(OperandKind::MemReg16(Register16::De)) => 2,
                    Some(OperandKind::SpecialI) => 3,
                    Some(OperandKind::SpecialR) => 3,
                    Some(OperandKind::Expression) => 2,
                    Some(OperandKind::Memory) => 4,
                    Some(OperandKind::Indexed) => 5,
                    _ => {
                        return None
                    }
                }
            },

            // Dest in 16bits reg
            Some(OperandKind::Reg16(dst)) => {
                match arg2 {
                    Some(OperandKind::Expression) => 3,
                    Some(OperandKind::Reg16(Register16::Hl))
                        if dst == Register16::Sp =>
                    {
                        2
                    },
                    Some(OperandKind::IndexReg16(_))
                        if dst == Register16::Sp =>
                    {
                        3
                    },
                    Some(OperandKind::Memory) if dst == Register16::Hl => 5,
                    Some(OperandKind::Memory) => 6,
                    _ => {
                        return None
                    }
                }
            },

            Some(OperandKind::IndexReg16(_)) => {
                match arg2 {
                    Some(OperandKind::Expression) => 4,
                    Some(OperandKind::Memory) => 6,
                    _ => {
                        return None
                    }
                }
            },

            Some(OperandKind::Memory) => {
                match arg2 {
                    Some(OperandKind::Reg8(Register8::A)) => 4,
                    Some(OperandKind::Reg16(Register16::Hl)) => 5,
                    Some(OperandKind::Reg16(_)) => 6,
                    Some(OperandKind::IndexReg16(_)) => 6,
                    _ => {
                        return None
                    }
                }
            },

            // IndexRegister8 destination (IXH/IXL/IYH/IYL)
            Some(OperandKind::IndexReg8) => {
                match arg2 {
                    Some(OperandKind::Reg8(_)) => 2,      // DD + opcode
                    Some(OperandKind::IndexReg8) => 2, // DD + opcode
                    Some(OperandKind::Expression) => 3,     // DD + opcode + imm
                    _ => {
                        return None
                    }
                }
            },

            Some(OperandKind::SpecialI)
            | Some(OperandKind::SpecialR) => {
                match arg2 {
                    Some(OperandKind::Reg8(Register8::A)) => 3,
                    _ => {
                        return None
                    }
                }
            },

            _ => return None
        }
    },

    &Mnemonic::Ldi | &Mnemonic::Ldd => 5,

    &Mnemonic::Exx
    | &Mnemonic::Di
    | &Mnemonic::Ei
    | &Mnemonic::ExHlDe
    | &Mnemonic::Cpl
    | &Mnemonic::Ccf
    | &Mnemonic::Scf
    | &Mnemonic::Rlca
    | &Mnemonic::Rrca
    | &Mnemonic::Rla
    | &Mnemonic::Rra
    | &Mnemonic::Halt
    | &Mnemonic::Nop => {
        // `NOP n` (basm's repeated-nop form) needs real
        // expression evaluation, which this crate has no
        // access to - `cpclib-asm` handles that case before
        // delegating here. A bare `nop` is 1.
        1
    },

    &Mnemonic::Daa => 1,
    &Mnemonic::Neg | &Mnemonic::Im => 2,
    &Mnemonic::Rld | &Mnemonic::Rrd => 5,
    &Mnemonic::Cpi | &Mnemonic::Cpd => 4,
    &Mnemonic::Cpir | &Mnemonic::Cpdr => 4,
    &Mnemonic::Ldir | &Mnemonic::Lddr => 5,

    &Mnemonic::ExMemSp => {
        match arg1 {
            Some(OperandKind::Reg16(Register16::Hl)) => 6,
            Some(OperandKind::IndexReg16(_)) => 7,
            _ => return None
        }
    },

    &Mnemonic::Nop2 => 2,

    &Mnemonic::In => {
        match (arg1, arg2) {
            (
                Some(OperandKind::Reg8(Register8::A)),
                Some(OperandKind::PortN)
            ) => 3,
            (Some(OperandKind::Reg8(_)), Some(OperandKind::PortC)) => 4,
            (Some(OperandKind::Expression), Some(OperandKind::PortC)) => 4,
            _ => return None
        }
    },

    Mnemonic::Ini | Mnemonic::Ind => 5,
    Mnemonic::Inir | Mnemonic::Indr => 5,

    &Mnemonic::Out => {
        match arg1 {
            Some(OperandKind::PortC) => 4, // XXX Not sure for out (c), 0
            Some(OperandKind::Expression) => 3,
            Some(OperandKind::PortN) => 3,
            _ => return None
        }
    },

    Mnemonic::Outi | Mnemonic::Outd => 5,
    Mnemonic::Otir | Mnemonic::Otdr => 5,

    &Mnemonic::Pop => {
        match arg1 {
            Some(OperandKind::Reg16(_)) => 3,
            Some(OperandKind::IndexReg16(_)) => 4,
            _ => return None
        }
    },

    &Mnemonic::Push => {
        match arg1 {
            Some(OperandKind::Reg16(_)) => 4,
            Some(OperandKind::IndexReg16(_)) => 5,
            _ => return None
        }
    },

    &Mnemonic::Bit => {
        match arg2 {
            Some(OperandKind::Reg8(_)) => 2,
            Some(OperandKind::MemReg16(_)) => 3,
            Some(OperandKind::Indexed) => 6,
            _ => return None
        }
    },

    &Mnemonic::Res | &Mnemonic::Set => {
        match arg2 {
            Some(OperandKind::Reg8(_)) => 2,
            Some(OperandKind::MemReg16(_)) => 4, // XXX only HL
            Some(OperandKind::Indexed) => 7,
            _ => return None
        }
    },

    &Mnemonic::Ret => {
        match arg1 {
            None => 3,
            Some(OperandKind::FlagTest) => 2,
            _ => return None
        }
    },

    &Mnemonic::Reti | &Mnemonic::Retn => 4,

    &Mnemonic::Rst => {
        match arg1 {
            Some(OperandKind::Expression) => 4,
            _ => return None
        }
    },

    &Mnemonic::Sbc => {
        match arg1 {
            Some(OperandKind::Reg8(_)) => {
                match arg2 {
                    Some(OperandKind::Reg8(_)) => 1,
                    Some(OperandKind::Indexed) => 5,
                    _ => 2
                }
            },
            Some(OperandKind::Reg16(Register16::Hl)) => 4,
            _ => return None
        }
    },

    // `arg1` is the optional explicit `A,` prefix for the
    // normal 8-bit form (never affects encoding) - `arg2`
    // is the real operand. The fake 16-bit form (`SUB
    // DE,rr`/`SUB HL,rr`) isn't handled by this table at
    // all (pre-existing gap, unchanged by this fix - it
    // would already panic here via the `_` arm).
    &Mnemonic::Sub => {
        match arg2.as_ref().or(arg1.as_ref()) {
            Some(OperandKind::Reg8(_)) => 1,
            Some(OperandKind::IndexReg8) => 2,
            Some(OperandKind::Expression) => 2,
            Some(OperandKind::MemReg16(Register16::Hl)) => 2,
            Some(OperandKind::Indexed) => 5,
            _ => return None
        }
    },

    &Mnemonic::Rlc
    | &Mnemonic::Rrc
    | &Mnemonic::Rl
    | &Mnemonic::Rr
    | &Mnemonic::Sla
    | &Mnemonic::Sra
    | &Mnemonic::Sl1
    | &Mnemonic::Srl => {
        match arg1 {
            Some(OperandKind::Reg8(_)) => 2,
            Some(OperandKind::MemReg16(_)) => 4,
            Some(OperandKind::Indexed) => 7,
            _ => return None
        }
    },

    // SRL8 rr: LD low,high : LD high,0
    // A fake instruction in its own right: costs the two real `LD`s it
    // becomes. Recurses into this same function rather than reaching back up
    // to `estimated_duration`, which is what used to sit here and is now the
    // caller.
    // A fake instruction in its own right: it becomes `ld low, high` then
    // `ld high, 0`. Priced by recursing with concrete operands - a different
    // instantiation of this same generic function, so the rules stay in one
    // place.
    &Mnemonic::Srl8 => {
        let (low, high) = match arg1 {
            Some(OperandKind::Reg16(reg)) => {
                (
                    DataAccess::Register8(reg.low()?),
                    DataAccess::Register8(reg.high()?)
                )
            },
            Some(OperandKind::IndexReg16(reg)) => {
                (
                    DataAccess::IndexRegister8(reg.low()),
                    DataAccess::IndexRegister8(reg.high())
                )
            },
            _ => return None
        };
        let zero = DataAccess::Expression(Expr::Value(0));
        opcode_duration(&Mnemonic::Ld, Some(&low), Some(&high))?
            + opcode_duration(&Mnemonic::Ld, Some(&high), Some(&zero))?
    },

    _ => {
        return None
    }
};
    Some(duration as u32)
}
