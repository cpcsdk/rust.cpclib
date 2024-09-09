use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};

use camino::Utf8Path;
use cpclib_runner::event::EventObserver;

use crate::task::Task;

#[derive(Clone)]
pub enum BndBuilderState<'a> {
    ComputeDependencies(&'a Utf8Path),
    RunTasks,
    Finish
}

#[derive(Clone)]
pub enum BndBuilderEvent<'a> {
    ChangeState(BndBuilderState<'a>),
    StartRule {
        rule: &'a Utf8Path,
        nb: usize,
        out_of: usize
    },
    StopRule(&'a Utf8Path),
    FailedRule(&'a Utf8Path),
    StartTask(Option<&'a Utf8Path>, &'a Task),
    StopTask(Option<&'a Utf8Path>, &'a Task, Duration),
    TaskStdout(&'a Utf8Path, &'a Task, &'a str),
    TaskStderr(&'a Utf8Path, &'a Task, &'a str),
    Stdout(&'a str),
    Stderr(&'a str)
}

pub trait BndBuilderObserver {
    fn update(&mut self, event: BndBuilderEvent);
}

pub trait BndBuilderObserved {
    #[inline]
    fn emit_stdout<S: AsRef<str>>(&self, s: S) {
        self.notify(BndBuilderEvent::Stdout(s.as_ref()))
    }
    #[inline]
    fn emit_stderr<S: AsRef<str>>(&self, s: S) {
        self.notify(BndBuilderEvent::Stderr(s.as_ref()))
    }
    #[inline]
    fn emit_task_stdout<S: AsRef<str>, P: AsRef<Utf8Path>>(&self, p: P, t: &Task, s: S) {
        self.notify(BndBuilderEvent::TaskStdout(p.as_ref(), t, s.as_ref()))
    }
    #[inline]
    fn emit_task_stderr<S: AsRef<str>, P: AsRef<Utf8Path>>(&self, p: P, t: &Task, s: S) {
        self.notify(BndBuilderEvent::TaskStderr(p.as_ref(), t, s.as_ref()))
    }
    #[inline]
    fn do_compute_dependencies<P: AsRef<Utf8Path>>(&self, p: P) {
        self.notify(BndBuilderEvent::ChangeState(
            BndBuilderState::ComputeDependencies(p.as_ref())
        ))
    }
    #[inline]
    fn do_run_tasks(&self) {
        self.notify(BndBuilderEvent::ChangeState(BndBuilderState::RunTasks))
    }
    #[inline]
    fn do_finish(&self) {
        self.notify(BndBuilderEvent::ChangeState(BndBuilderState::Finish))
    }
    #[inline]
    fn start_rule<P: AsRef<Utf8Path>>(&self, rule: P, nb: usize, out_of: usize) {
        self.notify(BndBuilderEvent::StartRule {
            rule: rule.as_ref(),
            nb,
            out_of
        })
    }
    #[inline]
    fn stop_rule<P: AsRef<Utf8Path>>(&self, task: P) {
        self.notify(BndBuilderEvent::StopRule(task.as_ref()))
    }
    #[inline]
    fn failed_rule<P: AsRef<Utf8Path>>(&self, task: P) {
        self.notify(BndBuilderEvent::FailedRule(task.as_ref()))
    }
    #[inline]
    fn start_task(&self, rule: Option<&Utf8Path>, task: &Task) {
        self.notify(BndBuilderEvent::StartTask(rule, task))
    }
    #[inline]
    fn stop_task(&self, rule: Option<&Utf8Path>, task: &Task, duration: Duration) {
        self.notify(BndBuilderEvent::StopTask(rule, task, duration))
    }
    #[inline]
    fn notify(&self, event: BndBuilderEvent<'_>) {
        for observer in self.observers() {
            observer
                .upgrade()
                .unwrap()
                .borrow_mut()
                .update(event.clone());
        }
    }
    fn observers(&self) -> &[BndBuilderObserverWeak];
    fn add_observer(&mut self, observer: BndBuilderObserverWeak);
}

#[derive(Clone)]
pub struct BndBuilderObserverStrong(Rc<RefCell<Box<dyn BndBuilderObserver>>>);
pub struct BndBuilderObserverWeak(Weak<RefCell<Box<dyn BndBuilderObserver>>>);
pub struct ListOfBndBuilderObserverStrong(Vec<BndBuilderObserverWeak>);

impl From<&BndBuilderObserverStrong> for BndBuilderObserverWeak {
    fn from(val: &BndBuilderObserverStrong) -> Self {
        val.downgrade()
    }
}
impl BndBuilderObserverStrong {
    pub fn new<O: BndBuilderObserver + Sized + 'static>(observer: O) -> Self {
        let observer = Box::new(observer);
        let observer = observer as Box<dyn BndBuilderObserver>;
        let observer = Rc::new(RefCell::new(observer));
        Self(observer)
    }

    pub fn new_default() -> Self {
        let observer = BndBuilderDefaultObserver::default();
        Self::new(observer)
    }

    pub fn downgrade(&self) -> BndBuilderObserverWeak {
        BndBuilderObserverWeak(Rc::downgrade(&self.0))
    }
}

impl Deref for BndBuilderObserverStrong {
    type Target = Rc<RefCell<Box<dyn BndBuilderObserver>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BndBuilderObserverStrong {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl BndBuilderObserverWeak {
    pub fn new<O: BndBuilderObserver + Sized + 'static>(
        observer: O
    ) -> (Rc<RefCell<Box<dyn BndBuilderObserver>>>, Self) {
        let observer = Box::new(observer);
        let observer = observer as Box<dyn BndBuilderObserver>;
        let observer = Rc::new(RefCell::new(observer));

        let weak = Rc::downgrade(&observer);

        (observer, Self(weak))
    }
}

impl Deref for BndBuilderObserverWeak {
    type Target = Weak<RefCell<Box<dyn BndBuilderObserver>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Vec<BndBuilderObserverWeak>> for ListOfBndBuilderObserverStrong {
    fn from(value: Vec<BndBuilderObserverWeak>) -> Self {
        Self(value)
    }
}

impl Deref for ListOfBndBuilderObserverStrong {
    type Target = Vec<BndBuilderObserverWeak>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Clone for ListOfBndBuilderObserverStrong {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl DerefMut for ListOfBndBuilderObserverStrong {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for ListOfBndBuilderObserverStrong {
    fn default() -> Self {
        Self(Vec::with_capacity(1))
    }
}

impl Clone for BndBuilderObserverWeak {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl BndBuilderObserved for ListOfBndBuilderObserverStrong {
    fn observers(&self) -> &[BndBuilderObserverWeak] {
        &self.0
    }

    fn add_observer(&mut self, observer: BndBuilderObserverWeak) {
        self.0.push(observer)
    }
}

impl EventObserver for ListOfBndBuilderObserverStrong {
    fn emit_stdout<S: Into<String>>(&self, s: S) {
        <Self as BndBuilderObserved>::emit_stdout(self, s.into())
    }

    fn emit_stderr<S: Into<String>>(&self, s: S) {
        <Self as BndBuilderObserved>::emit_stderr(self, s.into())
    }
}

pub struct RuleTaskEventDispatcher<'observed, 'rule, 'task, E>
where E: BndBuilderObserved
{
    observed: &'observed E,
    rule: Option<&'rule Utf8Path>,
    task: &'task Task,
    start: std::time::Instant
}

impl<'observed, 'rule, 'task, E> Drop for RuleTaskEventDispatcher<'observed, 'rule, 'task, E>
where E: BndBuilderObserved
{
    fn drop(&mut self) {
        self.observed
            .stop_task(self.rule, self.task, std::time::Instant::now() - self.start);
    }
}

impl<'observed, 'rule, 'task, E> RuleTaskEventDispatcher<'observed, 'rule, 'task, E>
where E: BndBuilderObserved
{
    #[inline]
    pub fn new(builder: &'observed E, rule: &'rule Utf8Path, task: &'task Task) -> Self {
        builder.start_task(Some(rule), task);
        Self {
            observed: builder,
            rule: Some(rule),
            task,
            start: Instant::now()
        }
    }

    #[inline]
    pub fn new_alone(observed: &'observed E, task: &'task Task) -> Self {
        observed.start_task(None, task);
        Self {
            observed,
            rule: None,
            task,
            start: Instant::now()
        }
    }
}

impl<'builder, 'rule, 'task, E> EventObserver for RuleTaskEventDispatcher<'builder, 'rule, 'task, E>
where E: BndBuilderObserved
{
    #[inline]
    fn emit_stdout<S: Into<String>>(&self, s: S) {
        match self.rule {
            Some(rule) => self.observed.emit_task_stdout(rule, self.task, s.into()),
            None => self.observed.emit_stdout(s.into())
        }
    }

    #[inline]
    fn emit_stderr<S: Into<String>>(&self, s: S) {
        match self.rule {
            Some(rule) => self.observed.emit_task_stderr(rule, self.task, s.into()),
            None => self.observed.emit_stderr(s.into())
        }
    }
}

#[derive(Default)]
pub struct BndBuilderDefaultObserver {}

impl BndBuilderDefaultObserver {
    pub fn new() -> Rc<RefCell<Box<dyn BndBuilderObserver>>> {
        Rc::new(RefCell::new(Box::new(BndBuilderDefaultObserver {})))
    }
}

impl BndBuilderObserver for BndBuilderDefaultObserver {
    fn update(&mut self, event: BndBuilderEvent) {
        match event {
            BndBuilderEvent::ChangeState(s) => {
                match s {
                    BndBuilderState::ComputeDependencies(p) => {
                        println!("> Compute dependencies for rule `{p}`")
                    },
                    BndBuilderState::RunTasks => println!("> Execute tasks"),
                    BndBuilderState::Finish => println!("> Done.")
                }
            },
            BndBuilderEvent::StartRule { rule, nb, out_of } => {
                println!("[{nb}/{out_of}] Handle {rule}")
            },
            BndBuilderEvent::StopRule(_) => {},
            BndBuilderEvent::FailedRule(_) => todo!(),
            BndBuilderEvent::StartTask(r, t) => {
                println!("\t$ {}", t);
            },
            BndBuilderEvent::StopTask(r, t, d) => {
                println!(
                    "\tElapsed time: {}",
                    fancy_duration::FancyDuration(d).truncate(1)
                )
            },
            BndBuilderEvent::TaskStdout(tgt, task, txt) => {
                for line in txt.lines() {
                    println!("[{tgt}]\t{line}")
                }
            },
            BndBuilderEvent::TaskStderr(tgt, task, txt) => {
                for line in txt.lines() {
                    eprintln!("[{tgt}]\t{line}")
                }
            },
            BndBuilderEvent::Stdout(s) => println!("{s}"),
            BndBuilderEvent::Stderr(s) => println!("{s}")
        }
    }
}
