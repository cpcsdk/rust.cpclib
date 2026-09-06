use std::io::Read;
use std::marker::PhantomData;

use cpclib_common::itertools::Itertools;
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::TaskStdin;

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

    /// With no arguments and something piped/redirected into it, `echo`
    /// behaves like `cat`: it reads and re-emits its stdin verbatim. With
    /// arguments, stdin is ignored (matches real `echo`/`cat` - "echo hi"
    /// never reads stdin even if some is available).
    fn inner_run_with_stdin<S: AsRef<str>>(
        &self,
        itr: &[S],
        o: &E,
        stdin: Option<TaskStdin>
    ) -> Result<(), String> {
        if itr.is_empty()
            && let Some(stdin @ (TaskStdin::File(_) | TaskStdin::Reader(_))) = stdin
        {
            let mut reader: Box<dyn Read> = match stdin {
                TaskStdin::File(path) => {
                    Box::new(
                        fs_err::File::open(&path)
                            .map_err(|e| format!("unable to open {path} for reading: {e}"))?
                    )
                },
                TaskStdin::Reader(r) => Box::new(r),
                TaskStdin::Empty => unreachable!("excluded by the if let guard above")
            };
            let mut buf = String::new();
            reader
                .read_to_string(&mut buf)
                .map_err(|e| format!("error reading stdin: {e}"))?;
            o.emit_stdout(&buf);
            return Ok(());
        }
        self.inner_run(itr, o)
    }

    fn get_command(&self) -> &str {
        ECHO_CMDS[0]
    }
}
