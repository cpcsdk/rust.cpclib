use cpclib_common::clap;
use cpclib_common::clap::{Parser, Subcommand};
use cpclib_lsp::CpcLspBackend;
use cpclib_lsp::config::{
    CONFIG_FILE_NAME, EXAMPLE_CONFIG_TOML, find_config_file, load_config,
    merge_missing_config_fields
};
use tower_lsp::{LspService, Server};

#[derive(Parser, Debug)]
#[command(name = "cpclib-lsp")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Write a default cpclib-lsp.toml into DIR (current directory if
    /// omitted) and exit, without starting the language server. Refuses to
    /// overwrite an existing file.
    #[arg(long, value_name = "DIR", num_args = 0..=1, default_missing_value = ".")]
    init_config: Option<std::path::PathBuf>,

    /// Add any config field missing from the cpclib-lsp.toml in DIR (current
    /// directory if omitted) - already-present fields (values and comments)
    /// are left untouched. Exits without starting the language server.
    /// Fails if no config file exists yet - use --init-config for that.
    #[arg(long, value_name = "DIR", num_args = 0..=1, default_missing_value = ".")]
    update_config: Option<std::path::PathBuf>,

    /// Accepted for compatibility with LSP clients that always pass it
    /// (`vscode-languageclient` unconditionally appends `--stdio` whenever
    /// it spawns a server with `TransportKind.stdio` - confirmed directly in
    /// its own source, `node_modules/vscode-languageclient/lib/node/main.js`).
    /// A no-op: stdio is the only transport this server ever speaks.
    #[arg(long)]
    stdio: bool
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run as bndbuild instead of starting the language server, so a single
    /// installed cpclib-lsp binary can serve both roles - editor
    /// integrations (VS Code Tasks, the "▶ Run" CodeLens, etc.) that need
    /// to actually invoke a build no longer require a *second* bndbuild
    /// binary on PATH, even though cpclib-lsp already links
    /// cpclib-bndbuild in full for its own cpclib.runRule/cpclib.runTask
    /// LSP commands - this just exposes that same, already-linked code as
    /// a CLI entry point too. Every argument after `bndbuild` is passed
    /// straight through to bndbuild's own CLI parser unchanged (e.g.
    /// `cpclib-lsp bndbuild -f build.bnd my-target`).
    Bndbuild {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>
    },
    /// Run as the debug adapter, speaking DAP on stdin/stdout.
    ///
    /// Same reasoning as `bndbuild` above: the editor already knows where this
    /// binary is, so exposing the adapter as a subcommand saves shipping and
    /// locating a second one. Stdout carries protocol frames only.
    Dap,
    /// Run as `cpclib-runner`'s own `emu` CLI (launch a `.sna`/`.dsk` in any
    /// installed or installable emulator), instead of starting the language
    /// server. Same reasoning as `bndbuild`/`dap` above - cpclib-lsp already
    /// links cpclib-runner in full, so this saves shipping a third binary
    /// just for an editor's "run/debug with a specific emulator" picker to
    /// invoke. Every argument after `emu` is passed straight through to
    /// `cpclib_runner::emucontrol::EmuCli`'s own parser unchanged (e.g.
    /// `cpclib-lsp emu --emulator winape --snapshot game.sna run`).
    Emu {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>
    },
    /// Print every emulator `cpclib-runner` knows how to run, as a JSON
    /// array, for an editor's "run/debug with..." picker to render without
    /// needing to link `cpclib-runner` itself - each entry names the exact
    /// string `emu --emulator` accepts, a display label, whether the DAP
    /// layer (this same binary's own `dap` subcommand) can debug it, and
    /// whether it is already installed.
    EmuList
}

/// A real, isolated clap subcommand (rather than a hand-rolled pre-`Cli::parse()`
/// raw-argv check) so `--help` actually lists `bndbuild` as a subcommand and
/// its own flags never risk colliding with `Cli`'s top-level ones
/// (`--init-config`/`--update-config`/`--stdio`) - clap subcommands get
/// their own isolated argument namespace by construction.
///
/// `args` is everything after `bndbuild` on the command line, passed
/// straight through unchanged to bndbuild's own argument parser
/// (`cpclib_bndbuild::build_args_parser`). A synthetic program-name slot
/// (`"bndbuild"`) is prepended since clap's parser always expects one at
/// position 0 (matching real `bndbuild`'s own `main.rs`, which relies on
/// `try_get_matches()` defaulting to `env::args_os()` - here that first
/// slot is synthesized instead of real).
fn run_as_bndbuild(args: Vec<String>) -> ! {
    use cpclib_bndbuild::app::BndBuilderApp;
    use cpclib_bndbuild::event::BndBuilderObserverRc;
    use cpclib_common::clap::error::ErrorKind;

    let cmd = cpclib_bndbuild::build_args_parser().color(cpclib_common::clap::ColorChoice::Always);
    let matches =
        match cmd.try_get_matches_from(std::iter::once("bndbuild".to_string()).chain(args)) {
            Ok(m) => m,
            Err(e) => {
                e.print().ok();
                let code = match e.kind() {
                    ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => 0,
                    _ => 2
                };
                std::process::exit(code);
            }
        };

    let mut app = BndBuilderApp::from_matches(matches);
    app.add_observer(BndBuilderObserverRc::new_default());
    if app.wants_progress() {
        install_cli_progress_reporting(&mut app);
    }
    let result = app.command().and_then(|command| command.execute());
    match result {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("Failure\n{e}");
            std::process::exit(1);
        }
    }
}

/// A single, unlikely-to-appear-in-real-output marker: real build/error text
/// (basm's own messages, a task's own stdout/stderr) is never expected to
/// contain a NUL byte, so a caller scanning line by line can tell a progress
/// line apart from everything else with a plain prefix check, no ambiguity.
const PROGRESS_LINE_MARKER: &str = "\u{0}CPCLIB_PROGRESS\u{0}";

/// Wires `--progress` into this one CLI invocation: every rule-level and
/// basm-internal (parse/load/pass/save) progress event becomes a single
/// JSON line on stderr, `{PROGRESS_LINE_MARKER}{"message":...,"percentage":...}`,
/// meant for a caller that *parses* progress (the VS Code extension's own
/// terminal-task wrapper around the "▶ Run" CodeLens on a real `.bnd` file,
/// which runs this exact `bndbuild` subcommand as a real subprocess and has
/// no other channel to learn how far along a build is). basm's own
/// human-facing indicatif terminal bars remain a separate, untouched
/// concern (`basm ... --progress` on a task's own command line still works
/// exactly as before) - this is additional, not a replacement.
///
/// Reuses the exact same [`cpclib_lsp::bndbuild::command::ProgressState`]/
/// [`cpclib_lsp::bndbuild::command::ProgressUpdate`] types the LSP server's
/// own `$/progress` reporting is built on (`cpclib-lsp/src/server/backend.rs`),
/// so "what a build phase is worth in percent" is computed identically
/// everywhere, not reimplemented a second time with its own quirks.
///
/// Broadcasts the sink onto this process's own global rayon pool - see
/// `cpclib_asm::progress::AsmProgressSink`'s own doc for why a plain
/// thread-local install alone would silently miss basm's `INCLUDE`-parsing
/// work, which runs on rayon worker threads. Broadcasting onto the *global*
/// pool (rather than a dedicated one, the way the long-lived LSP server
/// needs to) is safe specifically because this is a short-lived,
/// single-purpose CLI process: there is no second, concurrent build sharing
/// it to contaminate.
fn install_cli_progress_reporting(app: &mut cpclib_bndbuild::app::BndBuilderApp) {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use cpclib_bndbuild::event::{BndBuilderEvent, BndBuilderObserver, BndBuilderObserverRc};
    use cpclib_common::event::EventObserver;
    use cpclib_lsp::bndbuild::command::{ProgressState, ProgressUpdate};

    fn print_progress_line(message: &str, percentage: Option<u32>) {
        let payload = serde_json::json!({ "message": message, "percentage": percentage });
        eprintln!("{PROGRESS_LINE_MARKER}{payload}");
    }

    // basm-internal events (parse/load/pass/save) can fire per-token, up to
    // millions of times on a real project - printing one unconditionally on
    // every single one, each an `eprintln!` (a write syscall) plus a
    // downstream JSON.parse + UI update on the VS Code side, is what actually
    // slowed builds down, not the (measured, ~365ns) cost of `apply()`
    // itself. Mirrors `backend.rs`'s `start_build_progress` `$/progress`
    // throttle exactly, for the same reason: a rule/task transition is rare
    // and meaningful, so it's always sent immediately; the flood of
    // basm-internal phase events is capped to one line per window.
    const PROGRESS_THROTTLE: Duration = Duration::from_millis(200);

    #[derive(Debug)]
    struct ThrottledState {
        state: ProgressState,
        last_sent: Instant
    }

    let shared = Arc::new(Mutex::new(ThrottledState {
        state: ProgressState::new(),
        last_sent: Instant::now() - PROGRESS_THROTTLE
    }));

    struct CliProgressSink(Arc<Mutex<ThrottledState>>);
    impl cpclib_asm::progress::AsmProgressSink for CliProgressSink {
        fn on_progress(&self, event: cpclib_asm::progress::AsmProgressEvent) {
            let mut guard = self.0.lock().unwrap();
            let (message, percentage) = guard.state.apply(ProgressUpdate::Asm(event));
            let now = Instant::now();
            if now.duration_since(guard.last_sent) < PROGRESS_THROTTLE {
                return;
            }
            guard.last_sent = now;
            drop(guard);
            print_progress_line(&message, percentage);
        }
    }

    let sink: Arc<dyn cpclib_asm::progress::AsmProgressSink> =
        Arc::new(CliProgressSink(Arc::clone(&shared)));
    cpclib_asm::progress::install_progress_sink(Some(Arc::clone(&sink)));
    cpclib_common::rayon::broadcast(|_| {
        cpclib_asm::progress::install_progress_sink(Some(Arc::clone(&sink)));
    });

    #[derive(Debug)]
    struct CliRuleProgressObserver(Arc<Mutex<ThrottledState>>);
    impl EventObserver for CliRuleProgressObserver {
        fn emit_stdout(&self, _s: &str) {}

        fn emit_stderr(&self, _s: &str) {}
    }
    impl BndBuilderObserver for CliRuleProgressObserver {
        fn update(&self, event: BndBuilderEvent) {
            let update = match event {
                BndBuilderEvent::StartRule { rule, nb, out_of } => Some(ProgressUpdate::Rule {
                    rule: rule.to_string(),
                    nb,
                    out_of
                }),
                BndBuilderEvent::StartTask(_rule, task) => Some(ProgressUpdate::Task {
                    command: task.to_string()
                }),
                _ => None
            };
            if let Some(update) = update {
                let mut guard = self.0.lock().unwrap();
                let (message, percentage) = guard.state.apply(update);
                guard.last_sent = Instant::now();
                drop(guard);
                print_progress_line(&message, percentage);
            }
        }
    }
    app.add_observer(BndBuilderObserverRc::new(CliRuleProgressObserver(shared)));
}

/// A real, isolated clap subcommand for the same reason `run_as_bndbuild`'s
/// own doc comment gives - `args` is everything after `emu`, passed straight
/// through to `EmuCli`'s own parser, with a synthetic program-name slot
/// prepended the same way.
fn run_as_emu(args: Vec<String>) -> ! {
    use cpclib_runner::emucontrol::{EmuCli, handle_arguments};

    let cli = match EmuCli::try_parse_from(std::iter::once("emu".to_string()).chain(args)) {
        Ok(cli) => cli,
        Err(e) => {
            e.print().ok();
            let code = match e.kind() {
                cpclib_common::clap::error::ErrorKind::DisplayHelp
                | cpclib_common::clap::error::ErrorKind::DisplayVersion => 0,
                _ => 2
            };
            std::process::exit(code);
        }
    };

    match handle_arguments(cli, &()) {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("Failure\n{e}");
            std::process::exit(1);
        }
    }
}

fn run_as_emu_list() -> ! {
    let entries = cpclib_runner::emucontrol::list_emulators();
    let json = serde_json::json!(
        entries
            .iter()
            .map(|e| serde_json::json!({
                "id": e.id,
                "label": e.label,
                "debuggable": e.debuggable,
                "installed": e.installed,
                "dapId": e.dap_id
            }))
            .collect::<Vec<_>>()
    );
    println!("{json}");
    std::process::exit(0);
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Some(Command::Bndbuild { args }) = cli.command {
        run_as_bndbuild(args);
    }
    if let Some(Command::Emu { args }) = cli.command {
        run_as_emu(args);
    }
    if let Some(Command::EmuList) = cli.command {
        run_as_emu_list();
    }
    if let Some(Command::Dap) = cli.command {
        // Diagnostics to stderr: stdout is the protocol.
        if let Err(problem) = cpclib_dap::run_stdio() {
            eprintln!("cpclib-lsp dap: {problem}");
            std::process::exit(1);
        }
        std::process::exit(0);
    }
    if let Some(dir) = cli.init_config {
        let path = dir.join(CONFIG_FILE_NAME);
        if path.exists() {
            eprintln!(
                "{} already exists - remove it first to regenerate",
                path.display()
            );
            std::process::exit(1);
        }
        if let Err(e) = fs_err::write(&path, EXAMPLE_CONFIG_TOML) {
            eprintln!("failed to write {}: {e}", path.display());
            std::process::exit(1);
        }
        println!("Wrote {}", path.display());
        return;
    }
    if let Some(dir) = cli.update_config {
        let path = dir.join(CONFIG_FILE_NAME);
        let existing = match fs_err::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "cannot read {}: {e} - use --init-config to create one",
                    path.display()
                );
                std::process::exit(1);
            }
        };
        let (merged, added) = match merge_missing_config_fields(&existing) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("cannot parse {}: {e}", path.display());
                std::process::exit(1);
            }
        };
        if added.is_empty() {
            println!("{} is already up to date, no fields added", path.display());
            return;
        }
        if let Err(e) = fs_err::write(&path, merged) {
            eprintln!("failed to write {}: {e}", path.display());
            std::process::exit(1);
        }
        println!(
            "Added {} field(s) to {}: {}",
            added.len(),
            path.display(),
            added.join(", ")
        );
        return;
    }

    // Initialize tracing. `cpclib-lsp.toml`'s top-level `log` field (mirroring
    // `[dap] log`, see `LspConfig::log`'s doc comment) can point tracing at a
    // file instead of relying on `RUST_LOG` and a stderr that a GUI-launched
    // editor has nowhere to show. This runs before `initialize` is received,
    // so the workspace root isn't known from the LSP handshake yet - the
    // current directory is used instead, which is why `cpclib-vscode` sets it
    // to the workspace folder when it spawns this process.
    let workspace_root = std::env::current_dir().ok();
    let found_config = find_config_file(workspace_root.as_deref());
    let log_setting = load_config(workspace_root.as_deref()).config.log;

    let log_path = (!log_setting.trim().is_empty()).then(|| {
        let configured = std::path::Path::new(log_setting.trim());
        if configured.is_absolute() {
            configured.to_path_buf()
        }
        else {
            found_config
                .as_deref()
                .and_then(std::path::Path::parent)
                .map(|dir| dir.join(configured))
                .unwrap_or_else(|| {
                    workspace_root.clone().unwrap_or_default().join(configured)
                })
        }
    });

    match log_path {
        Some(path) => match std::fs::File::create(&path) {
            Ok(file) => {
                tracing_subscriber::fmt()
                    .with_env_filter(
                        tracing_subscriber::EnvFilter::try_from_default_env()
                            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug"))
                    )
                    .with_writer(std::sync::Mutex::new(file))
                    .with_ansi(false)
                    .init();
                tracing::info!("cpclib-lsp: writing trace log to {}", path.display());
            },
            Err(e) => {
                tracing_subscriber::fmt()
                    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                    .with_writer(std::io::stderr)
                    .init();
                tracing::warn!("cpclib-lsp: cannot write log file {}: {e}", path.display());
            }
        },
        None => {
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .with_writer(std::io::stderr)
                .init();
        }
    }

    tracing::info!("Starting cpclib-lsp server");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(CpcLspBackend::new);

    // `tower-lsp`'s own default is 4 - fine for one request at a time, but a
    // workspace-restore `did_open` burst (every previously-open tab, sent by
    // the editor within milliseconds of each other) means dozens of
    // notifications competing for only 4 concurrently-processed slots at
    // once, even though each individual handler now finishes in
    // microseconds-to-low-milliseconds (`did_open`'s own real work already
    // runs on `spawn_blocking`, off this limit entirely - see
    // `spawn_deferred_analysis`). Raised generously rather than tied to core
    // count: these are cheap async tasks queuing for a slot, not CPU-bound
    // work needing its own core.
    const LSP_CONCURRENCY_LEVEL: usize = 32;
    Server::new(stdin, stdout, socket)
        .concurrency_level(LSP_CONCURRENCY_LEVEL)
        .serve(service)
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression test: `vscode-languageclient` unconditionally appends
    /// `--stdio` when spawning a server configured with `TransportKind.stdio`
    /// (confirmed directly in its own source - not an assumption) - a
    /// `Cli` that doesn't accept this flag makes the real VSCode extension
    /// fail to start the server at all (`clap` rejects the unknown argument
    /// and exits with code 2), which is exactly what happened when this
    /// flag was first added without it.
    #[test]
    fn accepts_the_stdio_flag_every_real_lsp_client_actually_passes() {
        let cli = Cli::try_parse_from(["cpclib-lsp", "--stdio"]);
        assert!(cli.is_ok(), "{cli:?}");
    }

    /// `bndbuild`'s own flags (`-f`, positional targets, `-D`, etc.) must
    /// pass straight through untouched, including ones that would otherwise
    /// look like `Cli`'s own top-level flags if this weren't a properly
    /// isolated subcommand.
    #[test]
    fn bndbuild_subcommand_captures_every_trailing_argument_unchanged() {
        let cli = Cli::try_parse_from([
            "cpclib-lsp",
            "bndbuild",
            "-f",
            "build.bnd",
            "my-target",
            "-D",
            "FOO=1"
        ]);
        assert!(cli.is_ok(), "{cli:?}");
        match cli.unwrap().command {
            Some(Command::Bndbuild { args }) => {
                assert_eq!(args, vec!["-f", "build.bnd", "my-target", "-D", "FOO=1"]);
            },
            other => panic!("expected the bndbuild subcommand, got {other:?}")
        }
    }

    #[test]
    fn the_dap_subcommand_is_recognised() {
        let cli = Cli::try_parse_from(["cpclib-lsp", "dap"]).expect("dap parses");
        assert!(matches!(cli.command, Some(Command::Dap)));
    }

    #[test]
    fn no_subcommand_means_the_language_server_path() {
        let cli = Cli::try_parse_from(["cpclib-lsp", "--stdio"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn still_works_with_no_arguments_at_all() {
        let cli = Cli::try_parse_from(["cpclib-lsp"]);
        assert!(cli.is_ok(), "{cli:?}");
    }

    #[test]
    fn accepts_update_config_with_an_explicit_dir() {
        let cli = Cli::try_parse_from(["cpclib-lsp", "--update-config", "/tmp/somewhere"]);
        assert!(cli.is_ok(), "{cli:?}");
        assert_eq!(
            cli.unwrap().update_config,
            Some(std::path::PathBuf::from("/tmp/somewhere"))
        );
    }

    #[test]
    fn accepts_update_config_with_no_dir_defaulting_to_cwd() {
        let cli = Cli::try_parse_from(["cpclib-lsp", "--update-config"]);
        assert!(cli.is_ok(), "{cli:?}");
        assert_eq!(
            cli.unwrap().update_config,
            Some(std::path::PathBuf::from("."))
        );
    }

    /// End-to-end (not just CLI-arg parsing): `--update-config` against a
    /// real partial config file on disk adds the missing fields, in place.
    #[test]
    fn update_config_adds_missing_fields_to_a_real_file_on_disk() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let path = tmp.path().join(CONFIG_FILE_NAME);
        std::fs::write(path.as_std_path(), "[asm]\ncase_sensitive = false\n").unwrap();

        let existing = std::fs::read_to_string(path.as_std_path()).unwrap();
        let (merged, added) = merge_missing_config_fields(&existing).unwrap();
        assert!(!added.is_empty(), "{added:?}");
        std::fs::write(path.as_std_path(), &merged).unwrap();

        let on_disk = std::fs::read_to_string(path.as_std_path()).unwrap();
        assert!(on_disk.contains("case_sensitive = false"), "{on_disk}");
        let loaded = cpclib_lsp::config::load_config(Some(tmp.path().as_std_path()));
        assert!(loaded.error.is_none(), "{:?}", loaded.error);
        assert!(!loaded.config.asm.case_sensitive);
        assert!(loaded.config.asm.firmware_docs);
    }
}
