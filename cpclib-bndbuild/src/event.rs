use std::cell::RefCell;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use camino::Utf8Path;
use cpclib_asm::EnvEventObserver;
use cpclib_runner::event::EventObserver;

use crate::task::Task;

#[derive(Clone, Debug)]
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

pub trait BndBuilderObserver: EventObserver + EnvEventObserver {
    fn update(&mut self, event: BndBuilderEvent);
}

impl<T: BndBuilderObserver> BndBuilderObserver for Box<T> {
    fn update(&mut self, event: BndBuilderEvent) {
        self.deref_mut().update(event)
    }
}

impl<T: BndBuilderObserver> BndBuilderObserver for Arc<T> {
    fn update(&mut self, event: BndBuilderEvent) {
        Arc::<T>::get_mut(self).unwrap().update(event)
    }
}

pub trait BndBuilderObserved: Debug + Sync + Send {
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
        let observers = self.observers().clone();
        for observer in observers.iter() {
            observer.update(event.clone());
        }
    }
    fn observers(&self) -> Arc<ListOfBndBuilderObserverRc>;
    fn add_observer(&mut self, observer: BndBuilderObserverRc);
}

#[derive(Debug, Clone)]
pub struct BndBuilderObserverRc(Arc<RwLock<dyn BndBuilderObserver + Sync + Send>>);
#[derive(Clone, Debug)]
pub struct ListOfBndBuilderObserverRc(Vec<BndBuilderObserverRc>);

impl BndBuilderObserverRc {
    pub fn new<O: BndBuilderObserver + Sized + 'static + Sync + Send>(observer: O) -> Self {
        let observer = RwLock::new(observer);
        let observer = Arc::new(observer);
        Self(observer)
    }

    pub fn new_default() -> Self {
        let observer = BndBuilderDefaultObserver::default();
        Self::new(observer)
    }

    pub fn update(&self, event: BndBuilderEvent) {
        self.0.deref().write().unwrap().deref_mut().update(event);
    }
}

impl ListOfBndBuilderObserverRc {
    pub fn iter(&self) -> ListOfBndBuilderObserverRcIter {
        ListOfBndBuilderObserverRcIter { list: self, idx: 0 }
    }

    pub fn add_observer(&mut self, o: BndBuilderObserverRc) {
        self.0.push(o);
    }
}

pub struct ListOfBndBuilderObserverRcIter<'l> {
    list: &'l ListOfBndBuilderObserverRc,
    idx: usize
}

impl Iterator for ListOfBndBuilderObserverRcIter<'_> {
    type Item = BndBuilderObserverRc;

    fn next(&mut self) -> Option<Self::Item> {
        let vec = self.list.0.deref();
        if self.idx == vec.len() {
            None
        }
        else {
            let res = vec.get(self.idx).cloned();
            self.idx += 1;
            res
        }
    }
}

impl Deref for BndBuilderObserverRc {
    type Target = Arc<RwLock<dyn BndBuilderObserver + Sync + Send>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BndBuilderObserverRc {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<BndBuilderObserverRc>> for ListOfBndBuilderObserverRc {
    fn from(value: Vec<BndBuilderObserverRc>) -> Self {
        Self(value)
    }
}

impl Deref for ListOfBndBuilderObserverRc {
    type Target = Vec<BndBuilderObserverRc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ListOfBndBuilderObserverRc {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for ListOfBndBuilderObserverRc {
    fn default() -> Self {
        Self(Vec::with_capacity(1))
    }
}

impl EventObserver for ListOfBndBuilderObserverRc {
    fn emit_stdout(&self, s: &str) {
        for observer in self.0.clone().into_iter() {
            observer.0.deref().read().unwrap().emit_stdout(s);
        }
    }

    fn emit_stderr(&self, s: &str) {
        for observer in self.0.clone().into_iter() {
            observer.0.deref().read().unwrap().emit_stderr(s);
        }
    }
}

impl BndBuilderObserver for ListOfBndBuilderObserverRc {
    fn update(&mut self, event: BndBuilderEvent) {
        for observer in self.0.clone().into_iter() {
            let mut observer = observer.0.deref().write().unwrap();
            observer.update(event.clone());
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuleTaskEventDispatcher<'observed, 'rule, 'task, E>
where E: BndBuilderObserved
{
    observed: &'observed E,
    rule: Option<&'rule Utf8Path>,
    task: &'task Task,
    start: std::time::Instant
}

// impl<'observed, 'rule, 'task, E: BndBuilderObserved> Into<ListOfBndBuilderObserverRc> for RuleTaskEventDispatcher<'observed, 'rule, 'task, E>
// where E: BndBuilderObserved
// {
// fn into(self) -> ListOfBndBuilderObserverRc {
// let mut list = ListOfBndBuilderObserverRc::default();
// list.add_observer(self.into());
// list
// }
// }

// impl<'observed, 'rule, 'task, E: BndBuilderObserved> Into<BndBuilderObserverRc> for RuleTaskEventDispatcher<'observed, 'rule, 'task, E>
// where E: BndBuilderObserved
// {
// fn into(self) -> BndBuilderObserverRc {
// BndBuilderObserverRc(Rc::new(RefCell::new(Box::new(self.clone()))))
// }
// }

impl<E> Drop for RuleTaskEventDispatcher<'_, '_, '_, E>
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

impl<E> EventObserver for RuleTaskEventDispatcher<'_, '_, '_, E>
where E: BndBuilderObserved + Sync
{
    #[inline]
    fn emit_stdout(&self, s: &str) {
        match self.rule {
            Some(rule) => self.observed.emit_task_stdout(rule, self.task, s),
            None => self.observed.emit_stdout(s)
        }
    }

    #[inline]
    fn emit_stderr(&self, s: &str) {
        match self.rule {
            Some(rule) => self.observed.emit_task_stderr(rule, self.task, s),
            None => self.observed.emit_stderr(s)
        }
    }
}

impl<E> BndBuilderObserver for RuleTaskEventDispatcher<'_, '_, '_, E>
where E: BndBuilderObserved
{
    fn update(&mut self, event: BndBuilderEvent) {
        unreachable!()
    }
}

#[derive(Default, Debug)]
pub struct BndBuilderDefaultObserver {}

impl BndBuilderDefaultObserver {
    pub fn new() -> Rc<RefCell<Box<dyn BndBuilderObserver>>> {
        Rc::new(RefCell::new(Box::new(BndBuilderDefaultObserver {})))
    }
}

impl EventObserver for BndBuilderDefaultObserver {
    fn emit_stdout(&self, s: &str) {
        print!("{s}");
    }

    fn emit_stderr(&self, s: &str) {
        eprint!("{s}");
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
            BndBuilderEvent::Stdout(s) => print!("{s}"),
            BndBuilderEvent::Stderr(s) => print!("{s}")
        }
    }
}
