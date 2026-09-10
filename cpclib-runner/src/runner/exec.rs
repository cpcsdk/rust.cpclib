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

/// Environment variables a snap-packaged parent process (confirmed: VS
/// Code installed via `snap`) sets to point *its own* GTK/GLib/locale
/// module loading at its bundled `/snap/<name>/<rev>/...` tree - these leak
/// into every child process spawned from an integrated terminal or Task,
/// and can make an unrelated spawned GUI app (Qt or GTK-based) load one of
/// the snap's own bundled shared libraries as a side effect of its own
/// module/theme discovery.
///
/// `GTK_PATH` alone was confirmed by bisection to reproduce exactly this
/// crash for SugarboxV2 (a Qt app): with `GTK_PATH` inherited from a
/// snap-packaged VS Code, SugarboxV2 ends up loading a GTK module from
/// `$GTK_PATH`, which pulls in the snap's own (glibc-version-incompatible)
/// `libpthread.so.0` as a side effect, corrupting dynamic symbol
/// resolution for the rest of the process -
/// `symbol lookup error: .../snap/core20/current/lib/x86_64-linux-gnu/
/// libpthread.so.0: undefined symbol: __libc_pthread_init, version
/// GLIBC_PRIVATE`, reproduced identically outside VS Code too as long as
/// the shell inherited the same snap-set variables. The other variables
/// here are the same mechanism's closest relatives (GTK/GDK/GIO module and
/// schema discovery, and locale data) - not individually confirmed to
/// reproduce the crash on their own, but removing them for a spawned
/// emulator process is safe (a properly installed native app needs none of
/// them) and closes the same class of bug before some other emulator hits
/// it independently.
///
/// `SNAP_LIBRARY_PATH` is a different mechanism but the same class of leak:
/// snapd's own `opengl` plugin interface sets it (to
/// `/var/lib/snapd/lib/gl[32]`) so a confined snap app can reach the host's
/// real GPU driver despite its sandbox - observed leaking into a
/// snap-packaged VS Code integrated terminal's environment alongside
/// `GTK_PATH`, in a session where SugarboxV2 (run through Wine) failed with
/// "neither GLX nor EGL are enabled" / "Failed to create QRhi" - a Wine/Qt
/// process outside the snap sandbox has no business consulting a variable
/// meant for snap-confinement GPU passthrough, and stripping it is exactly
/// as safe as the GTK ones above.
/// `pub` so callers that spawn a GUI emulator directly rather than through
/// `Runner`/`ExternRunner` (an external debugger integration elsewhere in
/// this workspace spawns SugarboxV2/AMSpiriT Lite with its own
/// `std::process::Command`) can strip the same variables - the leak and
/// its fix are about the *child process*, not about which of this crate's
/// APIs happened to spawn it, so both paths need the same list.
#[cfg(target_os = "linux")]
pub const SNAP_LEAKED_ENV_VARS: &[&str] = &[
    "GTK_PATH",
    "GTK_EXE_PREFIX",
    "GDK_PIXBUF_MODULE_FILE",
    "GDK_PIXBUF_MODULEDIR",
    "GIO_MODULE_DIR",
    "GSETTINGS_SCHEMA_DIR",
    "LOCPATH",
    "SNAP_LIBRARY_PATH"
];

#[derive(Default, Clone, Copy, Debug)]
pub enum RunInDir {
    #[default]
    CurrentDir,
    AppDir
}

/// What a task reads as "stdin" when it has `<file` or is a non-first stage
/// of a `|` pipeline - see `cpclib_bndbuild::shell_pipe`. `Reader` is always
/// the read end of a real `std::io::pipe()` (every pipeline stage boundary
/// uses one, deliberately, for implementation simplicity - one code path
/// rather than special-casing which side is embedded/delegated), which
/// converts directly into a `Stdio` for a delegated consumer, and is a
/// plain `Read` for an embedded one - no extra thread/copy needed either way.
///
/// `Empty` is passed for a stage that has no actual input to read (the first
/// stage of a pipeline/redirection with no `<file`) but is still running
/// inside a redirected/piped context: a delegated runner still needs to
/// force its non-PTY code path there, since a PTY would merge stdout and
/// stderr into one stream and defeat the whole point of `>`/`|` - but a
/// runner like `Echo`/`Rm` that only activates its own stdin-consuming
/// behaviour for *real* input must treat this the same as no stdin at all.
pub enum TaskStdin {
    File(cpclib_common::camino::Utf8PathBuf),
    Reader(std::io::PipeReader),
    Empty
}

/// Where a Delegated task's real subprocess stdout is wired directly, when
/// it is redirected to a file or piped into the next stage - see
/// `cpclib_bndbuild::shell_pipe`.
///
/// This deliberately bypasses `EventObserver::emit_stdout`/the whole
/// String-based observer machinery for this data: that path decodes the
/// child's raw output as UTF-8 text one character at a time, which silently
/// drops or corrupts arbitrary binary output (e.g. `gource ... -o - |
/// ffmpeg ...`, where gource's stdout is a raw PPM/pixel byte stream, not
/// text - piping it through the text machinery left only a fragment of the
/// first frame's ASCII header surviving). Wiring the child's stdout `Stdio`
/// directly to this destination is both byte-exact and avoids the
/// buffering/copying entirely - exactly what a real shell `|` does.
///
/// Only meaningful for a `Runner` that spawns a real OS process and can
/// hand it a raw `Stdio` (`ExternRunner`, and anything built on it such as
/// `DelegatedRunner`); an embedded task has no "real OS stdout" to wire this
/// way and keeps emitting text through `EventObserver::emit_stdout` as
/// before, redirected/piped by `RedirectedObserver` instead - correct there
/// since an embedded task's output is always text it constructed itself.
pub enum TaskStdout {
    File(cpclib_common::camino::Utf8PathBuf, bool /* append */),
    Writer(std::io::PipeWriter)
}

pub trait Runner {
    type EventObserver: EventObserver;

    /// Run the task and return true if successfull
    fn run(&self, arguments: &str, o: &Self::EventObserver) -> Result<(), String> {
        self.run_redirected(arguments, o, None, None)
    }

    /// Same as `run`, but with an optional stdin source - the redirection/
    /// piping entry point. Default implementation ignores `stdin` and
    /// behaves exactly like `run`; only meaningful for a `Runner` that
    /// overrides `inner_run_with_stdin` (or `inner_run_redirected`).
    fn run_with_stdin(
        &self,
        arguments: &str,
        o: &Self::EventObserver,
        stdin: Option<TaskStdin>
    ) -> Result<(), String> {
        self.run_redirected(arguments, o, stdin, None)
    }

    /// Same as `run_with_stdin`, but also with an optional raw stdout
    /// destination - see `TaskStdout`. Default implementation ignores
    /// `stdout` and behaves exactly like `run_with_stdin`.
    fn run_redirected(
        &self,
        arguments: &str,
        o: &Self::EventObserver,
        stdin: Option<TaskStdin>,
        stdout: Option<TaskStdout>
    ) -> Result<(), String> {
        let args = get_all_args(arguments)?;
        self.inner_run_redirected(&args, o, stdin, stdout)
    }

    /// Implement the command specific action
    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &Self::EventObserver) -> Result<(), String>;

    /// Default: ignore `stdin`, fall back to `inner_run` - every existing
    /// `Runner` gets this for free and needs no change at all. Override
    /// this (not `inner_run`) in the handful of runners that should
    /// actually consume piped/redirected input.
    fn inner_run_with_stdin<S: AsRef<str>>(
        &self,
        itr: &[S],
        o: &Self::EventObserver,
        _stdin: Option<TaskStdin>
    ) -> Result<(), String> {
        self.inner_run(itr, o)
    }

    /// Default: ignore `stdout`, fall back to `inner_run_with_stdin` - every
    /// existing `Runner` gets this for free. Override this (not the other
    /// two) in a runner that can wire its real subprocess stdout directly to
    /// `TaskStdout` (currently only `ExternRunner`/`DelegatedRunner`).
    fn inner_run_redirected<S: AsRef<str>>(
        &self,
        itr: &[S],
        o: &Self::EventObserver,
        stdin: Option<TaskStdin>,
        _stdout: Option<TaskStdout>
    ) -> Result<(), String> {
        self.inner_run_with_stdin(itr, o, stdin)
    }

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
    #[cfg(feature = "transparent-x11")]
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
            #[cfg(feature = "transparent-x11")]
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
        self.inner_run_with_stdin(itr, o, None)
    }

    fn inner_run_with_stdin<S: AsRef<str>>(
        &self,
        itr: &[S],
        o: &E,
        stdin: Option<TaskStdin>
    ) -> Result<(), String> {
        self.inner_run_redirected(itr, o, stdin, None)
    }

    fn inner_run_redirected<S: AsRef<str>>(
        &self,
        itr: &[S],
        o: &E,
        stdin: Option<TaskStdin>,
        stdout: Option<TaskStdout>
    ) -> Result<(), String> {
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

        // Any stdin (a `<file` redirection, or a non-first pipeline stage)
        // or stdout (a `>`/`>>` redirection, or a non-last pipeline stage)
        // forces the plain-Command path on every platform: the PTY path
        // below defaults to canonical/cooked mode and gives no clean way to
        // signal EOF on stdin just by closing the write side, and it also
        // merges stdout/stderr into one stream, defeating `>`/`|` outright.
        if stdin.is_some() || stdout.is_some() {
            use std::io::BufReader;
            use std::process::{Child, Stdio};

            use utf8_chars::BufReadCharsExt;

            let mut cmd = std::process::Command::new(app);
            cmd.current_dir(&in_dir);
            for arg in &itr[1..] {
                cmd.arg(arg);
            }
            #[cfg(target_os = "linux")]
            for var in SNAP_LEAKED_ENV_VARS {
                cmd.env_remove(var);
            }
            let stdin_stdio = match stdin {
                Some(TaskStdin::File(path)) => Stdio::from(
                    std::fs::File::open(path.as_std_path())
                        .map_err(|e| format!("Unable to open {path} for reading. {e}"))?
                ),
                Some(TaskStdin::Reader(reader)) => Stdio::from(reader),
                Some(TaskStdin::Empty) => Stdio::null(),
                // Matches shell semantics: a command with only its output
                // redirected still inherits the real stdin (e.g. `foo >
                // out.txt` in bash). Not actually reachable today - every
                // caller that supplies `stdout` also supplies `stdin` - but
                // this is the correct fallback if that ever changes.
                None => Stdio::inherit()
            };

            // A raw stdout destination is wired DIRECTLY as the child's
            // Stdio - byte-exact, no decoding, no copying - see `TaskStdout`
            // for why. `None` keeps today's behaviour: captured and
            // forwarded as text through `emit_stdout`.
            let capture_stdout = stdout.is_none();
            let stdout_stdio = match stdout {
                Some(TaskStdout::File(path, append)) => {
                    let file = if append {
                        std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(path.as_std_path())
                    }
                    else {
                        std::fs::File::create(path.as_std_path())
                    }
                    .map_err(|e| format!("Unable to open {path} for writing. {e}"))?;
                    Stdio::from(file)
                },
                Some(TaskStdout::Writer(writer)) => Stdio::from(writer),
                None => Stdio::piped()
            };

            let cmd = cmd
                .stdin(stdin_stdio)
                .stderr(Stdio::piped())
                .stdout(stdout_stdio);
            let mut child: Child = cmd
                .spawn()
                .map_err(|e| format!("Error while launching {}. {}", app, e))?;
            let child_pid = child.id();
            register_child_pid(child_pid);
            let child_stdout = capture_stdout.then(|| {
                child
                    .stdout
                    .take()
                    .expect("Internal error, could not take stdout")
            });
            let child_stderr = child
                .stderr
                .take()
                .expect("Internal error, could not take stderr");

            thread::scope(|s| {
                if let Some(child_stdout) = child_stdout {
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
                }
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
                Err(match status.code() {
                    Some(code) => format!("Error while launching the command. (exit code {code})"),
                    None => "Error while launching the command. (terminated by signal)".to_owned()
                })
            };
        }

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
            #[cfg(target_os = "linux")]
            for var in SNAP_LEAKED_ENV_VARS {
                cmd.env_remove(var);
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
                Err(match status.code() {
                    Some(code) => format!("Error while launching the command. (exit code {code})"),
                    None => "Error while launching the command. (terminated by signal)".to_owned()
                })
            };
        }

        // Standard path: use a PTY (pseudo-terminal) so the child process sees a real
        // terminal and keeps stdout line-buffered rather than block-buffering it on a pipe.
        // This enables real-time output streaming (e.g. emulator output visible immediately).
        //
        // Note: the OS-level PTY (ConPTY on Windows, posix_openpt on Linux/macOS) merges
        // the child's stdout and stderr into a single stream through the pseudo-console.
        // There is no portable way to separate them when using a PTY, so all child output

        #[cfg(target_os = "macos")]
        {
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
                .spawn()
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
                Err(match status.code() {
                    Some(code) => format!("Error while launching the command. (exit code {code})"),
                    None => "Error while launching the command. (terminated by signal)".to_owned()
                })
            };
        }
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
        #[cfg(target_os = "linux")]
        for var in SNAP_LEAKED_ENV_VARS {
            cmd_builder.env_remove(var);
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
            return Err(format!(
                "Error while launching the command. (exit code {})",
                status.exit_code()
            ));
        }

        Ok(())
    }

    fn get_command(&self) -> &str {
        "external"
    }
}
