#[cfg(feature = "cmdline")]
pub use clap;

#[cfg(all(not(target_arch = "wasm32"), feature="rayon"))]
pub use rayon;
#[cfg(feature = "cmdline")]
pub use semver;
#[cfg(feature = "cmdline")]
pub use time;
use winnow::{PResult, combinator::{alt, opt, terminated, fail}, Parser, ascii::{hex_digit1, space0}, token::{one_of, take_while, tag_no_case}, error::{StrContext, ParserError, AddContext}, stream::{AsChar, StreamIsPartial, Compare, AsBytes}};
pub use {
    bitfield, bitflags, bitsets, bitvec, itertools, lazy_static,  num,
    resolve_path, smallvec, smol_str, strsim
};
use winnow::prelude::*;
use winnow::stream::BStr;
use winnow::stream::Stream;

pub use winnow;

#[inline]
/**
 *  (prefix) space number suffix
 */
pub fn parse_value<I, Error: ParserError<I>>(input: &mut I) -> PResult<u32, Error> 
where I: Stream + StreamIsPartial + for<'a> Compare<&'a str>,
<I as Stream>::Slice: AsBytes,
<I as Stream>::Token: AsChar,
<I as Stream>::Token: Clone,
I: for<'a> Compare<&'a [u8; 2]>,
I: for<'a> Compare<&'a [u8; 1]>, 
Error: AddContext<I, winnow::error::StrContext>
{

    #[derive(Clone, PartialEq, Debug)]
    #[repr(u32)]
    enum EncodingKind {
        Hex = 16,
        Bin = 2,
        Dec = 10,
        Unk = 255
    }

    // numbers have an optional prefix with an eventual space
    let encoding = opt(terminated(
        alt((
            alt((b"0x",b"0X", b"#", b"$", b"&")).value(EncodingKind::Hex) , // hexadecimal number
            alt((b"0b", b"0B", b"%")).value(EncodingKind::Bin), //binary number
        )), 
        space0
    ).context(StrContext::Label("Number prefix detection"))
    )
    .parse_next(input)?
    .unwrap_or(EncodingKind::Unk);

    let mut hex_digits_and_sep = take_while(1..,(
        ('0'..='9'), 
        ('a'..='f'), 
        ('A'..='F'), 
        '_')
    ).context(StrContext::Label("Read hexadecimal digits"));
    let mut dec_digits_and_sep = take_while(1..,(
        ('0'..='9'), 
        '_')
    ).context(StrContext::Label("Read decimal digits"));
    let mut bin_digits_and_sep = take_while(1..,(
        ('0'..='1'), 
        '_')
    ).context(StrContext::Label("Read binary digits"));

    let (encoding, digits) = match encoding {
        EncodingKind::Hex => (EncodingKind::Hex, hex_digits_and_sep.parse_next(input)?),
        EncodingKind::Bin => (EncodingKind::Bin, bin_digits_and_sep.parse_next(input)?),
        EncodingKind::Dec => unreachable!("No prefix exist for decimal kind"),
        EncodingKind::Unk => {
            // we parse for hexdecimal then guess the encoding
            let backup = input.checkpoint();
            let digits = hex_digits_and_sep.parse_next(input)?;
            let suffix = opt(tag_no_case("h")).parse_next(input)?;

            if suffix.is_some() {
                // we know if is hex
                (EncodingKind::Hex, digits)
            }
            else {
                // we need to choose between bin and dec so we reparse a second time :()
                input.reset(backup);
                let digits: &[u8] = digits.as_bytes();
                let last_digit = digits[digits.len()-1];
                if last_digit == b'b' || last_digit == b'B' {
                    // we need to check this is really a binary
                    let digits = bin_digits_and_sep.parse_next(input)?;
                    alt(('b' , 'B')).parse_next(input)?;
                    (EncodingKind::Bin, digits)
                } else {
                    (EncodingKind::Dec, dec_digits_and_sep.parse_next(input)?)
                }
            }
        }
    };

    // right here encoding anddigits are compatible
    debug_assert!(encoding != EncodingKind::Unk);
    let digits: &[u8] = digits.as_bytes();

    let base = encoding as u32;
    let mut number = 0;
    for digit in digits.into_iter().filter(|&&digit| digit != b'_') {
        let digit = *digit;
        let digit = if digit >=b'0' && digit <= b'9' {
            digit - b'0'
        } else if digit >= b'a' && digit <= b'f' {
            digit - b'a' + 10
        } else {
            digit - b'A' + 10
        } as u32;

        number = base*number + digit;
    }

    Ok(number)
}


#[cfg(test)]
mod tests {
    use winnow::{stream::AsBStr, error::{VerboseError, ContextError}};

    use super::*;

    #[test]
    fn test_parse_value() {
        let mut fortytwo = "42".as_bstr();
        assert_eq!(dbg!(parse_value::<_,  ContextError>.parse_next(&mut fortytwo)).unwrap(), 42);
        assert_eq!(parse_value::<_, ContextError>.parse(BStr::new(b"0x12")).unwrap(), 0x12);
        assert_eq!(parse_value::<_, ContextError>.parse(BStr::new(b"0x0000")).unwrap(), 0x0000);
        assert_eq!(parse_value::<_, ContextError>.parse(BStr::new(b"0x4000")).unwrap(), 0x4000);
        assert_eq!(parse_value::<_, ContextError>.parse(BStr::new(b"0x8000")).unwrap(), 0x8000);
        assert_eq!(parse_value::<_, ContextError>.parse(BStr::new(b"0xc000")).unwrap(), 0xc000);
        assert_eq!(parse_value::<_,  ContextError>.parse(BStr::new(b"0x1_2")).unwrap(), 0x12);
        assert_eq!(dbg!(parse_value::<_,  ContextError>.parse(BStr::new(b"0b0100101"))).unwrap(), 0b0100101);
        assert_eq!(dbg!(parse_value::<_,  ContextError>.parse(BStr::new(b"0b0_100_101"))).unwrap(), 0b0100101);
        assert_eq!(dbg!(parse_value::<_,  ContextError>.parse(BStr::new(b"%0100101"))).unwrap(), 0b0100101);
        assert_eq!(dbg!(parse_value::<_,  ContextError>.parse(BStr::new(b"0100101b"))).unwrap(), 0b0100101);
        assert_eq!(dbg!(parse_value::<_,  ContextError>.parse(BStr::new(b"160"))).unwrap(), 160);
        assert_eq!(dbg!(parse_value::<_,  ContextError>.parse(BStr::new(b"1_60"))).unwrap(), 160);
    }
}
