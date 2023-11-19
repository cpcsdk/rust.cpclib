use std::str::FromStr;


use cpclib_common::parse_value;
use cpclib_common::winnow::ascii::space0;
use cpclib_common::winnow::combinator::{alt, delimited, preceded, separated1};
use cpclib_common::winnow::error::{ErrMode, ErrorKind, ParserError};
use cpclib_common::winnow::stream::{
    AsBytes, AsChar, Compare, Stream, StreamIsPartial, UpdateSlice
};
use cpclib_common::winnow::token::{take_while};
use cpclib_common::winnow::{PResult, Parser};

use crate::flags::{FlagValue, SnapshotFlag};

pub fn parse_flag<I, Error: ParserError<I>>(input: &mut I) -> PResult<SnapshotFlag, Error>
where
    I: StreamIsPartial,
    I: Stream,
    <I as Stream>::Token: Clone,
    <I as Stream>::Token: AsChar,
    <I as Stream>::Slice: AsBytes
{
    let word = take_while(1.., (('a'..='Z'), ('A'..='Z'), ('0'..='9'), '_', '.', ':'))
        .parse_next(input)?;
    let word = unsafe { std::str::from_utf8_unchecked(word.as_bytes()) };

    match SnapshotFlag::from_str(word) {
        Ok(flag) => Ok(flag),
        Err(_e) => Err(ErrMode::from_error_kind(input, ErrorKind::Verify))
    }
}

pub fn parse_flag_value<I, Error: ParserError<I>>(input: &mut I) -> PResult<FlagValue>
where I: Stream + StreamIsPartial + for<'a> Compare<&'a str> + Clone + UpdateSlice,

    <I as Stream>::Slice: AsBytes,
    <I as Stream>::Token: AsChar,
    <I as Stream>::Token: Clone,
    I: for<'a> Compare<&'a [u8; 2]>,
    I: for<'a> Compare<&'a [u8; 1]>,
    I: StreamIsPartial,
    I: Stream
{
    alt((
        parse_value
        .map(|val| {
            if val > 255 {
                FlagValue::Word(val as _)
            }
            else {
                FlagValue::Byte(val as u8)
            }
        }),
        delimited(
            '[',
            separated1(
                preceded(space0, parse_flag_value::<I, Error>),
                preceded(space0, ',')
            ),
            ']'
        )
        .map(|val: Vec<FlagValue>| FlagValue::Array(val.to_vec()))
    ))
    .parse_next(input)
}

#[cfg(test)]
mod tests {

    use cpclib_common::winnow::error::ContextError;
    use cpclib_common::winnow::stream::AsBStr;
    use cpclib_common::winnow::BStr;

    use super::*;

    #[test]
    fn test_parse_flag_value() {
        let mut fortytwo = "42".as_bstr();
        assert_eq!(
            dbg!(parse_value::<_,ContextError>.parse_next(&mut fortytwo)).unwrap(),
            42
        );

        let mut fortytwo = "42".as_bstr();
        assert_eq!(
            dbg!(parse_flag_value::<_,ContextError>.parse_next(&mut fortytwo)).unwrap(),
            FlagValue::Byte(42)
        );

        let mut fortytwohundred = "420".as_bstr();
        assert_eq!(
            dbg!(parse_flag_value::<_, ContextError>.parse_next(&mut fortytwohundred))
                .unwrap(),
            FlagValue::Word(420)
        );

        let mut list = "[42, 420]".as_bstr();
        assert_eq!(
            dbg!(parse_flag_value::< _, ContextError>.parse_next(&mut list)).unwrap(),
            FlagValue::Array(vec![FlagValue::Byte(42), FlagValue::Word(420)])
        );
    }

    #[test]
    fn test_parse_flag() {
        assert_eq!(
            dbg!(parse_value::<_, ContextError>.parse(BStr::new(b"1_60"))).unwrap(),
            160
        );
        assert_eq!(
            dbg!(parse_flag::< _, ContextError>.parse(BStr::new(b"CRTC_REG:7"))).unwrap(),
            SnapshotFlag::CRTC_REG(Some(7))
        );
    }
}
