use std::marker::PhantomData;
use std::path::Path;

use clap::{Arg, ArgAction, Command, CommandFactory};
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapMatches;
use cpclib_runner::runner::{Runner, RunnerWithClap};

use crate::task::ASMFMT_CMDS;

pub struct AsmFmtRunner<E: EventObserver> {
    command: Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for AsmFmtRunner<E> {
    fn default() -> Self {
        let command = <cpclib_asmfmt::cli::Cli as CommandFactory>::command()
            .name(ASMFMT_CMDS[0])
            .no_binary_name(true)
            .disable_help_flag(true)
            .disable_version_flag(true)
            .arg(
                Arg::new("help")
                    .long("help")
                    .short('h')
                    .action(ArgAction::SetTrue)
                    .exclusive(true)
            )
            .arg(
                Arg::new("version")
                    .long("version")
                    .short('V')
                    .help("Print version")
                    .action(ArgAction::SetTrue)
                    .exclusive(true)
            );
        Self {
            command,
            _phantom: PhantomData
        }
    }
}

impl<E: EventObserver + 'static> RunnerWithClap for AsmFmtRunner<E> {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl<E: EventObserver + 'static> RunnerWithClapMatches for AsmFmtRunner<E> {}

impl<E: EventObserver + 'static> Runner for AsmFmtRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let itr: Vec<&str> = itr.iter().map(|s| s.as_ref()).collect();
        let matches = self.get_matches(&itr, o)?;
        if matches.is_none() {
            return Ok(());
        }
        let matches = matches.unwrap();

        if matches.get_flag("version") {
            o.emit_stdout(&format!(
                "basm-fmt {}\n",
                env!("CARGO_PKG_VERSION")
            ));
            return Ok(());
        }

        use std::path::PathBuf;
        let base = match cpclib_asmfmt::find_config_file() {
            None => cpclib_asmfmt::AsmFormatOptions::default(),
            Some(path) => match cpclib_asmfmt::load_config_from(&path) {
                Ok(cfg) => cfg,
                Err(e) => {
                    o.emit_stdout(&format!("warning: {}: {e}\n", path.display()));
                    cpclib_asmfmt::AsmFormatOptions::default()
                }
            }
        };
        let options = cpclib_asmfmt::cli::apply_cli_overrides(base, &matches);
        let inplace = matches.get_flag("inplace");
        let check = matches.get_flag("check");
        let files: Vec<PathBuf> = matches
            .get_many::<PathBuf>("files")
            .unwrap_or_default()
            .cloned()
            .collect();

        let mut all_changed = false;

        for path_buf in &files {
            let path: &Path = path_buf.as_path();
            let source = fs_err::read_to_string(path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            let formatted = cpclib_asmfmt::format(&source, &options)
                .map_err(|e| format!("{}: {e}", path.display()))?;

            if check {
                if formatted != source {
                    o.emit_stdout(&format!("{}: would be reformatted\n", path.display()));
                    all_changed = true;
                }
            } else if inplace {
                if formatted != source {
                    fs_err::write(path, &formatted)
                        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
                    o.emit_stdout(&format!("{}: reformatted\n", path.display()));
                }
            } else {
                o.emit_stdout(&formatted);
            }
        }

        if check && all_changed {
            Err("some files would be reformatted".to_owned())
        } else {
            Ok(())
        }
    }

    fn get_command(&self) -> &str {
        ASMFMT_CMDS[0]
    }
}
