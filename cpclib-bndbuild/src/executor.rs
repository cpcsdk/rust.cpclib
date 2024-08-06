use std::sync::LazyLock;

use crate::runners::basm::BasmRunner;
use crate::runners::bndbuild::BndBuildRunner;
use crate::runners::cp::CpRunner;
use crate::runners::disc::DiscManagerRunner;
use crate::runners::echo::EchoRunner;
use crate::runners::emulator::EmulatorRunner;
use crate::runners::imgconverter::ImgConverterRunner;
use crate::runners::r#extern::ExternRunner;
use crate::runners::rm::RmRunner;
use crate::runners::xfer::XferRunner;
use crate::runners::Runner;
use crate::task::Task;

pub static BASM_RUNNER: LazyLock<BasmRunner> = LazyLock::new(|| BasmRunner::default());
pub static BNDBUILD_RUNNER: LazyLock<BndBuildRunner> = LazyLock::new(|| BndBuildRunner::default());
pub static CP_RUNNER: LazyLock<CpRunner> = LazyLock::new(|| CpRunner::default());
pub static DISC_RUNNER: LazyLock<DiscManagerRunner> =
    LazyLock::new(|| DiscManagerRunner::default());
pub static ECHO_RUNNER: LazyLock<EchoRunner> = LazyLock::new(|| EchoRunner::default());
pub static EXTERN_RUNNER: LazyLock<ExternRunner> = LazyLock::new(|| ExternRunner::default());
pub static IMGCONV_RUNNER: LazyLock<ImgConverterRunner> =
    LazyLock::new(|| ImgConverterRunner::default());
pub static RM_RUNNER: LazyLock<RmRunner> = LazyLock::new(|| RmRunner::default());
pub static XFER_RUNNER: LazyLock<XferRunner> = LazyLock::new(|| XferRunner::default());

pub fn execute(task: &Task) -> Result<(), String> {
    match task {
        Task::Emulator(e,_ ) => {
            EmulatorRunner{emu:e.clone()}.run(task.args())
        },
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
