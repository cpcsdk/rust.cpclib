use cpclib_common::itertools::Itertools;
use cpclib_common::winnow::ascii::{line_ending, Caseless};
use cpclib_common::winnow::combinator::{
    alt, cut_err, eof, not, opt, preceded, repeat, terminated
};
use cpclib_common::winnow::error::{ContextError, ParserError, StrContext};
use cpclib_common::winnow::stream::AsChar;
use cpclib_common::winnow::token::{one_of, take_while};
/// ! Locomotive basic parser routines.
use cpclib_common::winnow::{ModalParser, *};
use paste::paste;

use crate::tokens::*;
use crate::{BasicError, BasicLine, BasicProgram};

type BasicSeveralTokensResult<'src> = ModalResult<Vec<BasicToken>, ContextError<StrContext>>;
type BasicOneTokenResult<'src> = ModalResult<BasicToken, ContextError<StrContext>>;
type BasicLineResult<'src> = ModalResult<BasicLine, ContextError<StrContext>>;

/// Parse complete basic program"],
pub fn parse_basic_program(input: &mut &str) -> ModalResult<BasicProgram, ContextError<StrContext>> {
    repeat(0.., parse_basic_line)
        .map(BasicProgram::new)
        .parse_next(input)
}

/// Parse a line
pub fn parse_basic_line<'src>(input: &mut &'src str) -> BasicLineResult<'src> {
    // get the number
    let line_number = dec_u16_inner
        .context(StrContext::Label("Wrong line number"))
        .parse_next(input)?;

    // forget the first space
    ' '.context(StrContext::Label("Missing space"))
        .parse_next(input)?;

    // get the tokens
    let tokens = repeat(0.., (parse_instruction, alt((eof, line_ending, ":"))))
        .fold(Vec::new, |mut acc: Vec<_>, (mut item, next)| {
            acc.append(&mut item);
            if !next.is_empty() {
                let char = next.chars().next().unwrap();
                match char {
                    ':' => acc.push(BasicToken::SimpleToken(BasicTokenNoPrefix::CharColon)),
                    '\n' => {},
                    _ => panic!("char '{}' is unhandled", char)
                }
            }
            acc
        })
        .parse_next(input)?;

    Ok(BasicLine::new(line_number, &tokens))
}

/// Parse any instruction.
/// In opposite to BASIC editor, parameters are verified (i.e. generated BASIC is valid)
pub fn parse_instruction<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    let mut res = parse_space0(input)?;

    let mut instruction = alt((
        alt((parse_rem,)).map(|i| vec![i]),
        parse_call,
        parse_input,
        parse_print,
        parse_assign
    ))
    .context(StrContext::Label("Unable to parse an instruction"))
    .parse_next(input)?;

    res.append(&mut instruction);

    let mut extra_space = parse_space0.parse_next(input)?;
    res.append(&mut extra_space);

    Ok(res)
}

pub fn parse_assign<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    enum Kind {
        Float,
        Int,
        String
    };
    let var = alt((
        parse_string_variable.map(|v| (Kind::String, v)),
        parse_integer_variable.map(|v| (Kind::Int, v)),
        parse_float_variable.map(|v| (Kind::Float, v))
    ))
    .parse_next(input)?;

    let mut space = ((parse_space0, '=', parse_space0)).parse_next(input)?;

    let mut val = match var.0 {
        Kind::Float | Kind::Int => {
            cut_err(
                parse_numeric_expression(NumericExpressionConstraint::None)
                    .context(StrContext::Label("Numeric expression expected"))
            )
            .parse_next(input)?
        },
        Kind::String => {
            cut_err(
                parse_string_expression.context(StrContext::Label("String expression expected"))
            )
            .parse_next(input)?
        },
    };

    let mut res = var.1;
    res.append(&mut space.0);
    res.push(BasicToken::SimpleToken(space.1.into()));
    res.append(&mut space.2);
    res.append(&mut val);

    Ok(res)
}

/// Parse a comment"],
pub fn parse_rem<'src>(input: &mut &'src str) -> BasicOneTokenResult<'src> {
    let sym = alt((
        Caseless("REM").value(BasicTokenNoPrefix::Rem),
        '\''.value(BasicTokenNoPrefix::SymbolQuote)
    ))
    .parse_next(input)?;

    let list = take_while(0.., |ch| ch != '\n').parse_next(input)?;

    Ok(BasicToken::Comment(sym, list.as_bytes().to_vec()))
}

pub fn parse_space<'src>(input: &mut &'src str) -> BasicOneTokenResult<'src> {
    one_of([' ', '\t'])
        .map(|c: char| BasicToken::SimpleToken(c.into()))
        .parse_next(input)
}

pub fn parse_space0<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    repeat(0.., parse_space).parse_next(input)
}
pub fn parse_space1<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    repeat(1.., parse_space).parse_next(input)
}

pub fn parse_char<'src>(input: &mut &'src str) -> BasicOneTokenResult<'src> {
    one_of(|c: char| {
        "#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~".chars().any(|c2| c2==c)
    })
    .map(|c: char| BasicToken::SimpleToken(c.into()))
    .parse_next(input)
}

pub fn parse_quote<'src>(input: &mut &'src str) -> BasicOneTokenResult<'src> {
    '"'.value(BasicToken::SimpleToken(
        BasicTokenNoPrefix::ValueQuotedString
    ))
    .parse_next(input)
}

pub fn parse_canal<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    (
        '#'.value(BasicToken::SimpleToken('#'.into())),
        one_of('0'..='7').map(|c| {
            BasicToken::SimpleToken(match c {
                '0' => BasicTokenNoPrefix::ConstantNumber0,
                '1' => BasicTokenNoPrefix::ConstantNumber1,
                '2' => BasicTokenNoPrefix::ConstantNumber2,
                '3' => BasicTokenNoPrefix::ConstantNumber3,
                '4' => BasicTokenNoPrefix::ConstantNumber4,
                '5' => BasicTokenNoPrefix::ConstantNumber5,
                '6' => BasicTokenNoPrefix::ConstantNumber6,
                '7' => BasicTokenNoPrefix::ConstantNumber7,
                _ => unreachable!()
            })
        })
    )
        .map(|(a, b)| vec![a, b])
        .parse_next(input)
}

pub fn parse_quoted_string<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    let start = parse_quote.parse_next(input)?;
    let mut content = repeat(0.., alt((parse_char, parse_space)))
        .fold(Vec::new, |mut acc, new| {
            acc.push(new);
            acc
        })
        .parse_next(input)?;
    let stop =
        cut_err(parse_quote.context(StrContext::Label("Unclosed string"))).parse_next(input)?;

    let mut res = vec![start];
    res.append(&mut content);
    res.push(stop);

    Ok(res)
}

/// Parse a comma optionally surrounded by space
pub fn parse_comma<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    let mut data = (
        parse_space0,
        ','.map(|c: char| BasicToken::SimpleToken(c.into())),
        parse_space0
    )
        .parse_next(input)?;

    data.0.push(data.1);
    data.0.append(&mut data.2);

    Ok(data.0)
}

/// Parse the Args SPC or TAB of a print expression
pub fn parse_print_arg_spc_or_tab<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    let (kind, open, param, close, mut space) = (
        alt((Caseless("SPC"), Caseless("TAB"))),
        '(',
        parse_decimal_value_16bits,
        ')',
        parse_space0
    )
        .parse_next(input)?;

    let mut tokens = kind
        .chars()
        .map(|c| BasicToken::SimpleToken(c.to_ascii_uppercase().into()))
        .collect_vec();
    tokens.push(BasicToken::SimpleToken(open.into()));
    tokens.push(param);
    tokens.push(BasicToken::SimpleToken(close.into()));
    tokens.append(&mut space);

    Ok(tokens)
}

/// Parse using argument of a print expression
pub fn parse_print_arg_using<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    let (using, mut space_a, mut format, mut space_b, sep, mut space_c) = (
        Caseless("USING"),
        parse_space0,
        cut_err(
            alt((
                parse_quoted_string, // TODO add filtering because this string is special
                parse_string_variable
            ))
            .context(StrContext::Label("FORMAT expected"))
        ),
        parse_space0,
        cut_err(one_of([',', ';']).context(StrContext::Label("; or , expected"))),
        parse_space0
    )
        .parse_next(input)?;

    let mut tokens = using
        .chars()
        .map(|c| BasicToken::SimpleToken(c.to_ascii_uppercase().into()))
        .collect_vec();
    tokens.append(&mut space_a);
    tokens.append(&mut format);
    tokens.append(&mut space_b);
    tokens.push(BasicToken::SimpleToken(sep.into()));
    tokens.append(&mut space_c);

    Ok(tokens)
}

pub fn parse_variable<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    alt((parse_string_variable, parse_integer_variable)).parse_next(input)
}

pub fn parse_string_variable<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    let name = terminated(parse_base_variable_name, '$').parse_next(input)?;

    let mut tokens = name;
    tokens.push(BasicToken::SimpleToken('$'.into()));

    Ok(tokens)
}

pub fn parse_integer_variable<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    let name = terminated(parse_base_variable_name, '%').parse_next(input)?;

    let mut tokens = name;
    tokens.push(BasicToken::SimpleToken('%'.into()));

    Ok(tokens)
}

pub fn parse_float_variable<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    let name = (parse_base_variable_name, opt('!')).parse_next(input)?;

    let mut tokens = name.0;
    if name.1.is_some() {
        tokens.push(BasicToken::SimpleToken('!'.into()));
    }

    Ok(tokens)
}

pub fn parse_base_variable_name<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    let first = one_of(('a'..='z', 'A'..='Z')).parse_next(input)?;

    let next =
        opt(take_while(0.., ('a'..='z', 'A'..='Z', '0'..='9')).verify(|s: &str| s.len() < 39))
            .parse_next(input)?;

    // TODO check that it is valid

    let mut tokens = vec![BasicToken::SimpleToken(first.into())];
    if let Some(next) = next {
        tokens.extend(next.chars().map(|c| BasicToken::SimpleToken(c.into())));
    }

    Ok(tokens)
}

/// Parse a single expression of a print
pub fn parse_print_expression<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    let (prefix, mut expr) = (
        opt(alt((parse_print_arg_spc_or_tab, parse_print_arg_using))),
        alt((
            parse_quoted_string,
            parse_variable,
            parse_basic_value.map(|v| vec![v]),
            parse_numeric_expression(NumericExpressionConstraint::None)
        ))
        .context(StrContext::Label("Missing expression to print"))
    )
        .parse_next(input)?;

    let mut tokens = prefix.unwrap_or_default();
    tokens.append(&mut expr);

    Ok(tokens)
}

/// Parse a list of expressions for print
pub fn parse_print_stream_expression<'src>(
    input: &mut &'src str
) -> BasicSeveralTokensResult<'src> {
    let mut first = parse_print_expression.parse_next(input)?;

    let mut next: Vec<Vec<BasicToken>> = repeat(
        0..,
        (one_of([';', ',']), parse_space0, parse_print_expression).map(
            |(sep, mut space_a, mut expr)| {
                let mut inner = Vec::with_capacity(1 + space_a.len() + expr.len());
                inner.push(BasicToken::SimpleToken(sep.into()));
                inner.append(&mut space_a);
                inner.append(&mut expr);

                inner
            }
        )
    )
    .parse_next(input)?;

    for other in &mut next {
        first.append(other);
    }

    Ok(first)
}

/// Parse a complete and valid print expression
pub fn parse_print<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    // print keyword
    let _ = Caseless("PRINT").parse_next(input)?;

    // space after keyword
    let mut tokens = vec![BasicToken::SimpleToken(BasicTokenNoPrefix::Print)];
    let mut space = parse_space0.parse_next(input)?;
    tokens.append(&mut space);

    // canal and space
    let canal = opt(parse_canal).parse_next(input)?;

    if let Some(mut canal) = canal {
        tokens.append(&mut canal);

        let mut comma = parse_comma.parse_next(input)?;
        tokens.append(&mut comma);
    }

    // list of expressions
    let exprs = opt(parse_print_stream_expression).parse_next(input)?;
    if let Some(mut exprs) = exprs {
        tokens.append(&mut exprs);
    }

    Ok(tokens)
}

pub fn parse_call<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    let (_, mut space_a, mut address) = (
        Caseless("CALL"),
        parse_space1,
        cut_err(
            parse_numeric_expression(NumericExpressionConstraint::Integer)
                .context(StrContext::Label("Address expected"))
        )
    )
        .parse_next(input)?;

    // TODO implement the optional arguments list

    let mut res = vec![BasicToken::SimpleToken(BasicTokenNoPrefix::Call)];
    res.append(&mut space_a);
    res.append(&mut address);

    Ok(res)
}

pub fn parse_input<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    let (_, mut space_a, canal, mut space_b, sep, mut space_c, string, args): (
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        Vec<_>
    ) = (
        Caseless("INPUT"),
        parse_space1,
        opt(parse_canal),
        parse_space0,
        opt(';'),
        parse_space0,
        opt(parse_quoted_string),
        repeat(1.., (parse_space0, ';', parse_space0, parse_variable))
    )
        .parse_next(input)?;

    // TODO implement the optional arguments list

    let mut res = vec![BasicToken::SimpleToken(BasicTokenNoPrefix::Input)];
    res.append(&mut space_a);
    if let Some(mut canal) = canal {
        res.append(&mut canal)
    };
    res.append(&mut space_b);
    if let Some(sep) = sep {
        res.push(BasicToken::SimpleToken(sep.into()))
    };
    res.append(&mut space_c);
    if let Some(mut string) = string {
        res.append(&mut string)
    };

    for mut arg in args.into_iter() {
        res.append(&mut arg.0);
        res.push(BasicToken::SimpleToken(arg.1.into()));
        res.append(&mut arg.2);
        res.append(&mut arg.3);
    }

    Ok(res)
}

/// TODO add the missing chars
// pub fn parse_char<'src>(input:&mut &'src str) -> BasicOneTokenResult<'src>{
// map(
// alt((
// alt((
// map(char(':'), |_| BasicTokenNoPrefix::StatementSeparator),
// map(char(' '), |_| BasicTokenNoPrefix::CharSpace),
// map(char('A'), |_| BasicTokenNoPrefix::CharUpperA),
// map(char('B'), |_| BasicTokenNoPrefix::CharUpperB),
// map(char('C'), |_| BasicTokenNoPrefix::CharUpperC),
// map(char('D'), |_| BasicTokenNoPrefix::CharUpperD),
// map(char('E'), |_| BasicTokenNoPrefix::CharUpperE),
// map(char('F'), |_| BasicTokenNoPrefix::CharUpperF),
// map(char('G'), |_| BasicTokenNoPrefix::CharUpperG),
// map(char('H'), |_| BasicTokenNoPrefix::CharUpperH),
// map(char('I'), |_| BasicTokenNoPrefix::CharUpperI),
// map(char('J'), |_| BasicTokenNoPrefix::CharUpperJ),
// map(char('K'), |_| BasicTokenNoPrefix::CharUpperK),
// map(char('L'), |_| BasicTokenNoPrefix::CharUpperL),
// map(char('M'), |_| BasicTokenNoPrefix::CharUpperM),
// map(char('N'), |_| BasicTokenNoPrefix::CharUpperN),
// map(char('O'), |_| BasicTokenNoPrefix::CharUpperO),
// map(char('P'), |_| BasicTokenNoPrefix::CharUpperP),
// map(char('Q'), |_| BasicTokenNoPrefix::CharUpperQ),
// map(char('R'), |_| BasicTokenNoPrefix::CharUpperR)
// )),
// alt((
// map(char('S'), |_| BasicTokenNoPrefix::CharUpperS),
// map(char('T'), |_| BasicTokenNoPrefix::CharUpperT),
// map(char('U'), |_| BasicTokenNoPrefix::CharUpperU),
// map(char('V'), |_| BasicTokenNoPrefix::CharUpperV),
// map(char('W'), |_| BasicTokenNoPrefix::CharUpperW),
// map(char('X'), |_| BasicTokenNoPrefix::CharUpperX),
// map(char('Y'), |_| BasicTokenNoPrefix::CharUpperY),
// map(char('Z'), |_| BasicTokenNoPrefix::CharUpperZ)
// )),
// alt((
// map(char('a'), |_| BasicTokenNoPrefix::CharLowerA),
// map(char('b'), |_| BasicTokenNoPrefix::CharLowerB),
// map(char('c'), |_| BasicTokenNoPrefix::CharLowerC),
// map(char('d'), |_| BasicTokenNoPrefix::CharLowerD),
// map(char('e'), |_| BasicTokenNoPrefix::CharLowerE),
// map(char('f'), |_| BasicTokenNoPrefix::CharLowerF),
// map(char('g'), |_| BasicTokenNoPrefix::CharLowerG),
// map(char('h'), |_| BasicTokenNoPrefix::CharLowerH),
// map(char('i'), |_| BasicTokenNoPrefix::CharLowerI),
// map(char('j'), |_| BasicTokenNoPrefix::CharLowerJ),
// map(char('k'), |_| BasicTokenNoPrefix::CharLowerK),
// map(char('l'), |_| BasicTokenNoPrefix::CharLowerL),
// map(char('m'), |_| BasicTokenNoPrefix::CharLowerM),
// map(char('n'), |_| BasicTokenNoPrefix::CharLowerN),
// map(char('o'), |_| BasicTokenNoPrefix::CharLowerO)
// )),
// alt((
// map(char('p'), |_| BasicTokenNoPrefix::CharLowerP),
// map(char('q'), |_| BasicTokenNoPrefix::CharLowerQ),
// map(char('r'), |_| BasicTokenNoPrefix::CharLowerR),
// map(char('s'), |_| BasicTokenNoPrefix::CharLowerS),
// map(char('t'), |_| BasicTokenNoPrefix::CharLowerT),
// map(char('u'), |_| BasicTokenNoPrefix::CharLowerU),
// map(char('v'), |_| BasicTokenNoPrefix::CharLowerV),
// map(char('w'), |_| BasicTokenNoPrefix::CharLowerW),
// map(char('x'), |_| BasicTokenNoPrefix::CharLowerX),
// map(char('y'), |_| BasicTokenNoPrefix::CharLowerY),
// map(char('z'), |_| BasicTokenNoPrefix::CharLowerZ)
// ))
// )),
// |token| BasicToken::SimpleToken(token)
// )(input)
// }
/// Parse the instructions that do not need a prefix byte
/// TODO Add all the other instructions"],

/// Parse a basic value
pub fn parse_basic_value<'src>(input: &mut &'src str) -> BasicOneTokenResult<'src> {
    alt((parse_floating_point, parse_integer_value_16bits)).parse_next(input)
}

pub fn parse_string_expression<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    alt((
        parse_quoted_string,
        parse_chr_dollar,
        parse_lower_dollar,
        parse_upper_dollar,
        parse_space_dollar,
        parse_str_dollar
    ))
    .parse_next(input)
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum NumericExpressionConstraint {
    None,
    Integer
}

/// TODO check that some generated functions do not generate strings even if they consume numbers
pub fn parse_numeric_expression<'code>(
    constraint: NumericExpressionConstraint
) -> impl Fn(&mut &'code str) -> BasicSeveralTokensResult<'code> {
    // XXX Functions must be parsed first
    move |input: &mut &'code str| {
        match constraint {
            NumericExpressionConstraint::None => {
                alt((
                    parse_asc,
                    parse_val,
                    parse_len,
                    parse_all_generated_numeric_functions_any,
                    parse_all_generated_numeric_functions_int,
                    parse_basic_value.map(|v| vec![v]),
                    parse_integer_variable,
                    parse_float_variable
                ))
                .parse_next(input)
            },
            NumericExpressionConstraint::Integer => {
                alt((
                    parse_asc,
                    parse_val,
                    parse_len,
                    parse_all_generated_numeric_functions_int,
                    parse_integer_value_16bits.map(|v| vec![v]),
                    parse_integer_variable
                ))
                .parse_next(input)
            },
        }
    }
}

fn parse_any_string_function<'code>(
    name: &'static str,
    code: BasicToken
) -> impl Fn(&mut &'code str) -> BasicSeveralTokensResult<'code> {
    move |input: &mut &'code str| -> BasicSeveralTokensResult<'code> {
        let (code, mut space_a, open, mut expr, close) = (
            Caseless(name).map(|_| code.clone()),
            parse_space0,
            '(',
            cut_err(parse_string_expression.context(StrContext::Label("Wrong parameter"))),
            cut_err(')'.context(StrContext::Label("Missing ')'")))
        )
            .parse_next(input)?;

        let mut res = Vec::new();
        res.push(code);
        res.append(&mut space_a);
        res.push(BasicToken::SimpleToken(BasicTokenNoPrefix::from(open)));
        res.append(&mut expr);
        res.push(BasicToken::SimpleToken(BasicTokenNoPrefix::from(close)));

        Ok(res)
    }
}

macro_rules! generate_string_functions {
    (
            $($name:ident: $code:expr),+

    )=> {
            $(paste! {
                pub fn [<parse_ $name:lower>]<'src>(input:&mut  &'src str) -> BasicSeveralTokensResult<'src>{
                        parse_any_string_function(
                            stringify!($name),
                            $code,
                        ).parse_next(input)
                }
            })+

            pub fn parse_all_generated_string_functions<'src>(input:&mut  &'src str) -> BasicSeveralTokensResult<'src>{
                alt((
                    $(
                        paste!{[<parse_ $name:lower>]},
                    )+
                )).parse_next(input)
            }

};
}

generate_string_functions! {
    ASC: BasicToken::PrefixedToken(BasicTokenPrefixed::Asc),
    LEN: BasicToken::PrefixedToken(BasicTokenPrefixed::Len),
    VAL: BasicToken::PrefixedToken(BasicTokenPrefixed::Val)
}

/// works with float on the amstrad cpc
fn parse_chr_dollar<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    parse_any_numeric_function(
        "CHR$",
        BasicToken::PrefixedToken(BasicTokenPrefixed::ChrDollar),
        NumericExpressionConstraint::None
    )
    .parse_next(input)
}

fn parse_space_dollar<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    parse_any_numeric_function(
        "SPACE$",
        BasicToken::PrefixedToken(BasicTokenPrefixed::SpaceDollar),
        NumericExpressionConstraint::None
    )
    .parse_next(input)
}

fn parse_str_dollar<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    parse_any_numeric_function(
        "STR$",
        BasicToken::PrefixedToken(BasicTokenPrefixed::StrDollar),
        NumericExpressionConstraint::None
    )
    .parse_next(input)
}

fn parse_lower_dollar<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    parse_any_string_function(
        "LOWER$",
        BasicToken::PrefixedToken(BasicTokenPrefixed::LowerDollar)
    )
    .parse_next(input)
}

fn parse_upper_dollar<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    parse_any_string_function(
        "UPPER$",
        BasicToken::PrefixedToken(BasicTokenPrefixed::UpperDollar)
    )
    .parse_next(input)
}

fn parse_any_numeric_function<'code>(
    name: &'static str,
    code: BasicToken,
    constraint: NumericExpressionConstraint
) -> impl Fn(&mut &'code str) -> BasicSeveralTokensResult<'code> {
    move |input: &mut &'code str| -> BasicSeveralTokensResult<'code> {
        let (code, mut space_a, open, mut expr, close) = (
            Caseless(name).map(|_| code.clone()),
            parse_space0,
            '(',
            cut_err(
                parse_numeric_expression(constraint).context(StrContext::Label("Wrong parameter"))
            )
            .context(StrContext::Label("Wrong parameter")),
            cut_err(')'.context(StrContext::Label("Missing ')'")))
        )
            .parse_next(input)?;

        let mut res = Vec::new();
        res.push(code);
        res.append(&mut space_a);
        res.push(BasicToken::SimpleToken(BasicTokenNoPrefix::from(open)));
        res.append(&mut expr);
        res.push(BasicToken::SimpleToken(BasicTokenNoPrefix::from(close)));

        Ok(res)
    }
}

// pub fn parse_abs<'src>(input:&mut  &'src str) -> BasicSeveralTokensResult<'src>{
// parse_any_numeric_function(
// "ABS",
// BasicToken::PrefixedToken(BasicTokenPrefixed::Abs)
// )(input)
// }

macro_rules! generate_numeric_functions {
    ( $(
        $const:ty | $kind:ident => {
            $($name:ident: $code:expr),+
        }
      )+

    )=> {$(
            $(paste! {
                pub fn [<parse_ $name:lower>]<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
                        parse_any_numeric_function(
                            stringify!($name),
                            $code,
                            $const
                        ).parse_next(input)
                }
            })+


            paste! {
                    pub fn [<parse_all_generated_numeric_functions_ $kind >]<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
                    alt((
                        $(
                            paste!{[<parse_ $name:lower>]},
                        )+
                    )).parse_next(input)
                }
            }
        )+
};
}

// Generate all the functions that consume a numerical expression
generate_numeric_functions! {

    NumericExpressionConstraint::None | any  => {
        ABS: BasicToken::PrefixedToken(BasicTokenPrefixed::Abs),
        ATN: BasicToken::PrefixedToken(BasicTokenPrefixed::Atn),
        CINT: BasicToken::PrefixedToken(BasicTokenPrefixed::Cint),
        COS: BasicToken::PrefixedToken(BasicTokenPrefixed::Cos),
        CREAL: BasicToken::PrefixedToken(BasicTokenPrefixed::Creal),
        EXP: BasicToken::PrefixedToken(BasicTokenPrefixed::Exp),
        FIX: BasicToken::PrefixedToken(BasicTokenPrefixed::Fix),
        INP: BasicToken::PrefixedToken(BasicTokenPrefixed::Inp),
        INT: BasicToken::PrefixedToken(BasicTokenPrefixed::Int),
        LOG: BasicToken::PrefixedToken(BasicTokenPrefixed::Log),
        PEEK: BasicToken::PrefixedToken(BasicTokenPrefixed::Peek),
        SGN: BasicToken::PrefixedToken(BasicTokenPrefixed::Sign),
        SIN: BasicToken::PrefixedToken(BasicTokenPrefixed::Sin),
        SQ: BasicToken::PrefixedToken(BasicTokenPrefixed::Sq),
        SQR: BasicToken::PrefixedToken(BasicTokenPrefixed::Sqr),
        TAN: BasicToken::PrefixedToken(BasicTokenPrefixed::Tan),
        UNT: BasicToken::PrefixedToken(BasicTokenPrefixed::Unt)
    }

    NumericExpressionConstraint::Integer | int => {
        INKEY:  BasicToken::PrefixedToken(BasicTokenPrefixed::Inkey),
        JOY:  BasicToken::PrefixedToken(BasicTokenPrefixed::Joy)
    }
}

// implementation stolen to https://github.com/EdouardBERGE/rasm/blob/master/rasm.c#L2295
pub fn f32_to_amstrad_float(nb: f64) -> Result<[u8; 5], BasicError> {
    let mut bits = [false; 32];
    let mut res = [0; 5];

    let (is_pos, nb) = if nb >= 0f64 { (true, nb) } else { (false, -nb) };

    let deci = nb.trunc() as u64;
    let _fract = nb.fract();

    let mut bitpos = 0;
    let mut exp: i32 = 0;
    let mut mantissa: u64 = 0;
    let mut mask: u64 = 0x80000000;

    if deci >= 1 {
        // nb is >=1
        mask = 0x80000000;

        // search for the first (from the left) bit to 1
        while (deci & mask) == 0 {
            mask /= 2;
        }
        // count the number of ContextErroraining bits
        while mask > 0 {
            exp += 1;
            mask /= 2;
        }
        // build the mantissa part of the decimal value
        mantissa = (nb * 2f64.powi(32 - exp) + 0.5) as _;
        if (mantissa & 0xFF00000000) != 0 {
            mantissa = 0xFFFFFFFF
        };

        mask = 0x80000000;
        while mask != 0 {
            bits[bitpos] = (mantissa & mask) != 0;
            bitpos += 1;
            mask /= 2;
        }
    }
    else {
        // <1
        if nb == 0.0 {
            exp = -128;
        }
        else {
            mantissa = (nb * 4294967296.0 + 0.5) as _; // as v is ALWAYS <1.0 we never reach the 32 bits maximum
            if (mantissa & 0xFF00000000) != 0 {
                mantissa = 0xFFFFFFFF;
            }

            mask = 0x80000000;
            // find first significant bit of fraction part
            while (mantissa & mask) == 0 {
                mask /= 2;
                exp -= 1;
            }

            mantissa = (nb * 2.0f64.powi(32 - exp) + 0.5) as _; // as v is ALWAYS <1.0 we never reach the 32 bits maximum
            if (mantissa & 0xFF00000000) != 0 {
                mantissa = 0xFFFFFFFF;
            }

            mask = 0x80000000;
            while mask != 0 {
                bits[bitpos] = (mantissa & mask) != 0;
                bitpos += 1;
                mask /= 2;
            }
        }
    }

    {
        // generate the mantissa bytes
        let mut ib: usize = 3;
        let mut ibb: u8 = 0x80;
        for j in 0..bitpos {
            if bits[j] {
                res[ib] |= ibb;
            }
            ibb /= 2;
            if ibb == 0 {
                ibb = 0x80;
                if ib != 0 {
                    ib -= 1
                }
                else {
                    debug_assert!(j == bitpos - 1);
                };
            }
        }
    }

    {
        // generate the exponent
        exp += 128;
        if !(0..=255).contains(&exp) {
            return Err(BasicError::ExponentOverflow);
        }
        else {
            res[4] = exp as _;
        }
    }

    {
        // Generate the sign bit
        if is_pos {
            res[3] &= 0x7F;
        }
        else {
            res[3] |= 0x80;
        }
    }

    Ok(res)
}

pub fn parse_floating_point<'src>(input: &mut &'src str) -> BasicOneTokenResult<'src> {
    let nb = (opt('-'), dec_u16_inner, '.', dec_u16_inner)
        .recognize()
        .map(|nb| f32_to_amstrad_float(nb.parse::<f64>().unwrap()))
        .verify(|res| res.is_ok())
        .context(StrContext::Label("Unable to parse float"))
        .parse_next(input)?;

    let bytes = nb.unwrap();
    let res = BasicToken::Constant(
        BasicTokenNoPrefix::ValueFloatingPoint,
        BasicValue::Float(bytes[0], bytes[1], bytes[2], bytes[3], bytes[4])
    );

    Ok(res)
}

pub fn parse_integer_value_16bits<'src>(input: &mut &'src str) -> BasicOneTokenResult<'src> {
    alt((parse_decimal_value_16bits, parse_hexadecimal_value_16bits)).parse_next(input)
}

/// Parse an hexadecimal value
pub fn parse_hexadecimal_value_16bits<'src>(input: &mut &'src str) -> BasicOneTokenResult<'src> {
    (
        opt('-'),
        preceded(alt((Caseless("&h"), "&")), hex_u16_inner)
    )
        .map(|(neg, val)| {
            let val = val as i16;

            BasicToken::Constant(
                BasicTokenNoPrefix::ValueIntegerHexadecimal16bits,
                BasicValue::new_integer(if neg.is_some() { -val } else { val })
            )
        })
        .parse_next(input)
}

/// TODO: add binary number
pub fn parse_decimal_value_16bits<'src>(input: &mut &'src str) -> BasicOneTokenResult<'src> {
    (opt('-'), terminated(dec_u16_inner, not('.')))
        .map(|(neg, val)| {
            let val = val as i16;
            BasicToken::Constant(
                BasicTokenNoPrefix::ValueIntegerDecimal16bits,
                BasicValue::new_integer(if neg.is_some() { -val } else { val })
            )
        })
        .parse_next(input)
}

/// XXX stolen to the asm parser
#[inline]
pub fn hex_u16_inner(input: &mut &str) -> ModalResult<u16, ContextError> {
    take_while(1..=4, AsChar::is_hex_digit)
        .map(|parsed: &str| {
            let mut res = 0_u32;
            for digit in parsed.chars() {
                let value = digit.to_digit(16).unwrap_or(0);
                res = value + (res * 16);
            }
            res
        })
        .verify(|res| *res < u32::from(u16::max_value()))
        .map(|res| res as u16)
        .parse_next(input)
}

/// XXX stolen to the asm parser
#[inline]
pub fn dec_u16_inner(input: &mut &str) -> ModalResult<u16, ContextError> {
    take_while(1.., '0'..='9')
        .verify(|parsed: &str| parsed.len() <= 5)
        .map(|parsed: &str| {
            let mut res = 0_u32;
            for e in parsed.chars() {
                let digit = e;
                let value = digit.to_digit(10).unwrap_or(0);
                res = value + (res * 10);
            }
            res
        })
        .verify(|nb| *nb < u32::from(u16::max_value()))
        .map(|nb| nb as u16)
        .parse_next(input)
}

pub fn test_parse<'code, P: ModalParser<&'code str, Vec<BasicToken>, ContextError>>(
    mut parser: P,
    code: &'code str
) -> BasicLine {
    let tokens = dbg!(parser.parse(code)).expect("Parse issue");

    BasicLine {
        line_number: 10,
        tokens,
        forced_length: None
    }
}

pub fn test_parse1<'code, P: ModalParser<&'code str, BasicToken, ContextError>>(
    mut parser: P,
    code: &'code str
) -> BasicLine {
    let tokens = dbg!(parser.parse(code)).expect("Parse issue");

    BasicLine {
        line_number: 10,
        tokens: vec![tokens],
        forced_length: None
    }
}

pub fn test_parse_and_compare<'code, P: ModalParser<&'code str, Vec<BasicToken>, ContextError>>(
    parser: P,
    code: &'code str,
    bytes: &[u8]
) {
    let prog = test_parse(parser, code);
    assert_eq!(bytes, prog.tokens_as_bytes().as_slice())
}

#[cfg(test)]
mod test {
    use crate::string_parser::*;

    #[test]
    fn check_number() {
        assert!(dbg!(dec_u16_inner(&mut "10")).is_ok());

        assert!(dbg!(parse_floating_point(&mut "67.98")).is_ok());
        assert!(dbg!(parse_floating_point(&mut "-67.98")).is_ok());

        match hex_u16_inner(&mut "1234") {
            Ok(value) => {
                println!("{:x}", &value);
                assert_eq!(0x1234, value);
            },
            Err(e) => {
                panic!("{:?}", e);
            }
        }

        match parse_hexadecimal_value_16bits(&mut "&1234") {
            Ok(value) => {
                println!("{:?}", &value);
                let bytes = value.as_bytes();
                assert_eq!(
                    bytes[0],
                    BasicTokenNoPrefix::ValueIntegerHexadecimal16bits as u8
                );
                assert_eq!(bytes[1], 0x34);
                assert_eq!(bytes[2], 0x12);
            },
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    fn check_line_tokenisation(code: &str) -> BasicLine {
        let res = parse_basic_line.parse(code);
        match res {
            Ok(line) => {
                println!("{:?}", &line);
                line
            },
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    fn check_token_tokenisation(code: &str) {
        let res = parse_instruction.parse(code);
        match res {
            Ok(line) => {
                println!("{} => {:?}", code, &line);
            },
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    #[test]
    fn test_lines() {
        check_line_tokenisation("10 call &0\n");
        check_line_tokenisation("10 call &0  \n");
        check_line_tokenisation("10 call &0: call &0\n");
    }

    #[test]
    fn test_comment() {
        check_line_tokenisation("10 REM fldsfksjfksjkg");
        check_line_tokenisation("10 ' fldsfksjfksjkg");

        let _line = check_line_tokenisation("10 REM fldsfksjfksjkg:CALL\n");
    }

    fn check_expression(code: &str) {
        let res = parse_numeric_expression(NumericExpressionConstraint::None).parse(code);
        match res {
            Ok(line) => {
                println!("{} => {:?}", code, &line);
            },
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    fn check_print_expression(code: &str) {
        let res = parse_print_expression.parse(code);
        match res {
            Ok(line) => {
                println!("{} => {:?}", code, &line);
            },
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    #[test]
    fn test_expression() {
        let exprs = ["ATN(1)", "ABS(-67.98)"];

        for exp in exprs.into_iter() {
            check_expression(exp);
            check_print_expression(exp);
        }
    }
}
