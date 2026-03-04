use cpclib_common::camino::Utf8Path;
use cpclib_common::clap::{self, CommandFactory, FromArgMatches, Parser};
use cpclib_common::itertools::Itertools;
use cpclib_runner::event::EventObserver;

use crate::runners::{Runner, RunnerWithClap};
use crate::task::MKDIR_CMDS;
use crate::expand_glob;

#[derive(Parser, Debug)]
#[command(
    name = "mkdir",
    about = "Create directories."
)]
struct MkdirArgs {
    /// Set to specify if missing parent directories must be created
    #[arg(short, long, help = "Create parent directories as needed")]
    parents: bool,

    /// Set to specify we ignore existing folders
    #[arg(short, long, help = "Do not error if directory already exists")]
    ignore: bool,

    /// Folders to create
    #[arg(required = true, help = "Directories to create")]
    directories: Vec<String>,
}

crate::define_fs_runner_struct!(MkdirRunner, MkdirArgs);

impl<E: EventObserver> Runner for MkdirRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let Some(matches) = self.get_matches(itr, o)? else {
            return Ok(());
        };
        let args = MkdirArgs::from_arg_matches(&matches)
            .map_err(|e| e.to_string())?;
        
        let mut errors = String::new();

        for fname in args.directories
            .iter()
            .flat_map(|s| expand_glob(s.as_str()))
        {
            let fname = Utf8Path::new(&fname);

            if fname.exists() {
                if !args.ignore {
                    errors.push_str(&format!(
                        "{fname} already exists. Use --ignore to not crash.\n",
                    ));
                }
            }
            else {
                let res = if args.parents {
                    fs_err::create_dir_all(fname)
                }
                else {
                    fs_err::create_dir(fname)
                };

                match res {
                    Ok(_) => {
                        o.emit_stdout(&format!("\t{fname} created"));
                    },
                    Err(e) => errors.push_str(&format!("Unable to create {fname}. {e}.\n"))
                };
            }
        }

        if errors.is_empty() {
            Ok(())
        }
        else {
            Err(errors)
        }
    }

    fn get_command(&self) -> &str {
        MKDIR_CMDS[0]
    }
}

#[cfg(test)]
mod test {
    use crate::runners::Runner;
    use crate::runners::fs::mkdir::MkdirRunner;

    #[test]
    fn test_mkdir_single_directory() {
        let temp_dir = camino_tempfile::tempdir().unwrap();
        let new_dir = temp_dir.path().join("new_dir");
        
        assert!(!new_dir.exists());

        // Create directory
        let mkdir = MkdirRunner::default();
        mkdir.inner_run(&[new_dir.to_string()], &()).unwrap();
        
        // Directory should exist
        assert!(new_dir.exists());
        assert!(new_dir.is_dir());
    }

    #[test]
    fn test_mkdir_multiple_directories() {
        let temp_dir = camino_tempfile::tempdir().unwrap();
        let dir1 = temp_dir.path().join("dir1");
        let dir2 = temp_dir.path().join("dir2");
        
        assert!(!dir1.exists());
        assert!(!dir2.exists());

        // Create multiple directories
        let mkdir = MkdirRunner::default();
        mkdir.inner_run(&[dir1.to_string(), dir2.to_string()], &()).unwrap();
        
        // Both directories should exist
        assert!(dir1.exists());
        assert!(dir1.is_dir());
        assert!(dir2.exists());
        assert!(dir2.is_dir());
    }

    #[test]
    fn test_mkdir_with_parents() {
        let temp_dir = camino_tempfile::tempdir().unwrap();
        let nested_dir = temp_dir.path().join("parent").join("child").join("grandchild");
        
        assert!(!nested_dir.exists());

        // Create nested directory with -p flag
        let mkdir = MkdirRunner::default();
        mkdir.inner_run(&["-p", nested_dir.as_str()], &()).unwrap();
        
        // Nested directory and all parents should exist
        assert!(nested_dir.exists());
        assert!(nested_dir.is_dir());
        assert!(nested_dir.parent().unwrap().exists());
        assert!(nested_dir.parent().unwrap().parent().unwrap().exists());
    }

    #[test]
    fn test_mkdir_without_parents_fails() {
        let temp_dir = camino_tempfile::tempdir().unwrap();
        let nested_dir = temp_dir.path().join("parent").join("child");
        
        assert!(!nested_dir.exists());

        // Try to create nested directory without -p flag
        let mkdir = MkdirRunner::default();
        let result = mkdir.inner_run(&[nested_dir.to_string()], &());
        
        // Should fail because parent doesn't exist
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unable to create"));
    }

    #[test]
    fn test_mkdir_existing_directory_fails() {
        let temp_dir = camino_tempfile::tempdir().unwrap();
        let dir = temp_dir.path().join("existing");
        
        // Create directory
        fs_err::create_dir(&dir).unwrap();
        assert!(dir.exists());

        // Try to create same directory again without --ignore
        let mkdir = MkdirRunner::default();
        let result = mkdir.inner_run(&[dir.to_string()], &());
        
        // Should fail
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_mkdir_existing_directory_with_ignore() {
        let temp_dir = camino_tempfile::tempdir().unwrap();
        let dir = temp_dir.path().join("existing");
        
        // Create directory
        fs_err::create_dir(&dir).unwrap();
        assert!(dir.exists());

        // Try to create same directory with --ignore flag
        let mkdir = MkdirRunner::default();
        let result = mkdir.inner_run(&["--ignore", dir.as_str()], &());
        
        // Should succeed
        assert!(result.is_ok());
    }

    #[test]
    fn test_mkdir_with_parents_and_ignore() {
        let temp_dir = camino_tempfile::tempdir().unwrap();
        let nested_dir = temp_dir.path().join("parent").join("child");
        
        // Create the directory first
        fs_err::create_dir_all(&nested_dir).unwrap();
        assert!(nested_dir.exists());

        // Try to create same nested directory with -p and -i flags
        let mkdir = MkdirRunner::default();
        let result = mkdir.inner_run(&["-p", "-i", nested_dir.as_str()], &());
        
        // Should succeed
        assert!(result.is_ok());
    }

    #[test]
    fn test_mkdir_help_flag() {
        let mkdir = MkdirRunner::default();
        let result = mkdir.inner_run(&["--help"], &());
        
        // Should succeed (help is printed via emit_stdout)
        assert!(result.is_ok());
    }

    #[test]
    fn test_mkdir_version_flag() {
        let mkdir = MkdirRunner::default();
        let result = mkdir.inner_run(&["--version"], &());
        
        // Should succeed (version is printed via emit_stdout)
        assert!(result.is_ok());
    }
}
