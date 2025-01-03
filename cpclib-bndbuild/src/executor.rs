use std::sync::Arc;

use cpclib_asm::EnvEventObserver;
use cpclib_runner::delegated::{DelegateApplicationDescription, DelegatedRunner, GithubCompilableApplication};
use cpclib_runner::emucontrol::EmuControlledRunner;
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::convgeneric::ConvGenericVersion;
use cpclib_runner::runner::fap::FAPVersion;
use cpclib_runner::runner::impdisc::ImpDskVersion;
use cpclib_runner::runner::martine::MartineVersion;
use cpclib_runner::runner::{ExternRunner, Runner};

use crate::event::{BndBuilderObserved, BndBuilderObserver};
use crate::runners::assembler::{Assembler, BasmRunner, OrgamsRunner};
use crate::runners::bndbuild::BndBuildRunner;
use crate::runners::cp::CpRunner;
use crate::runners::disassembler::BdasmRunner;
use crate::runners::disc::DiscManagerRunner;
use crate::runners::echo::EchoRunner;
use crate::runners::hideur::HideurRunner;
use crate::runners::imgconverter::ImgConverterRunner;
use crate::runners::rm::RmRunner;
use crate::runners::snapshot::SnapshotRunner;
use crate::runners::xfer::XferRunner;
use crate::task::Task;

impl Task {
    #[inline]
    pub fn configuration<E: EventObserver>(
        &self
    ) -> Option<DelegateApplicationDescription<E>> {
        match self {
            Task::Emulator(e, _) => {
                match e {
                    crate::runners::emulator::Emulator::DirectAccess(e) => {
                        let conf: DelegateApplicationDescription<E> = e.configuration();
                        Some(conf)
                    },
                    crate::runners::emulator::Emulator::ControlledAccess => None
                }
            },

            Task::Assembler(a, _) => {
                match a {
                    Assembler::Extern(extern_assembler) => {
                        Some(extern_assembler.configuration::<E>())
                    },
                    _ => None
                }
            },

            Task::ImpDsk(_) => Some(ImpDskVersion::default().configuration()),
            Task::Convgeneric(_) => Some(ConvGenericVersion::default().configuration()),
            Task::Martine(_) => Some(MartineVersion::default().configuration()),

            Task::Fap(_) => Some(FAPVersion::default().configuration()),

            _ => None
        }
    }
}

#[inline]
pub fn execute<E: BndBuilderObserver + 'static>(task: &Task, observer: &Arc<E>) -> Result<(), String> {
    match task {
        Task::Emulator(e, _) => {
            match e {
                crate::runners::emulator::Emulator::DirectAccess(e) => {
                    DelegatedRunner::<E> {
                        app: task.configuration::<E>().unwrap(),
                        cmd: e.get_command().to_owned()
                    }
                    .run(task.args(), observer)
                },
                crate::runners::emulator::Emulator::ControlledAccess => {
                    EmuControlledRunner::default().run(task.args(), observer)
                },
            }
        },
        Task::Assembler(a, _) => {
            match a {
                Assembler::Basm => BasmRunner::default().run(task.args(), observer),
                Assembler::Orgams => OrgamsRunner::default().run(task.args(), observer),
                Assembler::Extern(e) => {
                    DelegatedRunner {
                        app: e.configuration(),
                        cmd: a.get_command().to_owned()
                    }
                    .run(task.args(), observer)
                },
            }
        },
        Task::Disassembler(d, _) => {
            match d {
                crate::runners::disassembler::Disassembler::Bdasm => {
                    BdasmRunner::default().run(task.args(), observer)
                },
                crate::runners::disassembler::Disassembler::Extern(d) => {
                    DelegatedRunner {
                        app: d.configuration(),
                        cmd: d.get_command().to_owned()
                    }
                    .run(task.args(), observer)
                },
            }
        },
        Task::Tracker(d, _) => {
            DelegatedRunner {
                app: d.configuration(),
                cmd: d.get_command().to_owned()
            }
            .run(task.args(), observer)
        },
        Task::BndBuild(_) => BndBuildRunner::default().run(task.args(), observer),
        Task::Cp(_) => CpRunner::default().run(task.args(), observer),
        Task::Disc(_) => DiscManagerRunner::default().run(task.args(), observer),
        Task::Echo(_) => EchoRunner::default().run(task.args(), observer),
        Task::Extern(_) => ExternRunner::default().run(task.args(), observer),
        Task::Hideur(_) => HideurRunner::default().run(task.args(), observer),
        Task::Snapshot(_) => SnapshotRunner::default().run(task.args(), observer),
        Task::ImgConverter(_) => ImgConverterRunner::default().run(task.args(), observer),
        Task::ImpDsk(_) => {
            DelegatedRunner {
                app: task.configuration().unwrap(),
                cmd: ImpDskVersion::default().get_command().to_owned()
            }
            .run(task.args(), observer)
        },
        Task::Martine(_) => {
            DelegatedRunner {
                app: task.configuration().unwrap(),
                cmd: MartineVersion::default().get_command().to_owned()
            }
            .run(task.args(), observer)
        },
        Task::Rm(_) => RmRunner::default().run(task.args(), observer),
        Task::Xfer(_) => XferRunner::default().run(task.args(), observer),
        Task::Fap(_) => {
            DelegatedRunner {
                app: task.configuration().unwrap(),
                cmd: FAPVersion::default().get_command().to_owned()
            }
            .run(task.args(), observer)
        },
        Task::Convgeneric(standard_task_arguments) => {
            DelegatedRunner {
                app: task.configuration().unwrap(),
                cmd: ConvGenericVersion::default().get_command().to_owned()
            }
            .run(task.args(), observer)
        },
    }
    .or_else(|e| {
        if task.ignore_errors() {
            dbg!(&observer);
            observer.emit_stdout(&format!("\t\t[Error ignored] {}", e));
            Ok(())
        }
        else {
            Err(e)
        }
    })
}
