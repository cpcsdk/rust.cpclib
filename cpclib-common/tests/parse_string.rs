use winnow::ascii::escaped;
use winnow::combinator::{alt, terminated};
use winnow::stream::UpdateSlice;
use winnow::token::{none_of, one_of};
use winnow::{BStr, Located, PResult, Parser, Stateful};

fn parse_string<'src>(
    input: &mut Stateful<Located<&'src BStr>, ()>
) -> PResult<Stateful<Located<&'src BStr>, ()>> {
    let mut first = '"';
    let last = first;
    let normal = none_of(('\\', '"'));
    let escapable = one_of(('\\', '"'));

    first.parse_next(input)?;
    let content = alt((
        last.recognize(), // to be removed if any
        terminated(escaped(normal, '\\', escapable), last)
    ))
    .parse_next(input)?;

    let string = if content.len() == 1 && first == (content[0] as char) {
        &content[..0] // we remove " (it is not present for the others)
    }
    else {
        content
    };

    let string = (*input).update_slice(string);
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
            input: Located::new(BStr::new(string)),
            state: ()
        };
        let res = parse_string.parse(string);
        assert!(dbg!(&res).is_ok());
    }
}
