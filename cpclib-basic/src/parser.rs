use cpclib_common::itertools::Itertools;
use cpclib_common::nom::branch::*;
use cpclib_common::nom::bytes::complete::*;
use cpclib_common::nom::character::complete::*;
use cpclib_common::nom::combinator::*;
use cpclib_common::nom::error::*;
use cpclib_common::nom::multi::*;
use cpclib_common::nom::sequence::*;
/// ! Locomotive basic parser routines.
use cpclib_common::nom::*;
use paste::paste;

use crate::tokens::*;
use crate::{BasicError, BasicLine, BasicProgram};

type BasicSeveralTokensResult<'src> = IResult<&'src str, Vec<BasicToken>, VerboseError<&'src str>>;
type BasicOneTokenResult<'src> = IResult<&'src str, BasicToken, VerboseError<&'src str>>;
type BasicLineResult<'src> = IResult<&'src str, BasicLine, VerboseError<&'src str>>;

/// Parse complete basic program"],
pub fn parse_basic_program(input: &str) -> IResult<&str, BasicProgram, VerboseError<&str>> {
    let (input, lines) = fold_many0(
        parse_basic_line,
        || Vec::new(),
        |mut acc: Vec<_>, item| {
            acc.push(item);
            acc
        }
    )(input)?;

    dbg!(Ok((input, BasicProgram::new(lines))))
}

/// Parse a line
pub fn parse_basic_line(input: &str) -> BasicLineResult {
    // get the number
    let (input, line_number) = dec_u16_inner(input)?;

    // forget the first space
    let (input, _) = char(' ')(input)?;

    // get the tokens
    let (input, tokens) = fold_many0(
        pair(parse_instruction, alt((eof, line_ending, tag(":")))),
        || Vec::new(),
        |mut acc: Vec<_>, (mut item, next)| {
            dbg!(&item);
            dbg!(&next);
            acc.append(&mut item);
            if !next.is_empty() {
                let char = next.chars().next().unwrap();
                match char {
                    ':' => acc.push(BasicToken::SimpleToken(BasicTokenNoPrefix::CharColon)),
                    '\n' => {}
                    _ => panic!("char '{}' is unhandled", char)
                }
            }
            acc
        }
    )(input)?;

    Ok((input, BasicLine::new(line_number, &tokens)))
}

/// Parse any instruction.
/// In opposite to BASIC editor, parameters are verified (i.e. generated BASIC is valid)
pub fn parse_instruction(input: &str) -> BasicSeveralTokensResult {
    let (input, mut res) = parse_space0(input)?;

    let (input, mut instruction) = context(
        "Unable to parse an instruction",
        alt((
            map(alt((parse_rem,)), |i| vec![i]),
            parse_call,
            parse_input,
            parse_print,
            parse_assign
        ))
    )(input)?;

    res.append(&mut instruction);

    let (input, mut extra_space) = parse_space0(input)?;
    res.append(&mut extra_space);

    dbg!(Ok((input, res)))
}

pub fn parse_assign(input: &str) -> BasicSeveralTokensResult {
    enum Kind {
        Float,
        Int,
        String
    };
    let (input, var) = alt((
        map(parse_string_variable, |v| (Kind::String, v)),
        map(parse_integer_variable, |v| (Kind::Int, v)),
        map(parse_float_variable, |v| (Kind::Float, v))
    ))(input)?;

    let (input, mut space) = tuple((parse_space0, char('='), parse_space0))(input)?;

    let (input, mut val) = match var.0 {
        Kind::Float | Kind::Int => {
            cut(context(
                "Numeric expression expected",
                parse_numeric_expression(NumericExpressionConstraint::None)
            ))(input)?
        }
        Kind::String => {
            cut(context(
                "String expression expected",
                parse_string_expression
            ))(input)?
        }
    };

    let mut res = var.1;
    res.append(&mut space.0);
    res.push(BasicToken::SimpleToken(space.1.into()));
    res.append(&mut space.2);
    res.append(&mut val);

    Ok((input, res))
}

/// Parse a comment"],
pub fn parse_rem(input: &str) -> BasicOneTokenResult {
    let (input, sym) = alt((
        map(tag_no_case("REM"), |_| BasicTokenNoPrefix::Rem),
        map(char('\''), |_| BasicTokenNoPrefix::SymbolQuote)
    ))(input)?;

    let (input, list) = take_till(|ch| ch == '\n')(input)?;

    Ok((input, BasicToken::Comment(sym, list.as_bytes().to_vec())))
}

pub fn parse_space(input: &str) -> BasicOneTokenResult {
    map(one_of(" \t"), |c| BasicToken::SimpleToken(c.into()))(input)
}

pub fn parse_space0(input: &str) -> BasicSeveralTokensResult {
    many0(parse_space)(input)
}
pub fn parse_space1(input: &str) -> BasicSeveralTokensResult {
    many1(parse_space)(input)
}

pub fn parse_char(input: &str) -> BasicOneTokenResult {
    map(
        one_of("#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~"),
        |c| BasicToken::SimpleToken(c.into())
    )(input)
}

pub fn parse_quote(input: &str) -> BasicOneTokenResult {
    map(char('"'), |_| {
        BasicToken::SimpleToken(BasicTokenNoPrefix::ValueQuotedString)
    })(input)
}

pub fn parse_canal(input: &str) -> BasicSeveralTokensResult {
    let (input, (a, b)) = pair(
        map(char('#'), |c| BasicToken::SimpleToken(c.into())),
        map(one_of("01234567"), |c| {
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
    )(input)?;

    Ok((input, vec![a, b]))
}

pub fn parse_quoted_string(input: &str) -> BasicSeveralTokensResult {
    let (input, start) = parse_quote(input)?;
    let (input, mut content) = fold_many0(
        alt((parse_char, parse_space)),
        || Vec::new(),
        |mut acc, new| {
            acc.push(new);
            acc
        }
    )(input)?;
    let (input, stop) = cut(context("Unclosed string", parse_quote))(input)?;

    let mut res = vec![start];
    res.append(&mut content);
    res.push(stop);

    Ok((input, res))
}

/// Parse a comma optionally surrounded by space
pub fn parse_comma(input: &str) -> BasicSeveralTokensResult {
    let (input, mut data) = tuple((
        parse_space0,
        map(char(','), |c| BasicToken::SimpleToken(c.into())),
        parse_space0
    ))(input)?;

    data.0.push(data.1);
    data.0.append(&mut data.2);

    Ok((input, data.0))
}

/// Parse the Args SPC or TAB of a print expression
pub fn parse_print_arg_spc_or_tab(input: &str) -> BasicSeveralTokensResult {
    let (input, (kind, open, param, close, mut space)) = tuple((
        alt((tag_no_case("SPC"), tag_no_case("TAB"))),
        char('('),
        parse_decimal_value_16bits,
        char(')'),
        parse_space0
    ))(input)?;

    let mut tokens = kind
        .chars()
        .map(|c| BasicToken::SimpleToken(c.to_ascii_uppercase().into()))
        .collect_vec();
    tokens.push(BasicToken::SimpleToken(open.into()));
    tokens.push(param);
    tokens.push(BasicToken::SimpleToken(close.into()));
    tokens.append(&mut space);

    Ok((input, tokens))
}

/// Parse using argument of a print expression
pub fn parse_print_arg_using(input: &str) -> BasicSeveralTokensResult {
    let (input, (using, mut space_a, mut format, mut space_b, sep, mut space_c)) = tuple((
        tag_no_case("USING"),
        parse_space0,
        cut(context(
            "FORMAT expected",
            alt((
                parse_quoted_string, // TODO add filtering because this string is special
                parse_string_variable
            ))
        )),
        parse_space0,
        cut(context("; or , expected", one_of(",;"))),
        parse_space0
    ))(input)?;

    let mut tokens = using
        .chars()
        .map(|c| BasicToken::SimpleToken(c.to_ascii_uppercase().into()))
        .collect_vec();
    tokens.append(&mut space_a);
    tokens.append(&mut format);
    tokens.append(&mut space_b);
    tokens.push(BasicToken::SimpleToken(sep.into()));
    tokens.append(&mut space_c);

    Ok((input, tokens))
}

pub fn parse_variable(input: &str) -> BasicSeveralTokensResult {
    alt((parse_string_variable, parse_integer_variable))(input)
}

pub fn parse_string_variable(input: &str) -> BasicSeveralTokensResult {
    let (input, name) = terminated(parse_base_variable_name, char('$'))(input)?;

    let mut tokens = name;
    tokens.push(BasicToken::SimpleToken('$'.into()));

    Ok((input, tokens))
}

pub fn parse_integer_variable(input: &str) -> BasicSeveralTokensResult {
    let (input, name) = terminated(parse_base_variable_name, char('%'))(input)?;

    let mut tokens = name;
    tokens.push(BasicToken::SimpleToken('%'.into()));

    Ok((input, tokens))
}

pub fn parse_float_variable(input: &str) -> BasicSeveralTokensResult {
    let (input, name) = pair(parse_base_variable_name, opt(char('!')))(input)?;

    let mut tokens = name.0;
    if let Some(_) = name.1 {
        tokens.push(BasicToken::SimpleToken('!'.into()));
    }

    Ok((input, tokens))
}

pub fn parse_base_variable_name(input: &str) -> BasicSeveralTokensResult {
    let (input, first) = one_of("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")(input)?;

    let (input, next) = opt(verify(
        is_a("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"),
        |s: &str| s.len() < 39
    ))(input)?;

    // TODO check that it is valid

    let mut tokens = vec![BasicToken::SimpleToken(first.into())];
    if let Some(next) = next {
        tokens.extend(next.chars().map(|c| BasicToken::SimpleToken(c.into())));
    }

    Ok((input, tokens))
}

/// Parse a single expression of a print
pub fn parse_print_expression(input: &str) -> BasicSeveralTokensResult {
    let (input, (prefix, mut expr)) = tuple((
        opt(alt((parse_print_arg_spc_or_tab, parse_print_arg_using))),
        context(
            "Missing expression to print",
            alt((
                parse_quoted_string,
                parse_variable,
                map(parse_basic_value, |v| vec![v]),
                parse_numeric_expression(NumericExpressionConstraint::None)
            ))
        )
    ))(input)?;

    let mut tokens = if let Some(prefix) = prefix {
        prefix
    }
    else {
        Vec::new()
    };
    tokens.append(&mut expr);

    Ok((input, tokens))
}

/// Parse a list of expressions for print
pub fn parse_print_stream_expression(input: &str) -> BasicSeveralTokensResult {
    let (input, mut first) = dbg!(parse_print_expression(input))?;

    let (input, mut next) = many0(map(
        tuple((one_of(";,"), parse_space0, parse_print_expression)),
        |(sep, mut space_a, mut expr)| {
            let mut inner = Vec::new();
            inner.push(BasicToken::SimpleToken(sep.into()));
            inner.append(&mut space_a);
            inner.append(&mut expr);

            inner
        }
    ))(input)?;

    for mut other in &mut next {
        first.append(&mut other);
    }

    Ok((input, first))
}

/// Parse a complete and valid print expression
pub fn parse_print(input: &str) -> BasicSeveralTokensResult {
    dbg!(input);

    // print keyword
    let (input, _) = tag_no_case("PRINT")(input)?;

    // space after keyword
    let mut tokens = vec![BasicToken::SimpleToken(BasicTokenNoPrefix::Print)];
    let (input, mut space) = parse_space0(input)?;
    tokens.append(&mut space);

    dbg!(input);

    // canal and space
    let (input, canal) = opt(parse_canal)(input)?;

    dbg!(input);
    let input = if let Some(mut canal) = canal {
        tokens.append(&mut canal);

        let (input, mut comma) = parse_comma(input)?;
        tokens.append(&mut comma);
        input
    }
    else {
        input
    };

    dbg!(input);

    // list of expressions
    let (input, exprs) = opt(parse_print_stream_expression)(input)?;
    if let Some(mut exprs) = exprs {
        tokens.append(&mut exprs);
    }

    Ok((input, tokens))
}

pub fn parse_call(input: &str) -> BasicSeveralTokensResult {
    let (input, (_, mut space_a, mut address)) = tuple((
        tag_no_case("CALL"),
        parse_space1,
        cut(context(
            "Address expected",
            parse_numeric_expression(NumericExpressionConstraint::Integer)
        ))
    ))(input)?;

    // TODO implement the optional arguments list

    let mut res = vec![BasicToken::SimpleToken(BasicTokenNoPrefix::Call)];
    res.append(&mut space_a);
    res.append(&mut address);

    Ok((input, res))
}

pub fn parse_input(input: &str) -> BasicSeveralTokensResult {
    let (input, (_, mut space_a, canal, mut space_b, sep, mut space_c, string, args)) =
        tuple((
            tag_no_case("INPUT"),
            parse_space1,
            opt(parse_canal),
            parse_space0,
            opt(char(';')),
            parse_space0,
            opt(parse_quoted_string),
            many1(tuple((
                parse_space0,
                char(';'),
                parse_space0,
                parse_variable
            )))
        ))(input)?;

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

    Ok((input, res))
}

/// TODO add the missing chars
// pub fn parse_char(input: &str) -> BasicOneTokenResult{
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
pub fn parse_basic_value(input: &str) -> BasicOneTokenResult {
    alt((parse_floating_point, parse_integer_value_16bits))(input)
}

pub fn parse_string_expression(input: &str) -> BasicSeveralTokensResult {
    alt((
        parse_quoted_string,
        parse_chr_dollar,
        parse_lower_dollar,
        parse_upper_dollar,
        parse_space_dollar,
        parse_str_dollar
    ))(input)
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum NumericExpressionConstraint {
    None,
    Integer
}

/// TODO check that some generated functions do not generate strings even if they consume numbers
pub fn parse_numeric_expression<'code>(
    constraint: NumericExpressionConstraint
) -> impl Fn(&'code str) -> BasicSeveralTokensResult<'code> {
    // XXX Functions must be parsed first
    move |input: &'code str| {
        match constraint {
            NumericExpressionConstraint::None => {
                alt((
                    parse_asc,
                    parse_val,
                    parse_len,
                    parse_all_generated_numeric_functions_any,
                    parse_all_generated_numeric_functions_int,
                    map(parse_basic_value, |v| vec![v]),
                    parse_integer_variable,
                    parse_float_variable
                ))(input)
            }
            NumericExpressionConstraint::Integer => {
                alt((
                    parse_asc,
                    parse_val,
                    parse_len,
                    parse_all_generated_numeric_functions_int,
                    map(parse_integer_value_16bits, |v| vec![v]),
                    parse_integer_variable
                ))(input)
            }
        }
    }
}

fn parse_any_string_function<'code>(
    name: &'static str,
    code: BasicToken
) -> impl Fn(&'code str) -> BasicSeveralTokensResult<'code> {
    move |input: &'code str| -> BasicSeveralTokensResult<'code> {
        let (input, (code, mut space_a, open, mut expr, close)) = tuple((
            map(tag_no_case(name), |_| code.clone()),
            parse_space0,
            char('('),
            cut(context("Wrong parameter", parse_string_expression)),
            cut(context("Missing ')'", char(')')))
        ))(input)?;

        let mut res = Vec::new();
        res.push(code);
        res.append(&mut space_a);
        res.push(BasicToken::SimpleToken(
            BasicTokenNoPrefix::from(open).into()
        ));
        res.append(&mut expr);
        res.push(BasicToken::SimpleToken(
            BasicTokenNoPrefix::from(close).into()
        ));

        Ok((input, res))
    }
}

macro_rules! generate_string_functions {
    (
            $($name:ident: $code:expr),+

    )=> {
            $(paste! {
                pub fn [<parse_ $name:lower>](input: &str) -> BasicSeveralTokensResult {
                        parse_any_string_function(
                            stringify!($name),
                            $code,
                        )(input)
                }
            })+

            pub fn parse_all_generated_string_functions(input: &str) -> BasicSeveralTokensResult {
                alt((
                    $(
                        paste!{[<parse_ $name:lower>]},
                    )+
                ))(input)
            }

};
}

generate_string_functions! {
    ASC: BasicToken::PrefixedToken(BasicTokenPrefixed::Asc),
    LEN: BasicToken::PrefixedToken(BasicTokenPrefixed::Len),
    VAL: BasicToken::PrefixedToken(BasicTokenPrefixed::Val)
}

/// works with float on the amstrad cpc
fn parse_chr_dollar(input: &str) -> BasicSeveralTokensResult {
    parse_any_numeric_function(
        "CHR$",
        BasicToken::PrefixedToken(BasicTokenPrefixed::ChrDollar),
        NumericExpressionConstraint::None
    )(input)
}

fn parse_space_dollar(input: &str) -> BasicSeveralTokensResult {
    parse_any_numeric_function(
        "SPACE$",
        BasicToken::PrefixedToken(BasicTokenPrefixed::SpaceDollar),
        NumericExpressionConstraint::None
    )(input)
}

fn parse_str_dollar(input: &str) -> BasicSeveralTokensResult {
    parse_any_numeric_function(
        "STR$",
        BasicToken::PrefixedToken(BasicTokenPrefixed::StrDollar),
        NumericExpressionConstraint::None
    )(input)
}

fn parse_lower_dollar(input: &str) -> BasicSeveralTokensResult {
    parse_any_string_function(
        "LOWER$",
        BasicToken::PrefixedToken(BasicTokenPrefixed::LowerDollar)
    )(input)
}

fn parse_upper_dollar(input: &str) -> BasicSeveralTokensResult {
    parse_any_string_function(
        "UPPER$",
        BasicToken::PrefixedToken(BasicTokenPrefixed::UpperDollar)
    )(input)
}

fn parse_any_numeric_function<'code>(
    name: &'static str,
    code: BasicToken,
    constraint: NumericExpressionConstraint
) -> impl Fn(&'code str) -> BasicSeveralTokensResult<'code> {
    move |input: &'code str| -> BasicSeveralTokensResult<'code> {
        let (input, (code, mut space_a, open, mut expr, close)) = tuple((
            map(tag_no_case(name), |_| code.clone()),
            parse_space0,
            char('('),
            cut(context(
                "Wrong parameter",
                parse_numeric_expression(constraint.clone())
            )),
            cut(context("Missing ')'", char(')')))
        ))(input)?;

        let mut res = Vec::new();
        res.push(code);
        res.append(&mut space_a);
        res.push(BasicToken::SimpleToken(
            BasicTokenNoPrefix::from(open).into()
        ));
        res.append(&mut expr);
        res.push(BasicToken::SimpleToken(
            BasicTokenNoPrefix::from(close).into()
        ));

        Ok((input, res))
    }
}

// pub fn parse_abs(input: &str) -> BasicSeveralTokensResult {
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
                pub fn [<parse_ $name:lower>](input: &str) -> BasicSeveralTokensResult {
                        parse_any_numeric_function(
                            stringify!($name),
                            $code,
                            $const
                        )(input)
                }
            })+


            paste! {
                    pub fn [<parse_all_generated_numeric_functions_ $kind >](input: &str) -> BasicSeveralTokensResult {
                    alt((
                        $(
                            paste!{[<parse_ $name:lower>]},
                        )+
                    ))(input)
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
        // count the number of remaining bits
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
        if exp < 0 || exp > 255 {
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

pub fn parse_floating_point(input: &str) -> BasicOneTokenResult {
    let (input, nb) = context(
        "Unable to parse float",
        verify(
            map(
                recognize(tuple((
                    opt(char('-')),
                    dec_u16_inner,
                    char('.'),
                    dec_u16_inner
                ))),
                |nb| f32_to_amstrad_float(nb.parse::<f64>().unwrap())
            ),
            |res| res.is_ok()
        )
    )(input)?;

    let bytes = nb.unwrap();
    let res = BasicToken::Constant(
        BasicTokenNoPrefix::ValueFloatingPoint,
        BasicValue::Float(bytes[0], bytes[1], bytes[2], bytes[3], bytes[4])
    );

    Ok((input, res))
}

pub fn parse_integer_value_16bits(input: &str) -> BasicOneTokenResult {
    alt((parse_decimal_value_16bits, parse_hexadecimal_value_16bits))(input)
}

/// Parse an hexadecimal value
pub fn parse_hexadecimal_value_16bits(input: &str) -> BasicOneTokenResult {
    map(
        pair(
            opt(char('-')),
            preceded(alt((tag("&"), tag_no_case("&h"))), hex_u16_inner)
        ),
        |(neg, val)| {
            let val = val as i16;

            BasicToken::Constant(
                BasicTokenNoPrefix::ValueIntegerHexadecimal16bits,
                BasicValue::new_integer(if neg.is_some() { -val } else { val })
            )
        }
    )(input)
}

/// TODO: add binary number
pub fn parse_decimal_value_16bits(input: &str) -> BasicOneTokenResult {
    map(
        pair(opt(char('-')), terminated(dec_u16_inner, not(char('.')))),
        |(neg, val)| {
            let val = val as i16;
            BasicToken::Constant(
                BasicTokenNoPrefix::ValueIntegerDecimal16bits,
                BasicValue::new_integer(if neg.is_some() { -val } else { val })
            )
        }
    )(input)
}

/// XXX stolen to the asm parser
#[inline]
pub fn hex_u16_inner(input: &str) -> IResult<&str, u16, VerboseError<&str>> {
    match is_a("0123456789abcdefABCDEF")(input) {
        Err(e) => Err(e),
        Ok((remaining, parsed)) => {
            // Do not parse more than  characters for a u16
            if parsed.input_len() > 4 {
                Err(cpclib_common::nom::Err::Error(error_position!(
                    input,
                    ErrorKind::OneOf
                )))
            }
            else {
                let mut res = 0_u32;
                for e in parsed.iter_elements() {
                    let digit = e;
                    let value = digit.to_digit(16).unwrap_or(0);
                    res = value + (res * 16);
                }
                if res > u32::from(u16::max_value()) {
                    Err(cpclib_common::nom::Err::Error(error_position!(
                        input,
                        ErrorKind::OneOf
                    )))
                }
                else {
                    Ok((remaining, res as u16))
                }
            }
        }
    }
}

/// XXX stolen to the asm parser
#[inline]
pub fn dec_u16_inner(input: &str) -> IResult<&str, u16, VerboseError<&str>> {
    match is_a("0123456789")(input) {
        Err(e) => Err(e),
        Ok((remaining, parsed)) => {
            // Do not parse more than 5 characters for a u16
            if parsed.input_len() > 5 {
                Err(cpclib_common::nom::Err::Error(error_position!(
                    input,
                    ErrorKind::OneOf
                )))
            }
            else {
                let mut res = 0_u32;
                for e in parsed.iter_elements() {
                    let digit = e;
                    let value = digit.to_digit(10).unwrap_or(0);
                    res = value + (res * 10);
                }
                if res > u32::from(u16::max_value()) {
                    Err(cpclib_common::nom::Err::Error(error_position!(
                        input,
                        ErrorKind::TooLarge // Custom(0)
                    )))
                }
                else {
                    Ok((remaining, res as u16))
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::parser::*;

    #[test]
    fn check_number() {
        assert!(dec_u16_inner("10").is_ok());

        assert!(dbg!(parse_floating_point("67.98")).is_ok());
        assert!(dbg!(parse_floating_point("-67.98")).is_ok());

        match hex_u16_inner("1234".into()) {
            Ok((res, value)) => {
                println!("{:?}", &res);
                println!("{:x}", &value);
                assert_eq!(0x1234, value);
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }

        match parse_hexadecimal_value_16bits("&1234".into()) {
            Ok((res, value)) => {
                println!("{:?}", &res);
                println!("{:?}", &value);
                let bytes = value.as_bytes();
                assert_eq!(
                    bytes[0],
                    BasicTokenNoPrefix::ValueIntegerHexadecimal16bits as u8
                );
                assert_eq!(bytes[1], 0x34);
                assert_eq!(bytes[2], 0x12);
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    fn check_line_tokenisation(code: &str) -> BasicLine {
        let res = parse_basic_line(code.into());
        match res {
            Ok((res, line)) => {
                println!("{:?}", &line);
                println!("{:?}", &res);
                assert_eq!(res.len(), 0, "Line as not been completly consummed");
                line
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    fn check_token_tokenisation(code: &str) {
        let res = parse_instruction(code.into());
        match res {
            Ok((res, line)) => {
                println!("{} => {:?}", code, &line);
                assert_eq!(res.len(), 0, "Line as not been completly consummed");
            }
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
        let res = parse_numeric_expression(NumericExpressionConstraint::None)(code.into());
        match res {
            Ok((res, line)) => {
                println!("{} => {:?}", code, &line);
                assert_eq!(res.len(), 0, "Line as not been completly consummed");
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    fn check_print_expression(code: &str) {
        let res = parse_print_expression(code);
        match res {
            Ok((res, line)) => {
                println!("{} => {:?}", code, &line);
                assert_eq!(res.len(), 0, "Line as not been completly consummed");
            }
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

pub fn test_parse<P: Fn(&str) -> BasicSeveralTokensResult>(parser: P, code: &str) -> BasicLine {
    let (rest, tokens) = dbg!(parser(code)).expect("Parse issue");

    assert!(rest.is_empty());

    BasicLine {
        line_number: 10,
        tokens,
        forced_length: None
    }
}

pub fn test_parse1<P: Fn(&str) -> BasicOneTokenResult>(parser: P, code: &str) -> BasicLine {
    let (rest, tokens) = dbg!(parser(code)).expect("Parse issue");

    assert!(rest.is_empty());

    BasicLine {
        line_number: 10,
        tokens: vec![tokens],
        forced_length: None
    }
}

pub fn test_parse_and_compare<P: Fn(&str) -> BasicSeveralTokensResult>(
    parser: P,
    code: &str,
    bytes: &[u8]
) {
    let prog = test_parse(parser, code);
    assert_eq!(bytes, prog.tokens_as_bytes().as_slice())
}
