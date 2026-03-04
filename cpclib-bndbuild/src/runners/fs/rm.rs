use cpclib_common::camino::Utf8Path;
use cpclib_common::clap::{self, CommandFactory, FromArgMatches, Parser};
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::RunnerWithClap;

use crate::runners::Runner;
use crate::expand_glob;

#[derive(Parser, Debug)]
#[command(
    name = "rm",
    about = "Delete files."
)]
struct RmArgs {
    /// Files to delete
    #[arg(required = true, help = "Files to delete")]
    files: Vec<String>,
}

crate::define_fs_runner_struct!(RmRunner, RmArgs);

impl<E: EventObserver> Runner for RmRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let Some(matches) = self.get_matches(itr, o)? else {
            return Ok(());
        };
        let args = RmArgs::from_arg_matches(&matches)
            .map_err(|e| e.to_string())?;
        
        let mut errors = String::new();

        for fname in args.files.iter().flat_map(|s| expand_glob(s.as_str())) {
            let fname = Utf8Path::new(&fname);
            let res = if fname.is_dir() {
                fs_err::remove_dir_all(fname)
            }
            else {
                fs_err::remove_file(fname)
            };

            match res {
                Ok(_) => {
                    o.emit_stdout(&format!("\t{fname} removed\n" /* .display() */));
                },
                Err(e) => errors.push_str(&format!("Unable to remove {fname}. {e}.\n"))
            };
        }

        if errors.is_empty() {
            Ok(())
        }
        else {
            Err(errors)
        }
    }

    fn get_command(&self) -> &str {
        "rm"
    }
}

#[cfg(test)]
mod test {
    use std::io::Write;

    use crate::runners::Runner;
    use crate::runners::fs::rm::RmRunner;

    #[test]
    fn test_rm_single_file() {
        // Create a temporary file
        let mut temp = camino_tempfile::NamedUtf8TempFile::new().unwrap();
        temp.as_file_mut().write_all(b"test content").unwrap();
        let path = temp.into_temp_path();
        
        assert!(path.exists());

        // Run rm command
        let rm = RmRunner::default();
        rm.inner_run(&[path.to_string()], &()).unwrap();
        
        // File should be deleted
        assert!(!path.exists());
    }

    #[test]
    fn test_rm_multiple_files() {
        // Create multiple temporary files
        let mut temp1 = camino_tempfile::NamedUtf8TempFile::new().unwrap();
        let mut temp2 = camino_tempfile::NamedUtf8TempFile::new().unwrap();
        temp1.as_file_mut().write_all(b"test1").unwrap();
        temp2.as_file_mut().write_all(b"test2").unwrap();
        
        let path1 = temp1.into_temp_path();
        let path2 = temp2.into_temp_path();
        
        assert!(path1.exists());
        assert!(path2.exists());

        // Run rm command with multiple files
        let rm = RmRunner::default();
        rm.inner_run(&[path1.to_string(), path2.to_string()], &()).unwrap();
        
        // Both files should be deleted
        assert!(!path1.exists());
        assert!(!path2.exists());
    }

    #[test]
    fn test_rm_directory() {
        // Create a temporary directory
        let temp_dir = camino_tempfile::tempdir().unwrap();
        let dir_path = temp_dir.path().to_path_buf();
        
        // Create a file inside the directory
        let file_path = dir_path.join("test.txt");
        fs_err::write(&file_path, b"test").unwrap();
        
        assert!(dir_path.exists());
        assert!(file_path.exists());

        // Run rm command on directory
        let rm = RmRunner::default();
        rm.inner_run(&[dir_path.to_string()], &()).unwrap();
        
        // Directory and its contents should be deleted
        assert!(!dir_path.exists());
        assert!(!file_path.exists());
    }

    #[test]
    fn test_rm_nonexistent_file() {
        // Try to remove a file that doesn't exist
        let rm = RmRunner::default();
        let result = rm.inner_run(&["/nonexistent/file/path.txt"], &());
        
        // Should return an error
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unable to remove"));
    }

    #[test]
    fn test_rm_help_flag() {
        let rm = RmRunner::default();
        let result = rm.inner_run(&["--help"], &());
        
        // Should succeed (help is printed via emit_stdout)
        assert!(result.is_ok());
    }

    #[test]
    fn test_rm_version_flag() {
        let rm = RmRunner::default();
        let result = rm.inner_run(&["--version"], &());
        
        // Should succeed (version is printed via emit_stdout)
        assert!(result.is_ok());
    }
}
