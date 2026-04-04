use std::io::Read;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::thread;

use clap::builder::Styles;
use clap::builder::styling::AnsiColor;
use clap::{ArgMatches, Command, FromArgMatches, Parser};
use cpclib_common::itertools::Itertools;
#[cfg(feature = "transparent-x11")]
use transparent::{CommandExt, TransparentChild, TransparentRunner};

use crate::child_registry::{deregister_child_pid, register_child_pid};
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
    /// If version or help is requested, output them and consumes the args.
    /// Clap argument errors are emitted through `e.emit_stderr` before
    /// returning `Err`, so they are captured by any observer under test.
    fn get_matches<S: AsRef<str>>(
        &self,
        itr: &[S],
        e: &dyn EventObserver
    ) -> Result<Option<ArgMatches>, String> {
        let args = match self
            .get_clap_command()
            .clone()
            .try_get_matches_from(itr.iter().map(|s| s.as_ref()))
        {
            Ok(args) => args,
            Err(err) => {
                e.emit_stderr(&err.to_string());
                return Err(String::from("Argument parsing failed"));
            }
        };

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
        let args: Self::Args = Self::Args::from_arg_matches(&matches).map_err(|err| {
            let msg = format!("Failed to parse arguments: {err}");
            e.emit_stderr(&msg);
            msg
        })?;
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

    #[cfg(feature = "transparent-x11")]
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
            .map_err(|e| format!("Unable to get the current working directory {e}."))?;
        let cwd = fs_err::canonicalize(cwd)
            .map_err(|e| format!("Unable to get the current working directory {e}."))?;

        let in_dir = match self.in_dir {
            RunInDir::CurrentDir => cwd,
            RunInDir::AppDir => {
                let base = if app == "wine" { itr[1] } else { app };
                PathBuf::from(std::path::Path::new(base).parent().unwrap()) // this path is because of AMSpiriT
            }
        };

        // transparent-x11 uses std::process::Command; keep legacy pipe approach for that path
        #[cfg(feature = "transparent-x11")]
        if self.transparent {
            use std::io::BufReader;
            use std::process::{Child, Stdio};

            use utf8_chars::BufReadCharsExt;

            let mut cmd = std::process::Command::new(app);
            cmd.current_dir(&in_dir);
            for arg in &itr[1..] {
                cmd.arg(arg);
            }
            let cmd = cmd.stderr(Stdio::piped()).stdout(Stdio::piped());
            let mut child: Child = cmd
                .spawn_transparent(&TransparentRunner::new())
                .map_err(|e| format!("Error while launching {}. {}", app, e))?;
            let child_pid = child.id();
            register_child_pid(child_pid);
            let child_stdout = child
                .stdout
                .take()
                .expect("Internal error, could not take stdout");
            let child_stderr = child
                .stderr
                .take()
                .expect("Internal error, could not take stderr");
            thread::scope(|s| {
                s.spawn(|| {
                    let mut stdout = BufReader::new(child_stdout);
                    let mut current_string = String::new();
                    for c in stdout.chars().flatten() {
                        current_string.push(c);
                        if c == '\n' {
                            o.emit_stdout(&current_string);
                            current_string.clear();
                        }
                    }
                    if !current_string.is_empty() {
                        o.emit_stdout(&current_string);
                    }
                });
                s.spawn(|| {
                    let mut stderr = BufReader::new(child_stderr);
                    let mut current_string = String::new();
                    for c in stderr.chars().flatten() {
                        current_string.push(c);
                        if c == '\n' {
                            o.emit_stderr(&current_string);
                            current_string.clear();
                        }
                    }
                    if !current_string.is_empty() {
                        o.emit_stderr(&current_string);
                    }
                });
            });
            let status = child
                .wait()
                .map_err(|e| format!("Error while executing {}. {}", app, e))?;
            deregister_child_pid(child_pid);
            return if status.success() {
                Ok(())
            }
            else {
                Err("Error while launching the command.".to_owned())
            };
        }

        // Standard path: use a PTY (pseudo-terminal) so the child process sees a real
        // terminal and keeps stdout line-buffered rather than block-buffering it on a pipe.
        // This enables real-time output streaming (e.g. emulator output visible immediately).
        //
        // Note: the OS-level PTY (ConPTY on Windows, posix_openpt on Linux/macOS) merges
        // the child's stdout and stderr into a single stream through the pseudo-console.
        // There is no portable way to separate them when using a PTY, so all child output
        // is forwarded to emit_stdout.
        use portable_pty::{CommandBuilder, PtySize, native_pty_system};

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0
            })
            .map_err(|e| format!("Failed to create PTY: {e}"))?;

        let slave = pair.slave;
        let master = pair.master;

        let mut cmd_builder = CommandBuilder::new(app);
        cmd_builder.cwd(&in_dir);
        for arg in &itr[1..] {
            cmd_builder.arg(arg);
        }

        let mut child = slave
            .spawn_command(cmd_builder)
            .map_err(|e| format!("Error while launching {}. {e}", app))?;

        let child_pid_opt = child.process_id();
        if let Some(pid) = child_pid_opt {
            register_child_pid(pid);
        }

        let mut pty_reader = master
            .try_clone_reader()
            .map_err(|e| format!("Failed to get PTY reader: {e}"))?;

        // The scope body (main thread) waits for the child then drops the slave,
        // which signals EOF to the PTY master reader running in the spawned thread.
        let mut pty_exit = None;
        thread::scope(|s| {
            // PTY master → emit_stdout (merges both stdout and stderr from child)
            s.spawn(|| {
                let mut current_line = String::new();
                let mut buf = [0u8; 4096];
                loop {
                    match pty_reader.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let text = String::from_utf8_lossy(&buf[..n]);
                            for c in text.chars() {
                                current_line.push(c);
                                if c == '\n' {
                                    o.emit_stdout(&current_line);
                                    current_line.clear();
                                }
                            }
                        }
                    }
                }
                if !current_line.is_empty() {
                    o.emit_stdout(&current_line);
                }
            });
            // Wait for child, then close both slave and master.
            //
            // On Windows (ConPTY) the output pipe is only closed once the
            // pseudoconsole is destroyed, which happens when `master` is
            // dropped (calls CloseConsolePseudoConsole).  Dropping `slave`
            // alone is not enough — the reader thread blocks forever.
            // Dropping `master` here (inside the scope, before the implicit
            // join) forces the pty_reader to see EOF/error and exit.
            pty_exit = Some(child.wait());
            drop(slave);
            drop(master);
        });

        let status = pty_exit
            .unwrap()
            .map_err(|e| format!("Error while executing {}. {e}", app))?;

        if let Some(pid) = child_pid_opt {
            deregister_child_pid(pid);
        }

        if !status.success() {
            return Err("Error while launching the command.".to_owned());
        }

        Ok(())
    }

    fn get_command(&self) -> &str {
        "external"
    }
}
