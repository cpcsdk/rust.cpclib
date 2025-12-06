use std::marker::PhantomData;

use cpclib_common::camino::Utf8Path;
use cpclib_common::clap::{self, Arg, ArgAction};
use cpclib_common::itertools::Itertools;
use cpclib_runner::event::EventObserver;

use crate::runners::Runner;
use crate::task::MV_CMDS;
use crate::{built_info, expand_glob};

pub struct MvRunner<E: EventObserver> {
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for MvRunner<E> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData::<E>
        }
    }
}

impl<E: EventObserver> MvRunner<E> {
    pub fn render_help() -> String {
        clap::Command::new("mv")
            .before_help("Rename files.")
            .disable_help_flag(true)
            .after_help(format!(
                "Inner command of {} {}",
                built_info::PKG_NAME,
                built_info::PKG_VERSION
            ))
            .arg(
                Arg::new("arguments")
                    .action(ArgAction::Append)
                    .help("Files to move. With 2 files, first one is renamed as second one. With more than 2 files, last one is the destination directory.")
					.num_args(2..)
            )
            .render_long_help()
            .to_string()
    }
}

impl<E: EventObserver> Runner for MvRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], _o: &E) -> Result<(), String> {
        let mut errors = String::new();

        let fnames = itr
            .iter()
            .map(|s| s.as_ref())
            .flat_map(expand_glob)
            .collect_vec();
        let mut files = fnames.iter().map(Utf8Path::new).collect_vec();
        let nb_args = files.len();
        let dest = files.pop().unwrap();
        let dest_is_dir = dest.is_dir();

        if nb_args > 2 && !dest_is_dir {
            errors.push_str(&format!("{dest} must be a directory."))
        }
        else {
            let r#move = |from: &Utf8Path, error: &mut String| {
                let to = if dest_is_dir {
                    dest.join(from.file_name().unwrap())
                }
                else {
                    dest.to_path_buf()
                };

                std::fs::rename(from, &to).map_err(|e| {
                    error.push_str(&format!("Error when moving {from} to {to}. {e}.\n"))
                })
            };

            for src in files.into_iter() {
                let _ = r#move(src, &mut errors);
            }
        }

        if errors.is_empty() {
            Ok(())
        }
        else {
            Err(errors)
        }
    }

    fn get_command(&self) -> &'static str {
        MV_CMDS[0]
    }
}

#[cfg(test)]
mod test {
    use std::io::Write;

    use crate::runners::Runner;
    use crate::runners::fs::mv::MvRunner;

    #[test]

    fn test_move_1file_successful() {
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
        let mv = MvRunner::default();
        mv.inner_run(&[src.to_string(), dst.to_string()], &())
            .unwrap();
        assert!(dst.exists());
        assert!(!src.exists());
    }
}
