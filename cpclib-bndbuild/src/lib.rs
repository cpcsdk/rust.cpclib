#![feature(cfg_match)]

use std::env::current_dir;

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::clap;
use cpclib_common::clap::*;
use cpclib_common::itertools::Itertools;
use cpclib_runner::runner::RunnerWithClap;
use lazy_regex::regex_captures;
use serde::de::IntoDeserializer;
use serde::Deserialize;
use task::Task;
use thiserror::Error;

use crate::executor::*;
pub use crate::BndBuilder;

pub mod builder;
pub mod constraints;
pub mod executor;
pub mod rules;
pub mod runners;
pub mod task;

pub use builder::*;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub fn process_matches(cmd: Command, matches: &ArgMatches) -> Result<(), BndBuilderError> {

    {
        // handle the real behavior of bndbuild
        if matches.value_source("help") == Some(parser::ValueSource::CommandLine) {
            match matches.get_one::<String>("help").unwrap().as_str() {
                "bndbuild" => {
                    cmd.clone().print_long_help().unwrap();
                },
                "basm" => {
                    BASM_RUNNER.print_help();
                },
                "img2cpc" => {
                    IMGCONV_RUNNER.print_help();
                },
                "rm" => {
                    RM_RUNNER.print_help();
                },
                "xfer" => {
                    XFER_RUNNER.print_help();
                },
                _ => unimplemented!()
            };

            return Ok(());
        }

        if matches.get_flag("version") {
            println!(
                "{}\n{}\n{}\n{}",
                cmd.clone().render_long_version(),
                BASM_RUNNER.get_clap_command().render_long_version(),
                IMGCONV_RUNNER.get_clap_command().render_long_version(),
                XFER_RUNNER.get_clap_command().render_long_version()
            );
            return Ok(());
        }

        if matches.get_flag("init") {
            init_project(None)?;
            println!("Empty project initialized");
            return Ok(());
        }


        if matches.get_flag("direct") {
            let cmd = matches.get_many::<String>("target").unwrap().into_iter().map(|s| s.as_str())
            .join(" ");

            let task: Task = serde_yaml::from_str(&cmd)
                .map_err(|e| BndBuilderError::ParseError(e))?;
            execute(&task)
                .map_err(|e| BndBuilderError::AnyError(e))?;
            return Ok(());
        }

        // Get the file
        let fname = if let Some(fname) = matches.get_one::<String>("file") {
            fname.as_str()
        }
        else {
            let mut selected = &EXPECTED_FILENAMES[1];
            for fname in EXPECTED_FILENAMES {
                if Utf8Path::new(fname).exists() {
                    selected = fname;
                }
            }
            selected
        };

        let add = matches.get_one::<String>("add");

        // Read it
        if !Utf8Path::new(fname).exists() {
            if add.is_some() {
                std::fs::File::create(fname).expect("create empty {fname}");
            }
            else {
                eprintln!("{fname} does not exists.");
                if let Some(Some(fname)) = matches
                    .get_many::<String>("target")
                    .map(|s| s.into_iter().next())
                {
                    if fname.ends_with("bndbuild.yml") {
                        eprintln!("Have you forgotten to do \"-f {}\" ?", fname);
                    }
                }

                if matches
                    .get_many::<String>("target")
                    .map(|s| s.into_iter().any(|s| s == "init"))
                    .unwrap_or(false)
                {
                    eprintln!("Maybe you wanted to do --init.");
                }
                std::process::exit(1);
            }
        }

        // Get the variables definition
        let definitions = if let Some(definitions) = matches.get_many::<String>("DEFINE_SYMBOL") {
            definitions
                .into_iter()
                .map(|definition| {
                    let (symbol, value) = {
                        match definition.split_once("=") {
                            Some((symbol, value)) => (symbol, value),
                            None => (definition.as_str(), "1")
                        }
                    };
                    (symbol, value)
                })
                .collect_vec()
        }
        else {
            Default::default()
        };

        let (_path, content) = BndBuilder::decode_from_fname_with_definitions(fname, &definitions)?;
        if matches.get_flag("show") {
            println!("{content}");
            return Ok(());
        }

        let builder = BndBuilder::from_string(content)?;

        if let Some(add) = matches.get_one::<String>("add") {
            let targets = [add];
            let dependencies = matches
                .get_many::<String>("dep")
                .map(|l| l.collect_vec())
                .unwrap_or_default();
            let kind = matches.get_one::<String>("kind").unwrap();

            let builder = builder.add_default_rule(&targets, &dependencies, kind);
            builder.save(fname).expect("Error when saving the file");
            return Ok(());
        }

        // Print list if asked
        if matches.get_flag("list") {
            for rule in builder.rules() {
                println!(
                    "{}{}: {}",
                    if rule.is_enabled() { "" } else { "[disabled] " },
                    rule.targets().iter().map(|f| f.to_string()).join(" "),
                    rule.dependencies().iter().map(|f| f.to_string()).join(" "),
                );
                if let Some(help) = rule.help() {
                    println!("\t{}", help);
                }
            }
            return Ok(());
        }

        // Get the targets
        let targets_provided = matches.contains_id("target");
        let targets = if !targets_provided {
            if let Some(first) = builder.default_target() {
                vec![first]
            }
            else {
                return Err(BndBuilderError::NoTargets);
            }
        }
        else {
            matches
                .get_many::<String>("target")
                .unwrap()
                .map(|s| s.as_ref())
                .collect::<Vec<&Utf8Path>>()
        };

        if matches.get_flag("dot") {
            let dot = builder.to_dot();
            println!("{dot}")
        }
        else {
            // Execute the targets
            let mut first_loop = true;
            let watch_requested = matches.get_flag("watch");
            loop {
                for tgt in targets.iter() {
                    if first_loop || builder.outdated(tgt).unwrap_or(false) {
                        builder.execute(tgt).map_err(|e| {
                            if targets_provided {
                                e
                            }
                            else {
                                BndBuilderError::DefaultTargetError {
                                    source: Box::new(e)
                                }
                            }
                        })?;
                    }
                }

                if !watch_requested {
                    break;
                }

                std::thread::sleep(std::time::Duration::from_millis(1000)); // sleep 1s before trying to build
                first_loop = false;
            }
        }
    }

    Ok(())
}

pub fn build_args_parser() -> clap::Command {
    let basm_cmd = cpclib_basm::build_args_parser().name("basm");
    let img2cpc_cmd = cpclib_imgconverter::build_args_parser()
        .name("img2cpc")
        .disable_help_flag(false);
    let xfer_cmd = cpclib_xfertool::build_args_parser().name("xfer");
    let disc_cmd = cpclib_disc::dsk_manager_build_arg_parser().name("disc");

    Command::new("bndbuilder")
        .about("Benediction CPC demo project builder")
        .before_help("Can be used as a project builder similar to Make, but using a yaml project description, or can be used as any benedicition crossdev tool (basm, img2cpc, xfer, disc). This way only bndbuild needs to be installed.")
        .author("Krusty/Benediction")
        .version(built_info::PKG_VERSION)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("help")
                .long("help")
                .short('h')
                .value_name("CMD")
                .value_parser(["img2cpc", "basm", "rm", "bndbuild", "xfer"])
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
