use cpclib_common::camino::Utf8Path;
use cpclib_common::clap::{self, FromArgMatches, Parser};
use cpclib_common::itertools::Itertools;
use cpclib_runner::event::EventObserver;
#[allow(unused_imports)]
use cpclib_runner::runner::{Runner, RunnerWithClap};

use crate::expand_glob;
use crate::task::MV_CMDS;

#[derive(Parser, Debug)]
#[command(name = "mv", about = "Rename files.")]
struct MvArgs {
    /// Files to move. With 2 files, first one is renamed as second one. With more than 2 files, last one is the destination directory.
    #[arg(required = true, num_args = 2.., help = "Files to move. Last one is the destination")]
    files: Vec<String>
}

crate::define_fs_runner_struct!(MvRunner, MvArgs);

impl<E: EventObserver> Runner for MvRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let Some(matches) = self.get_matches(itr, o)?
        else {
            return Ok(());
        };
        let args = MvArgs::from_arg_matches(&matches).map_err(|e| e.to_string())?;

        let mut errors = String::new();

        let fnames = args
            .files
            .iter()
            .flat_map(|s| expand_glob(s.as_str()))
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

                fs_err::rename(from, &to).map_err(|e| {
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
        fs_err::remove_file(&dst).unwrap();

        assert!(src.exists());
        assert!(!dst.exists());

        // Run the test
        let mv = MvRunner::default();
        mv.inner_run(&[src.to_string(), dst.to_string()], &())
            .unwrap();
        assert!(dst.exists());
        assert!(!src.exists());
    }

    #[test]
    fn test_mv_rename_file() {
        // Create a temporary file
        let mut temp = camino_tempfile::NamedUtf8TempFile::new().unwrap();
        temp.as_file_mut().write_all(b"test content").unwrap();
        let src_path = temp.into_temp_path();

        // Create destination path in same directory
        let dst_path = src_path.parent().unwrap().join("renamed.txt");

        assert!(src_path.exists());
        assert!(!dst_path.exists());

        // Run mv command
        let mv = MvRunner::default();
        mv.inner_run(&[src_path.to_string(), dst_path.to_string()], &())
            .unwrap();

        // File should be renamed
        assert!(!src_path.exists());
        assert!(dst_path.exists());

        // Cleanup
        let _ = fs_err::remove_file(dst_path);
    }

    #[test]
    fn test_mv_multiple_files_to_directory() {
        // Create temporary files
        let mut temp1 = camino_tempfile::NamedUtf8TempFile::new().unwrap();
        let mut temp2 = camino_tempfile::NamedUtf8TempFile::new().unwrap();
        temp1.as_file_mut().write_all(b"test1").unwrap();
        temp2.as_file_mut().write_all(b"test2").unwrap();

        let path1 = temp1.into_temp_path();
        let path2 = temp2.into_temp_path();

        // Create destination directory
        let dest_dir = camino_tempfile::tempdir().unwrap();
        let dest_path = dest_dir.path().to_path_buf();

        assert!(path1.exists());
        assert!(path2.exists());
        assert!(dest_path.exists());
        assert!(dest_path.is_dir());

        // Run mv command with multiple files
        let mv = MvRunner::default();
        mv.inner_run(
            &[path1.to_string(), path2.to_string(), dest_path.to_string()],
            &()
        )
        .unwrap();

        // Files should be moved to directory
        assert!(!path1.exists());
        assert!(!path2.exists());
        assert!(dest_path.join(path1.file_name().unwrap()).exists());
        assert!(dest_path.join(path2.file_name().unwrap()).exists());
    }

    #[test]
    fn test_mv_file_to_directory() {
        // Create a temporary file
        let mut temp = camino_tempfile::NamedUtf8TempFile::new().unwrap();
        temp.as_file_mut().write_all(b"test").unwrap();
        let file_path = temp.into_temp_path();

        // Create destination directory
        let dest_dir = camino_tempfile::tempdir().unwrap();
        let dest_path = dest_dir.path().to_path_buf();

        assert!(file_path.exists());
        assert!(dest_path.is_dir());

        // Run mv command
        let mv = MvRunner::default();
        mv.inner_run(&[file_path.to_string(), dest_path.to_string()], &())
            .unwrap();

        // File should be moved into directory
        assert!(!file_path.exists());
        assert!(dest_path.join(file_path.file_name().unwrap()).exists());
    }

    #[test]
    fn test_mv_multiple_files_to_non_directory_fails() {
        // Create temporary files
        let mut temp1 = camino_tempfile::NamedUtf8TempFile::new().unwrap();
        let mut temp2 = camino_tempfile::NamedUtf8TempFile::new().unwrap();
        let mut dest_file = camino_tempfile::NamedUtf8TempFile::new().unwrap();

        temp1.as_file_mut().write_all(b"test1").unwrap();
        temp2.as_file_mut().write_all(b"test2").unwrap();
        dest_file.as_file_mut().write_all(b"dest").unwrap();

        let path1 = temp1.into_temp_path();
        let path2 = temp2.into_temp_path();
        let dest_path = dest_file.into_temp_path();

        // Run mv command - should fail because dest is not a directory
        let mv = MvRunner::default();
        let result = mv.inner_run(
            &[path1.to_string(), path2.to_string(), dest_path.to_string()],
            &()
        );

        // Should return an error
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be a directory"));
    }

    #[test]
    fn test_mv_nonexistent_file() {
        let temp_dir = camino_tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("dest.txt");

        // Try to move a file that doesn't exist
        let mv = MvRunner::default();
        let result = mv.inner_run(&["/nonexistent/file.txt", dest.as_str()], &());

        // Should return an error
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Error when moving"));
    }

    #[test]
    fn test_mv_help_flag() {
        let mv = MvRunner::default();
        let result = mv.inner_run(&["--help"], &());

        // Should succeed (help is printed via emit_stdout)
        assert!(result.is_ok());
    }

    #[test]
    fn test_mv_version_flag() {
        let mv = MvRunner::default();
        let result = mv.inner_run(&["--version"], &());

        // Should succeed (version is printed via emit_stdout)
        assert!(result.is_ok());
    }

    #[test]
    fn test_mv_help_flag_captured() {
        use cpclib_common::event::CapturingObserver;
        let mv = MvRunner::default();
        let obs = CapturingObserver::new();
        let result = mv.inner_run(&["--help"], &obs);
        assert!(result.is_ok(), "help should succeed");
        assert!(!obs.stdout_joined().is_empty(), "help text should appear in observer stdout");
        assert!(obs.get_stderr().is_empty(), "help should not emit to stderr");
    }

    #[test]
    fn test_mv_version_flag_captured() {
        use cpclib_common::event::CapturingObserver;
        let mv = MvRunner::default();
        let obs = CapturingObserver::new();
        let result = mv.inner_run(&["--version"], &obs);
        assert!(result.is_ok(), "version should succeed");
        assert!(!obs.stdout_joined().is_empty(), "version string should appear in observer stdout");
        assert!(obs.get_stderr().is_empty(), "version should not emit to stderr");
    }

    #[test]
    fn test_mv_invalid_arg_captured() {
        use cpclib_common::event::CapturingObserver;
        let mv = MvRunner::default();
        let obs = CapturingObserver::new();
        let result = mv.inner_run(&["--not-a-valid-flag"], &obs);
        assert!(result.is_err(), "invalid argument should fail");
        assert!(!obs.get_stderr().is_empty(), "clap error should be emitted to observer stderr");
        assert!(obs.get_stdout().is_empty(), "no stdout on arg error");
    }
}
