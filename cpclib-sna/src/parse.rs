use nom::branch::*;
use nom::bytes::complete::tag;
use nom::bytes::complete::tag_no_case;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::*;
use nom::multi::*;
use nom::sequence::*;
use nom::*;
use std::str::FromStr;

use crate::flags::{FlagValue, SnapshotFlag};

use separated_list1 as separated_nonempty_list;

pub fn parse_flag(input: &str) -> IResult<&str, SnapshotFlag, VerboseError<&str>> {
    let (input, word) =
        is_a("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_.:")(input)?;

    match SnapshotFlag::from_str(&word.to_uppercase()) {
        Ok(flag) => Ok((input, flag)),
        Err(_e) => Err(::nom::Err::Error(error_position!(input, ErrorKind::OneOf))),
    }
}

pub fn parse_flag_value(input: &str) -> IResult<&str, FlagValue, VerboseError<&str>> {
    alt((
        map(parse_value, |val| {
            if val > 255 {
                FlagValue::Word(val)
            } else {
                FlagValue::Byte(val as u8)
            }
        }),
        map(
            delimited(
                char('['),
                separated_nonempty_list(
                    preceded(space0, char(',')),
                    preceded(space0, parse_flag_value),
                ),
                char(']'),
            ),
            |val| FlagValue::Array(val.to_vec()),
        ),
    ))(input)
}

/// Read a value
fn parse_value(input: &str) -> IResult<&str, u16, VerboseError<&str>> {
    alt((hex_number, dec_number, bin_u16))(input)
}

/// TODO : move in a cpclib_parsecommon crate
/// Read an hexadecimal value
pub fn hex_number(input: &str) -> IResult<&str, u16, VerboseError<&str>> {
    alt((
        preceded(
            alt((tag_no_case("0x"), tag("#"), tag("$"), tag("&"))),
            inner_hex,
        ),
        terminated(inner_hex, tuple((tag_no_case("h"), not(alphanumeric1)))),
    ))(input)
}

#[inline]
/// Parse an usigned 16 bit number
pub fn dec_number(input: &str) -> IResult<&str, u16, VerboseError<&str>> {
    match is_a("0123456789")(input) {
        Err(e) => Err(e),
        Ok((remaining, parsed)) => {
            // Do not parse more than 5 characters for a u16
            if parsed.input_len() > 5 {
                Err(::nom::Err::Error(error_position!(input, ErrorKind::OneOf)))
            } else {
                let mut res = 0_u32;
                for e in parsed.iter_elements() {
                    let digit = e;
                    let value = digit.to_digit(10).unwrap_or(0);
                    res = value + (res * 10);
                }
                if res > u16::max_value() as u32 {
                    Err(::nom::Err::Error(error_position!(
                        input,
                        ErrorKind::Digit /*Custom(0)*/
                    )))
                } else {
                    Ok((remaining, res as u16))
                }
            }
        }
    }
}

/// Read an hexidecimal value
pub fn inner_hex(input: &str) -> IResult<&str, u16, VerboseError<&str>> {
    match is_a("0123456789abcdefABCDEF")(input) {
        Err(e) => Err(e),
        Ok((remaining, parsed)) => {
            // Do not parse more than  characters for a u16
            if parsed.input_len() > 4 {
                Err(::nom::Err::Error(error_position!(input, ErrorKind::OneOf)))
            } else {
                let mut res = 0_u32;
                for e in parsed.iter_elements() {
                    let digit = e;
                    let value = digit.to_digit(16).unwrap_or(0);
                    res = value + (res * 16);
                }
                if res > u16::max_value() as u32 {
                    Err(::nom::Err::Error(error_position!(input, ErrorKind::OneOf)))
                } else {
                    Ok((remaining, res as u16))
                }
            }
        }
    }
}

/// Parse a binary number
pub fn bin_u16(input: &str) -> IResult<&str, u16, VerboseError<&str>> {
    preceded(
        alt((tag_no_case("0b"), tag_no_case("%"))),
        fold_many1(
            alt((value(0, tag("0")), value(1, tag("1")))),
            0,
            |mut acc: u16, item: u16| {
                acc *= 2;
                acc += item;
                acc
            },
        ),
    )(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_value() {
        assert!(parse_value("0x12").is_ok());
    }

    #[test]
    fn test_parse_flag_value() {
        assert!(parse_value("0x12").is_ok());
        assert_eq!(parse_value("0x12").unwrap().0.len(), 0);
        assert!(parse_value("0").is_ok());
    }

    #[test]
    fn test_parse_flag() {
        assert!(parse_flag("CRTC_REG:7").is_ok());
    }
}
