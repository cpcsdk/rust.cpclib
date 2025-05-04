use std::io::{BufRead, BufReader};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::thread;

use clap::builder::styling::AnsiColor;
use clap::builder::Styles;
use clap::{ArgMatches, Command, FromArgMatches, Parser};
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

    /// Return the match objectthat encodes the corresponding options.
    /// If version or help is requested, output them and consumes the args
    fn get_matches<S: AsRef<str>>(
        &self,
        itr: &[S],
        e: &dyn EventObserver
    ) -> Result<Option<ArgMatches>, String> {
        let args = self
            .get_clap_command()
            .clone()
            .try_get_matches_from(itr.iter().map(|s| s.as_ref()))
            .map_err(|e| e.to_string())?;

        if args.get_flag("version") {
            self.emit_version(e);
            Ok(None)
        }
        else if args.get_flag("help") {
            self.emit_help(e);
            Ok(None)
        }
        else {
            Ok(Some(args))
        }
    }

    fn render_help() -> String {
        let cmd = Self::default().get_clap_command().clone();

        let styles = Styles::styled()
            .header(AnsiColor::Yellow.on_default())
            .usage(AnsiColor::Green.on_default())
            .literal(AnsiColor::Green.on_default())
            .placeholder(AnsiColor::Green.on_default());
        cmd.styles(styles)
            .disable_help_flag(true)
            .render_long_help()
            .ansi()
            .to_string()
    }

    fn render_version() -> String {
        Self::default().get_clap_command().clone().render_version()
    }
    fn emit_help(&self, e: &dyn EventObserver) {
        e.emit_stdout(&Self::render_help());
    }

    fn emit_version(&self, e: &dyn EventObserver) {
        e.emit_stdout(&Self::render_version());
    }
}

pub trait RunnerWithClapMatches: RunnerWithClap {}

pub trait RunnerWithClapDerive: RunnerWithClap {
    type Args: Parser;
    fn get_args<S: AsRef<str>>(
        &self,
        itr: &[S],
        e: &dyn EventObserver
    ) -> Result<Option<Self::Args>, String> {
        let matches = self.get_matches(itr, e)?;
        if matches.is_none() {
            return Ok(None);
        }
        let matches = matches.unwrap();
        let args: Self::Args = Self::Args::from_arg_matches(&matches).expect("BUG");
        Ok(Some(args))
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
        // /
        // for a in &itr {
        // eprintln!(">> {a}");
        // }
        // WARNING
        // Deactivated because if makes fail normal progam on Linux
        // however, it was maybe mandatory for Windows
        // let app = std::fs::canonicalize(&itr[0])
        //     .map_err(|e| format!("Wrong executable {}.{}", &itr[0], e.to_string()))?;
        let app = itr[0];

        let cwd = std::env::current_dir()
            .map_err(|e| format!("Unable to get the current working directory {}.", e))?;
        let cwd = std::fs::canonicalize(cwd)
            .map_err(|e| format!("Unable to get the current working directory {}.", e))?;

        let mut cmd = std::process::Command::new(app);

        let in_dir = match self.in_dir {
            RunInDir::CurrentDir => cwd,
            RunInDir::AppDir => {
                let base = if app == "wine" { itr[1] } else { app };
                PathBuf::from(std::path::Path::new(base).parent().unwrap()) // this path is because of AMSpiriT
            }
        };
        cmd.current_dir(in_dir);
        for arg in &itr[1..] {
            cmd.arg(arg);
        }

        let cmd = cmd.stderr(Stdio::piped()).stdout(Stdio::piped());

        #[derive(Debug)]
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

        use utf8_chars::BufReadCharsExt;
        thread::scope(|s| {
            s.spawn(|| {
                let mut stdout = BufReader::new(child_stdout);
                let mut current_string = String::new();
                for c in stdout.chars() {
                    // TODO handle a byte buffer
                    if let Ok(c) = c {
                        current_string.push(c);

                        if c == '\n' {
                            o.emit_stdout(&current_string);
                            current_string.clear();
                        }
                    }
                }
                if !current_string.is_empty() {
                    o.emit_stdout(&current_string);
                }
            });
            s.spawn(|| {
                // TODO use the same technique than stdout
                let stderr_lines = BufReader::new(child_stderr).lines();
                for line in stderr_lines {
                    let line = line.unwrap();
                    o.emit_stderr(&line);
                    o.emit_stderr("\n");
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
