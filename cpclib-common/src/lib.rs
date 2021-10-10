pub use bitfield;
pub use bitflags;
pub use bitsets;
pub use bitvec;
pub use itertools;
pub use lazy_static;
pub use nom;
pub use nom_locate;
pub use num;
pub use rayon;
pub use smallvec;
pub use strsim;


use nom::branch::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::*;
use nom::multi::*;
use nom::sequence::*;
use nom::*;
use nom_locate::LocatedSpan;


/// Read a valuepub
pub fn parse_value<'src, T>(
    input: LocatedSpan<&'src str, T>,
) -> IResult<LocatedSpan<&str, T>, u32, VerboseError<LocatedSpan<&'src str, T>>>
where
    T: Clone,
{
    alt((hex_number, bin_number, dec_number))(input)
}

#[inline]
/// Parse an usigned 32 bit number
pub fn dec_number<'src, T>(
    input: LocatedSpan<&'src str, T>,
) -> IResult<LocatedSpan<&str, T>, u32, VerboseError<LocatedSpan<&'src str, T>>>
where
    T: Clone,
{
    let (input, digits) = digit1(input)?;
    let number = digits
        .chars()
        .map(|c| c.to_digit(10).unwrap())
        .fold(0, |acc, val| acc * 10 + val);

    Ok((input, number))
}

pub fn hex_number<'src, T>(
    input: LocatedSpan<&'src str, T>,
) -> IResult<LocatedSpan<&str, T>, u32, VerboseError<LocatedSpan<&'src str, T>>>
where
    T: Clone,
{
    alt((hex_number1, hex_number2))(input)
}

pub fn hex_number1<'src, T>(
    input: LocatedSpan<&'src str, T>,
) -> IResult<LocatedSpan<&str, T>, u32, VerboseError<LocatedSpan<&'src str, T>>>
where
    T: Clone,
{
    let (input, digits) =
        preceded(alt((tag("0x"), tag("#"), tag("$"), tag("&"))), hex_digit1)(input)?;
    let number = digits
        .chars()
        .map(|c| c.to_digit(16).unwrap())
        .fold(0, |acc, val| acc * 16 + val);

    Ok((input, number))
}

pub fn hex_number2<'src, T>(
    input: LocatedSpan<&'src str, T>,
) -> IResult<LocatedSpan<&str, T>, u32, VerboseError<LocatedSpan<&'src str, T>>>
where
    T: Clone,
{
    let (input, digits) = terminated(hex_digit1, terminated(tag_no_case("h"), not(alpha1)))(input)?;
    let number = digits
        .chars()
        .map(|c| c.to_digit(16).unwrap())
        .fold(0, |acc, val| acc * 16 + val);

    Ok((input, number))
}

pub fn bin_number<'src, T>(
    input: LocatedSpan<&'src str, T>,
) -> IResult<LocatedSpan<&str, T>, u32, VerboseError<LocatedSpan<&'src str, T>>>
where
    T: Clone,
{
    let (input, digits) =
        preceded(alt((tag("0b"), tag("%"))), many1(alt((tag("0"), tag("1")))))(input)?;
    let number = digits
        .iter()
        .map(|d| d.chars().next().unwrap())
        .map(|c| c.to_digit(2).unwrap())
        .fold(0, |acc, val| acc * 2 + val);

    Ok((input, number))
}


#[cfg(test)]
mod tests {
	use super::*;

    #[test]
    fn test_parse_value() {
        assert!(parse_value(LocatedSpan::new("0x12")).is_ok());
    }

}