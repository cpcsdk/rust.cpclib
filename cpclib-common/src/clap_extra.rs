
use std::str::FromStr;

use camino::Utf8PathBuf;

pub fn utf8pathbuf_value_parser(must_exist: bool) -> impl Fn(&str) -> Result<Utf8PathBuf, String> {
    move |p: &str| {
        match Utf8PathBuf::from_str(p) {
            Ok(p) => {
                if !must_exist || p.exists() {
                    Ok(p)
                }
                else {
                    Err(format!("{} does not exists", p))
                }
            },
            Err(_) => Err(format!("{} is not a valid filename.", p))
        }
    }
}
