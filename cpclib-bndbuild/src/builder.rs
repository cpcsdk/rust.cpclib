use std::collections::HashSet;
use std::fmt::Debug;
use std::io::{BufReader, Read};
use std::ops::Deref;
use std::sync::Arc;

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::itertools::Itertools;
use minijinja::{context, Environment, Error, ErrorKind};

use crate::event::{
    BndBuilderObserved, BndBuilderObserver, BndBuilderObserverRc, ListOfBndBuilderObserverRc,
    RuleTaskEventDispatcher
};
use crate::rules::{self, Graph, Rule};
use crate::task::Task;
use crate::BndBuilderError;

pub const EXPECTED_FILENAMES: &[&str] = &["bndbuild.yml", "build.bnd", "bnd.build"];

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
    observers: Arc<ListOfBndBuilderObserverRc>
}

impl Debug for BndBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BndBuilder").finish()
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
            observers: Default::default()
        }
    }

    pub fn from_path<P: AsRef<Utf8Path>>(fname: P) -> Result<(Utf8PathBuf, Self), BndBuilderError> {
        let (p, content) = Self::decode_from_fname(fname)?;
        Self::from_string(content).map(|build| (p, build))
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
        Self::decode_from_reader(rdr, working_directory, definitions).map(|s| (fname.to_owned(), s))
    }

    pub fn save<P: AsRef<Utf8Path>>(&self, path: P) -> std::io::Result<()> {
        let contents = self.inner.borrow_owner().to_string();
        std::fs::write(path.as_ref(), contents)
    }

    pub fn decode_from_reader<P: AsRef<Utf8Path>, S1: AsRef<str>, S2: AsRef<str>>(
        mut rdr: impl Read,
        working_directory: Option<P>,
        definitions: &[(S1, S2)]
    ) -> Result<String, BndBuilderError> {

        // XXX here it is problematic to modify work dir
        if let Some(working_directory) = working_directory {
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
            .map_err(|e| {
                BndBuilderError::AnyError(e.to_string())
            })?;

        // apply jinja templating
        let mut env = Environment::new();
        fn error(error: String) -> Result<String, Error> {
            Err(Error::new(ErrorKind::InvalidOperation, error))
        }

        pub fn path_loader<'x, P: AsRef<std::path::Path> + 'x>(
            dir: P
        ) -> impl for<'a> Fn(&'a str) -> Result<Option<String>, Error> + Send + Sync + 'static
        {
            let dir = dir.as_ref().to_path_buf();
            move |name| {
                let path = dir.join(name); // TODO add a safety ??
                match std::fs::read_to_string(path) {
                    Ok(result) => Ok(Some(result)),
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
                    Err(err) => {
                        Err(
                            Error::new(ErrorKind::InvalidOperation, "could not read template")
                                .with_source(err)
                        )
                    },
                }
            }
        }

        env.set_loader(path_loader(std::env::current_dir().unwrap()));
        env.add_function("fail", error);
        for (key, value) in definitions {
            let key = key.as_ref();
            let value = value.as_ref();
            env.add_global(key, value);
        }
        env.render_str(&content, context!())
            .map_err(|e| {
                BndBuilderError::AnyError(e.to_string())
            })
    }

    pub fn from_string(content: String) -> Result<Self, BndBuilderError> {
        // extract information from the file
        let rules: rules::Rules =
            serde_yaml::from_str(&content).map_err(BndBuilderError::ParseError)?;

        let inner = BndBuilderInner::try_new(rules, |rules| rules.to_deps())?;

        Ok(BndBuilder {
            inner,
            observers: Default::default()
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

        if state.nb_deps == 0 {
            if self.has_rule(p) {
                self.do_run_tasks();
                state.nb_deps = 1;
                self.execute_rule(p, &mut state)?;
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
            for layer in layers.into_iter() {
                self.execute_layer(layer, &mut state)?;
            }
        }
        self.do_finish();

        Ok(())
    }

    fn execute_layer(
        &self,
        layer: HashSet<&Utf8Path>,
        state: &mut ExecutionState
    ) -> Result<(), BndBuilderError> {
        layer
            .into_iter()
            .map(|p| self.execute_rule(p, state))
            .collect::<Result<Vec<()>, BndBuilderError>>()?;
        Ok(())
    }

    fn execute_rule<'s, P: AsRef<Utf8Path> + 's>(
        &'s self,
        p: P,
        state: &mut ExecutionState
    ) -> Result<(), BndBuilderError> {
        let p = p.as_ref();

        let p: &'static Utf8Path = unsafe { std::mem::transmute(p) };
        let this: &'static Self = unsafe { std::mem::transmute(self) };

        state.task_count += 1;

        self.start_rule(p, state.task_count, state.nb_deps);

        if let Some(rule) = this.rule(p) {
            if !rule.is_enabled() {
                return Err(BndBuilderError::DisabledTarget(p.to_string()));
            }

            let done = rule.is_up_to_date();
            if done {
                self.emit_stdout("Rule {p} already exists\n");
                // nothing to do
            }
            else {
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
    pub fn outdated<P: AsRef<Utf8Path>>(&self, target: P) -> Result<bool, BndBuilderError> {
        self.inner.borrow_dependent().outdated(target, true)
    }

    #[inline]
    pub fn get_layered_dependencies(&self) -> Vec<HashSet<&Utf8Path>> {
        self.inner.borrow_dependent().get_layered_dependencies()
    }

    #[inline]
    pub fn get_layered_dependencies_for<'a, P: AsRef<Utf8Path>>(
        &'a self,
        p: &'a P
    ) -> Vec<HashSet<&'a Utf8Path>> {
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
