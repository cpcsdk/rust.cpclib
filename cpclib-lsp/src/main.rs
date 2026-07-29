use cpclib_common::clap;
use cpclib_common::clap::Parser;
use cpclib_lsp::CpcLspBackend;
use cpclib_lsp::config::{CONFIG_FILE_NAME, EXAMPLE_CONFIG_TOML, merge_missing_config_fields};
use tower_lsp::{LspService, Server};

#[derive(Parser, Debug)]
#[command(name = "cpclib-lsp")]
struct Cli {
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

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
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
