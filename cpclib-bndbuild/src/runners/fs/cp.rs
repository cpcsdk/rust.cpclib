use cpclib_common::camino::Utf8Path;
use cpclib_common::clap::{self, FromArgMatches, Parser};
use cpclib_common::itertools::Itertools;
use cpclib_runner::event::EventObserver;
#[allow(unused_imports)]
use cpclib_runner::runner::{Runner, RunnerWithClap};

use crate::expand_glob;
use crate::task::CP_CMDS;

#[derive(Parser, Debug)]
#[command(name = "cp", about = "Copy files.")]
struct CpArgs {
    /// Files to copy. Last one being the destination
    #[arg(required = true, num_args = 2.., help = "Files to copy. Last one being the destination")]
    files: Vec<String>
}

crate::define_fs_runner_struct!(CpRunner, CpArgs);

impl<E: EventObserver> Runner for CpRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let Some(matches) = self.get_matches(itr, o)?
        else {
            return Ok(());
        };
        let args = CpArgs::from_arg_matches(&matches).map_err(|e| e.to_string())?;

        let mut errors = String::new();

        let fnames = args
            .files
            .iter()
            .flat_map(|s| expand_glob(s.as_str()))
            .collect_vec();
        let files = fnames.iter().map(Utf8Path::new).collect_vec();
        let dest = files.last();

        let copy = |from: &Utf8Path, to: &Utf8Path, error: &mut String| {
            let to = if to.is_dir() {
                to.join(from.file_name().unwrap())
            }
            else {
                to.to_path_buf()
            };

            if from.is_dir() {
                dircpy::copy_dir_advanced(from, &to, true, true, true, Vec::new(), Vec::new())
                    .map_err(|e| {
                        error.push_str(&format!(
                            "Error when copying directory {from} to {to}. {e}.\n"
                        ))
                    })
                    .map(|_| {
                        fn count_files(path: &std::path::Path) -> usize {
                            match fs_err::metadata(path) {
                                Ok(meta) if meta.is_file() => 1,
                                Ok(meta) if meta.is_dir() => {
                                    fs_err::read_dir(path)
                                        .ok()
                                        .into_iter()
                                        .flat_map(|it| it.filter_map(Result::ok))
                                        .map(|entry| count_files(&entry.path()))
                                        .sum()
                                },
                                _ => 0
                            }
                        }

                        let copied_files = count_files(from.as_std_path());
                        o.emit_stdout(&format!(
                            "Copied directory {from} to {to} ({copied_files} files)."
                        ));
                    })
            }
            else {
                fs_err::copy(from, &to)
                    .map_err(|e| {
                        error.push_str(&format!("Error when copying {from} to {to}. {e}.\n"))
                    })
                    .map(|success| {
                        o.emit_stdout(&format!("Copied {from} to {to} ({success} bytes)."))
                    })
            }
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
        fs_err::remove_file(&dst).unwrap();

        assert!(src.exists());
        assert!(!dst.exists());

        // Run the test
        let cp = CpRunner::default();
        cp.inner_run(&[src.to_string(), dst.to_string()], &())
            .unwrap();
        assert!(dst.exists());
        assert!(src.exists());
    }

    #[test]
    fn test_cp_help_flag_captured() {
        use cpclib_common::event::CapturingObserver;
        let cp = CpRunner::default();
        let obs = CapturingObserver::new();
        let result = cp.inner_run(&["--help"], &obs);
        assert!(result.is_ok(), "help should succeed");
        assert!(
            !obs.stdout_joined().is_empty(),
            "help text should appear in observer stdout"
        );
        assert!(
            obs.get_stderr().is_empty(),
            "help should not emit to stderr"
        );
    }

    #[test]
    fn test_cp_version_flag_captured() {
        use cpclib_common::event::CapturingObserver;
        let cp = CpRunner::default();
        let obs = CapturingObserver::new();
        let result = cp.inner_run(&["--version"], &obs);
        assert!(result.is_ok(), "version should succeed");
        assert!(
            !obs.stdout_joined().is_empty(),
            "version string should appear in observer stdout"
        );
        assert!(
            obs.get_stderr().is_empty(),
            "version should not emit to stderr"
        );
    }

    #[test]
    fn test_cp_invalid_arg_captured() {
        use cpclib_common::event::CapturingObserver;
        let cp = CpRunner::default();
        let obs = CapturingObserver::new();
        let result = cp.inner_run(&["--not-a-valid-flag"], &obs);
        assert!(result.is_err(), "invalid argument should fail");
        assert!(
            !obs.get_stderr().is_empty(),
            "clap error should be emitted to observer stderr"
        );
        assert!(obs.get_stdout().is_empty(), "no stdout on arg error");
    }
}
