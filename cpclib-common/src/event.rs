use std::fmt::Debug;

pub trait EventObserver: Debug + Sync {
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
