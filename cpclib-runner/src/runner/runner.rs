use std::io::{BufRead, BufReader};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::thread;

use clap::{ArgMatches, Command};
use cpclib_common::itertools::Itertools;
use transparent::{CommandExt, TransparentChild, TransparentRunner};

use crate::event::EventObserver;
use crate::runner::arguments::get_all_args;

#[derive(Default, Clone, Copy, Debug)]
pub enum RunInDir {
    #[default]
    CurrentDir,
    AppDir
}

pub trait Runner {
    type EventObserver: EventObserver;

    /// Run the task and return true if successfull
    fn run(&self, arguments: &str, o: &Self::EventObserver) -> Result<(), String> {
        let args = get_all_args(arguments)?;
        self.inner_run(&args, o)
    }

    /// Implement the command specific action
    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &Self::EventObserver) -> Result<(), String>;

    fn get_command(&self) -> &str;
}

pub trait RunnerWithClap: Runner + Default {
    fn get_clap_command(&self) -> &Command;

    fn get_matches<S: AsRef<str>>(&self, itr: &[S]) -> Result<ArgMatches, String> {
        self.get_clap_command()
            .clone()
            .try_get_matches_from(itr.iter().map(|s| s.as_ref()))
            .map_err(|e| e.to_string())
    }

    fn render_help() -> String {
        Self::default()
            .get_clap_command()
            .clone()
            .disable_help_flag(true)
            .render_long_help()
            .to_string()
    }
}

pub struct ExternRunner<E: EventObserver> {
    in_dir: RunInDir,
    transparent: bool,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for ExternRunner<E> {
    fn default() -> Self {
        Self::new(RunInDir::CurrentDir)
    }
}

impl<E: EventObserver> ExternRunner<E> {
    pub fn new(in_dir: RunInDir) -> Self {
        Self {
            in_dir,
            transparent: false,
            _phantom: Default::default()
        }
    }

    pub fn new_transparent(in_dir: RunInDir) -> Self {
        Self {
            in_dir,
            transparent: true,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> Runner for ExternRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let itr = itr.iter().map(|s| s.as_ref()).collect_vec();

        // WARNING
        // Deactivated because if makes fail normal progam on Linux
        // however, it was maybe mandatory for Windows
        // let app = std::fs::canonicalize(&itr[0])
        //     .map_err(|e| format!("Wrong executable {}.{}", &itr[0], e.to_string()))?;
        let app = &itr[0];

        let cwd = std::env::current_dir()
            .map_err(|e| format!("Unable to get the current working directory {}.", e))?;
        let cwd = std::fs::canonicalize(cwd)
            .map_err(|e| format!("Unable to get the current working directory {}.", e))?;

        let mut cmd = std::process::Command::new(app);

        let in_dir = match self.in_dir {
            RunInDir::CurrentDir => cwd,
            RunInDir::AppDir => {
                let base = if app == &"wine" { itr[1] } else { app };
                PathBuf::from(std::path::Path::new(base).parent().unwrap()) // this path is because of AMSpiriT
            }
        };
        cmd.current_dir(in_dir);
        for arg in &itr[1..] {
            cmd.arg(arg);
        }

        let cmd = cmd.stderr(Stdio::piped()).stdout(Stdio::piped());

        enum MyChild {
            Transparent(TransparentChild),
            Standard(Child)
        }
        impl From<TransparentChild> for MyChild {
            fn from(value: TransparentChild) -> Self {
                Self::Transparent(value)
            }
        }
        impl From<Child> for MyChild {
            fn from(value: Child) -> Self {
                Self::Standard(value)
            }
        }
        impl Deref for MyChild {
            type Target = Child;

            fn deref(&self) -> &Self::Target {
                match self {
                    MyChild::Transparent(child) => child.deref(),
                    MyChild::Standard(child) => child
                }
            }
        }
        impl DerefMut for MyChild {
            fn deref_mut(&mut self) -> &mut Self::Target {
                match self {
                    MyChild::Transparent(child) => child.deref_mut(),
                    MyChild::Standard(child) => child
                }
            }
        }

        let mut cmd: MyChild = if self.transparent {
            cmd.spawn_transparent(&TransparentRunner::new())
                .map(|c| c.into())
        }
        else {
            cmd.spawn().map(|c| c.into())
        }
        .map_err(|e| format!("Error while launching {}. {}", &itr[0], e))?;

        // the process is running in another thread. We'll collect its outputs in yet other threads
        let child_stdout = cmd
            .stdout
            .take()
            .expect("Internal error, could not take stdout");
        let child_stderr = cmd
            .stderr
            .take()
            .expect("Internal error, could not take stderr");

        thread::scope(|s| {
            s.spawn(|| {
                let stdout_lines = BufReader::new(child_stdout).lines();
                for line in stdout_lines {
                    let line = line.unwrap();
                    o.emit_stdout(&line);
                }
            });
            s.spawn(|| {
                let stderr_lines = BufReader::new(child_stderr).lines();
                for line in stderr_lines {
                    let line = line.unwrap();
                    o.emit_stderr(&line);
                }
            });
        });

        let status = cmd
            .wait()
            .map_err(|e| format!("Error while executing {}. {}", &itr[0], e))?;

        if !status.success() {
            return Err("Error while launching the command.".to_owned());
        }

        Ok(())
    }

    fn get_command(&self) -> &str {
        "external"
    }
}
