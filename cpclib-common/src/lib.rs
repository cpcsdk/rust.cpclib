#[cfg(feature = "cmdline")]
pub use clap;
use nom::branch::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::*;
use nom::sequence::*;
use nom::*;
pub use nom_locate::LocatedSpan;
#[cfg(all(not(target_arch = "wasm32"), feature="rayon"))]
pub use rayon;
#[cfg(feature = "cmdline")]
pub use semver;
#[cfg(feature = "cmdline")]
pub use time;
pub use {
    bitfield, bitflags, bitsets, bitvec, itertools, lazy_static, nom, nom_locate, num,
    resolve_path, smallvec, smol_str, strsim
};

/// Read a valuepub
pub fn parse_value<T>(
    input: LocatedSpan<&str, T>
) -> IResult<LocatedSpan<&str, T>, u32, VerboseError<LocatedSpan<&str, T>>>
where T: Clone {
    alt((dec_number, hex_number, bin_number_or_decimal))(input)
}

#[inline]
/// Parse an usigned 32 bit number
pub fn dec_number<'src, T>(
    input: LocatedSpan<&'src str, T>
) -> IResult<LocatedSpan<&str, T>, u32, VerboseError<LocatedSpan<&'src str, T>>>
where T: Clone {
    let (input, digits) = terminated(
        verify(is_a("0123456789_"), |s: &LocatedSpan<&'src str, T>| {
            !s.starts_with('_')
        }),
        not(alpha1)
    )(input)?;
    let number = digits
        .chars()
        .filter(|c| *c != '_')
        .map(|c| c.to_digit(10).unwrap())
        .fold(0, |acc, val| acc * 10 + val);

    Ok((input, number))
}

#[inline]
pub fn hex_number<T>(
    input: LocatedSpan<&str, T>
) -> IResult<LocatedSpan<&str, T>, u32, VerboseError<LocatedSpan<&str, T>>>
where T: Clone {
    alt((hex_number1, hex_number2))(input)
}

// Prefixed version
#[inline]
pub fn hex_number1<'src, T>(
    input: LocatedSpan<&'src str, T>
) -> IResult<LocatedSpan<&str, T>, u32, VerboseError<LocatedSpan<&'src str, T>>>
where T: Clone {
    let (input, digits) = preceded(
        pair(alt((tag_no_case("0x"), tag("#"), tag("$"), tag("&"))), space0),
        verify(
            is_a("0123456789abcdefABCDEF_"),
            |s: &LocatedSpan<&'src str, T>| !s.starts_with('_')
        )
    )(input)?;
    let number = digits
        .chars()
        .filter(|c| *c != '_')
        .map(|c| c.to_digit(16).unwrap())
        .fold(0, |acc, val| acc * 16 + val);

    Ok((input, number))
}

#[inline]
pub fn hex_number2<'src, T>(
    input: LocatedSpan<&'src str, T>
) -> IResult<LocatedSpan<&str, T>, u32, VerboseError<LocatedSpan<&'src str, T>>>
where T: Clone {
    let (input, digits) = terminated(
        verify(
            is_a("0123456789abcdefABCDEF_"),
            |s: &LocatedSpan<&'src str, T>| !s.starts_with('_')
        ),
        terminated(is_a("hH"), not(alpha1))
    )(input)?;
    let number = digits
        .chars()
        .filter(|c| *c != '_')
        .map(|c| c.to_digit(16).unwrap())
        .fold(0, |acc, val| acc * 16 + val);

    Ok((input, number))
}

///
/// Parse a binary number, but fallback to a decimal number if there are no previx/suffix of binary number to avoid to call another parser for that
#[inline]
pub fn bin_number_or_decimal<'src, T>(
    input: LocatedSpan<&'src str, T>
) -> IResult<LocatedSpan<&str, T>, u32, VerboseError<LocatedSpan<&'src str, T>>>
where T: Clone {

    // Get the prefix of binary number
    let (input,prefix) = opt(alt((tag("0b"), tag("%"))))(input)?;

    // get the numbers
    let (input, digits) = verify(is_a("01_"), |s: &LocatedSpan<&'src str, T>| {
        !s.starts_with('_')
    })(input)?;

    // get the postfix if there are no prefixes
    let (input, is_binary) = if prefix.is_none() {
        let (input, prefix) = opt(tag("b"))(input)?;
        if prefix.is_some() {
            (input, true)
        }
        else {
            let (input, next) = not(is_a("23456789_"))(input)?;
            (input, false)
        }
    } else {
        (input, true)
    };
    
    // make the computation
    let number = if is_binary {
        digits
        .chars()
        .filter(|c| *c != '_')
        .map(|c| c.to_digit(2).unwrap())
        .fold(0, |acc, val| acc * 2 + val)
    } else {
        digits
            .chars()
            .filter(|c| *c != '_')
            .map(|c| c.to_digit(10).unwrap())
            .fold(0, |acc, val| acc * 10 + val)
    };

    Ok((input, number))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_value() {
        assert!(parse_value(LocatedSpan::new("0x12")).is_ok());
        assert!(dbg!(parse_value(LocatedSpan::new("0b0100101"))).is_ok());
        assert!(dbg!(parse_value(LocatedSpan::new("%0100101"))).is_ok());
        assert!(dbg!(parse_value(LocatedSpan::new("0100101b"))).is_ok());
        assert!(dbg!(parse_value(LocatedSpan::new("160"))).is_ok());
        assert!(dbg!(bin_number_or_decimal(LocatedSpan::new("160"))).is_err());
    }
}
