use std::borrow::Borrow;
use std::io::{BufRead, BufReader};
use std::marker::PhantomData;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::thread;

use clap::{ArgMatches, Command};
use cpclib_common::itertools::Itertools;

use crate::event::EventObserver;
use crate::runner::arguments::get_all_args;

pub trait Runner {
    type EventObserver: EventObserver;

    /// Run the task and return true if successfull
    fn run(&self, arguments: &str, o: &Self::EventObserver) -> Result<(), String> {
        let args = get_all_args(arguments)?;
        self.inner_run(&args, o)
    }

    /// Implement the command specific action
    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &Self::EventObserver) -> Result<(), String>;

    fn get_command(&self) -> &str;
}

pub trait RunnerWithClap: Runner + Default {
    fn get_clap_command(&self) -> &Command;

    fn get_matches<S: AsRef<str>>(&self, itr: &[S]) -> Result<ArgMatches, String> {
        self.get_clap_command()
            .clone()
            .try_get_matches_from(itr.iter().map(|s| s.as_ref()))
            .map_err(|e| e.to_string())
    }

    fn render_help() -> String {
        Self::default()
            .get_clap_command()
            .clone()
            .disable_help_flag(true)
            .render_long_help()
            .to_string()
    }
}

pub struct ExternRunner<E: EventObserver> {
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for ExternRunner<E> {
    fn default() -> Self {
        Self {
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> ExternRunner<E> {}
impl<E: EventObserver +'static> Runner for ExternRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let itr = itr.iter().map(|s| s.as_ref()).collect_vec();

        // WARNING
        // Deactivated because if makes fail normal progam on Linux
        // however, it was maybe mandatory for Windows
        // let app = std::fs::canonicalize(&itr[0])
        //     .map_err(|e| format!("Wrong executable {}.{}", &itr[0], e.to_string()))?;
        let app = &itr[0];

        let cwd = std::env::current_dir()
            .map_err(|e| format!("Unable to get the current working directory {}.", e))?;
        let cwd = std::fs::canonicalize(cwd)
            .map_err(|e| format!("Unable to get the current working directory {}.", e))?;

        let mut cmd = std::process::Command::new(app);
        cmd.current_dir(cwd);
        for arg in &itr[1..] {
            cmd.arg(arg);
        }
        let mut cmd = cmd
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Error while launching {}. {}", &itr[0], e))?;


        // the process is running in another thread. We'll collect its outputs in yet other threads
        let child_stdout = cmd
            .stdout
            .take()
            .expect("Internal error, could not take stdout");
        let child_stderr = cmd
            .stderr
            .take()
            .expect("Internal error, could not take stderr");


        let o: &'static E = unsafe{std::mem::transmute(o)};

        let o = Arc::new(Mutex::new(o));
        let oerr = o.clone();
        let oout = o.clone();

        let stdout_thread = thread::spawn(move || {
            let stdout_lines = BufReader::new(child_stdout).lines();
            for line in stdout_lines {
                let line = line.unwrap();
                if let Ok(o) = oout.lock() {
                    o.emit_stdout(&line);
                } else {
                    unimplemented!()
                }
            }
        });

        let stderr_thread = thread::spawn(move || {
            let stderr_lines = BufReader::new(child_stderr).lines();
            for line in stderr_lines {
                let line = line.unwrap();
                if let Ok(o) = oerr.lock() {
                    o.emit_stderr(&line);
                } else {
                    unimplemented!()
                }
            }
        });


        let status = cmd
            .wait()
            .map_err(|e| format!("Error while executing {}. {}", &itr[0], e))?;

        if !status.success() {
            return Err("Error while launching the command.".to_owned());
        }
        stdout_thread.join().unwrap();
        stderr_thread.join().unwrap();

        Ok(())
    }

    fn get_command(&self) -> &str {
        "external"
    }
}
