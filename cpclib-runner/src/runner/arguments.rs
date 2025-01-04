use glob::glob;
use shlex::split;

/// Get all args (split string as done in shell and apply glob matching)
pub fn get_all_args(arguments: &str) -> Result<Vec<String>, String> {
    let init_args = split(arguments)
        .ok_or_else(|| format!("There are errors in the arguments: {}", arguments))?;
    let mut res = Vec::new();
    for (idx, p) in init_args.into_iter().enumerate() {
        // XXX no pattern matching for the command name. 
        //     TODO check if it is ok to do that.
        //     It avoids issues when ./ is used in command name
        if idx == 0 {
            res.push(p)
        } else {
            match glob(&p) {
                Ok(entries) => {
                    let mut added = 0;
                    for entry in entries {
                        match entry {
                            Ok(p) => res.push(p.display().to_string()),
                            Err(e) => res.push(e.path().display().to_string())
                        }
                        added += 1;
                    }
                    if added == 0 {
                        res.push(p);
                    }
                },
                Err(_) => res.push(p)
            }
        }
    }

    Ok(res)
}

#[cfg(test)]
mod test {
    use crate::runner::arguments::get_all_args;

    #[test]
    fn test_arguments_handling() {
        assert_eq!(get_all_args("a b c d").unwrap(), vec!["a", "b", "c", "d"]);

        assert_eq!(
            get_all_args("a \"b\" c d").unwrap(),
            (vec!["a", "b", "c", "d"])
        );

        assert_eq!(
            get_all_args("a \"b1 b2\" \"c\" 'd'").unwrap(),
            (vec!["a", "b1 b2", "c", "d"])
        );

        assert_eq!(
            get_all_args("basm ucpm.asm --snapshot -o ucpm.sna --ace ucpm.rasm --lst ucpm.lst --override -DFNAME=\\\"UCPM\\\" \"-DDSK=\\\"u cpm.dsk\\\"\"").unwrap(),
            (vec!["basm", "ucpm.asm", "--snapshot", "-o", "ucpm.sna", "--ace", "ucpm.rasm", "--lst", "ucpm.lst", "--override", "-DFNAME=\"UCPM\"", "-DDSK=\"u cpm.dsk\""])
        );

        assert!(get_all_args("one_ok \"two_error").is_err());
    }
}
