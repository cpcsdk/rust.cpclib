use cpclib_common::clap::{Command, ArgAction, Arg, self};

use crate::built_info;

use super::{RunnerWithClap, Runner};


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
    fn inner_run(&self, itr: &[String]) -> Result<(), String> {
        let matches = self.get_matches(itr)?;

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
            }
            Err(e) => Err(format!("Error while assembling.\n{e}"))
        }
    }

    fn get_command(&self) -> &str {
        "basm"
    }
}

