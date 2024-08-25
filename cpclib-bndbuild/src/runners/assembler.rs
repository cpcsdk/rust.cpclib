use clap::{Arg, ArgAction, Command, CommandFactory, Parser};
use cpclib_asm::orgams::convert_source;
use cpclib_common::camino::Utf8PathBuf;
use cpclib_common::itertools::Itertools;
use cpclib_runner::emucontrol::{handle_arguments, EmuCli};
use cpclib_runner::runner::assembler::ExternAssembler;

use super::{Runner, RunnerWithClap};
use crate::built_info;
use crate::task::{BASM_CMDS, ORGAMS_CMDS};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Assembler {
    Basm,
    Orgams,
    Extern(ExternAssembler)
}

impl Assembler {
    pub fn get_command(&self) -> &str {
        match self {
            Assembler::Basm => BASM_CMDS[0],
            Assembler::Orgams => ORGAMS_CMDS[0],
            Assembler::Extern(a) => a.get_command()
        }
    }
}

#[derive(Parser, Debug)]
struct Orgams {
    #[arg(
        short,
        long,
        value_name = "DATA_SOURCE",
        help = "Data source (a folder for using albireo or a disc image)"
    )]
    from: Utf8PathBuf,

    #[command(flatten)]
    orgams: cpclib_runner::emucontrol::OrgamsCli
}

pub struct OrgamsRunner {
    command: clap::Command
}

impl Default for OrgamsRunner {
    fn default() -> Self {
        let command = <Orgams as CommandFactory>::command();
        Self { command }
    }
}

impl RunnerWithClap for OrgamsRunner {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl Runner for OrgamsRunner {
    fn inner_run<S: AsRef<str>>(&self, itr: &[S]) -> Result<(), String> {
        let mut itr = itr.iter().map(|s| s.as_ref()).collect_vec();
        itr.insert(0, "orgams");
        let matches = self.get_matches(&itr)?;

        let from = matches.get_one::<Utf8PathBuf>("from").unwrap();

        if matches.get_flag("basm2orgams") {
            if from.is_dir() {
                let src = matches.get_one::<String>("src").unwrap();
                let tgt = matches.get_one::<String>("dst").unwrap();
                convert_source(
                    from.join(src), 
                    from.join(tgt)
                ).map_err(|e| e.to_string())

            } else {
                unimplemented!()
            }
        } else {

            let mut real_arguments = Vec::new();
            real_arguments.push("orgams");
            if from.is_dir() {
                real_arguments.push("--albireo");
            }
            else {
                real_arguments.push("--drivea");
            }
            real_arguments.push(from.as_str());

            real_arguments.push("--emulator");
            real_arguments.push("ace");

            real_arguments.push("orgams");

            real_arguments.push("--src");
            real_arguments.push(matches.get_one::<String>("src").unwrap());

            if let Some(dst) = matches.get_one::<String>("dst") {
                real_arguments.push("--dst");
                real_arguments.push(dst);
            }

            let cli = EmuCli::parse_from(real_arguments);
            handle_arguments(cli)
        }
    }

    fn get_command(&self) -> &str {
        "orgams"
    }
}

pub struct BasmRunner {
    command: clap::Command
}

impl Default for BasmRunner {
    fn default() -> Self {
        let command = cpclib_basm::build_args_parser();
        // let mut command = command.group(
        // ArgGroup::new("ANY_INPUT")
        // .args(&["INLINE", "INPUT", "LIST_EMBEDDED", "VIEW_EMBEDDED"])
        // .required(true)
        // .conflicts_with("version")
        // );
        let command = command
            .no_binary_name(true)
            .disable_help_flag(true)
            .disable_version_flag(true)
            .arg(
                Arg::new("version")
                    .long("version")
                    .short('V')
                    .help("Print version")
                    .action(ArgAction::SetTrue)
                    .exclusive(true) // does not seem to work
                    .conflicts_with_all([
                        "ANY_INPUT",
                        "INLINE",
                        "INPUT",
                        "LIST_EMBEDDED",
                        "VIEW_EMBEDDED"
                    ])
            )
            .after_help(format!(
                "{} {} embedded by {} {}",
                cpclib_basm::built_info::PKG_NAME,
                cpclib_basm::built_info::PKG_VERSION,
                built_info::PKG_NAME,
                built_info::PKG_VERSION
            ));
        Self { command }
    }
}

impl RunnerWithClap for BasmRunner {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl Runner for BasmRunner {
    fn inner_run<S: AsRef<str>>(&self, itr: &[S]) -> Result<(), String> {
        let itr = itr.iter().map(|s| s.as_ref()).collect_vec();
        let matches = self.get_matches(&itr)?;

        if matches.get_flag("version") {
            println!("{}", self.get_clap_command().clone().render_version());
            return Ok(());
        }

        let start = std::time::Instant::now();

        match cpclib_basm::process(&matches) {
            Ok((env, warnings)) => {
                for warning in warnings {
                    eprintln!("{warning}");
                }

                let report = env.report(&start);
                println!("{report}");

                Ok(())
            },
            Err(e) => Err(format!("Error while assembling.\n{e}"))
        }
    }

    fn get_command(&self) -> &str {
        BASM_CMDS[0]
    }
}
