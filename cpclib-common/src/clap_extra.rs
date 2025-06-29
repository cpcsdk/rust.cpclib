use std::str::FromStr;

use camino::Utf8PathBuf;
use winnow::error::ContextError;
use winnow::Parser;

use crate::parse_value;

pub fn utf8pathbuf_value_parser(must_exist: bool) -> impl Fn(&str) -> Result<Utf8PathBuf, String> {
    move |p: &str| {
        match Utf8PathBuf::from_str(p) {
            Ok(p) => {
                if !must_exist || p.exists() {
                    Ok(p)
                }
                else {
                    Err(format!("{p} does not exists"))
                }
            },
            Err(_) => Err(format!("{p} is not a valid filename."))
        }
    }
}

pub fn clap_parse_any_positive_number(arg: &str) -> Result<u32, String> {
    parse_value::<_, ContextError>
        .parse(arg.as_bytes())
        .map_err(|e| format!("Error when parsingthe number. {e}"))
}
