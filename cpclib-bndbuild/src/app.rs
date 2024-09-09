use std::ops::Deref;
use std::str::FromStr;

use camino::{Utf8Path, Utf8PathBuf};
use clap::{parser, ArgMatches};
use cpclib_basm::build_args_parser;
use cpclib_common::itertools::Itertools;
use cpclib_runner::emucontrol::EmuControlledRunner;
use cpclib_runner::runner::RunnerWithClap;

use crate::event::{BndBuilderObserved, BndBuilderObserverWeak, ListOfBndBuilderObserverStrong};
use crate::runners::assembler::BasmRunner;
use crate::runners::bndbuild::BndBuildRunner;
use crate::runners::cp::CpRunner;
use crate::runners::disc::DiscManagerRunner;
use crate::runners::imgconverter::ImgConverterRunner;
use crate::runners::rm::RmRunner;
use crate::runners::xfer::XferRunner;
use crate::task::{
    is_basm_cmd, is_cp_cmd, is_disc_cmd, is_emuctrl_cmd, is_img2cpc_cmd, is_rm_cmd,
    is_xfer_cmd, Task
};
use crate::{execute, init_project, BndBuilder, BndBuilderError, EXPECTED_FILENAMES};

pub struct BndBuilderApp {
    matches: clap::ArgMatches,
    observers: ListOfBndBuilderObserverStrong
}

pub enum BndBuilderCommandInner {
    /// Print the help of the given command
    InnerHelp(String),
    /// Print the version of the various tools
    Version,
    /// Init a new project
    Init,
    /// Launch a direct command ans bypass bndbuild
    Direct(String),
    /// Add a task
    AddTask {
        task: String,
        dependencies: Vec<String>,
        kind: String,
        fname: Utf8PathBuf,
        builder: BndBuilder
    },
    /// Show the content of the file after interpolation
    Show(String),
    /// List the potential targets
    List(BndBuilder),
    /// Build the corresponding targets
    Build {
        targets: Option<Vec<Utf8PathBuf>>,
        watch: bool,
        current_step: usize,
        builder: BndBuilder
    },
    /// Generate the graphviz file on stdout
    Dot(BndBuilder)
}

pub struct BndBuilderCommand {
    inner: BndBuilderCommandInner,
    observers: ListOfBndBuilderObserverStrong
}

impl BndBuilderObserved for BndBuilderCommand {
    fn observers(&self) -> &[BndBuilderObserverWeak] {
        self.observers.observers()
    }

    fn add_observer(&mut self, observer: BndBuilderObserverWeak) {
        self.observers.add_observer(observer);
    }
}

impl Deref for BndBuilderCommand {
    type Target = BndBuilderCommandInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl BndBuilderCommandInner {
    pub fn is_build(&self) -> bool {
        match self {
            Self::Build { .. } => true,
            _ => false
        }
    }
}

impl BndBuilderCommand {
    /// Execute all the steps of the command
    pub fn execute(self) -> Result<(), BndBuilderError> {
        let mut step = Some(self);
        while let Some(inner) = step {
            step = inner.execute_one_step()?;
        }

        Ok(())
    }

    // Execute the first step of the  command and return None if if its finished are another command with this step removed
    pub fn execute_one_step(self) -> Result<Option<Self>, BndBuilderError> {
        let Self { inner, observers } = self;

        match inner {
            BndBuilderCommandInner::InnerHelp(runner) => {
                Self::execute_help(runner.as_str(), observers);
                Ok(None)
            },
            BndBuilderCommandInner::Version => {
                Self::execute_version(observers);
                Ok(None)
            },
            BndBuilderCommandInner::Init => {
                Self::execute_init(observers)?;
                Ok(None)
            },
            BndBuilderCommandInner::Direct(args) => {
                Self::execute_direct(args.as_str(), observers)?;
                Ok(None)
            },
            BndBuilderCommandInner::AddTask {
                task,
                dependencies,
                kind,
                fname,
                builder
            } => {
                Self::execute_add_task(task, dependencies, kind, builder, fname, observers)?;
                Ok(None)
            },
            BndBuilderCommandInner::Show(content) => {
                Self::execute_show(content, observers);
                Ok(None)
            },
            BndBuilderCommandInner::List(builder) => {
                Self::execute_list(builder, observers);
                Ok(None)
            },
            BndBuilderCommandInner::Build {
                targets,
                watch,
                current_step,
                builder
            } => Self::execute_build(targets, watch, current_step, builder, observers),
            BndBuilderCommandInner::Dot(builder) => {
                Self::execute_dot(builder, observers);
                Ok(None)
            }
        }
    }

    /// TODO wire parallal execution
    fn execute_build(
        init_targets: Option<Vec<Utf8PathBuf>>,
        watch: bool,
        mut current_step: usize,
        builder: BndBuilder,
        observers: ListOfBndBuilderObserverStrong
    ) -> Result<Option<Self>, BndBuilderError> {
        let targets_provided = init_targets.is_some();
        let targets = match init_targets.as_ref() {
            Some(targets) => targets.clone(),
            None => {
                if let Some(first) = builder.default_target() {
                    vec![first.to_owned()]
                }
                else {
                    return Err(BndBuilderError::NoTargets);
                }
            },
        };

        let tgt = &targets[current_step];

        if builder.outdated(tgt).unwrap_or(false) {
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

        let over = init_targets.as_ref().map(|v| v.len() - 1).unwrap_or(0) == current_step;

        if over {
            current_step = 0;
        }
        else {
            current_step += 1;
        }

        if over && !watch {
            Ok(None)
        }
        else {
            Ok(Some(BndBuilderCommand {
                inner: BndBuilderCommandInner::Build {
                    targets: init_targets,
                    watch,
                    current_step,
                    builder
                },
                observers
            }))
        }
    }

    fn execute_dot(builder: BndBuilder, _observers: ListOfBndBuilderObserverStrong) {
        let dot = builder.to_dot();
        builder.emit_stdout(dot);
        builder.emit_stdout("\n");
    }

    fn execute_list(builder: BndBuilder, observers: ListOfBndBuilderObserverStrong) {
        for rule in builder.rules() {
            builder.emit_stdout(format!(
                "{}{}: {}\n",
                if rule.is_enabled() { "" } else { "[disabled] " },
                rule.targets().iter().map(|f| f.to_string()).join(" "),
                rule.dependencies().iter().map(|f| f.to_string()).join(" "),
            ));
            if let Some(help) = rule.help() {
                observers.emit_stdout(format!("\t{}\n", help));
            }
        }
    }

    fn execute_show<S: AsRef<str>>(content: S, observers: ListOfBndBuilderObserverStrong) {
        observers.emit_stdout(content);
    }

    fn execute_add_task<S: AsRef<str>>(
        add: String,
        dependencies: Vec<S>,
        kind: String,
        builder: BndBuilder,
        fname: Utf8PathBuf,
        _observers: ListOfBndBuilderObserverStrong
    ) -> Result<(), BndBuilderError> {
        let targets = [add];
        let builder = builder.add_default_rule(&targets, &dependencies, &kind);
        builder.save(fname).expect("Error when saving the file");
        Ok(())
    }

    fn execute_direct(
        cmd: &str,
        observers: ListOfBndBuilderObserverStrong
    ) -> Result<(), BndBuilderError> {
        // TODO remove strong dependency to serde_yaml and replace it by Task
        let task: Task = serde_yaml::from_str(cmd).map_err(BndBuilderError::ParseError)?;

        execute(&task, &observers).map_err(BndBuilderError::AnyError)
    }

    /// Show the help of a given command
    /// TODO strenghten the help search by testing more keywords
    fn execute_help(runner: &str, observers: ListOfBndBuilderObserverStrong) {
        let help = if is_emuctrl_cmd(runner) {
            EmuControlledRunner::<()>::render_help()
        }
        else if is_basm_cmd(runner) {
            BasmRunner::<()>::render_help()
        }
        else if crate::task::is_bndbuild_cmd(runner) {
            BndBuildRunner::<()>::render_help()
        }
        else if is_cp_cmd(runner) {
            CpRunner::<()>::render_help()
        }
        else if is_disc_cmd(runner) {
            DiscManagerRunner::<()>::render_help()
        }
        else if is_img2cpc_cmd(runner) {
            ImgConverterRunner::<()>::render_help()
        }
        else if is_rm_cmd(runner) {
            RmRunner::<()>::render_help()
        }
        else if is_xfer_cmd(runner) {
            XferRunner::<()>::render_help()
        }
        else {
            unimplemented!("{runner}")
        };

        observers.emit_stdout(format!("{help}\n"));
    }

    fn execute_version(observers: ListOfBndBuilderObserverStrong) {
        observers.emit_stdout(format!(
            "{}\n{}\n{}\n{}",
            build_args_parser().clone().render_long_version(),
            BasmRunner::<()>::default()
                .get_clap_command()
                .render_long_version(),
            ImgConverterRunner::<()>::default()
                .get_clap_command()
                .render_long_version(),
            XferRunner::<()>::default()
                .get_clap_command()
                .render_long_version()
        ));
    }

    fn execute_init(observers: ListOfBndBuilderObserverStrong) -> Result<(), BndBuilderError> {
        init_project(None)?;
        observers.emit_stdout("Empty project initialized");
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
            Result::Ok(matches) => Ok(Some(Self::from_matches(matches))),
            Result::Err(e) => {
                match e.kind() {
                    clap::error::ErrorKind::DisplayHelp
                    | clap::error::ErrorKind::DisplayVersion => Ok(None),
                    _ => Err(e)
                }
            },
        }
    }

    pub fn from_matches(matches: ArgMatches) -> Self {
        Self {
            matches,
            observers: Vec::with_capacity(1).into()
        }
    }

    pub fn add_observer<O: Into<BndBuilderObserverWeak>>(&mut self, o: O) {
        self.observers.push(o.into());
    }

    /// Get the string that represents the builder script after interpolation
    pub fn get_buildfile_content(&self, fname: &Utf8Path) -> Result<String, BndBuilderError> {
        // Get the variables definition
        let definitions =
            if let Some(definitions) = self.matches.get_many::<String>("DEFINE_SYMBOL") {
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
        let get_inner = || -> Result<BndBuilderCommandInner, BndBuilderError> {
            let matches = &self.matches;

            // handle the real behavior of commands that bypass bndbuild
            if matches.value_source("help") == Some(parser::ValueSource::CommandLine) {
                return Ok(BndBuilderCommandInner::InnerHelp(
                    matches.get_one::<String>("help").unwrap().clone()
                ));
            }
            else if matches.get_flag("version") {
                return Ok(BndBuilderCommandInner::Version);
            }
            else if matches.get_flag("init") {
                return Ok(BndBuilderCommandInner::Init);
            }
            else if matches.get_flag("direct") {
                let cmd: String = matches
                    .get_many::<String>("target")
                    .unwrap()
                    .map(|s| s.as_str())
                    .join(" ");
                return Ok(BndBuilderCommandInner::Direct(cmd));
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
                    let mut error_msg = format!("{fname} does not exists.");
                    if let Some(Some(fname)) = matches
                        .get_many::<String>("target")
                        .map(|s| s.into_iter().next())
                    {
                        if fname.ends_with("bndbuild.yml") {
                            error_msg.push_str(&format!(
                                "\nHave you forgotten to do \"-f {}\" ?",
                                fname
                            ));
                        }
                    }

                    if matches
                        .get_many::<String>("target")
                        .map(|s| s.into_iter().any(|s| s == "init"))
                        .unwrap_or(false)
                    {
                        error_msg.push_str("\nMaybe you wanted to do --init.");
                    }
                    return Err(BndBuilderError::AnyError(error_msg));
                }
            }

            let content = self.get_buildfile_content(fname)?;

            if matches.get_flag("show") {
                return Ok(BndBuilderCommandInner::Show(content));
            }

            let mut builder = BndBuilder::from_string(content)?;
            for observer in self.observers.iter() {
                builder.add_observer(observer.clone());
            }

            if let Some(add) = matches.get_one::<String>("add") {
                let dependencies = matches
                    .get_many::<String>("dep")
                    .map(|l| l.cloned().collect_vec())
                    .unwrap_or_default();

                let kind = matches.get_one::<String>("kind").unwrap();
                return Ok(BndBuilderCommandInner::AddTask {
                    task: add.to_owned(),
                    dependencies,
                    kind: kind.to_owned(),
                    fname: fname.to_owned(),
                    builder
                });
            }
            // Print list if asked
            else if matches.get_flag("list") {
                return Ok(BndBuilderCommandInner::List(builder));
            }

            if matches.get_flag("dot") {
                Ok(BndBuilderCommandInner::Dot(builder))
            }
            else {
                // Get the targets
                let targets = matches.get_many::<String>("target").map(|targets_provided| targets_provided
                            .cloned()
                            .map(|s| Utf8PathBuf::from_str(&s).unwrap())
                            .collect::<Vec<Utf8PathBuf>>());

                let watch_requested = matches.get_flag("watch");

                Ok(BndBuilderCommandInner::Build {
                    targets,
                    watch: watch_requested,
                    current_step: 0,
                    builder
                })

                // Execute the targets
            }
        };

        get_inner().map(|inner| {
            BndBuilderCommand {
                inner,
                observers: self.observers.clone()
            }
        })
    }
}
