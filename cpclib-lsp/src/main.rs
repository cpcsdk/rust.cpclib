use cpclib_common::clap;
use cpclib_common::clap::{Parser, Subcommand};
use cpclib_lsp::CpcLspBackend;
use cpclib_lsp::config::{CONFIG_FILE_NAME, EXAMPLE_CONFIG_TOML, merge_missing_config_fields};
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
    let result = app.command().and_then(|command| command.execute());
    match result {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("Failure\n{e}");
            std::process::exit(1);
        }
    }
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
        if let Err(e) = std::fs::write(&path, EXAMPLE_CONFIG_TOML) {
            eprintln!("failed to write {}: {e}", path.display());
            std::process::exit(1);
        }
        println!("Wrote {}", path.display());
        return;
    }
    if let Some(dir) = cli.update_config {
        let path = dir.join(CONFIG_FILE_NAME);
        let existing = match std::fs::read_to_string(&path) {
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
        if let Err(e) = std::fs::write(&path, merged) {
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

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting cpclib-lsp server");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(CpcLspBackend::new);

    Server::new(stdin, stdout, socket).serve(service).await;
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
