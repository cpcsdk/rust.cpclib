#![feature(cfg_match)]
#![feature(os_str_display)]

use std::env::current_dir;

use app::BndBuilderApp;
use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::clap;
use cpclib_common::clap::*;
use cpclib_common::itertools::Itertools;
use cpclib_runner::emucontrol::EMUCTRL_CMD;
use cpclib_runner::runner::assembler::RASM_CMD;
use cpclib_runner::runner::emulator::{AMSPIRIT_CMD, CPCEC_CMD, SUGARBOX_V2_CMD, WINAPE_CMD};
use cpclib_runner::runner::impdisc::IMPDISC_CMD;
use cpclib_runner::runner::martine::MARTINE_CMD;
use lazy_regex::regex_captures;
use runners::hideur::HIDEUR_CMD;
use thiserror::Error;

use crate::executor::*;
pub use crate::BndBuilder;

pub mod app;
pub mod builder;
pub mod constraints;
pub mod event;
pub mod executor;
pub mod rules;
pub mod runners;
pub mod task;

pub use builder::*;
pub use cpclib_common;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub fn process_matches(matches: &ArgMatches) -> Result<(), BndBuilderError> {
    let cmd = BndBuilderApp::from_matches(matches.clone());
    cmd.command()?.execute()
}

pub fn build_args_parser() -> clap::Command {
    static COMMANDS_LIST: &[&str] = &[
        "basm",
        "bndbuild",
        "cp",
        "disc",
        "dsk",
        "echo",
        "extern",
        "img2cpc",
        "orgams",
        "rm",
        "xfer",
        AMSPIRIT_CMD,
        CPCEC_CMD,
        EMUCTRL_CMD,
        HIDEUR_CMD,
        IMPDISC_CMD,
        MARTINE_CMD,
        RASM_CMD,
        SUGARBOX_V2_CMD,
        WINAPE_CMD,
    ];

    Command::new("bndbuilder")
        .about("Benediction CPC demo project builder")
        .before_help("Can be used as a project builder similar to Make, but using a yaml project description, or can be used as any Benediction crossdev tool (basm, img2cpc, xfer, disc). This way only bndbuild needs to be installed.")
        .author("Krusty/Benediction")
        .version(built_info::PKG_VERSION)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("help")
                .long("help")
                .short('h')
                .value_name("CMD")
                .value_parser(COMMANDS_LIST.iter().collect_vec())
                .default_missing_value_os("bndbuild")
                .default_value("bndbuild")
                .num_args(0..=1)
                .help("Show the help of the given subcommand CMD.")
        )
        .arg(
            Arg::new("direct")
            .action(ArgAction::SetTrue)
            .long("direct")
            .help("Directly execute a command without trying to read a task file")
            .conflicts_with_all(["list", "init", "add"])
        )
        .arg(
            Arg::new("version")
                .long("version")
                .short('V')
                .help("Print version")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("dot")
                .long("dot")
                .alias("grapĥviz")
                .help("Generate the .dot representation of the selected bndbuild.yml file")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("show")
                .long("show")
                .help("Show the file AFTER interpreting the templates")
                .action(ArgAction::SetTrue)
                .conflicts_with("dot")
        )
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .action(ArgAction::Set)
                .value_name("FILE")
                .value_hint(ValueHint::FilePath)
                .help("Provide the YAML file for the given project.")
        )
        .arg(
            Arg::new("watch")
                .short('w')
                .long("watch")
                .action(ArgAction::SetTrue)
                .help("Watch the targets and permanently rebuild them when needed.")
                .conflicts_with_all(["dot", "show"])
        )
        .arg(
            Arg::new("list")
                .short('l')
                .long("list")
                .action(ArgAction::SetTrue)
                .help("List the available targets")
                .conflicts_with("dot")
        )
        .arg(
            Arg::new("DEFINE_SYMBOL")
                .help("Provide a symbol with its value (default set to 1)")
                .long("define")
                .short('D')
                .action(ArgAction::Append)
                .number_of_values(1)
        )
        .arg(
            Arg::new("clear")
                .long("clear-cache")
                .alias("clear")
                .short('c')
                .num_args(0..=1)
                .help("Clear cache folder that contains all automatically downloaded executables. Can optionally take one argument to clear the cache of the corresponding executable.")
                .exclusive(true)
        )
        .arg(
            Arg::new("init")
                .long("init")
                .action(ArgAction::SetTrue)
                .help("Init a new project by creating it")
                .conflicts_with("dot")
        )
        .arg(
            Arg::new("add")
                .long("add")
                .short('a')
                .help("Add a new basm target in an existing bndbuild.yml (or create it)")
                .conflicts_with("dot")
                .action(ArgAction::Set)
        )
        .arg(
            Arg::new("dep")
                .help("The source files")
                .long("dep")
                .short('d')
                .requires("add")
        )
        .arg(
            Arg::new("kind")
                .help("The kind of command to be added in the yaml file")
                .long("kind")
                .short('k')
                .value_parser(["basm", "img2cpc", "xfer"])
                .requires("add")
                .default_missing_value("basm")
        )

        .arg(
            Arg::new("target")
                .action(ArgAction::Append)
                .value_name("TARGET")
                .help("Provide the target(s) to run.")
                .conflicts_with_all(["list", "init", "add"])
        )
}

pub fn init_project(path: Option<&Utf8Path>) -> Result<(), BndBuilderError> {
    let path = path
        .map(|p| p.to_owned())
        .unwrap_or_else(|| Utf8PathBuf::from_path_buf(current_dir().unwrap()).unwrap());

    if !path.is_dir() {
        return Err(BndBuilderError::AnyError(format!(
            "{} is not a valid directory",
            path
        )));
    }

    let bndbuild_yml = path.join("bndbuild.yml");
    if bndbuild_yml.exists() {
        return Err(BndBuilderError::AnyError(format!(
            "{} already exists",
            bndbuild_yml
        )));
    }

    let main_asm = path.join("main.asm");
    if main_asm.exists() {
        return Err(BndBuilderError::AnyError(format!(
            "{} already exists",
            main_asm
        )));
    }

    let data_asm = path.join("data.asm");
    if main_asm.exists() {
        return Err(BndBuilderError::AnyError(format!(
            "{} already exists",
            data_asm
        )));
    }

    std::fs::write(&bndbuild_yml, include_bytes!("default_bndbuild.yml"))
        .map_err(|e| BndBuilderError::AnyError(e.to_string()))?;

    std::fs::write(&main_asm, include_bytes!("default_main.asm"))
        .map_err(|e| BndBuilderError::AnyError(e.to_string()))?;

    std::fs::write(&data_asm, include_bytes!("default_data.asm"))
        .map_err(|e| BndBuilderError::AnyError(e.to_string()))?;

    Ok(())
}

/// Expand glob patterns
/// {a,b} expension is always done even if file does not exists
/// *.a is done only when file exists
fn expand_glob(p: &str) -> Vec<String> {
    let expended = if let Some((_, start, middle, end)) = regex_captures!(r"^(.*)\{(.*)\}(.*)$", p)
    {
        middle
            .split(",")
            .map(|component| format!("{start}{component}{end}"))
            .collect_vec()
    }
    else {
        vec![p.to_owned()]
    };

    expended
        .into_iter()
        .flat_map(|p| {
            globmatch::Builder::new(p.as_str())
                .build("." /* std::env::current_dir().unwrap() */)
                .map(|builder| {
                    builder
                        .into_iter()
                        .map(|p2| {
                            match p2 {
                                Ok(p) => {
                                    let p = Utf8PathBuf::from_path_buf(p).unwrap();
                                    let s = p.to_string();
                                    if s.starts_with(".\\") {
                                        s[2..].to_owned()
                                    }
                                    else {
                                        s
                                    }
                                },
                                Err(_e) => p.clone()
                            }
                        })
                        .collect_vec()
                })
                .map(|v| {
                    if v.is_empty() {
                        vec![p.clone()]
                    }
                    else {
                        v
                    }
                })
                .unwrap_or(vec![p])
        })
        .collect_vec()
}

#[derive(Error, Debug)]
pub enum BndBuilderError {
    #[error("Unable to access file {fname}: {error}.")]
    InputFileError {
        fname: String,
        error: std::io::Error
    },
    #[error("Unable to setup working directory {fname}: {error}.")]
    WorkingDirectoryError {
        fname: String,
        error: std::io::Error
    },
    #[error("Unable to deserialize rules {0}.")]
    ParseError(serde_yaml::Error),
    #[error("Unable to build the dependency graph {0}.")]
    DependencyError(String),
    #[error("Unable to build {fname}: {msg}.")]
    ExecuteError { fname: String, msg: String },
    #[error("Unable to build default target.\n{source}")]
    DefaultTargetError { source: Box<BndBuilderError> },
    #[error("The file does not contain a target.")]
    NoTargets,
    #[error("The target {0} is disabled.")]
    DisabledTarget(String),
    #[error("Target {0} is not buildable.")]
    UnknownTarget(String),
    #[error("{0}")]
    AnyError(String)
}
