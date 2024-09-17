use std::marker::PhantomData;
use std::rc::Rc;

use cpclib_common::itertools::Itertools;
use cpclib_runner::event::EventObserver;

use super::Runner;
use crate::task::ECHO_CMDS;

pub struct EchoRunner<E: EventObserver> {
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for EchoRunner<E> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData::<E>
        }
    }
}

impl<E: EventObserver> Runner for EchoRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let txt = itr.iter().map(|s| s.as_ref()).join(" ");
        o.emit_stdout(&format!("{txt}\n"));
        Ok(())
    }

    fn get_command(&self) -> &str {
        ECHO_CMDS[0]
    }
}
