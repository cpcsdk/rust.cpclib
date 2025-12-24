// Directives module - contains directive-related constants and parsing functions
use std::borrow::Cow;
use std::cell::RefCell;
use std::ops::DerefMut;
use std::rc::Rc;
use std::str::FromStr;

use cpclib_common::itertools::Itertools;
use cpclib_common::smol_str::SmolStr;
use cpclib_common::winnow::ascii::{Caseless, alphanumeric1, line_ending};
use cpclib_common::winnow::combinator::{
    alt, cut_err, delimited, eof, not, opt, peek, preceded, repeat, repeat_till, separated,
    terminated
};
use cpclib_common::winnow::error::{AddContext, ErrMode, StrContext};
use cpclib_common::winnow::stream::{
    AsBStr, Checkpoint, LocatingSlice, Offset, Stateful, Stream, UpdateSlice
};
use cpclib_common::winnow::token::{none_of, one_of, take, take_till, take_while};
use cpclib_common::winnow::{BStr, ModalResult, Parser};
use cpclib_sna::flags::SnapshotFlag;
use cpclib_sna::{
    RemuBreakPointAccessMode, RemuBreakPointRunMode, RemuBreakPointType, SnapshotVersion
};
use cpclib_tokens::macro_segment::tokenize_macro_body;
use cpclib_tokens::{Expr, ExprFormat, FormattedExpr};

use super::common::{
    inner_code, inner_code_with_state, my_line_ending, my_many0_nocollect, my_space0, my_space1,
    parse_argname_and_value, parse_comma, parse_comment, parse_convertible_word,
    parse_optional_argname_and_value, parse_word
};
use super::context;
use super::expression::{
    expr, expr_list, ignore_ascii_case_allowed_label, located_expr, parse_any_function_call,
    parse_assemble, parse_expr_bracketed_list, parse_flag_value_inner, parse_fname, parse_label,
    parse_string
};
use super::instructions::{parse_nop, parse_opcode_no_arg};
use super::obtained::{LocatedToken, LocatedTokenInner};
use super::orgams::parse_orgams_fail;
use crate::hashed_choice;
use crate::preamble::*;

pub fn parse_while(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let _ = my_space0(input)?;
    let while_start = input.checkpoint();
    let _ = parse_directive_word(b"WHILE").parse_next(input)?;

    let cond = cut_err(located_expr.context(StrContext::Label("WHILE: error in condition")))
        .parse_next(input)?;

    // we must have either a new line or :
    alt((
        delimited(my_space0, ':', my_space0).value(()),
        preceded(my_space0, line_ending).value(())
    ))
    .parse_next(input)?;

    let inner = cut_err(inner_code.context(StrContext::Label("WHILE: issue in the content")))
        .parse_next(input)?;
    let _ = cut_err(
        preceded(
            my_space0,
            alt((
                parse_directive_word(b"ENDWHILE"),
                parse_directive_word(b"ENDW"),
                parse_directive_word(b"WEND")
            ))
        )
        .context(StrContext::Label("WHILE: not closed"))
    )
    .parse_next(input)?;

    let token =
        LocatedTokenInner::While(cond, inner).into_located_token_between(&while_start, *input);
    Ok(token)
}

pub fn parse_switch(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    my_many0_nocollect(alt((my_space1.value(()), my_line_ending.value(())))).parse_next(input)?;
    let switch_start = *input;
    let _ = parse_directive_word(b"SWITCH")(input)?;

    let value = cut_err(
        preceded(my_space0, located_expr).context(StrContext::Label("SWITCH: tested value"))
    )
    .parse_next(input)?;

    let mut cases_listing = Vec::with_capacity(4);
    let mut default_listing = None;

    loop {
        cut_err(
            repeat::<_, _, (), _, _>(
                0..,
                alt((
                    my_space1.value(()),
                    line_ending.value(()),
                    ':'.value(()),
                    parse_comment.value(())
                ))
            )
            .context(StrContext::Label("SWITCH: whitespace error"))
        )
        .parse_next(input)?;

        // after default it is mandatory to end the block
        let endswitch = if default_listing.is_some() {
            cut_err(
                preceded(
                    my_space0,
                    alt((
                        parse_directive_word(b"ENDS"),
                        parse_directive_word(b"ENDSWITCH")
                    ))
                    .value(true)
                )
                .context(StrContext::Label(
                    "SWITCH: endswitch not present after default listing."
                ))
            )
            .parse_next(input)?
        }
        else {
            preceded(
                my_space0,
                opt(alt((
                    parse_directive_word(b"ENDS"),
                    parse_directive_word(b"ENDSWITCH")
                )))
                .map(|e| e.is_some())
            )
            .parse_next(input)?
        };
        if endswitch {
            let token = LocatedTokenInner::Switch(value, cases_listing, default_listing)
                .into_located_token_between(&switch_start.checkpoint(), *input);
            return Ok(token);
        }

        let value = preceded(my_space0, opt(parse_directive_word(b"CASE"))).parse_next(input)?;
        if value.is_some() {
            let value = cut_err(
                delimited(my_space0, located_expr, opt(':'))
                    .context(StrContext::Label("SWITCH: case value error."))
            )
            .parse_next(input)?;

            let inner =
                cut_err(inner_code.context(StrContext::Label("SWITCH: error in case code")))
                    .parse_next(input)?;

            let do_break =
                opt(preceded(my_space0, parse_directive_word(b"BREAK"))).parse_next(input)?;

            cases_listing.push((value, inner, do_break.is_some()));
        }
        else {
            let _ = cut_err(
                delimited(
                    my_space0,
                    parse_directive_word(b"DEFAULT"),
                    opt((my_space0, ':'))
                )
                .context(StrContext::Label(
                    "Only CASE, DEFAULT or ENDSWITCH are expected."
                ))
            )
            .parse_next(input)?;
            let default =
                cut_err(inner_code.context(StrContext::Label("SWITCH: error in default case")))
                    .parse_next(input)?;
            default_listing = Some(default);
        }
    }
}

pub fn parse_for(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let for_start = input.checkpoint();
    let _ = preceded(my_space0, parse_directive_word(b"FOR")).parse_next(input)?;

    // Get parameters
    let counter = cut_err(parse_label(false)).parse_next(input)?;
    let start = cut_err(preceded(parse_comma, located_expr)).parse_next(input)?;
    let stop = cut_err(preceded(parse_comma, located_expr)).parse_next(input)?;
    let step = opt(preceded(parse_comma, located_expr)).parse_next(input)?;

    // Get loop content
    let inner = cut_err(inner_code.context(StrContext::Label("FOR: issue in the content")))
        .parse_next(input)?;

    // Collect end of loop
    let _ = cut_err(
        preceded(
            my_space0,
            alt((
                parse_directive_word(b"ENDFOR"),
                parse_directive_word(b"FEND"),
                parse_directive_word(b"ENDF")
            ))
        )
        .context(StrContext::Label("FOR: not closed"))
    )
    .parse_next(input)?;

    let token = LocatedTokenInner::For {
        label: counter.into(),
        start,
        stop,
        step,
        listing: inner
    }
    .into_located_token_between(&for_start, *input);
    Ok(token)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_confined(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    // let _ = my_space0(input)?;
    let confined_start = input.checkpoint();

    let _ = parse_directive_word(b"CONFINED").parse_next(input)?;

    let inner = cut_err(inner_code.context(StrContext::Label("CONFINED: issue in the content")))
        .parse_next(input)?;

    let _ = cut_err(
        preceded(
            my_space0,
            alt((
                parse_directive_word(b"ENDCONFINED"),
                parse_directive_word(b"CEND"),
                parse_directive_word(b"ENDC")
            ))
        )
        .context(StrContext::Label("CONFINED: not closed"))
    )
    .parse_next(input)?;

    let token =
        LocatedTokenInner::Confined(inner).into_located_token_between(&confined_start, *input);
    Ok(token)
}

pub fn parse_repeat(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let repeat_start = input.checkpoint();
    let _ = preceded(
        my_space0,
        alt((
            parse_directive_word(b"REP"),
            parse_directive_word(b"REPT"),
            parse_directive_word(b"REPEAT")
        ))
    )
    .parse_next(input)?;

    let count = opt(located_expr).parse_next(input)?;
    match count {
        Some(count) => {
            let counter = cut_err(
                opt(preceded(parse_comma, parse_label(false)))
                    .context(StrContext::Label("REPEAT: issue in the counter"))
            )
            .parse_next(input)?;
            let counter_start = opt(preceded(parse_comma, located_expr)).parse_next(input)?;
            let counter_step = opt(preceded(parse_comma, located_expr)).parse_next(input)?;

            let inner =
                cut_err(inner_code.context(StrContext::Label("REPEAT: issue in the content")))
                    .parse_next(input)?;

            let _ = cut_err(
                preceded(
                    my_space0,
                    alt((
                        parse_directive_word(b"ENDREPEAT"),
                        parse_directive_word(b"ENDREPT"),
                        parse_directive_word(b"ENDREP"),
                        parse_directive_word(b"ENDR"),
                        parse_directive_word(b"REND")
                    ))
                )
                .context(StrContext::Label("REPEAT: not closed"))
            )
            .parse_next(input)?;

            let token = LocatedTokenInner::Repeat(
                count,
                inner,
                counter.map(|c| c.into()),
                counter_start,
                counter_step
            )
            .into_located_token_between(&repeat_start, *input);
            Ok(token)
        },

        None => {
            let inner =
                cut_err(inner_code.context(StrContext::Label("REPEAT: issue in the content")))
                    .parse_next(input)?;

            let _ = cut_err(
                delimited(my_space0, parse_directive_word(b"UNTIL"), my_space0)
                    .context(StrContext::Label("REPEAT ... UNTIL: not closed"))
            )
            .parse_next(input)?;
            let cond =
                cut_err(located_expr.context(StrContext::Label("REPEAT UNTIL: condition error")))
                    .parse_next(input)?;
            let token = LocatedTokenInner::RepeatUntil(cond, inner)
                .into_located_token_between(&repeat_start, *input);
            Ok(token)
        }
    }
}

pub fn parse_iterate(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let iterate_start = input.checkpoint();
    let _ = preceded(
        my_space0,
        alt((
            parse_directive_word(b"ITERATE"),
            parse_directive_word(b"ITER")
        ))
    )
    .parse_next(input)?;

    let counter = cut_err(
        preceded(my_space0, parse_label(false))
            .context(StrContext::Label("ITERATE: issue in the counter"))
    )
    .parse_next(input)?;

    let comma_or_in = cut_err(
        preceded(my_space0, alt((parse_word(b"IN"), parse_comma)))
            .context(StrContext::Label("ITERATE: expected ',' or 'in'"))
    )
    .parse_next(input)?;

    let values = if comma_or_in.contains(&b',') {
        let values = cut_err(expr_list.context(StrContext::Label("ITERATE: values issue")))
            .parse_next(input)?;
        either::Either::Left(values)
    }
    else {
        let values = cut_err(
            alt((
                parse_expr_bracketed_list,
                parse_any_function_call,
                parse_assemble,
                parse_label(false).map(|l| LocatedExpr::Label(l.into()))
            ))
            .context(StrContext::Label("ITERATE: list issue"))
        )
        .parse_next(input)?;
        either::Either::Right(values)
    };

    let inner = cut_err(inner_code.context(StrContext::Label("ITERATE: issue in the content")))
        .parse_next(input)?;

    let _ = cut_err(
        (
            my_space0,
            alt((
                parse_directive_word(b"ENDITERATE"),
                parse_directive_word(b"ENDITER"),
                parse_directive_word(b"ENDI"),
                parse_directive_word(b"IEND")
            )),
            my_space0
        )
            .context(StrContext::Label("ITERATE: not closed"))
    )
    .parse_next(input)?;

    let token = LocatedTokenInner::Iterate(counter.into(), values, inner)
        .into_located_token_between(&iterate_start, *input);
    Ok(token)
}

/// TODO
pub fn parse_basic(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let basic_start = input.checkpoint();
    let _ = (my_space0, Caseless("LOCOMOTIVE"), my_space0).parse_next(input)?;

    // collect the labels that are spread to the basic environnement
    let args: Option<Vec<InnerZ80Span>> = opt(separated(
        1..,
        preceded(my_space0, parse_label(false)),
        parse_comma
    ))
    .parse_next(input)?;
    let args = args.map(|args| args.into_iter().map(Z80Span::from).collect_vec());

    (my_space0, opt(line_ending)).parse_next(input)?;

    let hidden_lines = opt(terminated(
        preceded(my_space0, parse_basic_hide_lines),
        my_space0
    ))
    .parse_next(input)?;

    (my_space0, opt(line_ending)).parse_next(input)?;

    // TODO factorize with the the code of parse_macro
    let before_content = input.checkpoint();
    let (_, end) = cut_err(
        repeat_till::<_, _, (), _, _, _, _>(
            0..,
            take(1usize),
            parse_directive_word(b"ENDLOCOMOTIVE")
        )
        .context(StrContext::Label(
            "BASIC: impossible to collect BASIC content"
        ))
    )
    .parse_next(input)?;

    let content_length = end.offset_from(&before_content);
    let mut content = *input;
    content.reset(&before_content);
    let content: &BStr = unsafe { std::mem::transmute(&content.as_bstr()[..content_length]) };
    let basic = (*input).update_slice(content); // TODO find a way to improve that part. I'd like to not make the conversion

    let _ = my_space0.parse_next(input)?;

    let token = LocatedTokenInner::Basic(args, hidden_lines, basic.into())
        .into_located_token_between(&basic_start, *input);
    Ok(token)
}

/// Parse the instruction to hide basic lines
pub fn parse_basic_hide_lines(
    input: &mut InnerZ80Span
) -> ModalResult<Vec<LocatedExpr>, Z80ParserError> {
    let _ = (Caseless("HIDE_LINES"), my_space1).parse_next(input)?;
    expr_list.parse_next(input)
}

pub fn parse_include(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let once_fname = (
        opt(delimited(my_space0, parse_word(b"ONCE"), my_space0)),
        cut_err(parse_fname.context(StrContext::Label("INCLUDE: error in fname")))
    )
        .parse_next(input)?;

    let (once, fname) = once_fname;

    let namespace = opt(preceded(
        delimited(
            my_space0,
            alt((Caseless("namespace"), Caseless("module"), Caseless("as"))),
            my_space0
        ),
        delimited(
            '"',
            parse_label(false),
            '"' // TODO modify to accept only labels without dot
        )
    ))
    .parse_next(input)?;

    Ok(LocatedTokenInner::Include(
        fname,
        namespace.map(|n| n.into()),
        once.is_some()
    ))
}

pub fn parse_incbin(
    transformation: BinaryTransformation
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let fname = preceded(my_space0, parse_fname).parse_next(input)?;

        let offset =
            opt(preceded((my_space0, (','), my_space0), located_expr)).parse_next(input)?;
        let length =
            opt(preceded((my_space0, (','), my_space0), located_expr)).parse_next(input)?;
        let _extended_offset =
            opt(preceded((my_space0, (','), my_space0), expr)).parse_next(input)?;
        let off =
            opt(preceded((my_space0, (','), my_space0), Caseless("OFF"))).parse_next(input)?;

        Ok(LocatedTokenInner::Incbin {
            fname,
            offset,
            length,
            extended_offset: None,
            off: off.is_some(),
            transformation
        })
    }
}

pub fn parse_save(
    save_kind: SaveKind
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        if save_kind == SaveKind::WriteDirect {
            (parse_word(b"DIRECT"), not((my_space0, "-1"))).parse_next(input)?;
        }
        else {
            not((parse_word(b"DIRECT"), my_space0, "-1")).parse_next(input)?;
        }

        let filename = located_expr.parse_next(input)?;

        let address = opt(preceded(parse_comma, opt(located_expr))).parse_next(input)?;

        let size = if address.is_some() {
            opt(preceded(parse_comma, opt(located_expr))).parse_next(input)?
        }
        else {
            None
        };

        let save_type = if size.is_some() && save_kind == SaveKind::Save {
            opt(preceded(
                parse_comma,
                alt((
                    parse_word(b"AMSDOS").value(SaveType::AmsdosBin),
                    parse_word(b"BASIC").value(SaveType::AmsdosBas),
                    parse_word(b"ASCII").value(SaveType::Ascii),
                    parse_word(b"DSK").value(SaveType::Disc(DiscType::Dsk)),
                    parse_word(b"HFE").value(SaveType::Disc(DiscType::Hfe)),
                    parse_word(b"DISC").value(SaveType::Disc(DiscType::Auto)),
                    parse_word(b"TAPE").value(SaveType::Tape)
                ))
            ))
            .parse_next(input)?
        }
        else if save_kind == SaveKind::WriteDirect {
            Some(SaveType::AmsdosBin)
        }
        else {
            None
        };

        let dsk_filename = if save_type.is_some() && save_kind == SaveKind::Save {
            opt(preceded(parse_comma, parse_fname)).parse_next(input)?
        }
        else {
            None
        };

        let side = if dsk_filename.is_some() && save_kind == SaveKind::Save {
            opt(preceded(parse_comma, located_expr)).parse_next(input)?
        }
        else {
            None
        };

        Ok(LocatedTokenInner::Save {
            filename,
            address: address.unwrap_or(None),
            size: size.unwrap_or(None),
            save_type,
            dsk_filename,
            side
        })
    }
}

pub fn parse_buildsna(
    directive_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        if !directive_name_parsed {
            parse_word(b"BUILDSNA").parse_next(input)?;
        }

        terminated(
            cut_err(opt(alt((
                Caseless("V2").value(SnapshotVersion::V2),
                Caseless("V3").value(SnapshotVersion::V3)
            ))))
            .map(|v: Option<SnapshotVersion>| LocatedTokenInner::BuildSna(v)),
            not(alphanumeric1)
        )
        .parse_next(input)
    }
}

#[derive(PartialEq)]
pub enum RunEnt {
    Run,
    Ent
}

pub fn parse_run(kind: RunEnt) -> impl Parser<InnerZ80Span, LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let exp = cut_err(located_expr.context(match &kind {
            RunEnt::Run => "RUN expects at least one expression (e.g. RUN $)",
            RunEnt::Ent => "ENT expects one expression"
        }))
        .parse_next(input)?;

        let ga = if kind == RunEnt::Run {
            opt(preceded((my_space0, (','), my_space0), located_expr)).parse_next(input)?
        }
        else {
            None
        };

        Ok(LocatedTokenInner::Run(exp, ga))
    }
}

pub fn parse_module(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let module_start = input.checkpoint();
    let _ = parse_directive_word(b"MODULE").parse_next(input)?;

    let name = cut_err(parse_label(false).context(StrContext::Label("MODULE: error in naming")))
        .parse_next(input)?;

    let inner = cut_err(inner_code.context(StrContext::Label("MODULE: issue in the content")))
        .parse_next(input)?;
    let _ = cut_err(
        preceded(my_space0, parse_directive_word(b"ENDMODULE"))
            .context(StrContext::Label("MODULE: not closed"))
    )
    .parse_next(input)?;

    let token = LocatedTokenInner::Module(name.into(), inner)
        .into_located_token_between(&module_start, *input);
    Ok(token)
}

/// Parse a sub-listing part that aims at being crunched after being assembled at first pass
pub fn parse_crunched_section(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedToken, Z80ParserError> {
    let crunched_start = input.checkpoint();
    let kind = preceded(
        my_space0,
        alt((
            #[cfg(not(target_arch = "wasm32"))]
            parse_directive_word(b"LZEXO").value(CrunchType::LZEXO),
            #[cfg(not(target_arch = "wasm32"))]
            parse_directive_word(b"LZ4").value(CrunchType::LZ4),
            parse_directive_word(b"LZ48").value(CrunchType::LZ48),
            parse_directive_word(b"LZ49").value(CrunchType::LZ49),
            #[cfg(not(target_arch = "wasm32"))]
            parse_directive_word(b"LZSHRINKLER").value(CrunchType::Shrinkler),
            #[cfg(not(target_arch = "wasm32"))]
            parse_directive_word(b"LZUPKR").value(CrunchType::Upkr),
            #[cfg(not(target_arch = "wasm32"))]
            parse_directive_word(b"LZX7").value(CrunchType::LZX7),
            #[cfg(not(target_arch = "wasm32"))]
            parse_directive_word(b"LZX0_BACKWARD").value(CrunchType::BackwardZx0),
            #[cfg(not(target_arch = "wasm32"))]
            parse_directive_word(b"LZX0").value(CrunchType::Zx0),
            #[cfg(not(target_arch = "wasm32"))]
            parse_directive_word(b"LZAPU").value(CrunchType::LZAPU),
            parse_directive_word(b"LZSA1").value(CrunchType::LZSA1),
            parse_directive_word(b"LZSA2").value(CrunchType::LZSA2)
        ))
    )
    .parse_next(input)?;

    let inner =
        cut_err(inner_code.context(StrContext::Label("CRUNCHED SECTION: issue in the content")))
            .parse_next(input)?;

    let _ = cut_err(
        (my_space0, parse_directive_word(b"LZCLOSE"), my_space0)
            .context(StrContext::Label("CRUNCHED SECTION section: not closed"))
    )
    .parse_next(input)?;

    let token = LocatedTokenInner::CrunchedSection(kind, inner)
        .into_located_token_between(&crunched_start, *input);
    Ok(token)
}

pub fn parse_range(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let start = cut_err(
        delimited(my_space0, located_expr, my_space0)
            .context(StrContext::Label("RANGE: wrong start address"))
    )
    .parse_next(input)?;
    let stop = cut_err(
        preceded(parse_comma, delimited(my_space0, located_expr, my_space0))
            .context(StrContext::Label("RANGE: wrong end address"))
    )
    .parse_next(input)?;
    let label = cut_err(
        preceded(
            parse_comma,
            delimited(my_space0, parse_label(false), my_space0)
        )
        .context(StrContext::Label("RANGE: wrong name"))
    )
    .parse_next(input)?;

    Ok(LocatedTokenInner::Range(label.into(), start, stop))
}

pub fn parse_protect(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let start = located_expr.parse_next(input)?;

    let end = preceded(parse_comma, located_expr).parse_next(input)?;

    Ok(LocatedTokenInner::Protect(start, end))
}

pub fn parse_org(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let val1 =
        cut_err(located_expr.context(StrContext::Label("Invalid argument"))).parse_next(input)?;
    let val2 = opt(preceded(parse_comma, located_expr)).parse_next(input)?;

    Ok(LocatedTokenInner::Org { val1, val2 })
}

pub fn parse_assert(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let expr = cut_err(located_expr.context(StrContext::Label("ASSERT: expression error")))
        .parse_next(input)?;

    let exps = cut_err(
        opt(preceded(parse_comma, parse_print_inner))
            .context(StrContext::Label("ASSERT: comment error"))
    )
    .parse_next(input)?;

    Ok(LocatedTokenInner::Assert(expr, exps))
}

pub fn parse_print(
    directive_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        if !directive_name_parsed {
            parse_word(b"PRINT").parse_next(input)?;
        }

        cut_err(parse_print_inner)
            .map(LocatedTokenInner::Print)
            .parse_next(input)
    }
}

pub fn parse_fail(
    directive_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        if !directive_name_parsed {
            parse_word(b"FAIL").parse_next(input)?;
        }

        opt(parse_print_inner)
            .map(LocatedTokenInner::Fail)
            .parse_next(input)
    }
}

pub fn parse_print_inner(
    input: &mut InnerZ80Span
) -> ModalResult<Vec<FormattedExpr>, Z80ParserError> {
    separated(
        1..,
        alt((
            formatted_expr,
            expr.map(FormattedExpr::from),
            parse_string.map(|s| {
                let s = s.as_ref();
                FormattedExpr::from(Expr::String(SmolStr::from_iter(s.chars())))
            })
        )),
        parse_comma
    )
    .parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
fn formatted_expr(input: &mut InnerZ80Span) -> ModalResult<FormattedExpr, Z80ParserError> {
    let _ = ('{').parse_next(input)?;
    let format = alt((
        Caseless("INT").value(ExprFormat::Int),
        Caseless("HEX4").value(ExprFormat::Hex(Some(4))),
        Caseless("HEX8").value(ExprFormat::Hex(Some(8))),
        Caseless("HEX2").value(ExprFormat::Hex(Some(2))),
        Caseless("HEX").value(ExprFormat::Hex(None)),
        Caseless("BIN8").value(ExprFormat::Bin(Some(8))),
        Caseless("BIN16").value(ExprFormat::Bin(Some(16))),
        Caseless("BIN32").value(ExprFormat::Bin(Some(32))),
        Caseless("BIN").value(ExprFormat::Bin(None))
    ))
    .parse_next(input)?;
    let _ = ('}').parse_next(input)?;

    let _ = my_space0(input)?;

    let exp = expr(input)?;

    Ok(FormattedExpr::Formatted(format, exp))
}

pub fn parse_align(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let boundary = located_expr.parse_next(input)?;
    let fill = opt(preceded(parse_comma, located_expr)).parse_next(input)?;

    Ok(LocatedTokenInner::Align(boundary, fill))
}

pub fn parse_breakpoint(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let expr = opt(terminated(located_expr, not('='))
        .with_taken()
        .verify(|(_e, s)| {
            // disallow labels that are similar to some keywords
            !s.eq_ignore_ascii_case(b"READ")
                && !s.eq_ignore_ascii_case(b"R")
                && !s.eq_ignore_ascii_case(b"WRITE")
                && !s.eq_ignore_ascii_case(b"W")
                && !s.eq_ignore_ascii_case(b"READWRITE")
                && !s.eq_ignore_ascii_case(b"RW")
                && !s.eq_ignore_ascii_case(b"MEM")
                && !s.eq_ignore_ascii_case(b"MEMORY")
                && !s.eq_ignore_ascii_case(b"EXEC")
                && !s.eq_ignore_ascii_case(b"EXECUTE")
                && !s.eq_ignore_ascii_case(b"STOP")
                && !s.eq_ignore_ascii_case(b"STOPPER")
                && !s.eq_ignore_ascii_case(b"WATCH")
                && !s.eq_ignore_ascii_case(b"WATCHER")
                && !s.contains(&b'=')
        })
        .map(|(e, _s)| e))
    .parse_next(input)?;

    let address = Rc::new(RefCell::new(expr.map(|expr| (None, expr))));
    let r#type = Rc::new(RefCell::new(None));
    let access = Rc::new(RefCell::new(None));
    let run = Rc::new(RefCell::new(None));
    let mask = Rc::new(RefCell::new(None));
    let size = Rc::new(RefCell::new(None));
    let value = Rc::new(RefCell::new(None));
    let value_mask = Rc::new(RefCell::new(None));
    let condition = Rc::new(RefCell::new(None));
    let name = Rc::new(RefCell::new(None));
    let step = Rc::new(RefCell::new(None));

    let first = std::rc::Rc::new(std::cell::RefCell::new(true));

    loop {
        cut_err(
            opt(parse_breakpoint_argument)
                .verify_map(|arg| {
                    // at the same time verify if it is ok and update
                    if let Some(arg) = arg {
                        match arg {
                            BreakPointArgument::Address { arg, value } => {
                                let mut address = address.borrow_mut();
                                let address = address.deref_mut();
                                if address.is_some() {
                                    None
                                }
                                else {
                                    address.replace((Some(arg), value));
                                    Some(())
                                }
                            },

                            BreakPointArgument::Type { arg, value } => {
                                let mut r#type = r#type.borrow_mut();
                                let r#type = r#type.deref_mut();
                                if r#type.is_some() {
                                    None
                                }
                                else {
                                    r#type.replace((Some(arg), value));
                                    Some(())
                                }
                            },

                            BreakPointArgument::Access { arg, value } => {
                                let mut access = access.borrow_mut();
                                let access = access.deref_mut();
                                if access.is_some() {
                                    None
                                }
                                else {
                                    access.replace((arg, value));
                                    Some(())
                                }
                            },

                            BreakPointArgument::Run { arg, value } => {
                                let mut run = run.borrow_mut();
                                let run = run.deref_mut();
                                if run.is_some() {
                                    None
                                }
                                else {
                                    run.replace((arg, value));
                                    Some(())
                                }
                            },

                            BreakPointArgument::Mask { arg, value } => {
                                let mut item = mask.borrow_mut();
                                let item = item.deref_mut();
                                if item.is_some() {
                                    None
                                }
                                else {
                                    item.replace((arg, value));
                                    Some(())
                                }
                            },

                            BreakPointArgument::Size { arg, value } => {
                                let mut item = size.borrow_mut();
                                let item = item.deref_mut();
                                if item.is_some() {
                                    None
                                }
                                else {
                                    item.replace((arg, value));
                                    Some(())
                                }
                            },

                            BreakPointArgument::Value { arg, value: val } => {
                                let mut item = value.borrow_mut();
                                let item = item.deref_mut();
                                if item.is_some() {
                                    None
                                }
                                else {
                                    item.replace((arg, val));
                                    Some(())
                                }
                            },

                            BreakPointArgument::ValueMask { arg, value } => {
                                let mut item = value_mask.borrow_mut();
                                let item = item.deref_mut();
                                if item.is_some() {
                                    None
                                }
                                else {
                                    item.replace((arg, value));
                                    Some(())
                                }
                            },

                            BreakPointArgument::Name { arg, value } => {
                                let mut item = name.borrow_mut();
                                let item = item.deref_mut();
                                if item.is_some() {
                                    None
                                }
                                else {
                                    item.replace((arg, value));
                                    Some(())
                                }
                            },

                            BreakPointArgument::Condition { arg, value } => {
                                let mut item = condition.borrow_mut();
                                let item = item.deref_mut();
                                if item.is_some() {
                                    None
                                }
                                else {
                                    item.replace((arg, value));
                                    Some(())
                                }
                            },

                            BreakPointArgument::Step { arg, value } => {
                                let mut item = step.borrow_mut();
                                let item = item.deref_mut();
                                if item.is_some() {
                                    None
                                }
                                else {
                                    item.replace((arg, value));
                                    Some(())
                                }
                            }
                        }
                    }
                    else if *first.borrow() {
                        Some(())
                    }
                    else {
                        None
                    }
                })
                .context(StrContext::Label("Breapoint parameter error"))
        )
        .parse_next(input)?;

        *first.borrow_mut() = false;

        if opt(parse_comma).parse_next(input)?.is_none() {
            break;
        }
    }

    let brk = LocatedTokenInner::Breakpoint {
        address: Rc::into_inner(address)
            .expect("Rc should have single owner")
            .into_inner()
            .map(|a| a.1),
        r#type: Rc::into_inner(r#type)
            .expect("Rc should have single owner")
            .into_inner()
            .map(|r| r.1),
        access: Rc::into_inner(access)
            .expect("Rc should have single owner")
            .into_inner()
            .map(|a| a.1),
        run: Rc::into_inner(run)
            .expect("Rc should have single owner")
            .into_inner()
            .map(|a| a.1),
        mask: Rc::into_inner(mask)
            .expect("Rc should have single owner")
            .into_inner()
            .map(|a| a.1),
        size: Rc::into_inner(size)
            .expect("Rc should have single owner")
            .into_inner()
            .map(|a| a.1),
        value: Rc::into_inner(value)
            .expect("Rc should have single owner")
            .into_inner()
            .map(|a| a.1),
        value_mask: Rc::into_inner(value_mask)
            .expect("Rc should have single owner")
            .into_inner()
            .map(|a| a.1),
        condition: Rc::into_inner(condition)
            .expect("Rc should have single owner")
            .into_inner()
            .map(|a| a.1),
        name: Rc::into_inner(name)
            .expect("Rc should have single owner")
            .into_inner()
            .map(|a| a.1),
        step: Rc::into_inner(step)
            .expect("Rc should have single owner")
            .into_inner()
            .map(|a| a.1)
    };

    Ok(brk)
}

#[derive(Debug)]
pub enum BreakPointArgument {
    Type {
        arg: Option<InnerZ80Span>,
        value: RemuBreakPointType
    },
    Access {
        arg: Option<InnerZ80Span>,
        value: RemuBreakPointAccessMode
    },
    Run {
        arg: Option<InnerZ80Span>,
        value: RemuBreakPointRunMode
    },
    Address {
        arg: InnerZ80Span,
        value: LocatedExpr
    },
    Mask {
        arg: InnerZ80Span,
        value: LocatedExpr
    },
    Size {
        arg: InnerZ80Span,
        value: LocatedExpr
    },
    Value {
        arg: InnerZ80Span,
        value: LocatedExpr
    },
    ValueMask {
        arg: InnerZ80Span,
        value: LocatedExpr
    },
    Condition {
        arg: InnerZ80Span,
        value: LocatedExpr
    },
    Name {
        arg: InnerZ80Span,
        value: LocatedExpr
    },
    Step {
        arg: InnerZ80Span,
        value: LocatedExpr
    }
}

pub fn parse_breakpoint_argument(
    input: &mut InnerZ80Span
) -> ModalResult<BreakPointArgument, Z80ParserError> {
    alt((
        parse_optional_argname_and_value("TYPE", &parse_breakpoint_type_value)
            .map(|(k, v)| BreakPointArgument::Type { arg: k, value: v }),
        parse_optional_argname_and_value("ACCESS", &parse_breakpoint_access_value)
            .map(|(k, v)| BreakPointArgument::Access { arg: k, value: v }),
        parse_optional_argname_and_value("RUNMODE", &parse_breakpoint_run_value)
            .map(|(k, v)| BreakPointArgument::Run { arg: k, value: v }),
        alt((
            parse_argname_and_value("ADDRESS", &located_expr),
            parse_argname_and_value("ADDR", &located_expr)
        ))
        .map(|(k, v)| BreakPointArgument::Address { arg: k, value: v }),
        parse_argname_and_value("MASK", &located_expr)
            .map(|(k, v)| BreakPointArgument::Mask { arg: k, value: v }),
        parse_argname_and_value("SIZE", &located_expr)
            .map(|(k, v)| BreakPointArgument::Size { arg: k, value: v }),
        parse_argname_and_value("VALUE", &located_expr)
            .map(|(k, v)| BreakPointArgument::Value { arg: k, value: v }),
        parse_argname_and_value("VALMASK", &located_expr)
            .map(|(k, v)| BreakPointArgument::ValueMask { arg: k, value: v }),
        parse_argname_and_value("STEP", &located_expr)
            .map(|(k, v)| BreakPointArgument::Step { arg: k, value: v }),
        parse_argname_and_value("CONDITION", &located_expr)
            .map(|(k, v)| BreakPointArgument::Condition { arg: k, value: v }),
        parse_argname_and_value("NAME", &located_expr)
            .map(|(k, v)| BreakPointArgument::Name { arg: k, value: v })
    ))
    .parse_next(input)
}

pub fn parse_breakpoint_type_value(
    input: &mut InnerZ80Span
) -> ModalResult<RemuBreakPointType, Z80ParserError> {
    parse_convertible_word(input)
}

pub fn parse_breakpoint_access_value(
    input: &mut InnerZ80Span
) -> ModalResult<RemuBreakPointAccessMode, Z80ParserError> {
    parse_convertible_word(input)
}

pub fn parse_breakpoint_run_value(
    input: &mut InnerZ80Span
) -> ModalResult<RemuBreakPointRunMode, Z80ParserError> {
    parse_convertible_word(input)
}

pub fn parse_bankset(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let count = located_expr.parse_next(input)?;

    Ok(LocatedTokenInner::Bankset(count))
}

pub fn parse_bank(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let count = opt(located_expr).parse_next(input)?;

    Ok(LocatedTokenInner::Bank(count))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_skip(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let count = cut_err(located_expr.context(StrContext::Label("SKIP: wrong expression")))
        .parse_next(input)?;

    Ok(LocatedTokenInner::Skip(count))
}

pub fn parse_defs(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let val = separated(
        1..,
        cut_err(
            (located_expr, opt(preceded(parse_comma, located_expr)))
                .context(StrContext::Label("Wrong argument"))
        ),
        parse_comma
    )
    .parse_next(input)?;

    Ok(LocatedTokenInner::Defs(val))
}

pub fn parse_snainit(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let fname = parse_fname(input)?;

    Ok(LocatedTokenInner::SnaInit(fname))
}

pub fn parse_struct_directive(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedToken, Z80ParserError> {
    alt((
        parse_struct_directive_inner,
        parse_macro_or_struct_call(false, true)
    ))
    .parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
fn parse_struct_directive_inner(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedToken, Z80ParserError> {
    // XXX Sadly the state is stored within the context that cannot
    //     by changed. So we can cannot really use parsing state sutf

    let input_start = input.checkpoint();
    let parsing_state = ParsingState::StructLimited;
    let directive = parse_directive_new(&parsing_state)
        .verify(move |d| d.is_accepted(&parsing_state))
        .parse_next(input)?;

    // Only one argument is allowed
    if (directive.is_db() || directive.is_dw()) && directive.data_exprs().len() > 1 {
        return Err(ErrMode::Cut(Z80ParserError::from_input(input).add_context(
            input,
            &input_start,
            "0 or 1 arguments are expected"
        )));
    }
    Ok(directive)
}

pub fn parse_struct(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let name = cut_err(parse_label(false)).parse_next(input)?;

    // TODO parse inner with filtering on the allowed operations
    // would be easier to write and would allow conditional operations
    let fields: Vec<(Z80Span, LocatedToken)> = cut_err(
        repeat(
            1..,
            delimited(
                repeat::<_, _, (), _, _>(
                    0..,
                    alt((
                        my_space1.value(()),
                        parse_comment.value(()),
                        line_ending.value(()),
                        ':'.value(())
                    ))
                ),
                (
                    terminated(
                        parse_label(false),
                        alt(((my_space0, ':', my_space0).take(), my_space1.take()))
                    )
                    .verify(|label: &InnerZ80Span| !label.eq_ignore_ascii_case(b"endstruct"))
                    .context(StrContext::Label("STRUCT: label error"))
                    .map(|span: InnerZ80Span| Z80Span::from(span)),
                    cut_err(
                        parse_struct_directive
                            .context(StrContext::Label("STRUCT: Invalid operation"))
                    )
                ),
                repeat::<_, _, (), _, _>(
                    0..,
                    alt((
                        my_space1.value(()),
                        parse_comment.value(()),
                        line_ending.value(()),
                        ':'.value(())
                    ))
                )
            )
        )
        .context(StrContext::Label("STRUCT: error in inner content"))
    )
    .parse_next(input)?;

    let _ = cut_err(preceded(
        my_space0,
        alt((
            parse_directive_word(b"ENDSTRUCT"),
            parse_directive_word(b"ENDS")
        ))
    ))
    .parse_next(input)?;

    Ok(LocatedTokenInner::Struct(name.into(), fields))
}

#[derive(PartialEq)]
pub enum ExportKind {
    Export,
    NoExport
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_export(
    code: ExportKind
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let labels: Vec<InnerZ80Span> = cut_err(
            separated(0.., parse_label(false), parse_comma)
                .context(StrContext::Label("Wrong parameters"))
        )
        .parse_next(input)?;
        let labels = labels.into_iter().map(Z80Span::from).collect_vec();

        if code == ExportKind::Export {
            Ok(LocatedTokenInner::Export(labels))
        }
        else {
            Ok(LocatedTokenInner::NoExport(labels))
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_snaset(
    directive_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        if !directive_name_parsed {
            parse_word(b"SNASET").parse_next(input)?;
        }

        let input_start = input.checkpoint();
        let flagname = cut_err(parse_label(false).context(SNASET_WRONG_LABEL)).parse_next(input)?;
        let _ = cut_err(parse_comma.context(SNASET_MISSING_COMMA)).parse_next(input)?;

        let values: Vec<_> = cut_err(separated(
            1..,
            parse_flag_value_inner.context(StrContext::Label("SNASET: wrong flag value")),
            delimited(my_space0, parse_comma, my_space0)
        ))
        .parse_next(input)?;

        let flagname = flagname.as_bstr();
        let flagname = unsafe { std::str::from_utf8_unchecked(flagname) };
        let (flagname, value) = if values.len() == 1 {
            (Cow::Borrowed(flagname), values[0].clone())
        }
        else {
            (
                Cow::Owned(format!("{}:{}", flagname, values[0].as_u16().unwrap())),
                values[1].clone()
            )
        };

        let flag = SnapshotFlag::from_str(flagname.as_ref()).map_err(|_e| {
            input.reset(&input_start);
            ErrMode::Backtrack(Z80ParserError::from_input(input).add_context(
                input,
                &input_start,
                "Wrong flag"
            ))
        })?;
        Ok(LocatedTokenInner::SnaSet(flag, value))
    }
}

pub fn parse_directive(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let parsing_state = input.state.state;
    parse_directive_new(&parsing_state)
        .verify(move |d| d.is_accepted(&parsing_state))
        .parse_next(input)
}

/// Here local_parsing_state only serves to adapt DB/DW/STR behavior in struct.
/// Maybe it should be used to control the directives of interest BEFORE there parsing instead of after.
/// No filtering is done
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_directive_new(
    local_parsing_state: &ParsingState
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> + '_ {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedToken, Z80ParserError> {
        let is_orgams = input.state.options().is_orgams();

        let input_start = input.checkpoint();

        // Get the first word that will drive the rest of parsing
        let word = delimited(
            my_space0,
            terminated(
                alphanumeric1,
                alt((eof.value(()), not(alt((b'.', b'_'))).value(())))
            ),
            my_space0
        )
        .parse_next(input)?;

        let within_struct = local_parsing_state == &ParsingState::StructLimited;

        //   dbg!("Directive:", unsafe{std::str::from_utf8_unchecked(word)});

        let token: LocatedTokenInner = match word.len() {
            2 => parse_directive_of_size_2(input, &input_start, is_orgams, within_struct, word),
            3 => parse_directive_of_size3(input, &input_start, is_orgams, within_struct, word),
            4 => parse_directive_of_size_4(input, &input_start, is_orgams, within_struct, word),
            5 => parse_directive_of_size_5(input, &input_start, is_orgams, within_struct, word),
            6 => parse_directive_of_size_6(input, &input_start, is_orgams, within_struct, word),
            7 => parse_directive_of_size_7(input, &input_start, is_orgams, within_struct, word),
            8 => parse_directive_of_size_8(input, &input_start, is_orgams, within_struct, word),
            10 => parse_directive_of_size_10(input, &input_start, is_orgams, within_struct, word),
            _ => parse_directive_of_size_others(input, &input_start, is_orgams, within_struct, word)
        }?;

        let token = token.into_located_token_between(&input_start, *input);
        Ok(token)
    }
}

fn parse_directive_of_size_others(
    input: &mut InnerZ80Span,
    input_start: &Checkpoint<
        Checkpoint<Checkpoint<&'static BStr, &'static BStr>, LocatingSlice<&'static BStr>>,
        Stateful<LocatingSlice<&'static BStr>, &'static context::ParserContext>
    >,
    _is_orgams: bool,
    _within_struct: bool,
    word: &[u8]
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    match &word.to_ascii_uppercase()[..] {
        // 12
        #[cfg(not(target_arch = "wasm32"))]
        b"INCSHRINKLER" => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::Shrinkler)).parse_next(input)
        },

        // 13
        b"STARTINGINDEX" => parse_startingindex.parse_next(input),

        _ => {
            input.reset(input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

fn parse_directive_of_size_10(
    input: &mut InnerZ80Span,
    input_start: &Checkpoint<
        Checkpoint<Checkpoint<&'static BStr, &'static BStr>, LocatingSlice<&'static BStr>>,
        Stateful<LocatingSlice<&'static BStr>, &'static context::ParserContext>
    >,
    _is_orgams: bool,
    _within_struct: bool,
    word: &[u8]
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    match fnv1a_ascii_upper(word) {
        h if hashed_choice!(h, word, b"ASMCONTROL") => parse_assembler_control.parse_next(input),
        h if hashed_choice!(h, word, b"BREAKPOINT") => parse_breakpoint.parse_next(input),
        h if hashed_choice!(h, word, b"DEFSECTION") => parse_range.parse_next(input),

        _ => {
            input.reset(input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

fn parse_directive_of_size_8(
    input: &mut InnerZ80Span,
    input_start: &Checkpoint<
        Checkpoint<Checkpoint<&'static BStr, &'static BStr>, LocatingSlice<&'static BStr>>,
        Stateful<LocatingSlice<&'static BStr>, &'static context::ParserContext>
    >,
    _is_orgams: bool,
    _within_struct: bool,
    word: &[u8]
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    match fnv1a_ascii_upper(word) {
        h if hashed_choice!(h, word, b"BINCLUDE") => {
            parse_incbin(BinaryTransformation::None).parse_next(input)
        },
        h if hashed_choice!(h, word, b"BUILDSNA") => parse_buildsna(true).parse_next(input),
        h if hashed_choice!(h, word, b"BUILDCPR") => Ok(LocatedTokenInner::BuildCpr),
        h if hashed_choice!(h, word, b"INCLZSA1") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::LZSA1)).parse_next(input)
        },
        h if hashed_choice!(h, word, b"INCLZSA2") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::LZSA1)).parse_next(input)
        },
        h if hashed_choice!(h, word, b"NOEXPORT") => {
            parse_export(ExportKind::NoExport).parse_next(input)
        },
        h if hashed_choice!(h, word, b"WAITNOPS") => parse_waitnops.parse_next(input),
        h if hashed_choice!(h, word, b"SNAPINIT") => parse_snainit.parse_next(input),

        _ => {
            input.reset(input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

fn parse_directive_of_size_7(
    input: &mut InnerZ80Span,
    input_start: &Checkpoint<
        Checkpoint<Checkpoint<&'static BStr, &'static BStr>, LocatingSlice<&'static BStr>>,
        Stateful<LocatingSlice<&'static BStr>, &'static context::ParserContext>
    >,
    _is_orgams: bool,
    _within_struct: bool,
    word: &[u8]
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    match fnv1a_ascii_upper(word) {
        h if hashed_choice!(h, word, b"INCLUDE") => parse_include.parse_next(input),
        h if hashed_choice!(h, word, b"BANKSET") => parse_bankset.parse_next(input),
        h if hashed_choice!(h, word, b"CHARSET") => parse_charset.parse_next(input),
        h if hashed_choice!(h, word, b"PROTECT") => parse_protect.parse_next(input),
        h if hashed_choice!(h, word, b"SECTION") => parse_section.parse_next(input),
        h if hashed_choice!(h, word, b"SNAINIT") => parse_snainit.parse_next(input),
        #[cfg(not(target_arch = "wasm32"))]
        h if hashed_choice!(h, word, b"INCUPKR") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::Upkr)).parse_next(input)
        },
        _ => {
            input.reset(input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

fn parse_directive_of_size_6(
    input: &mut InnerZ80Span,
    input_start: &Checkpoint<
        Checkpoint<Checkpoint<&'static BStr, &'static BStr>, LocatingSlice<&'static BStr>>,
        Stateful<LocatingSlice<&'static BStr>, &'static context::ParserContext>
    >,
    is_orgams: bool,
    _within_struct: bool,
    word: &[u8]
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    match fnv1a_ascii_upper(word) {
        h if hashed_choice!(h, word, b"ASSERT") => parse_assert.parse_next(input),

        h if hashed_choice!(h, word, b"EXPORT") => {
            parse_export(ExportKind::Export).parse_next(input)
        },
        h if hashed_choice!(h, word, b"INCBIN") => {
            parse_incbin(BinaryTransformation::None).parse_next(input)
        },
        #[cfg(not(target_arch = "wasm32"))]
        h if hashed_choice!(h, word, b"INCEXO") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::LZEXO)).parse_next(input)
        },
        #[cfg(not(target_arch = "wasm32"))]
        h if hashed_choice!(h, word, b"INCLZ4") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::LZ4)).parse_next(input)
        },

        h if hashed_choice!(h, word, b"INCL48") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::LZ48)).parse_next(input)
        },

        h if hashed_choice!(h, word, b"INCL49") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::LZ49)).parse_next(input)
        },

        #[cfg(not(target_arch = "wasm32"))]
        h if hashed_choice!(h, word, b"INCAPU") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::LZAPU)).parse_next(input)
        },

        #[cfg(not(target_arch = "wasm32"))]
        h if hashed_choice!(h, word, b"INCZX0") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::Zx0)).parse_next(input)
        },

        #[cfg(not(target_arch = "wasm32"))]
        h if hashed_choice!(h, word, b"INCZX0_BACKWARD") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::BackwardZx0)).parse_next(input)
        },

        h if hashed_choice!(h, word, b"RETURN") => parse_return.parse_next(input),
        h if hashed_choice!(h, word, b"SNASET") => parse_snaset(true).parse_next(input),

        h if hashed_choice!(h, word, b"STRUCT") => parse_struct.parse_next(input),
        h if hashed_choice!(h, word, b"TICKER") => parse_stable_ticker.parse_next(input),

        h if hashed_choice!(h, word, b"NOLIST") => Ok(LocatedTokenInner::NoList),

        h if hashed_choice!(h, word, b"IMPORT") && is_orgams => parse_include.parse_next(input), /* TODO filter to remove the orgams specificies */

        _ => {
            input.reset(input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

fn parse_directive_of_size_5(
    input: &mut InnerZ80Span,
    input_start: &Checkpoint<
        Checkpoint<Checkpoint<&'static BStr, &'static BStr>, LocatingSlice<&'static BStr>>,
        Stateful<LocatingSlice<&'static BStr>, &'static context::ParserContext>
    >,
    _is_orgams: bool,
    within_struct: bool,
    word: &[u8]
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    match fnv1a_ascii_upper(word) {
        h if hashed_choice!(h, word, b"ALIGN") => parse_align.parse_next(input),
        h if hashed_choice!(h, word, b"ABYTE") => {
            parse_db_or_dw_or_str(DbDwStr::Abyte, within_struct).parse_next(input)
        },
        h if hashed_choice!(h, word, b"LIMIT") => parse_limit.parse_next(input),
        h if hashed_choice!(h, word, b"PAUSE") => Ok(LocatedTokenInner::Pause),
        h if hashed_choice!(h, word, b"PRINT") => parse_print(true).parse_next(input),
        h if hashed_choice!(h, word, b"RANGE") => parse_range.parse_next(input),
        h if hashed_choice!(h, word, b"UNDEF") => parse_undef.parse_next(input),

        h if hashed_choice!(h, word, b"WRITE") => {
            alt((parse_save(SaveKind::WriteDirect), parse_write_direct_memory)).parse_next(input)
        },

        _ => {
            input.reset(input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

fn parse_directive_of_size_4(
    input: &mut InnerZ80Span,
    input_start: &Checkpoint<
        Checkpoint<Checkpoint<&'static BStr, &'static BStr>, LocatingSlice<&'static BStr>>,
        Stateful<LocatingSlice<&'static BStr>, &'static context::ParserContext>
    >,
    is_orgams: bool,
    within_struct: bool,
    word: &[u8]
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    match fnv1a_ascii_upper(word) {
        h if hashed_choice!(h, word, b"DEFB", b"DEFM", b"BYTE", b"TEXT") => {
            parse_db_or_dw_or_str(DbDwStr::Db, within_struct).parse_next(input)
        },

        h if hashed_choice!(h, word, b"FILL", b"DEFS", b"RMEM") => parse_defs.parse_next(input),

        h if hashed_choice!(h, word, b"BANK") => parse_bank.parse_next(input),
        h if hashed_choice!(h, word, b"FAIL") => parse_fail(true).parse_next(input),
        h if hashed_choice!(h, word, b"LIST") => Ok(LocatedTokenInner::List),
        h if hashed_choice!(h, word, b"READ") => parse_include.parse_next(input),

        h if hashed_choice!(h, word, b"SAVE") => parse_save(SaveKind::Save).parse_next(input),

        h if hashed_choice!(h, word, b"SKIP") && is_orgams => parse_skip.parse_next(input),

        h if hashed_choice!(h, word, b"WORD", b"DEFW") => {
            parse_db_or_dw_or_str(DbDwStr::Dw, within_struct).parse_next(input)
        },
        _ => {
            input.reset(input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

fn parse_directive_of_size3(
    input: &mut InnerZ80Span,
    input_start: &Checkpoint<
        Checkpoint<Checkpoint<&'static BStr, &'static BStr>, LocatingSlice<&'static BStr>>,
        Stateful<LocatingSlice<&'static BStr>, &'static context::ParserContext>
    >,
    is_orgams: bool,
    within_struct: bool,
    word: &[u8]
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    match fnv1a_ascii_upper(word) {
        h if hashed_choice!(h, word, b"BRK") && is_orgams => parse_breakpoint.parse_next(input),

        h if hashed_choice!(h, word, b"STR") => {
            parse_db_or_dw_or_str(DbDwStr::Str, within_struct).parse_next(input)
        },
        h if hashed_choice!(h, word, b"END") && !is_orgams => Ok(LocatedTokenInner::End),
        h if hashed_choice!(h, word, b"ENT") => parse_run(RunEnt::Ent).parse_next(input),
        h if hashed_choice!(h, word, b"MAP") => parse_map.parse_next(input),
        h if hashed_choice!(h, word, b"NOP") => parse_nop.parse_next(input),
        h if hashed_choice!(h, word, b"ORG") => parse_org.parse_next(input),
        h if hashed_choice!(h, word, b"RUN") => parse_run(RunEnt::Run).parse_next(input),
        _ => {
            input.reset(input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

fn parse_directive_of_size_2(
    input: &mut InnerZ80Span,
    input_start: &Checkpoint<
        Checkpoint<Checkpoint<&'static BStr, &'static BStr>, LocatingSlice<&'static BStr>>,
        Stateful<LocatingSlice<&'static BStr>, &'static context::ParserContext>
    >,
    is_orgams: bool,
    within_struct: bool,
    word: &[u8]
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    match fnv1a_ascii_upper(word) {
        h if hashed_choice!(h, word, b"BY") && is_orgams => {
            parse_db_or_dw_or_str(DbDwStr::Db, within_struct).parse_next(input)
        },

        h if hashed_choice!(h, word, b"DB", b"DM") => {
            parse_db_or_dw_or_str(DbDwStr::Db, within_struct).parse_next(input)
        },

        h if hashed_choice!(h, word, b"DS") => parse_defs.parse_next(input),

        h if hashed_choice!(h, word, b"DW") => {
            parse_db_or_dw_or_str(DbDwStr::Dw, within_struct).parse_next(input)
        },

        _ => {
            input.reset(input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

// Additional directive parsing functions migrated from parser.rs

#[derive(Clone, Copy, Debug)]
pub enum KindOfConditional {
    If,
    IfNot,
    IfDef,
    IfNdef,
    IfUsed,
    IfNused
}

pub fn parse_function_listing(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedListing, Z80ParserError> {
    inner_code_with_state(ParsingState::FunctionLimited, false).parse_next(input)
}

pub fn parse_function(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let function_start = input.checkpoint();
    let _ = preceded(my_space0, parse_directive_word(b"FUNCTION")).parse_next(input)?;
    let name = cut_err(parse_label(false).context(StrContext::Label("FUNCTION: wrong name")))
        .parse_next(input)?;

    let cloned = *input;
    let arguments: Vec<InnerZ80Span> = cut_err(
        preceded(
            opt(parse_comma),
            separated::<_, InnerZ80Span, Vec<InnerZ80Span>, _, _, _, _>(
                0..,
                delimited(
                    my_space0,
                    take_till(1.., |c| {
                        c == b'\n' || c == b'\r' || c == b':' || c == b',' || c == b' '
                    })
                    .map(|s: &[u8]| cloned.update_slice(s)),
                    my_space0
                ),
                parse_comma
            )
        )
        .context(StrContext::Label("FUNCTION: errors in parameters"))
    )
    .parse_next(input)?;
    let arguments = arguments.into_iter().map(|span| span.into()).collect_vec();

    cut_err(
        preceded(my_space0, my_line_ending)
            .context(StrContext::Label("FUNCTION: errors after parameters"))
    )
    .parse_next(input)?;

    let listing =
        cut_err(parse_function_listing.context(StrContext::Label("FUNCTION: invalid content")))
            .parse_next(input)?;

    repeat::<_, _, (), _, _>(0.., my_line_ending).parse_next(input)?;
    let _ = alt((
        parse_directive_word(b"ENDF"),
        parse_directive_word(b"ENDFUNCTION")
    ))
    .parse_next(input)?;

    Ok(LocatedTokenInner::Function(name.into(), arguments, listing)
        .into_located_token_between(&function_start, *input))
}

pub fn parse_macro(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let dir_start = input.checkpoint();
    let _ = parse_directive_word(b"MACRO").parse_next(input)?;

    let name = cut_err(parse_label(false).context(StrContext::Label("MACRO: wrong name")))
        .parse_next(input)?;

    parse_macro_inner(dir_start, name).parse_next(input)
}

pub fn parse_macro_inner(
    dir_start: <InnerZ80Span as Stream>::Checkpoint,
    name: InnerZ80Span
) -> impl FnMut(&mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedToken, Z80ParserError> {
        #[derive(Clone, Copy, Debug)]
        enum CommaOrParenthesis {
            Comma,
            Parenthesis
        }

        let comma_or_parenthesis = opt(alt((
            parse_comma.value(CommaOrParenthesis::Comma),
            '('.value(CommaOrParenthesis::Parenthesis)
        )))
        .parse_next(input)?;

        let arguments = separated::<_, _, Vec<&[u8]>, _, _, _, _>(
            0..,
            delimited(
                my_space0,
                take_till(1.., |c| {
                    c == b'\n'
                        || c == b'\r'
                        || c == b':'
                        || c == b','
                        || c == b' '
                        || c == b')'
                        || c == b';'
                }),
                my_space0
            ),
            parse_comma
        )
        .parse_next(input)?;

        if let Some(CommaOrParenthesis::Parenthesis) = comma_or_parenthesis {
            cut_err(
                (my_space0, ')', my_space0)
                    .value(())
                    .context("`)` expected`")
            )
            .parse_next(input)?;
        }

        let arguments = arguments
            .into_iter()
            .map(|span| (*input).update_slice(span))
            .map(|span| span.into())
            .collect_vec();

        alt((my_space0.value(()), my_line_ending.value(()))).parse_next(input)?;

        let before_content = input.checkpoint();
        let (_, end) = cut_err(
            repeat_till::<_, _, (), _, _, _, _>(
                0..,
                take(1usize),
                alt((
                    parse_directive_word(b"ENDM"),
                    parse_directive_word(b"ENDMACRO"),
                    parse_directive_word(b"MEND")
                ))
            )
            .context(StrContext::Label(
                "MACRO: impossible to collect macro content"
            ))
        )
        .parse_next(input)?;

        let content_length = end.offset_from(&before_content);
        let mut content = *input;
        content.reset(&before_content);
        let content: &BStr = unsafe { std::mem::transmute(&content.as_bstr()[..content_length]) };
        let content = (*input).update_slice(content);

        let content: Z80Span = content.into();
        let tokenized_content = tokenize_macro_body(content.as_str(), &arguments);
        Ok(LocatedTokenInner::Macro {
            name: name.into(),
            params: arguments,
            content,
            flavor: input.state.options().assembler_flavor,
            tokenized_content
        }
        .into_located_token_between(&dir_start, *input))
    }
}

pub fn parse_z80_directive_with_block(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedToken, Z80ParserError> {
    let _ = my_space0(input)?;

    if input.state.options().is_orgams() {
        alt((
            parse_macro.context(StrContext::Label("Error in macro")),
            parse_repeat.context(StrContext::Label("Error in repetition")),
            parse_conditional.context(StrContext::Label("Error in condition")),
            parse_orgams_fail
        ))
        .parse_next(input)
    }
    else {
        alt((
            parse_basic.context(StrContext::Label("Basic code embedding")),
            parse_macro.context(StrContext::Label("Error in macro")),
            parse_crunched_section.context(StrContext::Label("Error in crunched section")),
            parse_module.context(StrContext::Label("Error in module")),
            parse_confined.context(StrContext::Label("Error in confined")),
            parse_repeat.context(StrContext::Label("Error in repetition")),
            parse_for.context(StrContext::Label("Error in for")),
            parse_function.context(StrContext::Label("Error in function definition")),
            parse_switch.context(StrContext::Label("Error in switch")),
            parse_iterate.context(StrContext::Label("Error in iterate")),
            parse_while.context(StrContext::Label("Error in while")),
            parse_rorg.context(StrContext::Label("Error in rorg")),
            parse_conditional.context(StrContext::Label("Error in condition")),
            parse_assembler_control_max_passes_number
                .context(StrContext::Label("Error in assembler control"))
        ))
        .parse_next(input)
    }
}

pub fn parse_assign_operator(
    input: &mut InnerZ80Span
) -> ModalResult<Option<BinaryOperation>, Z80ParserError> {
    let start = input.checkpoint();
    let word = take_while(1..=3, |c| {
        c == b'='
            || c == b'<'
            || c == b'>'
            || c == b'+'
            || c == b'-'
            || c == b'*'
            || c == b'/'
            || c == b'%'
            || c == b'^'
            || c == b'|'
            || c == b'&'
    })
    .parse_next(input)?;
    let oper = match word {
        b"=" => None,

        b">>=" => Some(BinaryOperation::RightShift),
        b"<<=" => Some(BinaryOperation::LeftShift),

        b"+=" => Some(BinaryOperation::Add),
        b"-=" => Some(BinaryOperation::Sub),
        b"*=" => Some(BinaryOperation::Mul),
        b"/=" => Some(BinaryOperation::Div),
        b"%=" => Some(BinaryOperation::Mod),

        b"&=" => Some(BinaryOperation::BinaryAnd),
        b"|=" => Some(BinaryOperation::BinaryOr),
        b"^=" => Some(BinaryOperation::BinaryXor),

        b"&&=" => Some(BinaryOperation::BooleanAnd),
        b"||=" => Some(BinaryOperation::BooleanOr),

        _ => {
            return Err(ErrMode::Cut(Z80ParserError::from_input(input).add_context(
                input,
                &start,
                "Wrong symbol"
            )));
        }
    };

    Ok(oper)
}

pub fn parse_charset(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let charset =
        opt(alt((parse_charset_string, parse_charset_start_stop_end))).parse_next(input)?;

    Ok(charset
        .map(LocatedTokenInner::Charset)
        .unwrap_or_else(|| LocatedTokenInner::Charset(CharsetFormat::Reset)))
}

pub fn parse_charset_start_stop_end(
    input: &mut InnerZ80Span
) -> ModalResult<CharsetFormat, Z80ParserError> {
    let (start, stop, end) = (
        expr,
        preceded(parse_comma, expr),
        opt(preceded(parse_comma, expr))
    )
        .parse_next(input)?;

    let format = if let Some(end) = end {
        CharsetFormat::Interval(start, stop, end)
    }
    else {
        CharsetFormat::Char(start, stop)
    };
    Ok(format)
}

pub fn parse_charset_string(
    input: &mut InnerZ80Span
) -> ModalResult<CharsetFormat, Z80ParserError> {
    let chars = parse_string
        .context(StrContext::Label("Missing string"))
        .parse_next(input)?;
    let chars = unsafe { std::str::from_utf8_unchecked(chars.as_ref().as_bytes()) };
    let start = preceded(parse_comma, expr)
        .context(StrContext::Label("Missing start value"))
        .parse_next(input)?;
    let format = CharsetFormat::CharsList(chars.chars().collect_vec(), start);

    Ok(format)
}

pub fn parse_startingindex(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let start = opt(located_expr).parse_next(input)?;
    let step = opt(preceded(parse_comma, located_expr)).parse_next(input)?;

    Ok(LocatedTokenInner::StartingIndex { start, step })
}

pub fn parse_conditional(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let is_orgams = input.state.options().is_orgams();

    let if_clone = *input;
    let if_start = input.checkpoint();

    let mut conditions = Vec::with_capacity(4);
    let mut else_clause = None;

    loop {
        let first_loop = conditions.is_empty();

        let if_token_or_error = alt((
            parse_directive_word(b"IF").value(KindOfConditional::If),
            parse_directive_word(b"IFNOT").value(KindOfConditional::IfNot),
            parse_directive_word(b"IFDEF").value(KindOfConditional::IfDef),
            parse_directive_word(b"IFNDEF").value(KindOfConditional::IfNdef),
            parse_directive_word(b"IFUSED").value(KindOfConditional::IfUsed),
            parse_directive_word(b"IFEXIST").value(KindOfConditional::IfUsed),
            parse_directive_word(b"IFNUSED").value(KindOfConditional::IfNused)
        ))
        .parse_next(input);

        if first_loop && if_token_or_error.is_err() {
            input.reset(&if_start);
            return Err(if_token_or_error.unwrap_err());
        }

        let condition = if let Ok(test_kind) = if_token_or_error {
            let cond = cut_err(
                delimited(my_space0, parse_conditional_condition(test_kind), my_space0)
                    .context(StrContext::Label("Condition: error in the condition"))
            )
            .parse_next(input)?;
            Some(cond)
        }
        else {
            None
        };

        let _ = cut_err(
            alt((
                delimited(my_space0, parse_comment, line_ending).take(),
                line_ending.take(),
                ':'.take()
            ))
            .context(StrContext::Label(
                "Condition: condition must end by a new line or ':'"
            ))
        )
        .parse_next(input)
        .map_err(|e| e.add_context(input, &if_start, "Error in condition"))?;

        let code = cut_err(inner_code.context(StrContext::Label(
            "Condition: syntax error in conditionnal code"
        )))
        .parse_next(input)?;

        if let Some(condition) = condition {
            conditions.push((condition, code));

            let r#else = opt(preceded(
                repeat::<_, _, (), _, _>(
                    0..,
                    alt((my_space1.value(()), line_ending.value(()), ':'.value(())))
                ),
                (Caseless(b"ELSE"), my_space0)
            ))
            .parse_next(input)?;
            if r#else.is_none() {
                break;
            }
        }
        else {
            else_clause = Some(code);
            break;
        }
    }

    let _ = (
        opt(alt((
            delimited(my_space0, ':', my_space0).value(()),
            delimited(my_space0, parse_comment, line_ending).value(())
        ))),
        cut_err(preceded(
            my_space0,
            parse_directive_word(if is_orgams { b"END" } else { b"ENDIF" })
        ))
        .take()
    )
        .parse_next(input)
        .map_err(|e| e.add_context(&if_clone, &if_start, "End directive not found"))?;

    let token = LocatedTokenInner::If(conditions, else_clause)
        .into_located_token_between(&if_start, *input);
    Ok(token)
}

pub fn parse_conditional_condition(
    code: KindOfConditional
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTestKind, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTestKind, Z80ParserError> {
        match &code {
            KindOfConditional::If => located_expr.map(LocatedTestKind::True).parse_next(input),

            KindOfConditional::IfNot => located_expr.map(LocatedTestKind::False).parse_next(input),

            KindOfConditional::IfDef => {
                preceded(my_space0, parse_label(false))
                    .map(|l| LocatedTestKind::LabelExists(l.into()))
                    .parse_next(input)
            },

            KindOfConditional::IfNdef => {
                parse_label(false)
                    .map(|l| LocatedTestKind::LabelDoesNotExist(l.into()))
                    .parse_next(input)
            },

            KindOfConditional::IfUsed => {
                parse_label(false)
                    .map(|l| LocatedTestKind::LabelUsed(l.into()))
                    .parse_next(input)
            },

            KindOfConditional::IfNused => {
                parse_label(false)
                    .map(|l| LocatedTestKind::LabelNused(l.into()))
                    .parse_next(input)
            },
        }
    }
}

pub fn parse_assembler_control(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    cut_err(
        alt((
            parse_assembler_control_print_parse,
            parse_assembler_control_print_any_pass
        ))
        .context(StrContext::Label(
            "Wrong argument in ASSEMBLING_CONTROL directive"
        ))
    )
    .parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_assembler_control_max_passes_number(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedToken, Z80ParserError> {
    let asmctrl_start = input.checkpoint();

    let _ = preceded(
        my_space0,
        alt((parse_directive_word(b"ASMCONTROLENV"), my_space1))
    )
    .parse_next(input)?;

    let count = cut_err(preceded(
        (
            parse_word(b"SET_MAX_NB_OF_PASSES")
                .context(StrContext::Label("Missing modified option")),
            (my_space0, b'=', my_space0).context(StrContext::Label("Missing ="))
        ),
        located_expr.context(StrContext::Label("Expression expected"))
    ))
    .parse_next(input)?;

    let inner = cut_err(inner_code.context(StrContext::Label(
        "ASMCONTROLENV SET_MAX_NB_OF_PASSES: issue in the content"
    )))
    .parse_next(input)?;

    let _ = cut_err(
        preceded(
            my_space0,
            alt((
                parse_directive_word(b"ENDASMCONTROLENV"),
                parse_directive_word(b"ENDA")
            ))
        )
        .context(StrContext::Label("ASMCONTROLENV: not closed"))
    )
    .parse_next(input)?;

    Ok(LocatedTokenInner::AssemblerControl(
        LocatedAssemblerControlCommand::RestrictedAssemblingEnvironment {
            passes: Some(count),
            lst: inner
        }
    )
    .into_located_token_between(&asmctrl_start, *input))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_assembler_control_print_any_pass(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    preceded(
        (parse_word(b"PRINT_ANY_PASS"), parse_comma),
        parse_print_inner
    )
    .map(|p| {
        LocatedTokenInner::AssemblerControl(LocatedAssemblerControlCommand::PrintAtAssemblingState(
            p
        ))
    })
    .parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_assembler_control_print_parse(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let input2: InnerZ80Span = *input;

    preceded((parse_word(b"PRINT_PARSE"), parse_comma), parse_print_inner)
        .map(|p| {
            let msg = p.iter().map(|e| e.to_string()).join("");
            let ctx = input
                .state
                .current_filename
                .as_ref()
                .map(|p| p.to_string())
                .unwrap_or_else(|| {
                    input
                        .state
                        .context_name()
                        .map(|c| c.to_owned())
                        .unwrap_or_default()
                });
            let (line, column) = Z80Span::from(input2).relative_line_and_column();
            println!("[PARSE] {ctx}:{line}:{column} {msg}");
            p
        })
        .map(|p| {
            LocatedTokenInner::AssemblerControl(
                LocatedAssemblerControlCommand::PrintAtParsingState(p)
            )
        })
        .parse_next(input)
}

pub fn parse_stable_ticker(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    alt((parse_stable_ticker_start, parse_stable_ticker_stop)).parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_stable_ticker_start(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    preceded(
        (Caseless("start"), alt((my_space1, parse_comma))),
        cut_err(parse_label(false).context(StrContext::Label("Missing label")))
    )
    .map(|name| LocatedTokenInner::StableTicker(StableTickerAction::<Z80Span>::Start(name.into())))
    .parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_stable_ticker_stop(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    Caseless("stop").parse_next(input)?;

    let name = opt(preceded(
        alt((my_space1, parse_comma)),
        parse_label(false).map(Z80Span::from)
    ))
    .parse_next(input)?;

    Ok(LocatedTokenInner::StableTicker(
        StableTickerAction::<Z80Span>::Stop(name)
    ))
}

#[derive(PartialEq)]
pub enum DbDwStr {
    Abyte,
    Db,
    Dw,
    Str
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_db_or_dw_or_str(
    code: DbDwStr,
    empty_list_allowed: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let abyte_delta = if code == DbDwStr::Abyte {
            Some(
                cut_err(
                    terminated(located_expr, parse_comma)
                        .context(StrContext::Label("ABYTE: delta issue"))
                )
                .parse_next(input)?
            )
        }
        else {
            None
        };

        let expr = if empty_list_allowed {
            expr_list.parse_next(input).unwrap_or(Default::default())
        }
        else {
            expr_list
                .context(match code {
                    DbDwStr::Abyte => "ABYTE: error in arguments",
                    DbDwStr::Dw => "DEFW: error in arguments",
                    DbDwStr::Db => "DEFB: error in arguments",
                    DbDwStr::Str => "STR: error in arguments"
                })
                .parse_next(input)?
        };

        Ok(match code {
            DbDwStr::Db => LocatedTokenInner::Defb(expr),
            DbDwStr::Dw => LocatedTokenInner::Defw(expr),
            DbDwStr::Str => LocatedTokenInner::Str(expr),
            DbDwStr::Abyte => LocatedTokenInner::Abyte(abyte_delta.unwrap(), expr)
        })
    }
}

pub fn parse_macro_arg(input: &mut InnerZ80Span) -> ModalResult<LocatedMacroParam, Z80ParserError> {
    let _start_input = input.checkpoint();
    let cloned = *input;

    let param = alt((
        delimited(
            (my_space0, ('[')),
            separated(0.., parse_macro_arg, ','),
            ((']'), my_space0)
        )
        .map(|l: Vec<LocatedMacroParam>| {
            LocatedMacroParam::List(
                l.into_iter()
                    .map(|p| Box::new(p.clone()))
                    .collect::<Vec<_>>()
            )
        }),
        delimited(
            my_space0,
            (
                opt(Caseless("{eval}").value(())),
                alt((
                    located_expr.take(),
                    parse_string.take(),
                    repeat::<_, _, (), _, _>(
                        0..,
                        none_of((b' ', b',', b'\r', b'\n', b'\t', b']', b'[', b';', b':'))
                    )
                    .take()
                ))
            ),
            alt((my_space0.value(()), eof.value(())))
        )
        .map(|(eval, s)| (eval.is_some(), cloned.update_slice(s)))
        .map(|(eval, arg)| (eval, Z80Span::from(arg)))
        .map(|(eval, arg)| {
            if eval {
                LocatedMacroParam::EvaluatedArgument(arg)
            }
            else {
                LocatedMacroParam::RawArgument(arg)
            }
        })
    ))
    .parse_next(input)?;

    Ok(param)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_macro_or_struct_call_inner(
    for_struct: bool,
    name: InnerZ80Span
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| {
        let input_start = input.checkpoint();

        my_space0.parse_next(input)?;
        not(':').parse_next(input)?;

        if !ignore_ascii_case_allowed_label(
            name.as_bstr(),
            input.state.options().dotted_directive,
            input.state.options().assembler_flavor
        ) {
            return Err(ErrMode::Backtrack(
                Z80ParserError::from_input(input).add_context(
                    input,
                    &input_start,
                    if for_struct {
                        "STRUCT: forbidden name"
                    }
                    else {
                        "MACRO or STRUCT: forbidden name"
                    }
                )
            ));
        }

        let _ = (my_space0, not(parse_comment)).parse_next(input)?;

        let has_parenthesis = opt((
            '(',
            my_space0,
            not(alt((("void", my_space0).value(()), ')'.value(()))))
        ))
        .parse_next(input)?
        .is_some();
        let args: Vec<(LocatedMacroParam, &[u8])> = if peek(alt((
            eof::<_, Z80ParserError>.value(()),
            parse_comment.value(()),
            '\n'.value(()),
            ':'.value(())
        )))
        .parse_next(input)
        .is_ok()
        {
            vec![]
        }
        else {
            cut_err(
                alt((
                    delimited(
                        my_space0,
                        alt((
                            "()".value(()),
                            Caseless("(void)").value(()),
                            parse_comment.value(())
                        )),
                        my_space0
                    )
                    .value(Default::default()),
                    alt((
                        alt((Caseless("(void)"), "()")).value(Vec::new()),
                        separated(
                            1..,
                            alt((
                                parse_macro_arg.with_taken(),
                                my_space1
                                    .map(|space: InnerZ80Span| {
                                        LocatedMacroParam::RawArgument(space.into())
                                    })
                                    .with_taken()
                            )),
                            parse_comma
                        )
                    ))
                ))
                .context(if for_struct {
                    "STRUCT: error in arguments list"
                }
                else {
                    "MACRO or STRUCT: forbidden name"
                })
            )
            .parse_next(input)?
        };

        if has_parenthesis {
            (my_space0, ')', my_space0).parse_next(input)?;
        }

        if args.len() == 1 && args.first().unwrap().0.is_empty() {
            panic!();
        }

        if args.len() == 1 {
            let mut arg = (*input).update_slice(args[0].1);
            if alt((parse_word(b"NOP").take(), parse_opcode_no_arg.take()))
                .parse_next(&mut arg)
                .is_ok()
            {
                return Err(ErrMode::Cut(Z80ParserError::from_input(input).add_context(
                    input,
                    &input_start,
                    if for_struct {
                        "First argument of STRUCT cannot be an opcode with no argument"
                    }
                    else {
                        "First argument of MACRO or STRUCT cannot be an opcode with no argument"
                    }
                )));
            }
        }

        let args = args.into_iter().map(|(a, _b)| a).collect_vec();
        Ok(LocatedTokenInner::MacroCall(name.into(), args))
    }
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_macro_or_struct_call(
    _allowed_to_return_a_label: bool,
    for_struct: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedToken, Z80ParserError> {
        my_space0(input)?;
        let input_start = input.checkpoint();
        let name = terminated(
            parse_macro_name,
            not(alt((
                (
                    my_space0,
                    alt((':'.value(()), line_ending.value(()), eof.value(())))
                )
                    .take(),
                ('.').take()
            )))
        )
        .parse_next(input)?;

        if !ignore_ascii_case_allowed_label(
            name.as_bstr(),
            input.state.options().dotted_directive,
            input.state.options().assembler_flavor
        ) {
            return Err(ErrMode::Backtrack(
                Z80ParserError::from_input(input).add_context(
                    input,
                    &input_start,
                    if for_struct {
                        "STRUCT: forbidden name"
                    }
                    else {
                        "MACRO or STRUCT: forbidden name"
                    }
                )
            ));
        }

        let inner = parse_macro_or_struct_call_inner(for_struct, name).parse_next(input)?;
        let inner = inner.into_located_token_between(&input_start, *input);
        Ok(inner)
    }
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_directive_word(
    name: &'static [u8]
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<InnerZ80Span, Z80ParserError> + 'static {
    move |input: &mut InnerZ80Span| {
        if input.state.options().dotted_directive {
            preceded(b'.', parse_word(name)).parse_next(input)
        }
        else {
            parse_word(name).parse_next(input)
        }
    }
}

pub fn parse_macro_name(input: &mut InnerZ80Span) -> ModalResult<InnerZ80Span, Z80ParserError> {
    let dotted_directive = input.state.options().dotted_directive;
    let flavor = input.state.options().assembler_flavor;

    let name = (
        one_of((b'a'..=b'z', b'A'..=b'Z', b'_')),
        take_while(0.., (b'a'..=b'z', b'A'..=b'Z', b'0'..=b'9', b'_')),
        not('{')
    )
        .take()
        .verify(move |name: &[u8]| {
            !(!ignore_ascii_case_allowed_label(name, dotted_directive, flavor))
        })
        .parse_next(input)?;

    Ok((*input).update_slice(name))
}

pub fn parse_rorg(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let _ = my_space0.parse_next(input)?;
    let rorg_start = input.checkpoint();
    let _ = alt((Caseless("PHASE"), Caseless("RORG"))).parse_next(input)?;

    let exp = cut_err(
        delimited(my_space1, located_expr, my_space0)
            .context(StrContext::Label("RORG: error in the expression"))
    )
    .parse_next(input)?;

    let _ = my_line_ending.parse_next(input)?;

    let inner = inner_code.parse_next(input)?;

    let _ = cut_err(
        preceded(my_space0, alt((Caseless("DEPHASE"), Caseless("REND"))))
            .context(StrContext::Label("RORG: missing REND"))
    )
    .parse_next(input)?;

    let _rorg_stop = input.checkpoint();
    let token = LocatedTokenInner::Rorg(exp, inner).into_located_token_between(&rorg_start, *input);
    Ok(token)
}

pub fn parse_undef(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let label = parse_label(false).parse_next(input)?;

    Ok(LocatedTokenInner::Undef(label.into()))
}

pub fn parse_section(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let name = preceded(my_space0, parse_label(false)).parse_next(input)?;

    Ok(LocatedTokenInner::Section(name.into()))
}

/// parse write direct in memory / converted to a bank directive
/// we do not care of the parameters for roms as we are not working in an emulator
pub fn parse_write_direct_memory(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    // filter all the stuff before
    let _ = (
        Caseless("DIRECT"),
        my_space1,
        Caseless("-1"),
        parse_comma,
        Caseless("-1"),
        parse_comma
    )
        .parse_next(input)?;

    let bank =
        cut_err(located_expr.context(StrContext::Label("WRITE DIRECT -1, -1: BANK expected")))
            .parse_next(input)?;

    let token = LocatedTokenInner::Bank(Some(bank));

    Ok(LocatedTokenInner::WarningWrapper(
        Box::new(token),
        "Prefer BANK or PAGE directives to write direct -1, -1, XX".to_owned()
    ))
}

macro_rules! directive_with_expr {
    ($name:ident, $enum:tt) => {
        #[cfg_attr(not(target_arch = "wasm32"), inline)]
        #[cfg_attr(target_arch = "wasm32", inline(never))]
        pub fn $name(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
            let exp = located_expr.parse_next(input)?;

            Ok((LocatedTokenInner::$enum(exp)))
        }
    };
}

directive_with_expr!(parse_map, Map);
directive_with_expr!(parse_limit, Limit);
directive_with_expr!(parse_waitnops, WaitNops);
directive_with_expr!(parse_return, Return);

#[cfg(test)]
mod tests {
    use cpclib_tokens::{Expr, FormattedExpr};

    use super::*;
    use crate::parser::parser::test::parse_test;

    #[test]
    fn test_parse_assert_cases() {
        let cases = vec![
            ("42", true),
            ("1+2, 'ok'", true),
            ("0, {HEX2} 255", true),
            (
                "(BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES) || (BREAKPOINT_METHOD == BREAKPOINT_WITH_SNAPSHOT_MODIFICATION)",
                true
            ),
            (
                " (BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES) || (BREAKPOINT_METHOD == BREAKPOINT_WITH_SNAPSHOT_MODIFICATION)",
                true
            ),
            (
                "   (BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES) || (BREAKPOINT_METHOD == BREAKPOINT_WITH_SNAPSHOT_MODIFICATION)",
                true
            ),
            ("", false),
        ];

        for (input, expect_ok) in cases {
            let res = parse_test(parse_assert, input);
            if expect_ok {
                assert!(res.res.is_ok(), "Should parse successfully: {}", input);
            }
            else {
                assert!(res.res.is_err(), "Should error on input: {}", input);
            }
        }
    }
}
