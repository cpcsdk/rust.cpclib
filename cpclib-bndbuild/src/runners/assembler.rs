use cpclib_common::camino::Utf8Path;
use cpclib_common::clap::{self, Arg, ArgAction, Command};
use cpclib_common::itertools::Itertools;

use super::r#extern::ExternRunner;
use super::{Runner, RunnerWithClap};
use crate::built_info;
use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};
use crate::task::{BASM_CMDS, RASM_CMDS};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Assembler{
    Basm,
    Rasm(RasmVersion)
}

impl Assembler {
    pub fn get_command(&self) -> &str {
        match self {
            Assembler::Basm => &BASM_CMDS[0],
            Assembler::Rasm(_) => &RASM_CMDS[0],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RasmVersion {
    Consolidation2024, //V2_2_5
}

impl Default for RasmVersion {
    fn default() -> Self {
        Self::Consolidation2024
    }
}



cfg_match! {
    cfg(target_os = "linux") =>
    {
        impl RasmVersion {
            pub fn configuration(&self) -> DelegateApplicationDescription {
                match self {
                    RasmVersion::Consolidation2024  =>
                        DelegateApplicationDescription {
                            download_url: "https://github.com/EdouardBERGE/rasm/archive/refs/tags/v2.2.5.zip", // we assume a modern CPU
                            folder : "rasm_consolidation",
                            archive_format: ArchiveFormat::Zip,
                            exec_fname: "rasm",
                            compile: Some(Box::new(|path: &Utf8Path| -> Result<(), String>{
                                let command = vec!["make"];
                                ExternRunner::default().inner_run(&command)?;

                                let command = vec!["mv", "rasm.exe", "rasm"];
                                ExternRunner::default().inner_run(&command)?;
                                
                                Ok(())
                            }))
                        }
                    }
            }
        }

    }
    cfg(target_os = "windows") =>
    {
        impl RasmVersion {
            pub fn configuration(&self) -> DelegateApplicationDescription {
                match self {
                    RasmVersion::Consolidation2024  =>
                        DelegateApplicationDescription {
                            download_url: "https://github.com/EdouardBERGE/rasm/releases/download/v2.2.5/rasm_win64.exe", // we assume a modern CPU
                            folder : "rasm_consolidation",
                            archive_format: ArchiveFormat::Raw,
                            exec_fname: "rasm.exe",
                            compile: None
                        }
                    }
            }
        }

    }
    cfg(target_os = "macos") =>
    {

    }
    _ => {
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
