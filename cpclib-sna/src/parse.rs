use std::str::FromStr;

use nom::branch::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::*;
use nom::multi::*;
use nom::sequence::*;
use nom::*;
use nom_locate::LocatedSpan;



use crate::flags::{FlagValue, SnapshotFlag};

use separated_list1 as separated_nonempty_list;

pub fn parse_flag<'src, T> (input: LocatedSpan<&'src str, T>) -> IResult<LocatedSpan<&'src str, T>, SnapshotFlag, VerboseError<LocatedSpan<&'src str, T>>> 
where T:Clone
{
    let (input, word) =
        is_a("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_.:")(input)?;

    match SnapshotFlag::from_str(&word.to_string().to_uppercase()) {
        Ok(flag) => Ok((input, flag)),
        Err(_e) => Err(::nom::Err::Error(error_position!(input, ErrorKind::OneOf))),
    }
}

pub fn parse_flag_value<'src, T> (input: LocatedSpan<&'src str, T>) -> IResult<LocatedSpan<&'src str, T>, FlagValue, VerboseError<LocatedSpan<&'src str, T>>> 
where T:Clone
{
    alt((
        map(parse_value, |val| {
            if val > 255 {
                FlagValue::Word(val as _)
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
fn parse_value<'src, T>(input: LocatedSpan<&'src str, T>) -> IResult<LocatedSpan<&str, T>, u32, VerboseError<LocatedSpan<&'src str, T>>>
where T:Clone
{
    alt((hex_number, bin_number, dec_number))(input)
}


#[inline]
/// Parse an usigned 32 bit number
pub fn dec_number<'src, T>(input: LocatedSpan<&'src str, T>) -> IResult<LocatedSpan<&str, T>, u32, VerboseError<LocatedSpan<&'src str, T>>>
where T:Clone
{
    let (input, digits) = digit1(input)?;
    let number = digits.chars()
        .map(|c| c.to_digit(10).unwrap())
        .fold(0, |acc, val| {
            acc*10 + val
        });

    Ok((input, number))
}

pub fn hex_number<'src, T>(input: LocatedSpan<&'src str, T>) -> IResult<LocatedSpan<&str, T>, u32, VerboseError<LocatedSpan<&'src str, T>>>
where T:Clone
{
    let (input, digits) = preceded(
        alt((tag("0x"), tag("#"), tag("$"), tag("&"))),
        hex_digit1
    )(input)?;
    let number = digits.chars()
        .map(|c| c.to_digit(16).unwrap())
        .fold(0, |acc, val| {
            acc*16 + val
        });

    Ok((input, number))
}

pub fn bin_number<'src, T>(input: LocatedSpan<&'src str, T>) -> IResult<LocatedSpan<&str, T>, u32, VerboseError<LocatedSpan<&'src str, T>>>
where T:Clone
{
    let (input, digits) = preceded(
        alt((tag("0b"), tag("%"))),
        many1(alt((tag("0"), tag("1"))))
    )(input)?;
    let number = digits.iter()
        .map(|d| d.chars().next().unwrap())
        .map(|c| c.to_digit(2).unwrap())
        .fold(0, |acc, val| {
            acc*2 + val
        });

    Ok((input, number))
}

#[cfg(test)]
mod tests {
    use nom_locate::LocatedSpan;

    use super::*;

    #[test]
    fn test_parse_value() {
        assert!(parse_value(LocatedSpan::new("0x12")).is_ok());
    }

    #[test]
    fn test_parse_flag_value() {
        assert!(parse_value(LocatedSpan::new("0x12")).is_ok());
        assert_eq!(parse_value(LocatedSpan::new("0x12")).unwrap().0.len(), 0);
        assert!(parse_value(LocatedSpan::new("0")).is_ok());
    }

    #[test]
    fn test_parse_flag() {
        assert!(parse_flag(LocatedSpan::new("CRTC_REG:7")).is_ok());
    }
}
