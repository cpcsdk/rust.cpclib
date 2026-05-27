use winnow::ascii::{alphanumeric1, space0};
use winnow::combinator::{alt, delimited, not, opt, preceded, terminated};
use winnow::error::{AddContext, ContextError, ParserError, StrContext};
use winnow::stream::{AsBytes, AsChar, Compare, Stream, StreamIsPartial};
use winnow::token::{any, one_of, take_while};
use winnow::{BStr, ModalResult, Parser};

#[inline]
///  (prefix) space number suffix
pub fn parse_value<I, Error>(input: &mut I) -> ModalResult<u32, Error>
where
    I: Stream + StreamIsPartial + for<'a> Compare<&'a str>,
    <I as Stream>::Slice: AsBytes,
    <I as Stream>::Token: AsChar,
    <I as Stream>::Token: Clone,
    I: for<'a> Compare<&'a [u8; 2]>,
    I: for<'a> Compare<&'a [u8; 1]>,
    I: winnow::stream::Compare<u8>,
    Error: ParserError<I> + AddContext<I, winnow::error::StrContext>
{
    #[derive(Clone, PartialEq, Debug)]
    #[repr(u32)]
    enum EncodingKind {
        Hex = 16,
        Oct = 8,
        Bin = 2,
        Dec = 10,

        AmbiguousBinHex = 200,
        Unk = 255
    }

    let before_encoding: <I as Stream>::Checkpoint = input.checkpoint();

    // numbers have an optional prefix with an eventual space
    let encoding = opt(terminated(
        alt((
            alt((b"0x", b"0X", b"#", b"$", b"&")).value(EncodingKind::Hex), // hexadecimal number
            alt((b"0o", b"0O", b"@")).value(EncodingKind::Oct),             // octal number
            alt((b"0b", b"0B")).value(EncodingKind::AmbiguousBinHex),
            b"%".value(EncodingKind::Bin) // binary number
        )),
        space0
    )
    .context(StrContext::Label("Number prefix detection")))
    .parse_next(input)?
    .unwrap_or(EncodingKind::Unk);

    let hex_digits_and_sep = || {
        take_while(1.., (('0'..='9'), ('a'..='f'), ('A'..='F'), '_'))
            .context(StrContext::Label("Read hexadecimal digits"))
    };
    let mut oct_digits_and_sep =
        take_while(1.., (('0'..='7'), '_')).context(StrContext::Label("Read octal digits"));
    let mut dec_digits_and_sep =
        take_while(1.., (('0'..='9'), '_')).context(StrContext::Label("Read decimal digits"));
    let mut bin_digits_and_sep =
        take_while(1.., (('0'..='1'), '_')).context(StrContext::Label("Read binary digits"));

    let (encoding, digits) = match encoding {
        EncodingKind::Hex => (EncodingKind::Hex, hex_digits_and_sep().parse_next(input)?),
        EncodingKind::Oct => (EncodingKind::Oct, oct_digits_and_sep.parse_next(input)?),
        EncodingKind::Bin => (EncodingKind::Bin, bin_digits_and_sep.parse_next(input)?),
        EncodingKind::Dec => unreachable!("No prefix exist for decimal kind"),
        EncodingKind::AmbiguousBinHex => {
            // we parse for hexdecimal then guess the encoding
            let digits = opt(hex_digits_and_sep()).parse_next(input)?;
            let suffix = opt(alt((b'h', b'H')))
                .verify(|s| if digits.is_none() { s.is_some() } else { true })
                .parse_next(input)?;

            if suffix.is_some() {
                // this is an hexadecimal number and part of the encoding place was
                // TODO find a more efficient way to not redo that
                input.reset(&before_encoding);
                b'0'.parse_next(input)?; // eat 0
                let digits = hex_digits_and_sep().parse_next(input)?;
                let _suffix = alt((b'h', b'H')).parse_next(input)?;

                (EncodingKind::Hex, digits)
            }
            else {
                // this is a decimal number
                (EncodingKind::Bin, digits.unwrap())
            }
        },
        EncodingKind::Unk => {
            // we parse for hexdecimal then guess the encoding
            let backup = input.checkpoint();
            let digits = hex_digits_and_sep().parse_next(input)?;
            let suffix = opt(alt((b'h', b'H'))).parse_next(input)?;

            if suffix.is_some() {
                // we know if is hex
                (EncodingKind::Hex, digits)
            }
            else {
                // we need to choose between bin and dec so we reparse a second time :()
                input.reset(&backup);
                let digits: &[u8] = digits.as_bytes();
                let last_digit = digits[digits.len() - 1];
                if last_digit == b'b' || last_digit == b'B' {
                    // we need to check this is really a binary
                    let digits = bin_digits_and_sep.parse_next(input)?;
                    alt((b'b', b'B')).parse_next(input)?;
                    (EncodingKind::Bin, digits)
                }
                else {
                    (EncodingKind::Dec, dec_digits_and_sep.parse_next(input)?)
                }
            }
        }
    };

    // ensure there are no more numbers
    if encoding == EncodingKind::Hex {
        not(alphanumeric1)
            .context(StrContext::Label("This is not an hexadecimal number"))
            .parse_next(input)?;
    }

    // right here encoding anddigits are compatible
    debug_assert!(encoding != EncodingKind::Unk);
    debug_assert!(encoding != EncodingKind::AmbiguousBinHex);
    let digits: &[u8] = digits.as_bytes();

    let base = encoding as u32;
    let mut number = 0;
    for digit in digits.iter().filter(|&&digit| digit != b'_') {
        let digit = *digit;
        let digit = if digit.is_ascii_digit() {
            digit - b'0'
        }
        else if (b'a'..=b'f').contains(&digit) {
            digit - b'a' + 10
        }
        else {
            digit - b'A' + 10
        } as u32;

        number = base * number + digit;
    }

    Ok(number)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatternExprUnaryOp {
    Not,
    BinaryNot,
    Neg
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatternExprBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    ShiftLeft,
    ShiftRight,
    BitAnd,
    BitXor,
    BitOr,
    Equal,
    Different,
    LowerOrEqual,
    StrictlyLower,
    GreaterOrEqual,
    StrictlyGreater,
    BooleanAnd,
    BooleanOr
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatternExpr {
    Number(i32),
    Bool(bool),
    Char(u8),
    Identifier(String),
    Unary {
        op: PatternExprUnaryOp,
        expr: Box<PatternExpr>
    },
    Binary {
        op: PatternExprBinaryOp,
        left: Box<PatternExpr>,
        right: Box<PatternExpr>
    }
}

pub fn parse_pattern_number_literal(text: &str) -> Option<i32> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    let value = parse_value::<_, ContextError>.parse(BStr::new(text.as_bytes())).ok()?;
    i32::try_from(value).ok()
}

pub fn parse_pattern_expr(input: &str) -> Result<PatternExpr, String> {
    let mut input = input;
    let expr = parse_pattern_boolean_or(&mut input).map_err(|_| "Invalid pattern expression")?;
    input = input.trim_start();
    if input.is_empty() {
        Ok(expr)
    }
    else {
        Err("Invalid pattern expression".to_string())
    }
}

fn parse_pattern_boolean_or(input: &mut &str) -> ModalResult<PatternExpr> {
    let mut lhs = parse_pattern_boolean_and(input)?;
    while opt(preceded(space0, "||")).parse_next(input)?.is_some() {
        let rhs = parse_pattern_boolean_and(input)?;
        lhs = PatternExpr::Binary {
            op: PatternExprBinaryOp::BooleanOr,
            left: Box::new(lhs),
            right: Box::new(rhs)
        };
    }
    Ok(lhs)
}

fn parse_pattern_boolean_and(input: &mut &str) -> ModalResult<PatternExpr> {
    let mut lhs = parse_pattern_bit_or(input)?;
    while opt(preceded(space0, "&&")).parse_next(input)?.is_some() {
        let rhs = parse_pattern_bit_or(input)?;
        lhs = PatternExpr::Binary {
            op: PatternExprBinaryOp::BooleanAnd,
            left: Box::new(lhs),
            right: Box::new(rhs)
        };
    }
    Ok(lhs)
}

fn parse_pattern_bit_or(input: &mut &str) -> ModalResult<PatternExpr> {
    let mut lhs = parse_pattern_bit_xor(input)?;
    while opt(preceded(space0, terminated("|", not("|"))))
        .parse_next(input)?
        .is_some()
    {
        let rhs = parse_pattern_bit_xor(input)?;
        lhs = PatternExpr::Binary {
            op: PatternExprBinaryOp::BitOr,
            left: Box::new(lhs),
            right: Box::new(rhs)
        };
    }
    Ok(lhs)
}

fn parse_pattern_bit_xor(input: &mut &str) -> ModalResult<PatternExpr> {
    let mut lhs = parse_pattern_bit_and(input)?;
    while opt(preceded(space0, "^")).parse_next(input)?.is_some() {
        let rhs = parse_pattern_bit_and(input)?;
        lhs = PatternExpr::Binary {
            op: PatternExprBinaryOp::BitXor,
            left: Box::new(lhs),
            right: Box::new(rhs)
        };
    }
    Ok(lhs)
}

fn parse_pattern_bit_and(input: &mut &str) -> ModalResult<PatternExpr> {
    let mut lhs = parse_pattern_equality(input)?;
    while opt(preceded(space0, terminated("&", not("&"))))
        .parse_next(input)?
        .is_some()
    {
        let rhs = parse_pattern_equality(input)?;
        lhs = PatternExpr::Binary {
            op: PatternExprBinaryOp::BitAnd,
            left: Box::new(lhs),
            right: Box::new(rhs)
        };
    }
    Ok(lhs)
}

fn parse_pattern_equality(input: &mut &str) -> ModalResult<PatternExpr> {
    let mut lhs = parse_pattern_comparison(input)?;
    loop {
        if opt(preceded(space0, "==")).parse_next(input)?.is_some() {
            let rhs = parse_pattern_comparison(input)?;
            lhs = PatternExpr::Binary {
                op: PatternExprBinaryOp::Equal,
                left: Box::new(lhs),
                right: Box::new(rhs)
            };
        }
        else if opt(preceded(space0, "!=")).parse_next(input)?.is_some() {
            let rhs = parse_pattern_comparison(input)?;
            lhs = PatternExpr::Binary {
                op: PatternExprBinaryOp::Different,
                left: Box::new(lhs),
                right: Box::new(rhs)
            };
        }
        else {
            return Ok(lhs);
        }
    }
}

fn parse_pattern_comparison(input: &mut &str) -> ModalResult<PatternExpr> {
    let mut lhs = parse_pattern_shift(input)?;
    loop {
        if opt(preceded(space0, "<=")).parse_next(input)?.is_some() {
            let rhs = parse_pattern_shift(input)?;
            lhs = PatternExpr::Binary {
                op: PatternExprBinaryOp::LowerOrEqual,
                left: Box::new(lhs),
                right: Box::new(rhs)
            };
        }
        else if opt(preceded(space0, ">=")).parse_next(input)?.is_some() {
            let rhs = parse_pattern_shift(input)?;
            lhs = PatternExpr::Binary {
                op: PatternExprBinaryOp::GreaterOrEqual,
                left: Box::new(lhs),
                right: Box::new(rhs)
            };
        }
        else if opt(preceded(space0, "<")).parse_next(input)?.is_some() {
            let rhs = parse_pattern_shift(input)?;
            lhs = PatternExpr::Binary {
                op: PatternExprBinaryOp::StrictlyLower,
                left: Box::new(lhs),
                right: Box::new(rhs)
            };
        }
        else if opt(preceded(space0, ">")).parse_next(input)?.is_some() {
            let rhs = parse_pattern_shift(input)?;
            lhs = PatternExpr::Binary {
                op: PatternExprBinaryOp::StrictlyGreater,
                left: Box::new(lhs),
                right: Box::new(rhs)
            };
        }
        else {
            return Ok(lhs);
        }
    }
}

fn parse_pattern_shift(input: &mut &str) -> ModalResult<PatternExpr> {
    let mut lhs = parse_pattern_add_sub(input)?;
    loop {
        if opt(preceded(space0, "<<")).parse_next(input)?.is_some() {
            let rhs = parse_pattern_add_sub(input)?;
            lhs = PatternExpr::Binary {
                op: PatternExprBinaryOp::ShiftLeft,
                left: Box::new(lhs),
                right: Box::new(rhs)
            };
        }
        else if opt(preceded(space0, ">>")).parse_next(input)?.is_some() {
            let rhs = parse_pattern_add_sub(input)?;
            lhs = PatternExpr::Binary {
                op: PatternExprBinaryOp::ShiftRight,
                left: Box::new(lhs),
                right: Box::new(rhs)
            };
        }
        else {
            return Ok(lhs);
        }
    }
}

fn parse_pattern_add_sub(input: &mut &str) -> ModalResult<PatternExpr> {
    let mut lhs = parse_pattern_mul_div_mod(input)?;
    loop {
        if opt(preceded(space0, "+")).parse_next(input)?.is_some() {
            let rhs = parse_pattern_mul_div_mod(input)?;
            lhs = PatternExpr::Binary {
                op: PatternExprBinaryOp::Add,
                left: Box::new(lhs),
                right: Box::new(rhs)
            };
        }
        else if opt(preceded(space0, "-")).parse_next(input)?.is_some() {
            let rhs = parse_pattern_mul_div_mod(input)?;
            lhs = PatternExpr::Binary {
                op: PatternExprBinaryOp::Sub,
                left: Box::new(lhs),
                right: Box::new(rhs)
            };
        }
        else {
            return Ok(lhs);
        }
    }
}

fn parse_pattern_mul_div_mod(input: &mut &str) -> ModalResult<PatternExpr> {
    let mut lhs = parse_pattern_unary(input)?;
    loop {
        if opt(preceded(space0, "*")).parse_next(input)?.is_some() {
            let rhs = parse_pattern_unary(input)?;
            lhs = PatternExpr::Binary {
                op: PatternExprBinaryOp::Mul,
                left: Box::new(lhs),
                right: Box::new(rhs)
            };
        }
        else if opt(preceded(space0, "/")).parse_next(input)?.is_some() {
            let rhs = parse_pattern_unary(input)?;
            lhs = PatternExpr::Binary {
                op: PatternExprBinaryOp::Div,
                left: Box::new(lhs),
                right: Box::new(rhs)
            };
        }
        else if opt(preceded(space0, "%")).parse_next(input)?.is_some() {
            let rhs = parse_pattern_unary(input)?;
            lhs = PatternExpr::Binary {
                op: PatternExprBinaryOp::Mod,
                left: Box::new(lhs),
                right: Box::new(rhs)
            };
        }
        else {
            return Ok(lhs);
        }
    }
}

fn parse_pattern_unary(input: &mut &str) -> ModalResult<PatternExpr> {
    if opt(preceded(space0, "!")).parse_next(input)?.is_some() {
        return Ok(PatternExpr::Unary {
            op: PatternExprUnaryOp::Not,
            expr: Box::new(parse_pattern_unary(input)?)
        });
    }
    if opt(preceded(space0, "~")).parse_next(input)?.is_some() {
        return Ok(PatternExpr::Unary {
            op: PatternExprUnaryOp::BinaryNot,
            expr: Box::new(parse_pattern_unary(input)?)
        });
    }
    if opt(preceded(space0, "-")).parse_next(input)?.is_some() {
        return Ok(PatternExpr::Unary {
            op: PatternExprUnaryOp::Neg,
            expr: Box::new(parse_pattern_unary(input)?)
        });
    }

    parse_pattern_primary(input)
}

fn parse_pattern_primary(input: &mut &str) -> ModalResult<PatternExpr> {
    alt((
        delimited(preceded(space0, '('), parse_pattern_boolean_or, preceded(space0, ')')),
        parse_pattern_char,
        parse_pattern_number,
        parse_pattern_identifier
    ))
    .parse_next(input)
}

fn parse_pattern_char(input: &mut &str) -> ModalResult<PatternExpr> {
    let value = delimited(preceded(space0, '\''), any, '\'').parse_next(input)?;
    Ok(PatternExpr::Char(value as u8))
}

fn parse_pattern_number(input: &mut &str) -> ModalResult<PatternExpr> {
    let checkpoint = input.checkpoint();
    let token = preceded(
        space0,
        take_while(1.., |c: char| {
            c.is_ascii_alphanumeric() || matches!(c, '_' | '$' | '#' | '@' | '%')
        })
    )
    .parse_next(input)?;

    if token
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit() || matches!(c, '$' | '#' | '@' | '%' | '&'))
        && let Some(value) = parse_pattern_number_literal(token)
    {
        return Ok(PatternExpr::Number(value));
    }

    input.reset(&checkpoint);
    Err(winnow::error::ErrMode::Backtrack(winnow::error::ContextError::default()))
}

fn parse_pattern_identifier(input: &mut &str) -> ModalResult<PatternExpr> {
    let first = preceded(
        space0,
        one_of(('a'..='z', 'A'..='Z', '_', '.', '@'))
    )
    .parse_next(input)?;
    let rest: &str = take_while(0.., |c: char| {
        c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '@')
    })
    .parse_next(input)?;

    let mut ident = String::with_capacity(1 + rest.len());
    ident.push(first);
    ident.push_str(rest);

    if ident.eq_ignore_ascii_case("true") {
        return Ok(PatternExpr::Bool(true));
    }
    if ident.eq_ignore_ascii_case("false") {
        return Ok(PatternExpr::Bool(false));
    }

    Ok(PatternExpr::Identifier(ident))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pattern_number_literal_handles_project_formats() {
        assert_eq!(parse_pattern_number_literal("42"), Some(42));
        assert_eq!(parse_pattern_number_literal("1_60"), Some(160));
        assert_eq!(parse_pattern_number_literal("0x12"), Some(0x12));
        assert_eq!(parse_pattern_number_literal("$12"), Some(0x12));
        assert_eq!(parse_pattern_number_literal("0b0100101"), Some(0b0100101));
        assert_eq!(parse_pattern_number_literal("0100101b"), Some(0b0100101));
        assert_eq!(parse_pattern_number_literal("0b0h"), Some(0x0B0));
        assert_eq!(parse_pattern_number_literal("0bh"), Some(0x0B));
        assert_eq!(parse_pattern_number_literal("CH"), Some(0x0C));
        assert_eq!(parse_pattern_number_literal("CHECK"), None);
    }

    #[test]
    fn parse_pattern_expr_honors_precedence() {
        assert_eq!(
            parse_pattern_expr("1 + 2 * 3"),
            Ok(PatternExpr::Binary {
                op: PatternExprBinaryOp::Add,
                left: Box::new(PatternExpr::Number(1)),
                right: Box::new(PatternExpr::Binary {
                    op: PatternExprBinaryOp::Mul,
                    left: Box::new(PatternExpr::Number(2)),
                    right: Box::new(PatternExpr::Number(3))
                })
            })
        );
    }

    #[test]
    fn parse_pattern_expr_parses_identifiers_and_unary_ops() {
        assert_eq!(
            parse_pattern_expr("-foo + ~'A'"),
            Ok(PatternExpr::Binary {
                op: PatternExprBinaryOp::Add,
                left: Box::new(PatternExpr::Unary {
                    op: PatternExprUnaryOp::Neg,
                    expr: Box::new(PatternExpr::Identifier("foo".to_string()))
                }),
                right: Box::new(PatternExpr::Unary {
                    op: PatternExprUnaryOp::BinaryNot,
                    expr: Box::new(PatternExpr::Char(b'A'))
                })
            })
        );
    }

    #[test]
    fn parse_pattern_expr_parses_boolean_structure() {
        assert_eq!(
            parse_pattern_expr("true && value != 0"),
            Ok(PatternExpr::Binary {
                op: PatternExprBinaryOp::BooleanAnd,
                left: Box::new(PatternExpr::Bool(true)),
                right: Box::new(PatternExpr::Binary {
                    op: PatternExprBinaryOp::Different,
                    left: Box::new(PatternExpr::Identifier("value".to_string())),
                    right: Box::new(PatternExpr::Number(0))
                })
            })
        );
    }

    #[test]
    fn parse_pattern_expr_parses_parenthesized_shift_and_bitand() {
        assert_eq!(
            parse_pattern_expr("(1 + 2) << 3 & 7"),
            Ok(PatternExpr::Binary {
                op: PatternExprBinaryOp::BitAnd,
                left: Box::new(PatternExpr::Binary {
                    op: PatternExprBinaryOp::ShiftLeft,
                    left: Box::new(PatternExpr::Binary {
                        op: PatternExprBinaryOp::Add,
                        left: Box::new(PatternExpr::Number(1)),
                        right: Box::new(PatternExpr::Number(2))
                    }),
                    right: Box::new(PatternExpr::Number(3))
                }),
                right: Box::new(PatternExpr::Number(7))
            })
        );
    }

    #[test]
    fn parse_pattern_expr_mixes_bitwise_and_boolean_operators() {
        assert_eq!(
            parse_pattern_expr("a | b || c & d"),
            Ok(PatternExpr::Binary {
                op: PatternExprBinaryOp::BooleanOr,
                left: Box::new(PatternExpr::Binary {
                    op: PatternExprBinaryOp::BitOr,
                    left: Box::new(PatternExpr::Identifier("a".to_string())),
                    right: Box::new(PatternExpr::Identifier("b".to_string()))
                }),
                right: Box::new(PatternExpr::Binary {
                    op: PatternExprBinaryOp::BitAnd,
                    left: Box::new(PatternExpr::Identifier("c".to_string())),
                    right: Box::new(PatternExpr::Identifier("d".to_string()))
                })
            })
        );
    }

    #[test]
    fn parse_pattern_expr_mixes_shift_and_comparison() {
        assert_eq!(
            parse_pattern_expr("value << 1 < limit + 2"),
            Ok(PatternExpr::Binary {
                op: PatternExprBinaryOp::StrictlyLower,
                left: Box::new(PatternExpr::Binary {
                    op: PatternExprBinaryOp::ShiftLeft,
                    left: Box::new(PatternExpr::Identifier("value".to_string())),
                    right: Box::new(PatternExpr::Number(1))
                }),
                right: Box::new(PatternExpr::Binary {
                    op: PatternExprBinaryOp::Add,
                    left: Box::new(PatternExpr::Identifier("limit".to_string())),
                    right: Box::new(PatternExpr::Number(2))
                })
            })
        );
    }
}
