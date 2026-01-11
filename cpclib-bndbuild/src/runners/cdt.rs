use std::fs::File;
use std::marker::PhantomData;

use clap::{Arg, ArgAction, FromArgMatches};
use cpclib_common::event::EventObserver;
use cpclib_common::clap::{Parser, Command, CommandFactory};
use cpclib_runner::runner::{Runner, RunnerWithClap};
use rtzx::ui::commands::{Commands, run_convert, run_inspect, run_play};

use crate::task::RTZX_CMDS;
use rtzx::{self, TzxData};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CdtManager {
    Rtzx
}

impl CdtManager {
    pub fn get_command(&self) -> &str {
        match self {
            CdtManager::Rtzx => &RTZX_CMDS[0]
        }
    }
}

#[derive(Clone, Debug)]
pub struct RtzxRunner<E: EventObserver> {
    command: Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for RtzxRunner<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: EventObserver> RtzxRunner<E> {
    pub fn new() -> Self {
        Self {
            command: Cli::command()
             .no_binary_name(true)
            .disable_help_flag(true)
            .disable_version_flag(true)
            .arg(
                Arg::new("help")
                    .long("help")
                    .short('h')
                    .action(ArgAction::SetTrue)
                    .exclusive(true) // does not seem to work
            )
            .arg(
                Arg::new("version")
                    .long("version")
                    .short('V')
                    .help("Print version")
                    .action(ArgAction::SetTrue)
                    .exclusive(true) // does not s
            )
            ,
            _phantom: PhantomData::<E>
        }
    }
}


// XXX This logic should be in rtzx crate, but we have to duplicate it as it is in the main file
#[derive(Parser)]
#[command(version, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}


impl<E: EventObserver>  RunnerWithClap for RtzxRunner<E> {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}


impl<E: EventObserver> Runner for RtzxRunner<E> {
    type EventObserver = E;

    fn get_command(&self) -> &str {
        RTZX_CMDS[0]
    }

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let args: Vec<String> = itr.iter().map(|s| s.as_ref().to_string()).collect();
        let cli = self.get_matches(itr, o)?;
        if cli.is_none() {
            return Ok(());
        }
        let cli = cli.unwrap();
        let cli = Cli::from_arg_matches(&cli).unwrap();

        // XXX most of this logic should be in the rtzx crate
        let file_name = &cli.command.as_ref().and_then(|cmd| cmd.file_name()).expect("Filename not supplied");
        let display = file_name.display();

        // Open the path in read-only mode, returns `io::Result<File>`
        let file = match File::open(file_name) {
            Err(why) => return Err(format!("Couldn't open {}: {}", display, why)),
            Ok(file) => file,
        };

        let config = &cli.command.as_ref().and_then(|cmd| Some(cmd.config())).unwrap();

        let tzx_data = TzxData::parse_from(file);

        match &cli.command {
            Some(Commands::Inspect(args)) => run_inspect(file_name, &config, args.waveforms, &tzx_data),
            Some(Commands::Convert(args)) => run_convert(&args, &config, &tzx_data),
            Some(Commands::Play(_)) => run_play(file_name, &config, &tzx_data),
            None => Ok(()),
        }.map_err(|e| e.to_string())

    }
}