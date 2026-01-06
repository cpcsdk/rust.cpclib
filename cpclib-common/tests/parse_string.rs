use winnow::combinator::{alt, delimited, repeat};
use winnow::stream::UpdateSlice;
use winnow::token::{none_of, one_of};
use winnow::{BStr, LocatingSlice, ModalResult, Parser, Stateful};

fn parse_string<'src>(
    input: &mut Stateful<LocatingSlice<&'src BStr>, ()>
) -> ModalResult<Stateful<LocatingSlice<&'src BStr>, ()>> {
    let normal = none_of(('\\', '"')).void();
    let escaped = ('\\', one_of(('\\', '"'))).void();
    
    let content = delimited(
        '"',
        repeat::<_, _, (), _, _>(0.., alt((normal, escaped))).take(),
        '"'
    ).parse_next(input)?;

    let string = (*input).update_slice(content);
    Ok(string)
}

#[test]
fn test_parse_string() {
    for string in &[
        r#""kjkjhkl""#,
        r#""kjk'jhkl""#,
        r#""kj\"kjhkl""#,
        r#""""#,
        r#""fdfd\" et voila""#,
        r#""\" et voila""#
    ] {
        let string = Stateful {
            input: LocatingSlice::new(BStr::new(string)),
            state: ()
        };
        let res = parse_string.parse(string);
        assert!(dbg!(&res).is_ok());
    }
}
