use std::io::{self, Write};
use std::ops::Deref;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use clap::{ArgMatches, parser};
use clap_complete::{Shell, generate};
use cpclib_common::event::EventObserver;
use cpclib_common::itertools::Itertools;
use cpclib_runner::delegated::base_cache_folder;
use cpclib_runner::emucontrol::EmulatorFacadeRunner;
use cpclib_runner::runner::RunnerWithClap;

use crate::env::create_template_env;
use crate::event::{
    BndBuilderObserved, BndBuilderObserver, BndBuilderObserverRc, ListOfBndBuilderObserverRc
};
use crate::runners::assembler::{BasmRunner, OrgamsRunner};
use crate::runners::bndbuild::BndBuildRunner;
use crate::runners::disc::DiscManagerRunner;
use crate::runners::fs::cp::CpRunner;
use crate::runners::fs::rm::RmRunner;
use crate::runners::hideur::HideurRunner;
use crate::runners::img2cpc::ImgToCpcRunner;
use crate::runners::xfer::XferRunner;
use crate::task::{
    Task, is_amspirit_cmd, is_basm_cmd, is_cp_cmd, is_disc_cmd, is_echo_cmd, is_emuctrl_cmd,
    is_extern_cmd, is_hideur_cmd, is_img2cpc_cmd, is_orgams_cmd, is_rm_cmd, is_two_cdt_cmd,
    is_winape_cmd, is_xfer_cmd
};
use crate::{
    ALL_APPLICATIONS, BndBuilder, BndBuilderError, EXPECTED_FILENAMES, execute, init_project
};

/// Main application structure that manages bndbuild commands and execution.
///
/// This struct encapsulates the command-line interface and coordinates
/// the execution of build commands, targets, and related operations.
pub struct BndBuilderApp {
    matches: clap::ArgMatches,
    observers: Arc<ListOfBndBuilderObserverRc>,
    #[cfg(feature = "rayon")]
    force_serial: bool
}

#[derive(Debug, Clone)]
pub enum WatchState {
    NoWatch,
    WatchFirstRound,
    WatchNextRounds { last_build: SystemTime }
}

impl WatchState {
    pub fn request_watch(&self) -> bool {
        !matches!(self, WatchState::NoWatch)
    }

    pub fn disable_phony(&self) -> bool {
        match self {
            Self::NoWatch => false,
            Self::WatchFirstRound => false,
            Self::WatchNextRounds { .. } => true
        }
    }

    pub fn last_build(&self) -> Option<&SystemTime> {
        match self {
            Self::WatchNextRounds { last_build } => Some(last_build),
            _ => None
        }
    }
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
    Direct(String, bool),
    /// Add a task
    AddTask {
        task: String,
        dependencies: Vec<String>,
        kind: String,
        fname: Utf8PathBuf,
        builder: BndBuilder
    },
    /// Show the content of the file after interpolation. Eventulaly number the lines
    Show(String, bool),
    /// List the potential targets
    List(BndBuilder),
    /// Build the corresponding targets
    Build {
        targets: Option<Vec<Utf8PathBuf>>,
        watch: WatchState,
        current_step: usize,
        builder: BndBuilder
    },
    /// Generate the graphviz file on stdout
    /// Fields: builder, output_path, graph_details, include_dependencies, source_file
    Dot(BndBuilder, Option<String>, bool, bool, Option<Utf8PathBuf>),
    /// Update the executable from github artifact or the command if specified
    Update(Option<String>)
}

#[derive(Debug)]
pub struct BndBuilderCommand {
    inner: BndBuilderCommandInner,
    observers: Arc<ListOfBndBuilderObserverRc>
}

impl BndBuilderCommand {
    /// Returns true if the command is a help command
    pub fn is_help(&self) -> bool {
        matches!(self.deref(), BndBuilderCommandInner::InnerHelp(_))
    }

    pub fn new(inner: BndBuilderCommandInner, observers: Arc<ListOfBndBuilderObserverRc>) -> Self {
        Self { inner, observers }
    }

    pub fn clear_observers(&mut self) {
        self.observers = Arc::new(ListOfBndBuilderObserverRc::default());
    }
}

impl BndBuilderObserved for BndBuilderCommand {
    fn observers(&self) -> Arc<ListOfBndBuilderObserverRc> {
        Arc::clone(&self.observers)
    }

    fn add_observer(&mut self, observer: BndBuilderObserverRc) {
        Arc::get_mut(&mut self.observers)
            .expect("Failed to get mutable reference to observers (multiple Arc references exist)")
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
            let previous_dir = std::env::current_dir().map_err(|e| {
                BndBuilderError::WorkingDirectoryError {
                    fname: "<current>".to_owned(),
                    error: e
                }
            })?;
            let res = inner.execute_one_step();
            std::env::set_current_dir(previous_dir).map_err(|e| {
                BndBuilderError::WorkingDirectoryError {
                    fname: "<previous>".to_owned(),
                    error: e
                }
            })?;
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
            BndBuilderCommandInner::Direct(args, with_expansion) => {
                Self::execute_direct(args.as_str(), with_expansion, &observers)?;
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
            BndBuilderCommandInner::Show(content, numbered) => {
                Self::execute_show(content, numbered, &observers);
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
            BndBuilderCommandInner::Dot(builder, g, details, include_deps, source_file) => {
                Self::execute_dot(builder, g.as_deref(), details, include_deps, source_file.as_deref(), &observers)?;
                Ok(None)
            }
        }
    }

    /// TODO wire parallal execution
    fn execute_build(
        init_targets: Option<Vec<Utf8PathBuf>>,
        watch: WatchState,
        mut current_step: usize,
        mut builder: BndBuilder,
        observers: Arc<ListOfBndBuilderObserverRc>
    ) -> Result<Option<Self>, BndBuilderError> {
        // Sync the command's observer chain into the builder so that all
        // rule/task events (StartRule, StopRule, StartTask, …) reach the
        // same observers as direct emit_stdout/emit_stderr calls.
        builder.clear_observers();
        for obs in observers.iter() {
            builder.add_observer(obs.clone());
        }

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
        let last_build = if builder.outdated(&watch, tgt)? {
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
            Some(SystemTime::now())
        }
        else {
            observers.emit_stdout(&format!("Target {} is already up to date.\n", tgt));
            None
        };

        // set up the next step if any
        let over = init_targets.as_ref().map(|v| v.len() - 1).unwrap_or(0) == current_step;

        if over {
            current_step = 0;
        }
        else {
            current_step += 1;
        }

        if over && !watch.request_watch() {
            Ok(None)
        }
        else {
            std::thread::sleep(Duration::from_millis(2000)); // duration to wait
            Ok(Some(BndBuilderCommand {
                inner: BndBuilderCommandInner::Build {
                    targets: init_targets,
                    watch: WatchState::WatchNextRounds {
                        last_build: last_build.unwrap_or_else(|| {
                            watch.last_build().cloned().unwrap_or_else(|| {
                                // Fallback to current time if metadata is unavailable
                                tgt.metadata()
                                    .ok()
                                    .and_then(|m| m.modified().ok())
                                    .unwrap_or_else(std::time::SystemTime::now)
                            })
                        })
                    },
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
        details: bool,
        include_deps: bool,
        source_file: Option<&Utf8Path>,
        observers: &O
    ) -> Result<(), BndBuilderError> {
        let dot = if include_deps {
            builder.to_dot_multi(source_file, details)
        } else {
            builder.to_dot(details)
        };

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
                        .expect("Failed to take stdin from child process")
                        .write_all(dot.as_bytes())
                        .map_err(|e| {
                            BndBuilderError::AnyError(format!(
                                "Unable to send the dot content. {e}"
                            ))
                        })?;
                    let output = child.wait_with_output().map_err(|e| {
                        BndBuilderError::AnyError(format!("Error when executing  dot. {e}"))
                    })?;
                    fs_err::write(path, output.stdout).map_err(|e| {
                        BndBuilderError::AnyError(format!("Error while saving {path}. {e}"))
                    })
                },
                Some("dot") => {
                    fs_err::write(path, dot).map_err(|e| BndBuilderError::AnyError(e.to_string()))
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
            observers.emit_stdout(&dot);
            Ok(())
        }
    }

    fn execute_list<O: BndBuilderObserver>(builder: BndBuilder, observers: O) {
        let ordered_rules = builder
            .rules()
            .iter()
            .sorted_by_cached_key(|r| r.targets().iter().map(|f| f.to_string()).join(" "));
        for rule in ordered_rules.into_iter() {
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

    fn execute_show<S: AsRef<str>>(content: S, numbered: bool, observers: &dyn BndBuilderObserver) {
        if numbered {
            let content = content.as_ref();
            for (idx, line) in content.lines().enumerate() {
                let idx = idx + 1;
                observers.emit_stdout(&format!("{idx:03} {line}\n"));
            }
        }
        else {
            observers.emit_stdout(content.as_ref());
        }
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
        let builder = builder.add_default_rule(&targets, &dependencies, &kind)?;
        builder.save(&fname).map_err(|e| {
            BndBuilderError::WorkingDirectoryError {
                fname: fname.to_string(),
                error: e
            }
        })?;
        Ok(())
    }

    fn execute_direct<O>(
        cmd: &str,
        with_expansion: bool,
        observers: &Arc<O>
    ) -> Result<(), BndBuilderError>
    where
        O: BndBuilderObserver + 'static
    {
        // TODO remove strong dependency to serde_yaml and replace it by Task

        let cmd = if with_expansion {
            let env = create_template_env::<&str, &str, &str>(None, &[]); // TODO add definition handling
            env.render_str(cmd, minijinja::context!()).map_err(|e| {
                BndBuilderError::AnyError(format!("Error when handling cmd expansion. {e}"))
            })?
        }
        else {
            cmd.to_string()
        };

        let task: Task = serde_yaml::from_str(&cmd).map_err(|e| {
            BndBuilderError::from((
                e,
                if with_expansion {
                    "<direct:expanded>"
                }
                else {
                    "<direct>"
                }
                .into(),
                cmd.as_str()
            ))
        })?;

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
            ImgToCpcRunner::<()>::render_help()
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
        else if is_two_cdt_cmd(runner) {
            r"2CDT will transfer files into a .CDT/.TZX tape image, in Amstrad CPC/CPC+
KC Compact form.

Usage: 2CDT [arguments] <input filename> <.cdt image>

-n              - Blank CDT file before use
-b <number>         - Specify Baud rate (default 2000)
-s <0 or 1>     - Specify 'Speed Write'.
                  0 = 1000 baud, 1 = 2000 baud (default)
-t <method>     - TZX Block Write Method.
                  0 = Pure Data, 1 = Turbo Loading (default)
-m <method>     - Data method
                  0 = blocks (default)
                  1 = headerless (Firmware function: CAS READ - &BCA1)
                  2 = spectrum
                  3 = Two blocks. First block of 2K, second block has remainder
-H <number>     = Headerless sync byte (default &16)
-X <number>     = Define or override execution address (default is &1000 if no header)
-L <number>     = Define or override load address (default is &1000 if no header)
-F <number>     = Define or override file type (0=BASIC, 2=Binary (default if no header), 22=ASCII) etc. Applies to Data method 0
-p <number>     = Set initial pause in milliseconds (default 3000ms)
-P              = Add a 1ms pause for buggy emulators that ignore first block
-r <tape filename>
                - Add <input filename> as <tape filename> to CDT (rename file)
".to_owned()
        }
        else {
            match Task::from_str(&format!("{runner} --help")) {
                Ok(task) => {
                    let _ = execute(&task, observers); // we ignore potential error
                    "TODO / handle string collect instead of stdout output".into()
                },
                Err(e) => {
                    format!("Failed to create help task: {e}")
                }
            }
        };

        observers.emit_stdout(&format!("{help}\n"));
    }

    fn execute_version(observers: &dyn BndBuilderObserver) {
        observers.emit_stdout(&format!(
            "bndbuilder {}\nCompiled: {}\n\n{}\n\n{}\n\n{}",
            crate::built_info::PKG_VERSION,
            crate::built_info::BUILT_TIME_UTC,
            BasmRunner::<()>::default()
                .get_clap_command()
                .render_long_version(),
            ImgToCpcRunner::<()>::default()
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
            let binary_name = std::env::current_exe()
                .ok()
                .and_then(|p| p.file_stem().map(|s| s.to_os_string()))
                .and_then(|s| s.into_string().ok())
                .unwrap_or_else(|| "bndbuild".to_string());
            observers.emit_stdout(&format!("> Update {binary_name}\n"));
            let asset_url = if cfg!(target_os = "windows") {
                format!(
                    "https://github.com/cpcsdk/rust.cpclib/releases/download/latest/{binary_name}.exe"
                )
            }
            else {
                format!(
                    "https://github.com/cpcsdk/rust.cpclib/releases/download/latest/{binary_name}"
                )
            };
            let mut tmp_exec_path = camino_tempfile::Builder::new()
                .prefix("self_update")
                .tempfile()
                .map_err(|e| BndBuilderError::AnyError(format!("Temporary file error. {e}")))?;
            let tmp_exec = tmp_exec_path.as_file_mut();

            self_update::Download::from_url(&asset_url).download_to(tmp_exec)?;
            self_update::self_replace::self_replace(tmp_exec_path).map_err(|e| {
                BndBuilderError::UpdateError(format!("Failed to replace binary: {e}"))
            })?;
            Ok(())
        };

        let update_all = |can_install| -> Result<(), BndBuilderError> {
            #[cfg(feature = "self-update")]
            update_self()?;

            let mut failures = Vec::new();
            for cmd in
                ALL_APPLICATIONS.iter().filter_map(
                    |(cmd, clearable)| {
                        if *clearable { Some(cmd[0]) } else { None }
                    }
                )
            {
                let res = update_command(cmd, can_install);
                if let Err(e) = res {
                    observers.emit_stderr(&format!(">> Failure when updating {cmd}\n"));
                    observers.emit_stderr(&format!("   {e}\n"));
                    failures.push((cmd.to_string(), e));
                }
            }

            if failures.is_empty() {
                observers.emit_stdout(">> All applications updated successfully\n");
                Ok(())
            }
            else {
                Err(BndBuilderError::AnyError(format!(
                    "{} application(s) failed to update ({}).",
                    failures.len(),
                    failures
                        .iter()
                        .map(|(cmd, _)| cmd.as_str())
                        .collect::<Vec<&str>>()
                        .join(", ")
                )))
            }
        };

        if let Some(cmd) = cmd {
            match cmd {
                #[cfg(feature = "self-update")]
                "self" => update_self(),
                #[cfg(not(feature = "self-update"))]
                "self" => {
                    Err(BndBuilderError::AnyError(
                        "Self-update feature is not enabled in this build".to_string()
                    ))
                },
                "all" => update_all(true),
                "installed" => update_all(false),
                cmd => update_command(cmd, true)
            }
        }
        else {
            #[cfg(feature = "self-update")]
            let res = update_self();
            #[cfg(not(feature = "self-update"))]
            let res = Err(BndBuilderError::AnyError(
                "Self-update feature is not enabled in this build".to_string()
            ));
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

        fs_err::remove_dir_all(folder)
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
        let cmd = crate::build_args_parser().color(clap::ColorChoice::Always);

        match cmd.clone().try_get_matches() {
            Result::Ok(matches) => Ok(Some(Self::from_matches(matches))),
            Result::Err(e) => {
                match e.kind() {
                    clap::error::ErrorKind::DisplayHelp
                    | clap::error::ErrorKind::DisplayVersion => {
                        e.print().ok();
                        Ok(None)
                    },
                    _ => Err(e)
                }
            },
        }
    }

    pub fn from_matches(matches: ArgMatches) -> Self {
        let should_not_display_cpu = [
            "help",
            "version",
            "init",
            "update",
            "clear",
            "direct",
            "completion"
        ]
        .into_iter()
        .any(|id| matches.contains_id(id));

        #[cfg(feature = "rayon")]
        let force_serial = matches.get_flag("serial");
        #[cfg(feature = "rayon")]
        if !should_not_display_cpu {
            use cpclib_common::rayon;
            let num_cpus = rayon::current_num_threads();

            if force_serial {
                eprintln!(
                    "--> Forcing serial execution of bndbuild. Other tools still have access to {} threads\n",
                    num_cpus
                );
            }
            else if num_cpus != 1 {
                eprintln!("--> Using {} threads for parallel execution\n", num_cpus);
            }
        }
        Self {
            matches,
            observers: Arc::new(Vec::with_capacity(1).into()),
            #[cfg(feature = "rayon")]
            force_serial
        }
    }

    /// Return the build file passed via `-f / --file`, if any.
    pub fn build_file(&self) -> Option<&str> {
        self.matches.get_one::<String>("file").map(|s| s.as_str())
    }

    /// Return the profile output path passed via `--profile`, if any.
    pub fn profile_output(&self) -> Option<&str> {
        self.matches.get_one::<String>("profile").map(|s| s.as_str())
    }

    pub fn add_observer<O: Into<BndBuilderObserverRc>>(&mut self, o: O) {
        Arc::get_mut(&mut self.observers)
            .expect("Failed to get mutable reference to observers (multiple Arc references exist)")
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
                    matches
                        .get_one::<String>("help")
                        .expect("'help' argument is required by clap configuration")
                        .clone()
                ));
            }
            else if matches.get_flag("version") {
                return Ok(BndBuilderCommandInner::Version);
            }
            else if matches.get_flag("init") {
                return Ok(BndBuilderCommandInner::Init);
            }
            else if cfg!(feature = "self-update") && matches.contains_id("update") {
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
                return Ok(BndBuilderCommandInner::Direct(
                    cmd,
                    matches.get_flag("with_expansion")
                ));
            }
            else if let Some(generator) = matches.get_one::<Shell>("completion").copied() {
                return Ok(BndBuilderCommandInner::GenerateCompletion(generator));
            }
            else if matches.contains_id("completion") {
                // Shell not specified, auto-detect using query-shell
                let detected_shell = query_shell::Shell::get().map_err(|e| {
                    BndBuilderError::AnyError(format!("Failed to detect shell: {}", e))
                })?;

                let shell = match detected_shell {
                    query_shell::Shell::Bash => Shell::Bash,
                    query_shell::Shell::Zsh => Shell::Zsh,
                    query_shell::Shell::Fish => Shell::Fish,
                    query_shell::Shell::Powershell => Shell::PowerShell,
                    query_shell::Shell::Elvish => Shell::Elvish,
                    _ => {
                        return Err(BndBuilderError::AnyError(format!(
                            "Detected shell {:?} is not supported for completion generation",
                            detected_shell
                        )));
                    }
                };

                return Ok(BndBuilderCommandInner::GenerateCompletion(shell));
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
                        && EXPECTED_FILENAMES.iter().any(|end| fname.ends_with(*end))
                    {
                        error_msg.push_str(&format!("\nHave you forgotten to do \"-f {fname}\" ?"));
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

            // Capture the launch CWD before get_buildfile_content may change it
            // via decode_from_reader -> set_current_dir. This lets us absolutize
            // any user-supplied output paths (dot, etc.) correctly.
            let launch_cwd = std::env::current_dir().ok();

            let content = self.get_buildfile_content(fname)?;

            if matches.get_flag("show") {
                return Ok(BndBuilderCommandInner::Show(
                    content,
                    matches.get_flag("numbered")
                ));
            }

            let mut builder = BndBuilder::from_string(
                content,
                Some(fname.as_ref()),
                #[cfg(feature = "rayon")]
                self.force_serial
            )?;
            for observer in self.observers.iter() {
                builder.add_observer(observer.clone());
            }

            if let Some(add) = matches.get_one::<String>("add") {
                let dependencies = matches
                    .get_many::<String>("dep")
                    .map(|l| l.cloned().collect_vec())
                    .unwrap_or_default();

                let kind = matches
                    .get_one::<String>("kind")
                    .expect("'kind' argument is required by clap configuration");
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
                let graph_details = matches.get_flag("graph_details");
                let include_deps = matches.get_flag("dot_dependencies");
                // Absolutize the source build file path so execute_dot can
                // resolve cross-file BndBuild references correctly.
                let source_file_abs: Option<Utf8PathBuf> = {
                    let p = fname.as_std_path();
                    let abs_str = if p.is_absolute() {
                        fname.as_str().to_owned()
                    } else if let Some(ref cwd) = launch_cwd {
                        cwd.join(p).to_string_lossy().into_owned()
                    } else {
                        fname.as_str().to_owned()
                    };
                    Some(Utf8PathBuf::from(abs_str))
                };
                if let Some(g) = matches.get_one::<String>("dot") {
                    // Resolve relative dot output paths against the launch CWD
                    // (not the build-file dir that set_current_dir may have set).
                    let g_abs = {
                        let p = std::path::Path::new(g.as_str());
                        if p.is_absolute() {
                            g.to_owned()
                        } else if let Some(ref cwd) = launch_cwd {
                            cwd.join(p).to_string_lossy().into_owned()
                        } else {
                            g.to_owned()
                        }
                    };
                    Ok(BndBuilderCommandInner::Dot(
                        builder,
                        Some(g_abs),
                        graph_details,
                        include_deps,
                        source_file_abs
                    ))
                }
                else {
                    Ok(BndBuilderCommandInner::Dot(builder, None, graph_details, include_deps, source_file_abs))
                }
            }
            else {
                // Get the targets
                let targets = matches
                    .get_many::<String>("target")
                    .map(|targets_provided| {
                        targets_provided
                            .cloned()
                            .map(|s| {
                                Utf8PathBuf::from_str(&s)
                                    .expect("Clap-provided strings should be valid UTF-8 paths")
                            })
                            .collect::<Vec<Utf8PathBuf>>()
                    });

                let watch_requested = matches.get_flag("watch");

                Ok(BndBuilderCommandInner::Build {
                    targets,
                    watch: if watch_requested {
                        WatchState::WatchFirstRound
                    }
                    else {
                        WatchState::NoWatch
                    },
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
