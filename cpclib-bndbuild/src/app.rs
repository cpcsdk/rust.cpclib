use std::io::{self, Write};
use std::ops::Deref;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use clap::{parser, ArgMatches};
use clap_complete::{generate, Shell};
use cpclib_basm::build_args_parser;
use cpclib_common::event::EventObserver;
use cpclib_common::itertools::Itertools;
use cpclib_runner::delegated::base_cache_folder;
use cpclib_runner::emucontrol::EmulatorFacadeRunner;
use cpclib_runner::runner::RunnerWithClap;

use crate::event::{
    BndBuilderObserved, BndBuilderObserver, BndBuilderObserverRc, ListOfBndBuilderObserverRc
};
use crate::runners::assembler::{BasmRunner, OrgamsRunner};
use crate::runners::bndbuild::BndBuildRunner;
use crate::runners::disc::DiscManagerRunner;
use crate::runners::fs::cp::CpRunner;
use crate::runners::fs::rm::RmRunner;
use crate::runners::hideur::HideurRunner;
use crate::runners::imgconverter::ImgConverterRunner;
use crate::runners::xfer::XferRunner;
use crate::task::{
    is_amspirit_cmd, is_basm_cmd, is_cp_cmd, is_disc_cmd, is_echo_cmd, is_emuctrl_cmd,
    is_extern_cmd, is_hideur_cmd, is_img2cpc_cmd, is_orgams_cmd, is_rm_cmd, is_winape_cmd,
    is_xfer_cmd, Task
};
use crate::{
    execute, init_project, BndBuilder, BndBuilderError, ALL_APPLICATIONS, EXPECTED_FILENAMES
};

pub struct BndBuilderApp {
    matches: clap::ArgMatches,
    observers: Arc<ListOfBndBuilderObserverRc>
}

#[derive(Debug)]
pub enum BndBuilderCommandInner {
    /// Print the help of the given command
    InnerHelp(String),
    /// Print the version of the various tools
    Version,
    /// Init a new project
    Init,
    /// Clear cache folder
    Clear(Option<String>),
    GenerateCompletion(Shell),
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
    Dot(BndBuilder, Option<String>),
    /// Update the executable from github artifact or the command if specified
    Update(Option<String>)
}

#[derive(Debug)]
pub struct BndBuilderCommand {
    inner: BndBuilderCommandInner,
    observers: Arc<ListOfBndBuilderObserverRc>
}

impl BndBuilderCommand {
    pub fn new(inner: BndBuilderCommandInner, observers: Arc<ListOfBndBuilderObserverRc>) -> Self {
        Self { inner, observers }
    }
}

impl BndBuilderObserved for BndBuilderCommand {
    fn observers(&self) -> Arc<ListOfBndBuilderObserverRc> {
        Arc::clone(&self.observers)
    }

    fn add_observer(&mut self, observer: BndBuilderObserverRc) {
        Arc::get_mut(&mut self.observers)
            .unwrap()
            .add_observer(observer);
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
            let previous_dir = std::env::current_dir().unwrap();
            let res = inner.execute_one_step();
            std::env::set_current_dir(previous_dir).unwrap();
            step = res?;
        }

        Ok(())
    }

    // Execute the first step of the  command and return None if if its finished are another command with this step removed
    pub fn execute_one_step(self) -> Result<Option<Self>, BndBuilderError> {
        let Self { inner, observers } = self;

        match inner {
            BndBuilderCommandInner::GenerateCompletion(generator) => {
                let mut cmd = crate::build_args_parser();
                let name = cmd.get_name().to_string();
                generate(generator, &mut cmd, name, &mut io::stdout());
                Ok(None)
            },
            BndBuilderCommandInner::InnerHelp(runner) => {
                Self::execute_help(runner.as_str(), &observers);
                Ok(None)
            },
            BndBuilderCommandInner::Version => {
                Self::execute_version(&observers);
                Ok(None)
            },
            BndBuilderCommandInner::Update(cmd) => {
                Self::execute_update(&observers, cmd.as_deref())?;
                Ok(None)
            },
            BndBuilderCommandInner::Init => {
                Self::execute_init(&observers)?;
                Ok(None)
            },
            BndBuilderCommandInner::Clear(command) => {
                Self::execute_clear(&observers, command.as_deref())?;
                Ok(None)
            },
            BndBuilderCommandInner::Direct(args) => {
                Self::execute_direct(args.as_str(), &observers)?;
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
                Self::execute_show(content, &observers);
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
            BndBuilderCommandInner::Dot(builder, g) => {
                Self::execute_dot(builder, g.as_deref(), &observers)?;
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
        observers: Arc<ListOfBndBuilderObserverRc>
    ) -> Result<Option<Self>, BndBuilderError> {
        let targets_provided = init_targets.is_some();

        // get the list of targets
        let targets = match init_targets.as_ref() {
            Some(targets) => targets.clone(),
            None => {
                assert_eq!(current_step, 0);
                if let Some(first) = builder.default_target() {
                    vec![first.to_owned()]
                }
                else {
                    return Err(BndBuilderError::NoTargets);
                }
            }
        };

        // select the right one
        let tgt = &targets[current_step];

        // execute if needed
        if builder.outdated(tgt)? {
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

        // set up the next step if any
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

    fn execute_dot<O: BndBuilderObserver>(
        builder: BndBuilder,
        g: Option<&str>,
        _observers: &O
    ) -> Result<(), BndBuilderError> {
        let dot = builder.to_dot(false); // TODO use an argument for that

        if let Some(g) = g {
            let path: &Utf8Path = Utf8Path::new(g);
            match path.extension() {
                Some(ext) if (ext == "svg") | (ext == "png") => {
                    let mut child = Command::new("dot")
                        .arg(format!("-T{ext}"))
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .spawn()
                        .map_err(|e| {
                            BndBuilderError::AnyError(format!("Unable to spawn dot. {e}"))
                        })?;
                    child
                        .stdin
                        .take()
                        .unwrap()
                        .write_all(dot.as_bytes())
                        .map_err(|e| {
                            BndBuilderError::AnyError(format!(
                                "Unable to send the dot content. {e}"
                            ))
                        })?;
                    let output = child.wait_with_output().map_err(|e| {
                        BndBuilderError::AnyError(format!("Error when executing  dot. {e}"))
                    })?;
                    std::fs::write(path, output.stdout).map_err(|e| {
                        BndBuilderError::AnyError(format!("Error while saving {path}. {e}"))
                    })
                },
                Some("dot") => {
                    std::fs::write(path, dot).map_err(|e| BndBuilderError::AnyError(e.to_string()))
                },
                Some(ext) => {
                    Err(BndBuilderError::AnyError(format!(
                        "Invalid extension {ext} for {path}"
                    )))
                },
                None => {
                    Err(BndBuilderError::AnyError(format!(
                        "Missing extension for {path}"
                    )))
                },
            }
        }
        else {
            builder.emit_stdout(dot);
            builder.emit_stdout("\n");
            Ok(())
        }
    }

    fn execute_list<O: BndBuilderObserver>(builder: BndBuilder, observers: O) {
        for rule in builder.rules() {
            builder.emit_stdout(format!(
                "{}{}: {}\n",
                if rule.is_enabled() { "" } else { "[disabled] " },
                rule.targets().iter().map(|f| f.to_string()).join(" "),
                rule.dependencies().iter().map(|f| f.to_string()).join(" "),
            ));
            if let Some(help) = rule.help() {
                observers.emit_stdout(&format!("\t{help}\n"));
            }
        }
    }

    fn execute_show<S: AsRef<str>>(content: S, observers: &dyn BndBuilderObserver) {
        observers.emit_stdout(content.as_ref());
    }

    fn execute_add_task<S: AsRef<str>, O: BndBuilderObserver>(
        add: String,
        dependencies: Vec<S>,
        kind: String,
        builder: BndBuilder,
        fname: Utf8PathBuf,
        _observers: O
    ) -> Result<(), BndBuilderError> {
        let targets = [add];
        let builder = builder.add_default_rule(&targets, &dependencies, &kind);
        builder.save(fname).expect("Error when saving the file");
        Ok(())
    }

    fn execute_direct<O>(cmd: &str, observers: &Arc<O>) -> Result<(), BndBuilderError>
    where O: BndBuilderObserver + 'static {
        // TODO remove strong dependency to serde_yaml and replace it by Task
        let task: Task = serde_yaml::from_str(cmd).map_err(BndBuilderError::ParseError)?;

        execute(&task, observers).map_err(BndBuilderError::AnyError)
    }

    /// Show the help of a given command
    /// TODO strenghten the help search by testing more keywords
    fn execute_help<O: BndBuilderObserver + 'static>(runner: &str, observers: &Arc<O>) {
        let help = if is_emuctrl_cmd(runner) {
            EmulatorFacadeRunner::<()>::render_help()
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
        else if is_hideur_cmd(runner) {
            HideurRunner::<()>::render_help()
        }
        else if is_orgams_cmd(runner) {
            OrgamsRunner::<()>::render_help()
        }
        else if is_extern_cmd(runner) {
            "Launch an external command.

Usage: extern <program> [arguments]...

Arguments:
  <program>
          The program to execute
  [arguments]...
          The arguments of the program"
                .to_owned()
        }
        else if is_echo_cmd(runner) {
            "Print the arguments.

Usage: echo [arguments]...

Arguments:
  [arguments]...
          Words to print"
                .to_owned()
        }
        else if is_amspirit_cmd(runner) {
            r"AMSpiriT peut être exécuté par une ligne de commande, en mode console par exemple, permettant
d’automatiser certaines séquences de démarrage.
De nouvelles commandes seront progressivement ajoutées selon les besoins.
Commandes disponibles :
Les commandes en ligne sont standardisées.
--autorun Exécute automatiquement un enregistrement Cassette
--crtc=X Fixe le type de CRTC au démarrage (X = 0, 1, 1b, 2 ou 4)
--file=file Charge un fichier dsk, ipf, hfe, cdt, wav, sna (le chemin doit être complet)
--csl=file Charge un fichier script « Cpc Scripting Language » (le chemin doit être complet)
--fullscreen Exécute AmspiriT en mode plein écran
--joystick Active le joystick (Mapping clavier)
--keybPC Clavier en mode mapping PC => CPC
--keybCPC Clavier en mode CPC (pas de mapping) – Disponible sur quelques claviers
--nojoystick Désactive le josystick
--mute Désactive le son
--romX=file_rom Charge un fichier ROM dans un emplacement X (X varie entre 1 et 15)
A noter que les ROMs chargées ne seront pas mémorisées par AmspiriT
--run=Filename Lance un programme présent sur une disquette ou une Rom.
--config-file=rep Fixe le répertoire de AmspiriT où se situe le fichier de configuration".to_owned()
        }
        else if is_winape_cmd(runner) {
            // http://www.winape.net/help/parameters.html
            r" When starting WinAPE a disc image filename can be specified as a parameter (without the slash option). The following parameters can be specified on the command line:

Parameter	Function
filename	Specify the filename for the disc image to be used in Drive A:
/A	Automatically run the program in Drive A:. To specify the name of the program to run use /A:filename. To start a disc using a CP/M boot sector use /A:|CPM
/T:filename	Automatically start typing from the given Auto-type file.
/SN:filename	Specify a Snapshot file to be loaded and automatically started.
/SYM:filename	Load a file containing assembler/debugger symbols.
/SHUTDOWN	Shut down Windows when WinAPE is closed. Use /SHUTDOWN:FORCE to force shutdown if required.

For example, to start WinAPE using the disc image frogger.dsk contained within a Zip file frogger.zip and run the program named frogger use:

WinAPE frogger.zip\:frogger.dsk /a:frogger            
            
            ".to_owned()
        }
        else if is_rm_cmd(runner) {
            RmRunner::<()>::render_help()
        }
        else if is_xfer_cmd(runner) {
            XferRunner::<()>::render_help()
        }
        else {
            let task = Task::from_str(&format!("{runner} --help")).unwrap();
            let _ = execute(&task, observers); // we ignore potential error
            "TODO / handle string collect instead of stdout output".into()
        };

        observers.emit_stdout(&format!("{help}\n"));
    }

    fn execute_version(observers: &dyn BndBuilderObserver) {
        observers.emit_stdout(&format!(
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

    
    fn execute_update(
        observers: &dyn BndBuilderObserver,
        cmd: Option<&str>
    ) -> Result<(), BndBuilderError> {
        let update_command = |command, can_install| -> Result<(), BndBuilderError> {
            observers.emit_stdout(&format!("> Update {command}\n"));

            let task =
                Task::from_str(command).map_err(|e| BndBuilderError::AnyError(e.to_string()))?;

            match task.configuration::<()>() {
                Some(conf) => {
                    let installed = conf.cache_folder().exists();
                    // try to delete only if exists
                    if installed {
                        Self::execute_clear(observers, Some(command))?;
                    }

                    if installed || can_install {
                        conf.install(&()).map_err(BndBuilderError::UpdateError)
                    }
                    else {
                        Ok(())
                    }
                },
                None => {
                    Err(BndBuilderError::AnyError(format!(
                        "{command} is not an embedded command."
                    )))
                },
            }
        };

        #[cfg(feature = "self-update")]
        let update_self = || -> Result<(), BndBuilderError> {
            observers.emit_stdout("> Update bndbuild\n");
            let (asset_url, asset_name) = if cfg!(target_os = "windows") {
                (
                    "https://github.com/cpcsdk/rust.cpclib/releases/download/latest/bndbuild.exe",
                    "bndbuild.exe"
                )
            }
            else if cfg!(not(target_os = "windows")) {
                (
                    "https://github.com/cpcsdk/rust.cpclib/releases/download/latest/bndbuild",
                    "bndbuild"
                )
            }
            else {
                unimplemented!()
            };
            let mut tmp_exec_path = camino_tempfile::Builder::new()
                .prefix("self_update")
                .tempfile()
                .map_err(|e| BndBuilderError::AnyError(format!("Temporary file error. {e}")))?;
            let tmp_exec = tmp_exec_path.as_file_mut();

            self_update::Download::from_url(asset_url).download_to(tmp_exec)?;
            self_update::self_replace::self_replace(tmp_exec_path).unwrap();
            Ok(())
        };

        let update_all = |can_install| -> Result<(), BndBuilderError> {
            #[cfg(feature = "self-update")]
            update_self()?;
            for cmd in ALL_APPLICATIONS.iter().filter_map(|(cmd, clearable)| {
                if *clearable {
                    Some(cmd[0])
                }
                else {
                    None
                }
            }) {
                update_command(cmd, can_install)?;
            }
            Ok(())
        };

        if let Some(cmd) = cmd {
            match cmd {
                #[cfg(feature = "self-update")]
                "self" => update_self(),
                #[cfg(not(feature = "self-update"))]
                "self" => unimplemented!("This feature has not been activated"),
                "all" => update_all(true),
                "installed" => update_all(false),
                cmd => update_command(cmd, true)
            }
        }
        else {
            #[cfg(feature = "self-update")]
            let res = update_self();
            #[cfg(not(feature = "self-update"))]
            let res = unimplemented!("This feature has not been activated");
            res
        }
    }

    fn execute_init(observers: &dyn BndBuilderObserver) -> Result<(), BndBuilderError> {
        init_project(None)?;
        observers.emit_stdout("Empty project initialized");
        Ok(())
    }

    fn execute_clear(
        observers: &dyn BndBuilderObserver,
        command: Option<&str>
    ) -> Result<(), BndBuilderError> {
        let folder = if let Some(command) = command {
            match Task::from_str(command)
                .map_err(|e| BndBuilderError::AnyError(e.to_string()))?
                .configuration::<()>()
            {
                Some(conf) => conf.cache_folder(),
                None => {
                    return Err(BndBuilderError::AnyError(format!(
                        "{command} is not an embedded command."
                    )));
                }
            }
        }
        else {
            base_cache_folder().to_owned()
        };

        std::fs::remove_dir_all(folder)
            .context("Error when removing cache folder")
            .map_err(|e| BndBuilderError::AnyError(e.to_string()))?;
        observers.emit_stdout(">> Cache folder cleared\n");
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
            observers: Arc::new(Vec::with_capacity(1).into())
        }
    }

    pub fn add_observer<O: Into<BndBuilderObserverRc>>(&mut self, o: O) {
        Arc::get_mut(&mut self.observers)
            .unwrap()
            .add_observer(o.into());
    }

    /// Get the string that represents the builder script after interpolation
    pub fn get_buildfile_content<P: AsRef<Utf8Path>>(
        &self,
        fname: P
    ) -> Result<String, BndBuilderError> {
        let fname = fname.as_ref();
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
            else if matches.contains_id("update") {
                if let Some(update) = matches.get_one::<String>("update") {
                    return Ok(BndBuilderCommandInner::Update(Some(update.to_owned())));
                }
                else {
                    return Ok(BndBuilderCommandInner::Update(None));
                }
            }
            else if matches.contains_id("clear") {
                if let Some(clear) = matches.get_one::<String>("clear") {
                    return Ok(BndBuilderCommandInner::Clear(Some(clear.to_owned())));
                }
                else {
                    return Ok(BndBuilderCommandInner::Clear(None));
                }
            }
            else if matches.get_flag("direct") {
                let cmd: String = matches
                    .get_many::<String>("target")
                    .ok_or_else(|| {
                        BndBuilderError::AnyError("--direct needs a command".to_owned())
                    })?
                    .map(|s| s.as_str())
                    .join(" ");
                return Ok(BndBuilderCommandInner::Direct(cmd));
            }
            else if let Some(generator) = matches.get_one::<Shell>("completion").copied() {
                return Ok(BndBuilderCommandInner::GenerateCompletion(generator));
            }

            // Search for the file to handle
            let fname: Utf8PathBuf = if let Some(fname) = matches.get_one::<String>("file") {
                let fname: &Utf8Path = fname.as_str().into();

                if fname.is_dir() {
                    EXPECTED_FILENAMES
                        .iter()
                        .map(|f| fname.join(f))
                        .find(|p| p.exists())
                        .unwrap_or_else(|| fname.to_owned())
                }
                else {
                    fname.to_owned()
                }
            }
            else {
                let mut selected = &EXPECTED_FILENAMES[1];
                for fname in EXPECTED_FILENAMES {
                    if Utf8Path::new(fname).exists() {
                        selected = fname;
                    }
                }
                selected.into()
            };

            let fname = &fname;

            // the other commands need the build file to exist
            if !fname.is_file() {
                {
                    let mut error_msg = if fname.is_dir() {
                        format!("Build directory `{fname}` does not contains a build file.")
                    }
                    else {
                        format!("Build file `{fname}` does not exists.")
                    };
                    if let Some(Some(fname)) = matches
                        .get_many::<String>("target")
                        .map(|s| s.into_iter().next())
                    {
                        if EXPECTED_FILENAMES.iter().any(|end| fname.ends_with(*end)) {
                            error_msg.push_str(&format!(
                                "\nHave you forgotten to do \"-f {fname}\" ?"
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

            if matches.contains_id("dot") {
                if let Some(g) = matches.get_one::<String>("dot") {
                    Ok(BndBuilderCommandInner::Dot(builder, Some(g.to_owned())))
                }
                else {
                    Ok(BndBuilderCommandInner::Dot(builder, None))
                }
            }
            else {
                // Get the targets
                let targets = matches
                    .get_many::<String>("target")
                    .map(|targets_provided| {
                        targets_provided
                            .cloned()
                            .map(|s| Utf8PathBuf::from_str(&s).unwrap())
                            .collect::<Vec<Utf8PathBuf>>()
                    });

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
                observers: Arc::clone(&self.observers)
            }
        })
    }
}
