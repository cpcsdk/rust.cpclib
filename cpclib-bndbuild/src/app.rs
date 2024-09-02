use std::str::FromStr;

use camino::{Utf8Path, Utf8PathBuf};
use clap::{parser, ArgMatches};
use cpclib_basm::build_args_parser;
use cpclib_common::itertools::Itertools;
use cpclib_runner::runner::RunnerWithClap;

use crate::{execute, init_project, task::Task, BndBuilder, BndBuilderError, BASM_RUNNER, EXPECTED_FILENAMES, IMGCONV_RUNNER, RM_RUNNER, XFER_RUNNER};


pub struct BndBuilderApp {
    matches: clap::ArgMatches
}

pub enum BndBuilderCommand {
    /// Print the help of the given command
    InnerHelp(String),
    /// Print the version of the various tools
    Version,
    /// Init a new project
    Init,
    /// Launch a direct command ans bypass bndbuild
    Direct(String),
    /// Add a task
    AddTask{task: String, dependencies: Vec<String>, kind: String, fname: Utf8PathBuf, builder: BndBuilder},
    /// Show the content of the file after interpolation
    Show(String),
    /// List the potential targets
    List(BndBuilder),
    /// Build the corresponding targets
    Build{targets: Option<Vec<Utf8PathBuf>>, watch: bool, builder: BndBuilder},
    /// Generate the graphviz file on stdout
    Dot(BndBuilder)
}


impl BndBuilderCommand {
    pub fn is_build(&self) -> bool {
        match self {
            Self::Build { .. } => true,
            _ => false
        }
    }
}

impl BndBuilderCommand {
    // Execute the first step of the  command and return None if if its finished are another command with this step removed
    pub fn execute_one_step(self) -> Result<Option<Self>, BndBuilderError> {
        match self {
            BndBuilderCommand::InnerHelp(runner) => {
                Self::execute_help(runner.as_str());
                Ok(None)
            },
            BndBuilderCommand::Version => {
                Self::execute_version();
                Ok(None)

            },
            BndBuilderCommand::Init => {
                Self::execute_init()?;
                Ok(None)
            },
            BndBuilderCommand::Direct(args) => {
                Self::execute_direct(args.as_str())?;
                Ok(None)
            }
            BndBuilderCommand::AddTask { task, dependencies, kind, fname, builder} => {
              Self::execute_add_task(&task, &dependencies, &kind,  builder, &fname)?;
              Ok(None)
            },
            BndBuilderCommand::Show(content) => {
               Self::execute_show(content);
               Ok(None)
            }
            BndBuilderCommand::List(builder) => {
                Self::execute_list(builder);
                Ok(None)
            },
            BndBuilderCommand::Build { targets, watch, builder } => {
                Self::execute_build(targets.as_ref().map(|v| v.as_slice()), watch, builder)
            },
            BndBuilderCommand::Dot(builder) => {
                Self::execute_dot(builder);
                Ok(None)
            },
        }
    }


    /// TODO wire parallal execution
    fn execute_build<P: AsRef<Utf8Path>>(targets: Option<&[P]>, watch: bool, builder: BndBuilder) -> Result<Option<Self>, BndBuilderError> {
        let targets_provided = targets.is_some();
        let targets = match targets {
            Some(targets) => {
                targets.into_iter().map(|s| s.as_ref().to_owned()).collect_vec()
            },
            None => if let Some(first) = builder.default_target() {
                vec![first.to_owned()]
            }
            else {
                return Err(BndBuilderError::NoTargets);
            }
        };
       
        let mut first_loop = true;
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

            if !watch {
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(1000)); // sleep 1s before trying to build
            first_loop = false;
        }

        Ok(())
    }

    fn execute_dot(builder: BndBuilder) {
        let dot = builder.to_dot();
        println!("{dot}")
    }

    fn execute_list(builder: BndBuilder) {
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
    }    

    fn execute_show(content: String) {
        println!("{content}");
    }

    fn execute_add_task<S: AsRef<str>>(add: &str, dependencies: &[S], kind: &str, builder: BndBuilder, fname: &Utf8Path) -> Result<(), BndBuilderError> {
        let targets = [add];
        let builder = builder.add_default_rule(&targets, &dependencies, kind);
        builder.save(fname).expect("Error when saving the file");
        return Ok(());
    }

    fn execute_direct(cmd: &str) -> Result<(), BndBuilderError> {
        // TODO remove strong dependency to serde_yaml and replace it by Task
        let task: Task = serde_yaml::from_str(&cmd).map_err(BndBuilderError::ParseError)?;
        execute(&task).map_err(BndBuilderError::AnyError)
    }

    /// Show the help of a given command
    /// TODO strenghten the help search by testing more keywords
    fn execute_help(runner: &str) {
        match runner {
            "bndbuild" => {
                build_args_parser().print_long_help().unwrap();
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
            _ => unimplemented!("Help not (yet) implemented for {runner}")
        }
    }

    fn execute_version() {
        println!(
            "{}\n{}\n{}\n{}",
            build_args_parser().clone().render_long_version(),
            BASM_RUNNER.get_clap_command().render_long_version(),
            IMGCONV_RUNNER.get_clap_command().render_long_version(),
            XFER_RUNNER.get_clap_command().render_long_version()
        );
    }

    fn execute_init() -> Result<(), BndBuilderError>{
        init_project(None)?;
        println!("Empty project initialized");
        Ok(())
    }
}



impl BndBuilderApp {

    /// Create the `BndBuildApp`
    /// - Return a wrapped self if we have to do something
    /// - Return a wrapped None if help has been displayed (TODO check if this case happens really)
    /// - Return an error in case of arguments error
    pub fn new() -> Result<Option<Self>, clap::error::Error> {
        let cmd = crate::build_args_parser();
        match cmd.clone().try_get_matches() {
            Result::Ok(matches) => {
                Ok(Some(Self {
                    matches
                }))
            },
            Result::Err(e) => {
                match e.kind() {
                        clap::error::ErrorKind::DisplayHelp |
                        clap::error::ErrorKind::DisplayVersion => Ok(None),
                        _ => Err(e),
                    }
            },
        }
    }

    pub fn from_matches(matches: ArgMatches) -> Self {
        Self{matches}
    }

    /// Get the string that represents the builder script after interpolation
    pub fn get_buildfile_content(&self, fname: &Utf8Path) -> Result<String, BndBuilderError> {
        // Get the variables definition
        let definitions = if let Some(definitions) = self.matches.get_many::<String>("DEFINE_SYMBOL") {
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

        BndBuilder::decode_from_fname_with_definitions(fname, &definitions)
            .map(|(_path, content)| content)
    }

    /// Extract the command to execute from the arguments of the command line
    pub fn command(&self) -> Result<BndBuilderCommand, BndBuilderError> {
        let matches = &self.matches;
        
        // handle the real behavior of commands that bypass bndbuild
        if matches.value_source("help") == Some(parser::ValueSource::CommandLine) {
            return Ok(BndBuilderCommand::InnerHelp(matches.get_one::<String>("help").unwrap().clone()));
        }
        else if matches.get_flag("version") {
            return Ok(BndBuilderCommand::Version);
        }
        else if matches.get_flag("init") {
            return Ok(BndBuilderCommand::Init)
        }
        else if matches.get_flag("direct") {
            let cmd: String = matches
                .get_many::<String>("target")
                .unwrap()
                .map(|s| s.as_str())
                .join(" ");
            return Ok(BndBuilderCommand::Direct(cmd))
        }


        // Search for the file to handle
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
        let fname = Utf8Path::new(fname);



        // the other commands need the build file to exist
        if !Utf8Path::new(fname).exists() {
            {
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

        let content = self.get_buildfile_content(fname)?;

        if matches.get_flag("show") {
            return Ok(BndBuilderCommand::Show(content));
        }

        let builder = BndBuilder::from_string(content)?;

        if let Some(add) = matches.get_one::<String>("add") {
            let dependencies = matches
                .get_many::<String>("dep")
                .map(|l| l.cloned().collect_vec())
                .unwrap_or_default();
            
            let kind = matches.get_one::<String>("kind").unwrap();
            return Ok(BndBuilderCommand::AddTask { 
                task: add.to_owned(),
                dependencies: dependencies,
                kind: kind.to_owned(), 
                fname: fname.to_owned(), 
                builder
            });
        }
        // Print list if asked
        else if matches.get_flag("list") {
            return Ok(BndBuilderCommand::List(builder));
        }


        if matches.get_flag("dot") {
            return Ok(BndBuilderCommand::Dot(builder));
        }
        else {
            // Get the targets
            let targets = if let Some(targets_provided) = matches.get_many::<String>("target") {
                Some(targets_provided
                    .cloned()
                    .map(|s| Utf8PathBuf::from_str(&s).unwrap())
                    .collect::<Vec<Utf8PathBuf>>())
            }
            else {
                None
            };
                
            let watch_requested = matches.get_flag("watch");

            return Ok(BndBuilderCommand::Build { targets, watch: watch_requested, builder});
            
            // Execute the targets

        }
    }

}