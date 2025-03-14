use winnow::ascii::{alphanumeric1, space0};
use winnow::combinator::{alt, not, opt, terminated};
use winnow::error::{AddContext, ParserError, StrContext};
use winnow::stream::{AsBytes, AsChar, Compare, Stream, StreamIsPartial};
use winnow::token::take_while;
use winnow::{ModalResult, Parser};

#[inline]
///  (prefix) space number suffix
pub fn parse_value<I, Error: ParserError<I>>(input: &mut I) -> ModalResult<u32, Error>
where
    I: Stream + StreamIsPartial + for<'a> Compare<&'a str>,
    <I as Stream>::Slice: AsBytes,
    <I as Stream>::Token: AsChar,
    <I as Stream>::Token: Clone,
    I: for<'a> Compare<&'a [u8; 2]>,
    I: for<'a> Compare<&'a [u8; 1]>,
    I: winnow::stream::Compare<u8>,
    Error: AddContext<I, winnow::error::StrContext>
{
    #[derive(Clone, PartialEq, Debug)]
    #[repr(u32)]
    enum EncodingKind {
        Hex = 16,
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
    let mut dec_digits_and_sep =
        take_while(1.., (('0'..='9'), '_')).context(StrContext::Label("Read decimal digits"));
    let mut bin_digits_and_sep =
        take_while(1.., (('0'..='1'), '_')).context(StrContext::Label("Read binary digits"));

    let (encoding, digits) = match encoding {
        EncodingKind::Hex => (EncodingKind::Hex, hex_digits_and_sep().parse_next(input)?),
        EncodingKind::Bin => (EncodingKind::Bin, bin_digits_and_sep.parse_next(input)?),
        EncodingKind::Dec => unreachable!("No prefix exist for decimal kind"),
        EncodingKind::AmbiguousBinHex => {
            // we parse for hexdecimal then guess the encoding
            let digits = opt(hex_digits_and_sep()).parse_next(input)?;
            let suffix = opt(alt((b'h', b'H')))
                .verify(|s| {
                    if digits.is_none() {
                        s.is_some()
                    }
                    else {
                        true
                    }
                })
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
