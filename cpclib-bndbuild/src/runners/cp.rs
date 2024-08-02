use std::path::Path;

use cpclib_common::clap::{self, Arg, ArgAction};
use cpclib_common::itertools::Itertools;

use super::Runner;
use crate::task::CP_CMDS;
use crate::{built_info, expand_glob};

#[derive(Default)]
pub struct CpRunner {}

impl CpRunner {
    pub fn print_help(&self) {
        clap::Command::new("cp")
            .before_help("Copy files.")
            .disable_help_flag(true)
            .after_help(format!(
                "Inner command of {} {}",
                built_info::PKG_NAME,
                built_info::PKG_VERSION
            ))
            .arg(
                Arg::new("arguments")
                    .action(ArgAction::Append)
                    .help("Files to copy. Last one being the destination")
            )
            .print_long_help()
            .unwrap();
    }
}

impl Runner for CpRunner {
    fn inner_run<S: AsRef<str>>(&self, itr: &[S]) -> Result<(), String> {
        let mut errors = String::new();

        let fnames = itr
            .iter()
            .map(|s| s.as_ref())
            .map(expand_glob)
            .flatten()
            .collect_vec();
        let files = fnames.iter().map(|fname| Path::new(fname)).collect_vec();
        let dest = files.last();

        let copy = |from: &Path, to: &Path, error: &mut String| {
            std::fs::copy(from, to).map_err(|e| {
                error.push_str(&format!(
                    "Error when copying {} to {}. {}.\n",
                    from.display(),
                    to.display(),
                    e.to_string()
                ))
            });
        };

        match files.len() {
            0 => {
                errors.push_str("No source and destination provided\n");
            },

            1 => {
                errors.push_str("No source or destination provided\n");
            },

            2 => {
                let dest = dest.unwrap();
                let src = files.first().unwrap();
                let dest = if dest.is_dir() {
                    dest.join(src.file_name().unwrap())
                }
                else {
                    dest.to_path_buf()
                };
                copy(src, &dest, &mut errors);
            },

            _ => {
                let dest = dest.unwrap();
                if !dest.is_dir() {
                    errors.push_str(&format!("{} must be a directory.", dest.display()))
                }
                else {
                    let files = &files[..files.len() - 1];
                    for src in files.iter() {
                        let dst = dest.join(src.file_name().unwrap());
                        copy(src, &dst, &mut errors);
                    }
                }
            }
        };

        if errors.is_empty() {
            Ok(())
        }
        else {
            Err(errors)
        }
    }

    fn get_command(&self) -> &'static str {
        &CP_CMDS[0]
    }
}

#[cfg(test)]
mod test {
    use std::io::Write;

    use crate::runners::cp::CpRunner;
    use crate::runners::Runner;

    #[test]

    fn test_copy_successful() {
        // prepare the files for the test
        let mut src = tempfile::NamedTempFile::new().unwrap();
        let dst = tempfile::NamedTempFile::new().unwrap();

        src.as_file_mut().write("test".as_bytes()).unwrap();

        let src = src.into_temp_path();
        let dst = dst.into_temp_path();
        std::fs::remove_file(&dst).unwrap();

        assert!(src.exists());
        assert!(!dst.exists());

        // Run the test
        let cp = CpRunner::default();
        cp.inner_run(&[src.display().to_string(), dst.display().to_string()])
            .unwrap();
        assert!(dst.exists());
    }
}
