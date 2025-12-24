// Instructions module - contains instruction parsing functions and constants

use crate::hashed_choice;
use cpclib_common::winnow::ascii::{Caseless, alpha1};
use cpclib_common::winnow::combinator::{
    alt, cut_err, delimited, not, opt, preceded, separated, terminated
};
use cpclib_common::winnow::error::{ErrMode, StrContext};
use cpclib_common::winnow::stream::{Stream, UpdateSlice};
use cpclib_common::winnow::{ModalResult, Parser};
use cpclib_tokens::{DataAccessElem, ExprElement, FlagTest, Mnemonic};

use super::error::*;
use super::obtained::*;
use super::*;

// Instruction mnemonics constant - used to forbid label naming conflicts
pub const INSTRUCTIONS: &[&[u8]] = &[
    b"ADC", b"ADD", b"AND", b"BIT", b"CALL", b"CCF", b"CP", b"CPD", b"CPDR", b"CPI", b"CPIR",
    b"CPL", b"DAA", b"DEC", b"DI", b"DJNZ", b"EI", b"EX", b"EXX", b"HALT", b"IM", b"IN", b"INC",
    b"IND", b"INDR", b"INI", b"INIR", b"JP", b"JR", b"LD", b"LDD", b"LDDR", b"LDI", b"LDIR",
    b"NEG", b"NOP", b"OR", b"OTDR", b"OTIR", b"OUT", b"OUTD", b"OUTI", b"POP", b"PUSH", b"RES",
    b"RET", b"RETI", b"RETN", b"RL", b"RLA", b"RLC", b"RLCA", b"RLD", b"RR", b"RRA", b"RRC",
    b"RRCA", b"RRD", b"RST", b"SBC", b"SCF", b"SET", b"SLA", b"SRA", b"SRL", b"SUB", b"XOR",
    b"SL1", b"SLL", b"EXA", b"EXD"
];

/// Parse any opcode having no argument
pub fn parse_opcode_no_arg(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let cloned = *input;
    let input_start = input.checkpoint();

    let token: LocatedToken = preceded(
        my_space0,
        alpha1.verify_map(|word: &[u8]| {
            match fnv1a_ascii_upper(word) {
                h if hashed_choice!(h, word, b"CCF") => Some(Mnemonic::Ccf),
                h if hashed_choice!(h, word, b"CPD") => Some(Mnemonic::Cpd),
                h if hashed_choice!(h, word, b"CPDR") => Some(Mnemonic::Cpdr),
                h if hashed_choice!(h, word, b"CPI") => Some(Mnemonic::Cpi),
                h if hashed_choice!(h, word, b"CPIR") => Some(Mnemonic::Cpir),
                h if hashed_choice!(h, word, b"CPL") => Some(Mnemonic::Cpl),
                h if hashed_choice!(h, word, b"DAA") => Some(Mnemonic::Daa),
                h if hashed_choice!(h, word, b"DI") => Some(Mnemonic::Di),
                h if hashed_choice!(h, word, b"EI") => Some(Mnemonic::Ei),
                h if hashed_choice!(h, word, b"EXX") => Some(Mnemonic::Exx),
                h if hashed_choice!(h, word, b"HALT") => Some(Mnemonic::Halt),
                h if hashed_choice!(h, word, b"IND") => Some(Mnemonic::Ind),
                h if hashed_choice!(h, word, b"INDR") => Some(Mnemonic::Indr),
                h if hashed_choice!(h, word, b"INI") => Some(Mnemonic::Ini),
                h if hashed_choice!(h, word, b"INIR") => Some(Mnemonic::Inir),
                h if hashed_choice!(h, word, b"LDD") => Some(Mnemonic::Ldd),
                h if hashed_choice!(h, word, b"LDDR") => Some(Mnemonic::Lddr),
                h if hashed_choice!(h, word, b"LDI") => Some(Mnemonic::Ldi),
                h if hashed_choice!(h, word, b"LDIR") => Some(Mnemonic::Ldir),
                h if hashed_choice!(h, word, b"NEG") => Some(Mnemonic::Neg),
                h if hashed_choice!(h, word, b"NOPS2") => Some(Mnemonic::Nop2),
                h if hashed_choice!(h, word, b"OTDR") => Some(Mnemonic::Otdr),
                h if hashed_choice!(h, word, b"OTIR") => Some(Mnemonic::Otir),
                h if hashed_choice!(h, word, b"OUTD") => Some(Mnemonic::Outd),
                h if hashed_choice!(h, word, b"OUTDR") => Some(Mnemonic::Otdr),
                h if hashed_choice!(h, word, b"OUTI") => Some(Mnemonic::Outi),
                h if hashed_choice!(h, word, b"OUTIR") => Some(Mnemonic::Otir),
                h if hashed_choice!(h, word, b"RETI") => Some(Mnemonic::Reti),
                h if hashed_choice!(h, word, b"RETN") => Some(Mnemonic::Retn),
                h if hashed_choice!(h, word, b"RLA") => Some(Mnemonic::Rla),
                h if hashed_choice!(h, word, b"RLCA") => Some(Mnemonic::Rlca),
                h if hashed_choice!(h, word, b"RLD") => Some(Mnemonic::Rld),
                h if hashed_choice!(h, word, b"RRA") => Some(Mnemonic::Rra),
                h if hashed_choice!(h, word, b"RRCA") => Some(Mnemonic::Rrca),
                h if hashed_choice!(h, word, b"RRD") => Some(Mnemonic::Rrd),
                h if hashed_choice!(h, word, b"SCF") => Some(Mnemonic::Scf),
                _ => None,
            }
        })
    )
    .with_taken()
    .map(|(mne, span)| {
        let span = cloned.update_slice(span);
        LocatedTokenInner::OpCode(mne, None, None, None).into_located_token_at(span)
    })
    .parse_next(input)?;

    // http://rasm.wikidot.com/directives:repete
    // Some instructions may have repeated counts, so we modify them
    let token: LocatedToken = match &token.inner.as_ref().left().unwrap() {
        LocatedTokenInner::OpCode(
            Mnemonic::Ldi
            | Mnemonic::Ldd
            | Mnemonic::Rlca
            | Mnemonic::Rrca
            | Mnemonic::Ini
            | Mnemonic::Ind
            | Mnemonic::Outi
            | Mnemonic::Outd
            | Mnemonic::Halt,
            located_data_access,
            located_data_access1,
            register8
        ) => {
            debug_assert!(located_data_access.is_none());
            debug_assert!(located_data_access1.is_none());
            debug_assert!(register8.is_none());

            let repeat = opt(preceded(my_space1, located_expr)).parse_next(input)?;
            if let Some(repeat) = repeat {
                LocatedTokenInner::RepeatToken {
                    token: Box::new(token),
                    repeat
                }
                .into_located_token_between(&input_start, *input)
            }
            else {
                token
            }
        },

        _ => token
    };

    Ok(token)
}

// ============================================================================
// INSTRUCTION PARSING FUNCTIONS (moved from parser.rs)
// ============================================================================

/// Parse ex af, af' instruction
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_ex_af(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    (
        //        parse_word(b"EX"),
        parse_register_af,
        parse_comma,
        parse_word(b"AF'")
    )
        .map(|_| LocatedTokenInner::new_opcode(Mnemonic::ExAf, None, None))
        .parse_next(input)
}

/// Parse ex hl, de instruction
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_ex_hl_de(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    alt((
        (
            //          Caseless("EX"),
            //          space1,
            parse_register_hl,
            parse_comma,
            parse_register_de
        )
            .value(()),
        (
            //            Caseless("EX"),
            //        space1,
            parse_register_de,
            parse_comma,
            parse_register_hl
        )
            .value(())
    ))
    .map(|_| LocatedTokenInner::new_opcode(Mnemonic::ExHlDe, None, None))
    .parse_next(input)
}

/// Parse ex (sp), hl
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_ex_mem_sp(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let destination = (
        //     Caseless("EX"),
        //      space1,
        ('('),
        my_space0,
        parse_register_sp,
        my_space0,
        (')'),
        parse_comma,
        alt((parse_register_hl, parse_indexregister16))
    )
        .parse_next(input)?;

    Ok(LocatedTokenInner::new_opcode(
        Mnemonic::ExMemSp,
        Some(destination.6),
        None
    ))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_ld(
    mnemonic_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        alt((
            parse_ld_fake(mnemonic_name_parsed),
            parse_ld_normal(mnemonic_name_parsed)
        ))
        .parse_next(input)
    }
}

/// Parse artifical LD instruction (would be replaced by several real instructions)
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_ld_fake(
    mnemonic_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        if !mnemonic_name_parsed {
            terminated(parse_word(b"LD"), my_space1).parse_next(input)?;
        }

        let dst = alt((
            terminated(
                alt((parse_register16, parse_indexregister16)),
                not(alt((Caseless(".low"), Caseless(".high"))))
            ),
            parse_hl_address,
            parse_indexregister_with_index
        ))
        .parse_next(input)?;

        let _ = parse_comma(input)?;

        // TODO - add https://z00m128.github.io/sjasmplus/documentation.html#s_fake_instructions

        let src = if dst.is_register_hl() {
            opt(parse_register_sp).parse_next(input)?
        }
        else {
            None
        };

        let src = if let Some(src) = src {
            src
        }
        else if dst.is_register16() {
            alt((
                terminated(
                    alt((parse_register16, parse_indexregister16)),
                    not(alt((Caseless(".low"), Caseless(".high"))))
                ),
                parse_hl_address,
                parse_indexregister_with_index
            ))
            .parse_next(input)?
        }
        else
        // mem-like
        {
            terminated(
                parse_register16,
                not(alt((Caseless(".low"), Caseless(".high"))))
            )
            .parse_next(input)?
        };

        let token = LocatedTokenInner::new_opcode(Mnemonic::Ld, Some(dst), Some(src));

        let warning = LocatedTokenInner::WarningWrapper(
            Box::new(token),
            "This is a fake instruction assembled using several opcodes".into()
        );

        Ok(warning)
    }
}

/// Parse the valids LD versions
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_ld_normal(
    mnemonic_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        if !mnemonic_name_parsed {
            parse_word(b"LD").parse_next(input)?;
        }

        let _start = *input;
        let dst = cut_err(
            alt((
                parse_reg_address,
                parse_indexregister_address,
                parse_indexregister_with_index,
                parse_register_sp,
                terminated(
                    parse_register16,
                    not(alt((Caseless(".low"), Caseless(".high"))))
                ),
                parse_register8,
                parse_indexregister16,
                parse_indexregister8,
                parse_register_i,
                parse_register_r,
                parse_hl_address,
                parse_address
            ))
            .context(StrContext::Label("LD: wrong destination"))
        )
        .parse_next(input)?;

        let _ = cut_err(parse_comma.context(StrContext::Label("LD: missing comma")))
            .parse_next(input)?;

        // src possibilities depend on dst
        let src = cut_err(cut_err(parse_ld_normal_src(&dst)))
            .context(StrContext::Label("LD: wrong source"))
            .parse_next(input)?;

        let token = LocatedTokenInner::new_opcode(Mnemonic::Ld, Some(dst), Some(src));

        Ok(token)
    }
}

/// Parse the source of LD depending on its destination
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
fn parse_ld_normal_src(
    dst: &LocatedDataAccess
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedDataAccess, Z80ParserError> + '_ {
    move |input: &mut InnerZ80Span| {
        let input_start = input.checkpoint();
        if dst.is_register_sp() {
            alt((
                parse_register_hl,
                parse_indexregister16,
                parse_address,
                parse_expr
            ))
            .parse_next(input)
        }
        else if dst.is_address_in_register16() || dst.is_address_in_indexregister16() {
            // by construction is t is HL/IX/IY
            alt((parse_register8, parse_expr)).parse_next(input)
        }
        else if dst.is_register16() | dst.is_indexregister16() {
            alt((parse_address, parse_expr)).parse_next(input)
        }
        else if dst.is_register8() {
            // todo find a way to merge them together
            if dst.is_register_a() {
                alt((
                    parse_indexregister_with_index,
                    parse_reg_address,
                    parse_indexregister_address,
                    parse_address,
                    parse_register8,
                    parse_indexregister8,
                    parse_register_i,
                    parse_register_r,
                    parse_expr
                ))
                .parse_next(input)
            }
            else {
                alt((
                    parse_indexregister_address,
                    parse_indexregister_with_index,
                    parse_hl_address,
                    parse_address,
                    parse_register8,
                    parse_indexregister8.verify(|src| {
                        !((dst.is_register_h() || dst.is_register_l())
                            && (src.is_register_ixl()
                                || src.is_register_ixh()
                                || src.is_register_ixl()
                                || src.is_register_ixh()))
                    }),
                    parse_expr
                ))
                .parse_next(input)
            }
        }
        else if dst.is_indexregister8() {
            alt((
                parse_indexregister_address,
                parse_indexregister_with_index,
                parse_hl_address,
                parse_address,
                parse_register8,
                (alt((parse_register_ixh, parse_register_ixl))
                    .verify(|_| dst.is_register_ixl() || dst.is_register_ixh())),
                (alt((parse_register_iyh, parse_register_iyl))
                    .verify(|_| dst.is_register_iyl() || dst.is_register_iyh())),
                parse_expr
            ))
            .parse_next(input)
        }
        else if dst.is_memory() {
            alt((
                parse_register16,
                parse_register8,
                parse_register_sp,
                parse_indexregister16
            ))
            .parse_next(input)
        }
        else if dst.is_address_in_register16() {
            parse_register8(input)
        }
        else if dst.is_indexregister_with_index() {
            alt((parse_register8, parse_expr)).parse_next(input)
        }
        else if dst.is_register_i() || dst.is_register_r() {
            parse_register_a(input)
        }
        else {
            input.reset(&input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

/// Parse RES, SET and BIT instructions
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_res_set_bit(
    res_or_set: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let bit = cut_err(parse_expr.context(StrContext::Label("Wrong bit definition")))
            .parse_next(input)?;

        let _ = cut_err(parse_comma).parse_next(input)?;

        let operand = cut_err(
            alt((
                parse_register8,
                parse_hl_address,
                parse_indexregister_with_index
            ))
            .context(StrContext::Label("Wrong destination"))
        )
        .parse_next(input)?;

        // Res can copy the result in a reg
        // not bit http://www.z80.info/z80undoc.htm
        let hidden_arg = if res_or_set == Mnemonic::Bit {
            None
        }
        else {
            opt(preceded(parse_comma, parse_register8)).parse_next(input)?
        };

        Ok(LocatedTokenInner::OpCode(
            res_or_set,
            Some(bit),
            Some(operand),
            hidden_arg.map(|d| d.get_register8().unwrap())
        ))
    }
}

/// Parse CP tokens
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_cp(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    //   preceded(
    //    parse_word(b"CP"),

    preceded(
        opt((parse_register_a, parse_comma)),
        cut_err(
            alt((
                parse_register8,
                parse_indexregister8,
                parse_hl_address,
                parse_indexregister_with_index,
                parse_expr
            ))
            .context(StrContext::Label("CP: wrong argument"))
        )
        .map(
            //   )
            |operand| LocatedTokenInner::new_opcode(Mnemonic::Cp, Some(operand), None)
        )
    )
    .parse_next(input)
}

/// ...
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_djnz(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    preceded(opt(parse_comma), parse_expr)
        .map(|expr| LocatedTokenInner::new_opcode(Mnemonic::Djnz, Some(expr), None))
        .parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
/// ...
pub fn parse_logical_operator(
    operator: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        // we optionaly allow a, as a first register
        let operand = preceded(
            opt((parse_register_a, my_space0, parse_comma, my_space0)),
            alt((
                parse_register8,
                parse_indexregister8,
                parse_hl_address,
                parse_indexregister_with_index,
                parse_expr
            ))
        )
        .context(StrContext::Label("Wrong logical operand"))
        .parse_next(input)?;

        Ok(LocatedTokenInner::new_opcode(operator, Some(operand), None))
    }
}

/// Substraction with A register
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_sub(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    //  let _ =Caseless("SUB").parse_next(input)?;
    //  let _ =space1(input)?;

    let _first = opt(terminated(parse_register_a, parse_comma)).parse_next(input)?;

    let operand = alt((
        parse_register8,
        parse_indexregister8,
        parse_hl_address,
        parse_indexregister_with_index,
        parse_expr
    ))
    .parse_next(input)?;

    Ok(LocatedTokenInner::new_opcode(
        Mnemonic::Sub,
        Some(operand),
        None
    ))
}

/// Par se the SBC instruction
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_sbc(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    //  let _ =Caseless("SBC").parse_next(input)?;
    //   let _ =space1(input)?;

    let opera = opt(terminated(
        alt((parse_register_a, parse_register_hl)),
        parse_comma
    ))
    .parse_next(input)?;

    let operb = if opera.as_ref().map(|r| r.is_register_a()).unwrap_or(true) {
        alt((
            parse_register8,
            parse_indexregister8,
            parse_hl_address,
            parse_indexregister_with_index,
            parse_expr
        ))
        .parse_next(input)
    }
    else {
        alt((parse_register16, parse_register_sp)).parse_next(input)
    }?;

    Ok(LocatedTokenInner::new_opcode(
        Mnemonic::Sbc,
        opera,
        Some(operb)
    ))
}

/// Parse ADC and ADD instructions
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_add_or_adc(
    add_or_adc: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let first = opt(terminated(
            alt((parse_register_a, parse_register_hl, parse_indexregister16)),
            parse_comma
        ))
        .parse_next(input)?;

        let second = if first.as_ref().map(|f| f.is_register8()).unwrap_or(true) {
            alt((
                parse_register8,
                parse_indexregister8,
                parse_hl_address,
                parse_indexregister_with_index,
                parse_expr
            ))
            .parse_next(input)
        }
        else if first.as_ref().unwrap().is_register16() {
            alt((parse_register16, parse_register_sp)).parse_next(input) // Case for HL XXX AF is accepted whereas it is not the case in real life
        }
        else if first.as_ref().unwrap().is_indexregister16() {
            alt((
                parse_register_bc,
                parse_register_de,
                parse_register_hl,
                parse_register_sp,
                parse_register_ix.verify(|_| first.as_ref().unwrap().is_register_ix()),
                parse_register_iy.verify(|_| first.as_ref().unwrap().is_register_iy())
            ))
            .parse_next(input)
        }
        else {
            return Err(ErrMode::Cut(Z80ParserError::from_input(input)));
        }?;

        Ok(LocatedTokenInner::new_opcode(
            add_or_adc,
            first,
            Some(second)
        ))
    }
}

/// ...
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_push_n_pop(
    push_or_pop: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let mut registers: Vec<_> = separated(
            1..,
            alt((parse_register16, parse_indexregister16)),
            parse_comma
        )
        .parse_next(input)?;

        if registers.len() > 1 {
            match push_or_pop {
                Mnemonic::Push => Ok(LocatedTokenInner::MultiPush(registers)),
                Mnemonic::Pop => Ok(LocatedTokenInner::MultiPop(registers)),
                _ => unreachable!()
            }
        }
        else {
            let register = registers.remove(0);
            Ok(LocatedTokenInner::new_opcode(
                push_or_pop,
                Some(register),
                None
            ))
        }
    }
}

/// ...
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_ret(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let (cond, cond_bytes) = opt(parse_flag_test).with_taken().parse_next(input)?;

    let token = LocatedTokenInner::new_opcode(
        Mnemonic::Ret,
        cond.map(|cond| {
            LocatedDataAccess::FlagTest(cond, (*input).update_slice(cond_bytes).into())
        }),
        None
    );

    Ok(token)
}

/// ...
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_inc_dec(
    inc_or_dec: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let register = alt((
            parse_register16,
            parse_indexregister16,
            parse_register8,
            parse_indexregister8,
            parse_register_sp,
            parse_hl_address,
            parse_indexregister_with_index
        ))
        .parse_next(input)?;

        Ok(LocatedTokenInner::new_opcode(
            inc_or_dec,
            Some(register),
            None
        ))
    }
}

/// TODO manage other out formats
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_out(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    //  let _ =parse_word(b"OUT").parse_next(input)?;

    // get the port proposal
    let port = alt((parse_portc, parse_portnn)).parse_next(input)?;

    // the vlaue depends on the port
    let cloned = *input;
    let (value, span) = if port.is_port_c() {
        // reg c
        opt(preceded(
            parse_comma,
            alt((
                parse_register8,
                alt((
                    parse_word(b"f").take(),
                    parse_value
                        .verify(|e| e.is_value() && e.value() == 0)
                        .take()
                ))
                .map(|w| {
                    LocatedDataAccess::Expression(LocatedExpr::Value(
                        0,
                        cloned.update_slice(w).into()
                    ))
                })
            ))
        ))
        .with_taken()
        .parse_next(input)?
    }
    else {
        preceded(parse_comma, parse_register_a)
            .map(Some)
            .with_taken()
            .parse_next(input)?
    };

    let cloned = *input;
    let value = value.unwrap_or(LocatedDataAccess::Expression(LocatedExpr::Value(0, {
        cloned.update_slice(span).into()
    })));

    Ok(LocatedTokenInner::new_opcode(
        Mnemonic::Out,
        Some(port),
        Some(value)
    ))
}

/// Parse all the in flavors
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_in(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    // let _ =parse_word(b"IN").parse_next(input)?;
    let cloned = *input;
    // get the port proposal
    let (destination, span) = opt(terminated(
        alt((
            parse_register8,
            alt((Caseless("f").take(), "0")).map(|span| {
                LocatedDataAccess::Expression(LocatedExpr::Value(
                    0,
                    cloned.update_slice(span).into()
                ))
            })
        )),
        parse_comma
    ))
    .with_taken()
    .parse_next(input)?;

    let cloned = *input;
    let destination = destination.unwrap_or(LocatedDataAccess::Expression(LocatedExpr::Value(
        0,
        cloned.update_slice(span).into()
    )));

    let port = cut_err(alt((
        parse_portc,
        parse_portnn.verify(|_| {
            destination
                .get_register8()
                .map(|r| r.is_a())
                .unwrap_or(false)
        })
    )))
    .parse_next(input)?;

    Ok(LocatedTokenInner::new_opcode(
        Mnemonic::In,
        Some(destination),
        Some(port)
    ))
}

/// Parse the rst instruction
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_rst(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    // let _ =parse_word(b"RST").parse_next(input)?;
    let val = parse_expr(input)?;

    Ok(LocatedTokenInner::new_opcode(
        Mnemonic::Rst,
        Some(val),
        None
    ))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_rst_fake(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let (flag, _, val) = (
        parse_flag_test
            .verify(|t| {
                t == &FlagTest::Z || t == &FlagTest::NZ || t == &FlagTest::C || t == &FlagTest::NC
            })
            .with_taken(),
        parse_comma,
        parse_expr
    )
        .parse_next(input)?;

    let flag = {
        let span = (*input).update_slice(flag.1);
        LocatedDataAccess::FlagTest(flag.0, span.into())
    };

    let token = LocatedTokenInner::new_opcode(Mnemonic::Rst, Some(flag), Some(val));
    let warning = LocatedTokenInner::WarningWrapper(
        Box::new(token),
        "This is a fake instruction assembled using several opcodes".into()
    );

    Ok(warning)
}

/// Parse the IM instruction
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_im(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    // let _ =parse_word(b"IM").parse_next(input)?;
    let val = parse_expr(input)?;

    Ok(LocatedTokenInner::new_opcode(Mnemonic::Im, Some(val), None))
}

/// Parse all RLC, RL, RR, SLA, SRA flavors
/// RLC A
/// RLC B
/// RLC C
/// RLC D
/// RLC E
/// RLC H
/// RLC L
/// RLC (HL)
/// RLC (IX+n)
/// RLC (IY+n)
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_shifts_and_rotations(
    oper: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let _start = *input;
        let arg = alt((
            parse_register8,
            parse_hl_address,
            parse_indexregister_with_index
        ))
        .parse_next(input)?;

        // hidden opcodes
        let arg2 = opt(preceded(parse_comma, parse_register8)).parse_next(input)?;

        Ok(LocatedTokenInner::new_opcode(oper, Some(arg), arg2))
    }
}

pub fn parse_shifts_and_rotations_fake(
    oper: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let _start = *input;
        let arg = alt((parse_register16,)).parse_next(input)?;

        let token = LocatedTokenInner::new_opcode(oper, Some(arg), None);
        let warning = LocatedTokenInner::WarningWrapper(
            Box::new(token),
            "This is a fake instruction assembled using several opcodes".into()
        );

        Ok(warning)
    }
}

/// TODO reduce the flag space for jr"],
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_call_jp_or_jr(
    call_jp_or_jr: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let _start = *input;

        let flag_test =
            opt(terminated(parse_flag_test.with_taken(), parse_comma)).parse_next(input)?;

        let dst = cut_err(
            alt((

                    alt((
                        parse_hl_address,
                        parse_indexregister_address,
                        parse_register_hl,
                        parse_indexregister16
                    ))
                .verify(|_| call_jp_or_jr.is_jp() && flag_test.is_none()), // not possible for call and for jp/jr when there is flag
                parse_expr
            ))
            .context(match call_jp_or_jr {
                Mnemonic::Jp => StrContext::Label("JP: wrong parameter"),
                Mnemonic::Jr => StrContext::Label("JR: wrong parameter"),
                Mnemonic::Call => StrContext::Label("CALL: wrong parameter"),
                _ => unreachable!()
            })
        )
        .parse_next(input)?;

        // Allow to parse JP HL as to be JP (HL) original notation is misleading
        let dst = match dst {
            LocatedDataAccess::IndexRegister16(reg, span) => {
                LocatedDataAccess::MemoryIndexRegister16(reg, span)
            },
            LocatedDataAccess::Register16(reg, span) => {
                LocatedDataAccess::MemoryRegister16(reg, span)
            },
            other => other
        };

        let flag_test = flag_test.map(|(f, s)| {
            let span = (*input).update_slice(s);
            LocatedDataAccess::FlagTest(f, span.into())
        });

        Ok(LocatedTokenInner::new_opcode(
            call_jp_or_jr,
            flag_test,
            Some(dst)
        ))
    }
}

pub fn parse_nop(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let val = cut_err(
        opt(located_expr.map(LocatedDataAccess::from)).context(StrContext::Label(
            "Wrong argument. NOP expects an expression"
        ))
    )
    .parse_next(input)?;
    Ok(LocatedTokenInner::OpCode(Mnemonic::Nop, val, None, None))
}

/// Parse (C) used in in/out
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_portc(input: &mut InnerZ80Span) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    let span = alt((
        (b'(', my_space0, parse_register_c, my_space0, b')'),
        (b'[', my_space0, parse_register_c, my_space0, b']')
    ))
    .take()
    .parse_next(input)?;
    let span = (*input).update_slice(span);

    Ok(LocatedDataAccess::PortC(span.into()))
}

/// Parse (nn) used in in/out
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_portnn(input: &mut InnerZ80Span) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    let (address, span) = alt((
        delimited("(", located_expr, preceded(my_space0, ")")),
        delimited("[", located_expr, preceded(my_space0, "]"))
    ))
    .with_taken()
    .parse_next(input)?;
    let span = (*input).update_slice(span);

    Ok(LocatedDataAccess::PortN(address, span.into()))
}
