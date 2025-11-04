use std::marker::PhantomData;

use cpclib_common::camino::Utf8Path;
use cpclib_common::clap::{self, Arg, ArgAction};
use cpclib_common::itertools::Itertools;
use cpclib_runner::event::EventObserver;

use crate::runners::Runner;
use crate::task::CP_CMDS;
use crate::{built_info, expand_glob};

pub struct CpRunner<E: EventObserver> {
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for CpRunner<E> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData::<E>
        }
    }
}

impl<E: EventObserver> CpRunner<E> {
    pub fn render_help() -> String {
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
            .render_long_help()
            .to_string()
    }
}

impl<E: EventObserver> Runner for CpRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], _o: &E) -> Result<(), String> {
        let mut errors = String::new();

        let fnames = itr
            .iter()
            .map(|s| s.as_ref())
            .flat_map(expand_glob)
            .collect_vec();
        let files = fnames.iter().map(Utf8Path::new).collect_vec();
        let dest = files.last();

        let copy = |from: &Utf8Path, to: &Utf8Path, error: &mut String| {
            std::fs::copy(from, to)
                .map_err(|e| error.push_str(&format!("Error when copying {from} to {to}. {e}.\n")))
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
                let _ = copy(src, &dest, &mut errors);
            },

            _ => {
                let dest = dest.unwrap();
                if !dest.is_dir() {
                    errors.push_str(&format!("{dest} must be a directory."))
                }
                else {
                    let files = &files[..files.len() - 1];
                    for src in files.iter() {
                        let dst = dest.join(src.file_name().unwrap());
                        let _ = copy(src, &dst, &mut errors);
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
        CP_CMDS[0]
    }
}

#[cfg(test)]
mod test {
    use std::io::Write;

    use crate::runners::Runner;
    use crate::runners::fs::cp::CpRunner;

    #[test]

    fn test_copy_successful() {
        // prepare the files for the test
        let mut src = camino_tempfile::NamedUtf8TempFile::new().unwrap();
        let dst = camino_tempfile::NamedUtf8TempFile::new().unwrap();

        src.as_file_mut().write("test".as_bytes()).unwrap();

        let src = src.into_temp_path();
        let dst = dst.into_temp_path();
        std::fs::remove_file(&dst).unwrap();

        assert!(src.exists());
        assert!(!dst.exists());

        // Run the test
        let cp = CpRunner::default();
        cp.inner_run(&[src.to_string(), dst.to_string()], &())
            .unwrap();
        assert!(dst.exists());
    }
}
