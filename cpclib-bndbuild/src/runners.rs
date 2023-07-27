/// Manage the standard tasks
use cpclib_basm;
use cpclib_common::{
    clap::{self, Arg, ArgAction, Command, ArgGroup},
    itertools::Itertools,
};
use glob::glob;
use shlex::split;

use crate::built_info;

/// Get all args (split string as done in shell and apply glob matching)
fn get_all_args(arguments: &str) -> Vec<String> {
    let init_args = split(arguments).unwrap_or_default();
    let mut res = Vec::new();
    for p in init_args {
        match glob(&p) {
            Ok(entries) => {
                let mut added = 0;
                for entry in entries {
                    match entry {
                        Ok(p) => res.push(p.display().to_string()),
                        Err(e) => res.push(e.path().display().to_string()),
                    }
                    added += 1;
                }
                if added == 0 {
                    res.push(p);
                }
            }
            Err(_) => res.push(p),
        }
    }
    res
}

pub trait Runner {
    /// Run the task and return true if successfull
    fn run(&self, arguments: &str) -> Result<(), String> {
        println!("\t$ {} {}", self.get_command(), arguments);
        let args = get_all_args(&arguments.replace(r"\", r"\\"));
        self.inner_run(&args)
    }

    /// Implement the command specific action
    fn inner_run(&self, itr: &[String]) -> Result<(), String>;

    fn get_command(&self) -> &str;
}

pub trait RunnerWithClap: Runner {
    fn get_clap_command(&self) -> &Command;
    fn print_help(&self) {
        self.get_clap_command()
            .clone()
            .disable_help_flag(true)
            .print_long_help()
            .unwrap();
    }
}

#[derive(Default)]
pub struct ExternRunner {}
impl ExternRunner {

}
impl Runner for ExternRunner {
    fn inner_run(&self, itr: &[String]) -> Result<(), String> {
        let app = std::fs::canonicalize(&itr[0])
            .map_err(|e| format!("Wrong executable {}.{}", &itr[0], e.to_string()))?;

        let cwd = std::env::current_dir()
            .map_err(|e| format!("Unable to get the current working directory {}.", e.to_string()))?;
        let cwd = std::fs::canonicalize(cwd)
            .map_err(|e| format!("Unable to get the current working directory {}.", e.to_string()))?;


        let mut cmd = std::process::Command::new(app);
        cmd.current_dir(cwd);
        for arg in &itr[1..] {
            cmd.arg(dbg!(arg));
        }
        let mut handle = cmd.spawn()
            .map_err(|e| format!("Error while launching {}. {}", &itr[0], e.to_string()))?;

        let status = handle.wait()
            .map_err(|e| format!("Error while executing {}. {}", &itr[0], e.to_string()))?;

        if !status.success() {
            return Err("Error while launching the command.".to_owned())
        }
        Ok(())
    }

    fn get_command(&self) -> &str {
        "extern"
    }
}

#[derive(Default)]
pub struct RmRunner {}

impl RmRunner {
    pub fn print_help(&self) {
        clap::Command::new("rm")
            .before_help("Delete files.")
            .disable_help_flag(true)
            .after_help(format!(
                "Inner command of {} {}",
                built_info::PKG_NAME,
                built_info::PKG_VERSION
            ))
            .arg(
                Arg::new("arguments")
                    .action(ArgAction::Append)
                    .help("Files to delete."),
            )
            .print_long_help()
            .unwrap();
    }
}
impl Runner for RmRunner {
    fn inner_run(&self, itr: &[String]) -> Result<(), String> {
        for fname in itr.into_iter() {
            std::fs::remove_file(&fname)
                .map_err(|e| format!("Unable to remove {}:{}.", fname, e.to_string()))?;
        }
        Ok(())
    }

    fn get_command(&self) -> &str {
        "rm"
    }
}

pub struct XferRunner {
    command: clap::Command
}


impl Default for XferRunner {
    fn default() -> Self {
        let command = cpclib_xfertool::build_args_parser();
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
        )   
        .after_help(format!(
            "{} {} embedded by {} {}",
            cpclib_xfertool::built_info::PKG_NAME,
            cpclib_xfertool::built_info::PKG_VERSION,
            built_info::PKG_NAME,
            built_info::PKG_VERSION
        ));
    Self { command }
    }
}

impl RunnerWithClap for XferRunner {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}


impl Runner for XferRunner {
    fn inner_run(&self, itr: &[String]) -> Result<(), String> {
        let args = self.command.clone();

        let matches = self
            .command
            .clone()
            .try_get_matches_from(itr)
            .map_err(|e| e.to_string())?;


        if matches.get_flag("version") {
            println!("{}", self.get_clap_command().clone().render_version());
            return Ok(());
        }

        cpclib_xfertool::process(&matches).map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        "xfer"
    }
}


pub struct BasmRunner {
    command: clap::Command,
}

impl Default for BasmRunner {
    fn default() -> Self {
        let command = cpclib_basm::build_args_parser();
/* 
        let mut command = command.group(
            ArgGroup::new("ANY_INPUT")
            .args(&["INLINE", "INPUT", "LIST_EMBEDDED", "VIEW_EMBEDDED"])
            .required(true)
            .conflicts_with("version")
        );
*/
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
                    .conflicts_with_all(["ANY_INPUT", "INLINE", "INPUT", "LIST_EMBEDDED", "VIEW_EMBEDDED"])
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
        let matches = self
            .command
            .clone()
            .try_get_matches_from(itr)
            .map_err(|e| e.to_string())?;


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
            Err(e) => Err(format!("Error while assembling.\n{e}")),
        }
    }

    fn get_command(&self) -> &str {
        "basm"
    }
}

#[derive(Default)]
pub struct EchoRunner {}

impl Runner for EchoRunner {
    fn inner_run(&self, itr: &[String]) -> Result<(), String> {
        let txt = itr.iter().join(" ");
        println!("{txt}");
        Ok(())
    }

    fn get_command(&self) -> &str {
        "echo"
    }
}

pub struct ImgConverterRunner {
    command: Command,
}

impl Default for ImgConverterRunner {
    fn default() -> Self {
        let command = cpclib_imgconverter::build_args_parser()
            .after_help(format!(
                "{} {} embedded by {} {}",
                cpclib_imgconverter::built_info::PKG_NAME,
                cpclib_imgconverter::built_info::PKG_VERSION,
                built_info::PKG_NAME,
                built_info::PKG_VERSION
            ))
          //  .disable_help_flag(true)
            .disable_version_flag(true)
            .arg(
                Arg::new("version")
                    .long("version")
                    .short('V')
                    .help("Print version")
                    .action(ArgAction::SetTrue)
                    .exclusive(true),
            )
            .no_binary_name(true);
        Self { command }
    }
}

impl RunnerWithClap for ImgConverterRunner {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl Runner for ImgConverterRunner {
    fn inner_run(&self, itr: &[String]) -> Result<(), String> {
        let args = self.command.clone();

        let matches = self
            .command
            .clone()
            .try_get_matches_from(itr)
            .map_err(|e| dbg!(e.to_string()))?;


        if matches.get_flag("version") {
            println!("{}", self.get_clap_command().clone().render_version());
            return Ok(());
        }

        cpclib_imgconverter::process(&matches, args).map_err(|e| dbg!(e).to_string())
    }

    fn get_command(&self) -> &str {
        "img2cpc"
    }
}
