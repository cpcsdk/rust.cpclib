use glob::glob;
use shlex::split;

/// Get all args (split string as done in shell and apply glob matching)
pub fn get_all_args(arguments: &str) -> Vec<String> {
    let init_args = split(arguments).unwrap_or_default();
    let mut res = Vec::new();
    for p in init_args {
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
    res
}
