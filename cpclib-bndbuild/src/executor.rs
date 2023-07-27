use std::ops::Deref;

use cpclib_common::lazy_static::lazy_static;

use crate::{
    runners::{BasmRunner, EchoRunner, ImgConverterRunner, RmRunner, Runner},
    task::Task,
};
use crate::runners::XferRunner;
lazy_static! {
    pub static ref  BASM_RUNNER: BasmRunner = BasmRunner::default();
    pub static ref RM_RUNNER: RmRunner = RmRunner::default();
    pub static ref ECHO_RUNNER: EchoRunner = EchoRunner::default();
    pub static ref IMGCONV_RUNNER: ImgConverterRunner = ImgConverterRunner::default();
    pub static ref XFER_RUNNER: XferRunner = XferRunner::default();
}

pub fn execute(task: &Task) -> Result<(), String> {
    let (runner, args) = match task {
        Task::Basm(_) => (BASM_RUNNER.deref() as &dyn Runner, task.args()),
        Task::Rm(_) => (RM_RUNNER.deref() as &dyn Runner, task.args()),
        Task::Echo(_) => (ECHO_RUNNER.deref() as &dyn Runner, task.args()),
        Task::ImgConverter(_) => (IMGCONV_RUNNER.deref() as &dyn Runner, task.args()),
        Task::Xfer(_) => (XFER_RUNNER.deref() as &dyn Runner, task.args())
    };

    runner.run(args).or_else(|e| {
        if task.ignore_errors() {
            println!("\t\tError ignored. {}", e);
            Ok(())
        } else {
            Err(e)
        }
    })
}
