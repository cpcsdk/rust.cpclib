use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{BufReader, Read};
use std::ops::Deref;
use std::sync::Arc;
#[cfg(feature = "rayon")]
use std::sync::RwLock;

use anstyle::{self, RgbColor};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term::termcolor::Buffer;
use codespan_reporting::term::{Chars, emit};
use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::itertools::Itertools;
#[cfg(feature = "rayon")]
use cpclib_common::rayon::iter::{ParallelBridge, ParallelIterator};
use minijinja::context;

use crate::BndBuilderError;
use crate::app::WatchState;
use crate::env::create_template_env;
use crate::event::{
    BndBuilderObserved, BndBuilderObserver, BndBuilderObserverRc, ListOfBndBuilderObserverRc,
    RuleTaskEventDispatcher
};
use crate::rules::{self, Graph, Rule};
use crate::task::Task;

pub const EXPECTED_FILENAMES: &[&str] = &[
    "bndbuild.yml",
    "build.bnd",
    "bnd.build",
    "BNDBUILD.YML",
    "BUILD.BND",
    "BND.BUILD" // ACE fuck up by uppercasing files
];

#[derive(Default)]
struct ExecutionState {
    nb_deps: usize,
    task_count: usize
}

self_cell::self_cell! {
    /// WARNING the BndBuilder changes the current working directory.
    /// This is probably a problematic behavior. Need to think about it later
    struct BndBuilderInner {
        owner: rules::Rules,
        #[covariant]
        dependent: Graph,
    }
}

pub struct BndBuilder {
    inner: BndBuilderInner,
    observers: Arc<ListOfBndBuilderObserverRc>,
    #[cfg(feature = "rayon")]
    force_serial: bool,
}

impl Debug for BndBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("BndBuilder");
        #[cfg(feature = "rayon")]
        dbg.field("force_serial", &self.force_serial);
        dbg.finish()
    }
}

impl Deref for BndBuilder {
    type Target = rules::Rules;

    fn deref(&self) -> &Self::Target {
        self.inner.borrow_owner()
    }
}

impl BndBuilder {
    fn task_observer<'b, 'r, 't>(
        &'b self,
        rule: &'r Utf8Path,
        task: &'t Task
    ) -> Arc<Box<RuleTaskEventDispatcher<'b, 'r, 't, Self>>> {
        Arc::new(Box::new(RuleTaskEventDispatcher::new(self, rule, task)))
    }

    pub fn add_default_rule<S1, S2>(self, targets: &[S1], dependencies: &[S2], kind: &str) -> Self
    where
        S1: AsRef<str>,
        S2: AsRef<str>
    {
        let rule = Rule::new_default(targets, dependencies, kind);
        let mut rules = self.inner.into_owner();
        rules.add(rule);

        let inner = BndBuilderInner::try_new(rules, |rules| rules.to_deps()).unwrap();
        BndBuilder {
            inner,
            observers: Default::default(),
            #[cfg(feature = "rayon")]
            force_serial: self.force_serial,
        }
    }

    pub fn from_path<P: AsRef<Utf8Path>>(fname: P, #[cfg(feature = "rayon")] force_serial: bool) -> Result<(Utf8PathBuf, Self), BndBuilderError> {
        let (p, content) = Self::decode_from_fname(fname)?;
        Self::from_string(content, Some(p.as_ref()), #[cfg(feature = "rayon")] force_serial).map(|build| (p, build))
    }

    pub fn decode_from_fname<P: AsRef<Utf8Path>>(
        fname: P
    ) -> Result<(Utf8PathBuf, String), BndBuilderError> {
        Self::decode_from_fname_with_definitions(fname, &Vec::<(String, String)>::new())
    }

    pub fn decode_from_fname_with_definitions<
        P: AsRef<Utf8Path>,
        S1: AsRef<str>,
        S2: AsRef<str>
    >(
        fname: P,
        definitions: &[(S1, S2)]
    ) -> Result<(Utf8PathBuf, String), BndBuilderError> {
        let fname = fname.as_ref();

        // when a folder is provided try to look for a build file
        let fname = if fname.is_dir() {
            let mut selected = fname.to_owned();
            for extra in EXPECTED_FILENAMES {
                let tentative = fname.join(extra);
                if tentative.is_file() {
                    selected = tentative;
                    break;
                }
            }
            selected
        }
        else {
            fname.to_owned()
        };
        let fname = fname.as_path();

        let file = std::fs::File::open(fname).map_err(|e| {
            BndBuilderError::InputFileError {
                fname: fname.to_string(),
                error: e
            }
        })?;

        let path = Utf8Path::new(fname).parent().unwrap();
        let working_directory = if path.is_dir() { Some(path) } else { None };

        let rdr = BufReader::new(file);
        // Pass the filename as a string to decode_from_reader
        Self::decode_from_reader(rdr, working_directory, definitions, path)
            .map(|s| (fname.to_owned(), s))
    }

    pub fn save<P: AsRef<Utf8Path>>(&self, path: P) -> std::io::Result<()> {
        let contents = self.inner.borrow_owner().to_string();
        std::fs::write(path.as_ref(), contents)
    }

    pub fn decode_from_reader<P: AsRef<Utf8Path>, S1: AsRef<str>, S2: AsRef<str>>(
        mut rdr: impl Read,
        working_directory: Option<P>,
        definitions: &[(S1, S2)],
        filename: &Utf8Path
    ) -> Result<String, BndBuilderError> {
        // XXX here it is problematic to modify work dir
        if let Some(working_directory) = &working_directory {
            let working_directory = working_directory.as_ref();
            std::env::set_current_dir(working_directory).map_err(|e| {
                BndBuilderError::WorkingDirectoryError {
                    fname: working_directory.to_string(),
                    error: e
                }
            })?;
        }

        // get the content of the file
        let mut content = Default::default();
        rdr.read_to_string(&mut content)
            .map_err(|e| BndBuilderError::AnyError(e.to_string()))?;

        // apply jinja templating
        let env = create_template_env(working_directory.as_ref(), definitions);

        // generate the template
        env.render_str(&content, context!()).map_err(|e| {
            let src = e.template_source().unwrap();
            let range = e.range().unwrap();
            let message = e.detail().unwrap();

            // Use the provided filename for SimpleFile
            let file = SimpleFile::new(filename, src);
            let diagnostic = Diagnostic::error()
                .with_message(e.kind().to_string())
                .with_labels(vec![Label::primary((), range).with_message(message)]);
            let mut rendered = Vec::new();
            {
                let (config, mut buffer) = if cfg!(feature = "colored_errors") {
                    (codespan_reporting::term::Config::default(), Buffer::ansi())
                }
                else {
                    let mut conf = codespan_reporting::term::Config::default();
                    conf.chars = Chars::ascii();
                    (conf, Buffer::no_color())
                };
                let _ = emit(&mut buffer, &config, &file, &diagnostic);
                rendered = buffer.into_inner();
            }
            let report = String::from_utf8_lossy(&rendered).to_string();

            BndBuilderError::TemplateError(report)
        })
    }

    pub fn from_string(content: String, filename: Option<&Utf8Path>, #[cfg(feature = "rayon")] force_serial: bool) -> Result<Self, BndBuilderError> {
        // extract information from the file
        let mut rules: rules::Rules = serde_yaml::from_str(&content)
            .map_err(|e: serde_yaml::Error| BndBuilderError::from((e, filename.unwrap_or_else(|| Utf8Path::new("<string>")), content.as_str())))?;


        // force --serial argument in bndbuild tasks if required
        #[cfg(feature = "rayon")]
        {
            if force_serial {
                rules.iter_mut().for_each(|r| 
                    r.commands_mut().iter_mut()
                        .for_each(|c| {
                            use crate::task::InnerTask;

                            if let InnerTask::BndBuild(args) = &mut c.inner {
                                if ! args.args.contains("--serial") {
                                    // BUG --serial is detected even if not part of bndbuild arguments
                                    args.args = format!("--serial {}", args.args);
                                }
                            }
                        })
                );
            }
        }

        let inner = BndBuilderInner::try_new(rules, |rules| rules.to_deps())?;

        Ok(BndBuilder {
            inner,
            observers: Default::default(),
            #[cfg(feature = "rayon")]
            force_serial: force_serial,
        })
    }

    /// Return the default target if any
    pub fn default_target(&self) -> Option<&Utf8Path> {
        self.inner.borrow_owner().default_target()
    }
}

impl BndBuilder {
    /// Execute the target after all its predecessors
    pub fn execute<P: AsRef<Utf8Path>>(&self, target: P) -> Result<(), BndBuilderError> {
        let p = target.as_ref();

        self.do_compute_dependencies(p);
        let layers = self.get_layered_dependencies_for(&p);

        let mut state = ExecutionState {
            nb_deps: layers.iter().map(|l| l.len()).sum::<usize>(),
            task_count: 0
        };

        let nb_deps = state.nb_deps;

        #[cfg(feature = "rayon")]
        let state = Arc::new(RwLock::new(state));

        #[cfg(not(feature = "rayon"))]
        let state = &mut state;

        if nb_deps == 0 {
            if self.has_rule(p) {
                self.do_run_tasks();
                {
                    #[cfg(feature = "rayon")]
                    let mut state = state.write().unwrap();
                    state.nb_deps = 1;
                }
                self.execute_rule(p, state)?;
            }
            else {
                return Err(BndBuilderError::ExecuteError {
                    fname: p.to_string(),
                    msg: "no rule to build it".to_owned()
                });
            }
        }
        else {
            self.do_run_tasks();
            for layer in layers.iter() {
                // Each layer is TaskTargetsForLayer, which contains a set of TaskTargets
                self.execute_layer(
                    layer,
                    #[cfg(feature = "rayon")]
                    state.clone(),
                    #[cfg(not(feature = "rayon"))]
                    state
                )?;
            }
        }
        self.do_finish();

        Ok(())
    }

    fn execute_layer(
        &self,
        layer: &crate::rules::graph::TaskTargetsForLayer,
        #[cfg(not(feature = "rayon"))] state: &mut ExecutionState,
        #[cfg(feature = "rayon")] state: Arc<RwLock<ExecutionState>>
    ) -> Result<(), BndBuilderError> {

        // Store the files without rules. They are most probably existing files
        let mut without_rule = Vec::new();

        // get the rule of the expected targets
        let mut parallel_tasks: HashMap<&Rule, &crate::rules::graph::TaskTargets> = HashMap::default();
        let mut serial_tasks: HashMap<&Rule, &crate::rules::graph::TaskTargets> = HashMap::default();
        for task_targets in &layer.tasks {
            let repr = task_targets.representative_target();
            if let Some(r) = self.get_rule(repr) {

                #[cfg(feature = "rayon")]
                let parallelisze = r.is_parallelizable() && !self.force_serial;
                #[cfg(not(feature = "rayon"))]
                let parallelisze = r.is_parallelizable();

                if parallelisze {
                    parallel_tasks.insert(r, task_targets);
                }
                else {
                    serial_tasks.insert(r, task_targets);
                }
            }
            else {
                // If no rule for the representative target, push the whole TaskTargets reference
                without_rule.push(task_targets);
            }
        }


        // count the files that are not produced
        for targets in without_rule.into_iter() {
            #[cfg(feature = "rayon")]
            let mut state = state.write().unwrap();

            for p in targets.targets.iter() {
                state.task_count += 1;
                self.start_rule(p, state.task_count, state.nb_deps);
                if !p.exists() {
                    return Err(BndBuilderError::ExecuteError {
                        fname: p.to_string(),
                        msg: "no rule to build it".to_owned()
                    });
                }
                self.stop_rule(p);
            }
        }


        // Helper closure to execute a group and collect errors from any iterator
        macro_rules! launch_tasks {
            ($iter: expr) => {
                $iter.map(|task_targets| {
                    self.execute_task_targets_group(
                        task_targets,
                        #[cfg(feature = "rayon")]
                        state.clone(),
                        #[cfg(not(feature = "rayon"))]
                        state
                    )
                })
                .filter_map(Result::err)
                .collect::<Vec<BndBuilderError>>()
            }
        };

        // Serial tasks: always sequential
        let serial_errs = launch_tasks!(serial_tasks.values());

        // Parallel tasks: parallel if rayon, else sequential, or forced serial
        #[cfg(feature = "rayon")]
        let parallel_errs = {
            use cpclib_common::rayon::prelude::*;
            launch_tasks!(parallel_tasks.values().par_bridge())
        };
        #[cfg(not(feature = "rayon"))]
        let parallel_errs = launch_tasks!(parallel_tasks.values());

        let mut errs = serial_errs;
        errs.extend(parallel_errs);
        if !errs.is_empty() {
            let errs = errs.into_iter()
                .enumerate()
                .map(|(i, e)| 
                format!("Error {}:\n{}", 
                    i + 1,
                    e.to_string()
                ))
                .join("\n");
            return Err(BndBuilderError::AnyError(errs));
        }
        Ok(())
    }

    fn execute_task_targets_group(
        &self,
        task_targets: &crate::rules::graph::TaskTargets,
        #[cfg(not(feature = "rayon"))] state: &mut ExecutionState,
        #[cfg(feature = "rayon")] state: Arc<RwLock<ExecutionState>>
    ) -> Result<(), BndBuilderError> {
        let mut targets: Vec<_> = task_targets.targets.iter().collect();
        // Use representative_target as the main one
        let repr = task_targets.representative_target();
        // Remove the representative from the list to get the others
        targets.retain(|&&p| p != repr);
        let other_paths = if !targets.is_empty() {
            Some(targets)
        }
        else {
            None
        };

        if let Some(ps) = other_paths.as_ref() {
            ps.iter().for_each(|p| {
                #[cfg(feature = "rayon")]
                let mut state = state.write().unwrap();
                state.task_count += 1;
                self.start_rule(*p, state.task_count, state.nb_deps);
            });
        }
        let res = self.execute_rule(repr, state);
        if res.is_ok()
            && let Some(ps) = other_paths.as_ref()
        {
            ps.iter().for_each(|p| self.stop_rule(*p));
        }
        res
    }

    fn execute_rule<'s, P: AsRef<Utf8Path> + 's>(
        &'s self,
        p: P,
        #[cfg(not(feature = "rayon"))] state: &mut ExecutionState,
        #[cfg(feature = "rayon")] state: Arc<RwLock<ExecutionState>>
    ) -> Result<(), BndBuilderError> {
        let p = p.as_ref();

        let p: &'static Utf8Path = unsafe { std::mem::transmute(p) };
        let this: &'static Self = unsafe { std::mem::transmute(self) };

        {
            #[cfg(feature = "rayon")]
            let mut state = state.write().unwrap();
            state.task_count += 1;
            self.start_rule(p, state.task_count, state.nb_deps);
        }

        if let Some(rule) = this.rule(p) {
            let (disabled, done) = if rule.is_disabled() {
                self.emit_stderr(format!("The target {p} is disabled and ignored."));
                (true, true)
            }
            else {
                let done = rule.is_up_to_date(None, Some(p));
                if done {
                    self.emit_stdout(format!("Rule {p} already exists\n"));
                }
                (false, done)
            };

            if !done {
                // execute all the tasks for this rule
                for task in rule.commands() {
                    let task_observer = this.task_observer(p, task);
                    crate::execute(task, &task_observer).map_err(|e| {
                        BndBuilderError::ExecuteError {
                            fname: p.to_string(),
                            msg: e
                        }
                    })?;
                }
            }

            // check if all the targets have been created
            if !disabled && !rule.is_phony() {
                let wrong_files = rule.targets().iter().filter(|t| !t.exists()).join(" ");
                if !wrong_files.is_empty() {
                    let orange = anstyle::Style::new()
                        .fg_color(Some(anstyle::Color::Rgb(RgbColor(255, 165, 0))));
                    self.emit_stderr(

                            format!(
                                "{}The following target(s) have not been generated: {wrong_files}. There is probably an error in your build file.\n{}",  
                            
                            orange.render(),
                            orange.render_reset()
                            )
                    

                    );
                }
            }
        }
        else if !p.exists() {
            return Err(BndBuilderError::ExecuteError {
                fname: p.to_string(),
                msg: "no rule to build it".to_owned()
            });
        }
        else {
            self.emit_stdout(format!("\t{} is already up to date\n", &p));
        }

        self.stop_rule(p);

        Ok(())
    }
}

impl BndBuilder {
    #[inline]
    pub fn outdated<P: AsRef<Utf8Path>>(
        &self,
        watch: &WatchState,
        target: P
    ) -> Result<bool, BndBuilderError> {
        self.inner.borrow_dependent().outdated(target, watch, true)
    }

    #[inline]
    pub fn get_layered_dependencies(&self) -> crate::rules::graph::LayeredDependenciesByTask<'_> {
        self.inner.borrow_dependent().get_layered_dependencies()
    }

    #[inline]
    pub fn get_layered_dependencies_for<'a, P: AsRef<Utf8Path>>(
        &'a self,
        p: &'a P
    ) -> crate::rules::graph::LayeredDependenciesByTask<'a> {
        self.inner
            .borrow_dependent()
            .get_layered_dependencies_for(p)
    }

    #[inline]
    pub fn get_rule<P: AsRef<Utf8Path>>(&self, tgt: P) -> Option<&Rule> {
        self.inner.borrow_owner().rule(tgt)
    }

    #[inline]
    pub fn has_rule<P: AsRef<Utf8Path>>(&self, tgt: P) -> bool {
        self.get_rule(tgt).is_some()
    }

    #[inline]
    pub fn rules(&self) -> &[Rule] {
        self.inner.borrow_owner().rules()
    }

    pub fn targets(&self) -> Vec<&Utf8Path> {
        self.rules()
            .iter()
            .flat_map(|r| r.targets())
            .map(|p| p.as_path())
            .collect_vec()
    }
}

impl BndBuilderObserved for BndBuilder {
    fn observers(&self) -> Arc<ListOfBndBuilderObserverRc> {
        Arc::clone(&self.observers)
    }

    fn add_observer(&mut self, observer: BndBuilderObserverRc) {
        Arc::get_mut(&mut self.observers)
            .unwrap()
            .add_observer(observer);
    }

    fn emit_stdout<S: AsRef<str>>(&self, s: S) {
        self.notify(crate::event::BndBuilderEvent::Stdout(s.as_ref()))
    }

    fn emit_stderr<S: AsRef<str>>(&self, s: S) {
        self.notify(crate::event::BndBuilderEvent::Stderr(s.as_ref()))
    }

    fn emit_task_stdout<S: AsRef<str>, P: AsRef<Utf8Path>>(&self, p: P, t: &Task, s: S) {
        self.notify(crate::event::BndBuilderEvent::TaskStdout(
            p.as_ref(),
            t,
            s.as_ref()
        ))
    }

    fn emit_task_stderr<S: AsRef<str>, P: AsRef<Utf8Path>>(&self, p: P, t: &Task, s: S) {
        self.notify(crate::event::BndBuilderEvent::TaskStderr(
            p.as_ref(),
            t,
            s.as_ref()
        ))
    }

    fn do_compute_dependencies<P: AsRef<Utf8Path>>(&self, p: P) {
        self.notify(crate::event::BndBuilderEvent::ChangeState(
            crate::event::BndBuilderState::ComputeDependencies(p.as_ref())
        ))
    }

    fn do_run_tasks(&self) {
        self.notify(crate::event::BndBuilderEvent::ChangeState(
            crate::event::BndBuilderState::RunTasks
        ))
    }

    fn do_finish(&self) {
        self.notify(crate::event::BndBuilderEvent::ChangeState(
            crate::event::BndBuilderState::Finish
        ))
    }

    fn start_rule<P: AsRef<Utf8Path>>(&self, rule: P, nb: usize, out_of: usize) {
        self.notify(crate::event::BndBuilderEvent::StartRule {
            rule: rule.as_ref(),
            nb,
            out_of
        })
    }

    fn stop_rule<P: AsRef<Utf8Path>>(&self, task: P) {
        self.notify(crate::event::BndBuilderEvent::StopRule(task.as_ref()))
    }

    fn failed_rule<P: AsRef<Utf8Path>>(&self, task: P) {
        self.notify(crate::event::BndBuilderEvent::FailedRule(task.as_ref()))
    }

    fn start_task(&self, rule: Option<&Utf8Path>, task: &Task) {
        self.notify(crate::event::BndBuilderEvent::StartTask(rule, task))
    }

    fn stop_task(&self, rule: Option<&Utf8Path>, task: &Task, duration: std::time::Duration) {
        self.notify(crate::event::BndBuilderEvent::StopTask(
            rule, task, duration
        ))
    }

    fn notify(&self, event: crate::event::BndBuilderEvent<'_>) {
        for observer in self.observers.iter() {
            observer.write().unwrap().update(event.clone());
        }
    }
}
