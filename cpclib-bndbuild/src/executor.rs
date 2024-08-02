use cpclib_common::lazy_static::lazy_static;

use crate::runners::basm::BasmRunner;
use crate::runners::bndbuild::BndBuildRunner;
use crate::runners::cp::CpRunner;
use crate::runners::disc::DiscManagerRunner;
use crate::runners::echo::EchoRunner;
use crate::runners::imgconverter::ImgConverterRunner;
use crate::runners::r#extern::ExternRunner;
use crate::runners::rm::RmRunner;
use crate::runners::xfer::XferRunner;
use crate::runners::Runner;
use crate::task::Task;

lazy_static! {
    pub static ref BASM_RUNNER: BasmRunner = BasmRunner::default();
    pub static ref BNDBUILD_RUNNER: BndBuildRunner = BndBuildRunner::default();
    pub static ref CP_RUNNER: CpRunner = CpRunner::default();
    pub static ref DISC_RUNNER: DiscManagerRunner = DiscManagerRunner::default();
    pub static ref ECHO_RUNNER: EchoRunner = EchoRunner::default();
    pub static ref EXTERN_RUNNER: ExternRunner = ExternRunner::default();
    pub static ref IMGCONV_RUNNER: ImgConverterRunner = ImgConverterRunner::default();
    pub static ref RM_RUNNER: RmRunner = RmRunner::default();
    pub static ref XFER_RUNNER: XferRunner = XferRunner::default();
}

pub fn execute(task: &Task) -> Result<(), String> {
    match task {
        Task::Basm(_) => BASM_RUNNER.run(task.args()),
        Task::BndBuild(_) => BNDBUILD_RUNNER.run(task.args()),
        Task::Cp(_) => CP_RUNNER.run(task.args()),
        Task::Disc(_) => DISC_RUNNER.run(task.args()),
        Task::Echo(_) => ECHO_RUNNER.run(task.args()),
        Task::Extern(_) => EXTERN_RUNNER.run(task.args()),
        Task::ImgConverter(_) => IMGCONV_RUNNER.run(task.args()),
        Task::Rm(_) => RM_RUNNER.run(task.args()),
        Task::Xfer(_) => XFER_RUNNER.run(task.args())
    }
    .or_else(|e| {
        if task.ignore_errors() {
            println!("\t\tError ignored. {}", e);
            Ok(())
        }
        else {
            Err(e)
        }
    })
}
