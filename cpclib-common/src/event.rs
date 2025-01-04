use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;

pub trait EventObserver: Debug + Sync + Send {
    fn emit_stdout(&self, s: &str);
    fn emit_stderr(&self, s: &str);
}

impl EventObserver for () {
    fn emit_stdout(&self, s: &str) {
        println!("{}", s)
    }

    fn emit_stderr(&self, s: &str) {
        eprintln!("{}", s)
    }
}

impl<E: EventObserver> EventObserver for Option<E> {
    fn emit_stdout(&self, s: &str) {
        if let Some(e) = self {
            e.emit_stdout(s)
        }
    }

    fn emit_stderr(&self, s: &str) {
        if let Some(e) = self {
            e.emit_stderr(s)
        }
    }
}

impl<E: EventObserver> EventObserver for &E {
    fn emit_stdout(&self, s: &str) {
        (*self).emit_stdout(s)
    }

    fn emit_stderr(&self, s: &str) {
        (*self).emit_stderr(s)
    }
}

impl<E: EventObserver> EventObserver for Box<E> {
    fn emit_stdout(&self, s: &str) {
        self.as_ref().emit_stdout(s)
    }

    fn emit_stderr(&self, s: &str) {
        self.as_ref().emit_stderr(s)
    }
}

impl<T: EventObserver> EventObserver for Arc<T> {
    fn emit_stdout(&self, s: &str) {
        self.deref().emit_stdout(s)
    }

    fn emit_stderr(&self, s: &str) {
        self.deref().emit_stderr(s)
    }
}
