#![feature(cfg_select)]
#![feature(closure_lifetime_binder)]

use std::env::current_dir;
use std::sync::OnceLock;

use app::BndBuilderApp;
use clap_complete::Shell;
use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::clap;
use cpclib_common::clap::*;
use cpclib_common::itertools::Itertools;
use lazy_regex::regex_captures;
#[cfg(feature = "fap")]
use task::FAP_CMDS;
use task::{
    ACE_CMDS, AMSPIRIT_CMDS, AT_CMDS, BASM_CMDS, BDASM_CMDS, BNDBUILD_CMDS, CAPRICEFOREVER_CMDS,
    CHIPNSFX_CMDS, CONVGENERIC_CMDS, CP_CMDS, CPCEC_CMDS, CPCEMUPOWER_CMDS, DISARK_CMDS, DISC_CMDS,
    ECHO_CMDS, EMUCTRL_CMDS, EXTERN_CMDS, HIDEUR_CMDS, HSPC_CMDS, IMG2CPC_CMDS, IMPDISC_CMDS,
    MARTINE_CMDS, ORGAMS_CMDS, RASM_CMDS, RM_CMDS, SJASMPLUS_CMDS, SONG2AKM_CMDS, SUGARBOX_CMDS,
    UZ80_CMDS, VASM_CMDS, WINAPE_CMDS, XFER_CMDS
};
use thiserror::Error;

pub use crate::BndBuilder;
use crate::event::BndBuilderObserverRc;
use crate::executor::*;
use crate::task::GRAFX2_CMDS;

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

pub fn process_matches_with_observer(
    matches: &ArgMatches,
    o: BndBuilderObserverRc
) -> Result<(), BndBuilderError> {
    let mut cmd = BndBuilderApp::from_matches(matches.clone());
    cmd.add_observer(o);
    cmd.command()?.execute()
}

pub const ALL_APPLICATIONS: &[(&[&str], bool)] = &[
    (ACE_CMDS, true), // true for clearable, false for others
    (AMSPIRIT_CMDS, true),
    (AT_CMDS, true),
    (BASM_CMDS, false),
    (BDASM_CMDS, false),
    (BNDBUILD_CMDS, false),
    (CHIPNSFX_CMDS, true),
    (CONVGENERIC_CMDS, true),
    (CP_CMDS, false),
    (CPCEC_CMDS, true),
    (CPCEMUPOWER_CMDS, true),
    (CAPRICEFOREVER_CMDS, true),
    (DISC_CMDS, false),
    (DISARK_CMDS, true),
    (ECHO_CMDS, false),
    (EMUCTRL_CMDS, false),
    (EXTERN_CMDS, false),
    #[cfg(feature = "fap")]
    (FAP_CMDS, true),
    (GRAFX2_CMDS, true),
    (HIDEUR_CMDS, false),
    (HSPC_CMDS, true),
    (IMG2CPC_CMDS, false),
    (IMPDISC_CMDS, true),
    (MARTINE_CMDS, true),
    (ORGAMS_CMDS, false),
    (RASM_CMDS, true),
    (RM_CMDS, false),
    (SJASMPLUS_CMDS, true),
    (SONG2AKM_CMDS, true),
    (SUGARBOX_CMDS, true),
    (UZ80_CMDS, true),
    (VASM_CMDS, true),
    (WINAPE_CMDS, true),
    (XFER_CMDS, false)
];

pub fn commands_list() -> &'static (Vec<&'static str>, Vec<&'static str>) {
    static COMMANDS_LIST: OnceLock<(Vec<&str>, Vec<&str>)> = OnceLock::new();
    COMMANDS_LIST.get_or_init(|| {
        let all_applications = ALL_APPLICATIONS;

        let mut all = Vec::with_capacity(all_applications.iter().map(|l| l.0.len()).sum());
        let mut clearable = Vec::with_capacity(
            all_applications
                .iter()
                .map(|l| if l.1 { l.0.len() } else { 0 })
                .sum()
        );
        for l in all_applications.iter() {
            all.extend_from_slice(l.0);
            if l.1 {
                clearable.extend_from_slice(l.0);
            }
        }

        all.sort();
        clearable.sort();

        (all, clearable)
    })
}

pub fn build_args_parser() -> clap::Command {
    static COMMANDS_LIST: OnceLock<(Vec<&str>, Vec<&str>)> = OnceLock::new();
    let (commands_list, clearable_list) = commands_list();
    let updatable_list = clearable_list;

    let cmd = Command::new("bndbuilder")
        .about("Benediction CPC demo project builder")
        .before_help("Can be used as a project builder similar to Make, but using a yaml project description, or can be used as any Benediction crossdev tool (basm, img2cpc, xfer, disc). This way only bndbuild needs to be installed.")
        .author("Krusty/Benediction")
        .version(built_info::PKG_VERSION)
        .disable_help_flag(true)
        .disable_version_flag(true)
        ;

    #[cfg(feature = "self-update")]
    let cmd = cmd.arg(
            Arg::new("update")
                .long("update")
                .short('u')
                .num_args(0..=1)
                .value_parser(updatable_list.iter().chain(&["self", "all", "installed"]).collect_vec())
                .help("Update (or install) bndbuild or a given embedded application if provided. There are specific cases: if `all` is provided, it update all applications and bndbuild itself, if `installed` is provided it update only installed applications, if `self` is provide, it updates bndbuild itself.")
                .exclusive(true)
        );

    let cmd = cmd
        .arg(
            Arg::new("help")
                .long("help")
                .short('h')
                .value_name("CMD")
                .value_parser(commands_list.clone())
                .default_missing_value_os("bndbuild")
                .default_value("bndbuild")
                .num_args(0..=1)
                .help("Show the help of the given subcommand CMD.")
        )
        .arg(
            Arg::new("direct")
            .action(ArgAction::SetTrue)
            .long("direct")
            .help(format!("Bypass the task file and directly execute a command along: [{}].", commands_list.iter().join(", ")))
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
                .num_args(0..=1)
                .value_hint(ValueHint::FilePath)
                .help("Generate the graphviz representation of the selected bndbuild.yml file. If no file is provided, it prints the .dot representation. Otherwise it saves it on disc (only .dot, .png and .svg are possible. dot command MUST be installed and available in PATH)")
        )
        .arg(
            Arg::new("show")
                .long("show")
                .help("Show the file AFTER interpreting the templates")
                .action(ArgAction::SetTrue)
                .conflicts_with("dot")
        )
        .arg(
            Arg::new("numbered")
            .long("numbered")
            .help("number the lines when showing them")
            .action(ArgAction::SetTrue)
            .requires("show")
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
                .value_parser(clearable_list.clone())
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
                .value_parser(commands_list.clone())
                .requires("add")
                .default_missing_value("basm")
        )

        .arg(
            Arg::new("target")
                .action(ArgAction::Append)
                .value_name("TARGET")
                .help("Provide the target(s) to run.")
                .conflicts_with_all(["list", "init", "add"])
        );

    // TODO use query_shell https://crates.io/crates/query-shell to get the proper shell

    cmd.arg(
        Arg::new("completion")
            .long("completion")
            .action(ArgAction::Set)
            .help("Generate autocompletion configuration")
            .value_parser(value_parser!(Shell))
    )
}

pub fn init_project(path: Option<&Utf8Path>) -> Result<(), BndBuilderError> {
    let path = path
        .map(|p| p.to_owned())
        .unwrap_or_else(|| Utf8PathBuf::from_path_buf(current_dir().unwrap()).unwrap());

    if !path.is_dir() {
        return Err(BndBuilderError::AnyError(format!(
            "{path} is not a valid directory"
        )));
    }

    let bndbuild_yml = path.join("bndbuild.yml");
    if bndbuild_yml.exists() {
        return Err(BndBuilderError::AnyError(format!(
            "{bndbuild_yml} already exists"
        )));
    }

    let main_asm = path.join("main.asm");
    if main_asm.exists() {
        return Err(BndBuilderError::AnyError(format!(
            "{main_asm} already exists"
        )));
    }

    let data_asm = path.join("data.asm");
    if main_asm.exists() {
        return Err(BndBuilderError::AnyError(format!(
            "{data_asm} already exists"
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
                .map(|v| if v.is_empty() { vec![p.clone()] } else { v })
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
    AnyError(String),
    #[cfg(feature = "self-update")]
    #[error("Self-update error: {0}")]
    SelfUpdateError(self_update::errors::Error),
    #[error("Udate error: {0}")]
    UpdateError(String)
}

#[cfg(feature = "self-update")]
impl From<self_update::errors::Error> for BndBuilderError {
    fn from(value: self_update::errors::Error) -> Self {
        Self::SelfUpdateError(value)
    }
}
