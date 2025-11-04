use std::sync::Arc;

use cpclib_runner::delegated::{
    DelegateApplicationDescription, DelegatedRunner, GithubCompilableApplication
};
use cpclib_runner::emucontrol::EmulatorFacadeRunner;
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::ay::ayt::AytVersion;
#[cfg(feature = "fap")]
use cpclib_runner::runner::ay::fap::FAPVersion;
use cpclib_runner::runner::ay::minimiser::MinimiserVersion;
use cpclib_runner::runner::convgeneric::ConvGenericVersion;
use cpclib_runner::runner::grafx2::Grafx2Version;
use cpclib_runner::runner::hspcompiler::HspCompilerVersion;
use cpclib_runner::runner::impdisc::ImpDskVersion;
use cpclib_runner::runner::martine::MartineVersion;
use cpclib_runner::runner::{ExternRunner, Runner};

use crate::event::{BndBuilderObserved, BndBuilderObserver};
use crate::runners::assembler::{Assembler, BasmRunner, OrgamsRunner};
#[cfg(feature = "fap")]
use crate::runners::ay::YmCruncher;
use crate::runners::bndbuild::BndBuildRunner;
use crate::runners::crunch::CrunchRunner;
use crate::runners::disassembler::{BdasmRunner, Disassembler};
use crate::runners::disc::DiscManagerRunner;
use crate::runners::echo::EchoRunner;
use crate::runners::fade::FadeRunner;
use crate::runners::fs::cp::CpRunner;
use crate::runners::fs::mkdir::MkdirRunner;
use crate::runners::fs::rm::RmRunner;
use crate::runners::hideur::HideurRunner;
use crate::runners::imgconverter::ImgConverterRunner;
use crate::runners::snapshot::SnapshotRunner;
use crate::runners::xfer::XferRunner;
use crate::task::InnerTask;

impl InnerTask {
    #[inline]
    pub fn configuration<E: EventObserver>(&self) -> Option<DelegateApplicationDescription<E>> {
        match self {
            InnerTask::Emulator(e, _) => {
                match e {
                    crate::runners::emulator::Emulator::EmulatorProxy(e) => {
                        let conf: DelegateApplicationDescription<E> = e.configuration();
                        Some(conf)
                    },
                    crate::runners::emulator::Emulator::EmulatorFacade => None
                }
            },

            InnerTask::Assembler(a, _) => {
                match a {
                    Assembler::Extern(extern_assembler) => {
                        Some(extern_assembler.configuration::<E>())
                    },
                    _ => None
                }
            },

            InnerTask::YmCruncher(c, _) => {
                match c {
                    YmCruncher::Ayt => Some(AytVersion::default().configuration()),
                    YmCruncher::Miny => Some(MinimiserVersion::default().configuration()),
                    #[cfg(feature = "fap")]
                    YmCruncher::Fap => Some(FAPVersion::default().configuration())
                }
            },
            InnerTask::Convgeneric(_) => Some(ConvGenericVersion::default().configuration()),
            InnerTask::Disassembler(d, _) => {
                match d {
                    Disassembler::Extern(e) => Some(e.configuration()),
                    _ => None
                }
            },

            InnerTask::Grafx2(_) => Some(Grafx2Version::default().configuration()),

            InnerTask::HspCompiler(_) => Some(HspCompilerVersion::default().configuration()),
            InnerTask::ImpDsk(_) => Some(ImpDskVersion::default().configuration()),
            InnerTask::Martine(_) => Some(MartineVersion::default().configuration()),
            InnerTask::Tracker(t, _) => Some(t.configuration()),

            _ => None
        }
    }
}

#[inline]
pub fn execute<E: BndBuilderObserver + 'static>(
    task: &InnerTask,
    observer: &Arc<E>
) -> Result<(), String> {
    match task {
        InnerTask::Emulator(e, _) => {
            match e {
                crate::runners::emulator::Emulator::EmulatorProxy(e) => {
                    DelegatedRunner::<E>::new(
                        task.configuration::<E>().unwrap(),
                        e.get_command().to_owned()
                    )
                    .run(task.args(), observer)
                },
                crate::runners::emulator::Emulator::EmulatorFacade => {
                    EmulatorFacadeRunner::default().run(dbg!(task.args()), observer)
                },
            }
        },
        InnerTask::Assembler(a, _) => {
            match a {
                Assembler::Basm => BasmRunner::default().run(task.args(), observer),
                Assembler::Orgams => OrgamsRunner::default().run(task.args(), observer),
                Assembler::Extern(e) => {
                    DelegatedRunner::<E>::new(e.configuration(), a.get_command().to_owned())
                        .run(task.args(), observer)
                },
            }
        },
        InnerTask::YmCruncher(c, _) => {
            match c {
                YmCruncher::Ayt => {
                    DelegatedRunner::<E>::new(
                        task.configuration::<E>().unwrap(),
                        c.get_command().to_owned()
                    )
                    .run(task.args(), observer)
                },
                YmCruncher::Miny => {
                    DelegatedRunner::<E>::new(
                        task.configuration::<E>().unwrap(),
                        c.get_command().to_owned()
                    )
                    .run(task.args(), observer)
                },
                #[cfg(feature = "fap")]
                YmCruncher::Fap => {
                    DelegatedRunner::<E>::new(
                        task.configuration::<E>().unwrap(),
                        c.get_command().to_owned()
                    )
                    .run(task.args(), observer)
                },
            }
        },

        InnerTask::Crunch(_) => CrunchRunner::default().run(task.args(), observer),
        InnerTask::Disassembler(d, _) => {
            match d {
                crate::runners::disassembler::Disassembler::Bdasm => {
                    BdasmRunner::default().run(task.args(), observer)
                },
                crate::runners::disassembler::Disassembler::Extern(d) => {
                    DelegatedRunner::<E>::new(d.configuration(), d.get_command().to_owned())
                        .run(task.args(), observer)
                },
            }
        },
        InnerTask::SongConverter(d, _) => {
            DelegatedRunner::<E>::new(d.configuration(), d.get_command().to_owned())
                .run(task.args(), observer)
        },

        InnerTask::Tracker(d, _) => {
            DelegatedRunner::<E>::new(d.configuration(), d.get_command().to_owned())
                .run(task.args(), observer)
        },
        InnerTask::BndBuild(_) => BndBuildRunner::default().run(task.args(), observer),
        InnerTask::Cp(_) => CpRunner::default().run(task.args(), observer),
        InnerTask::Disc(_) => DiscManagerRunner::default().run(task.args(), observer),
        InnerTask::Echo(_) => EchoRunner::default().run(task.args(), observer),
        InnerTask::Extern(_) => ExternRunner::default().run(task.args(), observer),
        InnerTask::Fade(_) => FadeRunner::<E>::default().run(task.args(), observer),
        InnerTask::Hideur(_) => HideurRunner::default().run(task.args(), observer),
        InnerTask::Snapshot(_) => SnapshotRunner::default().run(task.args(), observer),
        InnerTask::ImgConverter(_) => ImgConverterRunner::default().run(task.args(), observer),
        InnerTask::ImpDsk(_) => {
            DelegatedRunner::<E>::new(
                task.configuration().unwrap(),
                ImpDskVersion::default().get_command().to_owned()
            )
            .run(task.args(), observer)
        },
        InnerTask::HspCompiler(_) => {
            DelegatedRunner::<E>::new(
                task.configuration().unwrap(),
                HspCompilerVersion::default().get_command().to_owned()
            )
            .run(task.args(), observer)
        },
        InnerTask::Martine(_) => {
            DelegatedRunner::<E>::new(
                task.configuration().unwrap(),
                MartineVersion::default().get_command().to_owned()
            )
            .run(task.args(), observer)
        },
        InnerTask::Mkdir(_) => MkdirRunner::default().run(task.args(), observer),
        InnerTask::Rm(_) => RmRunner::default().run(task.args(), observer),
        InnerTask::Xfer(_) => XferRunner::default().run(task.args(), observer),

        InnerTask::Grafx2(_) => {
            DelegatedRunner::<E>::new(
                task.configuration().unwrap(),
                Grafx2Version::default().get_command().to_owned()
            )
            .run(task.args(), observer)
        },
        InnerTask::Convgeneric(_standard_task_arguments) => {
            DelegatedRunner::<E>::new(
                task.configuration().unwrap(),
                ConvGenericVersion::default().get_command().to_owned()
            )
            .run(task.args(), observer)
        },
    }
    .or_else(|e| {
        if task.ignore_errors() {
            observer.emit_stdout(&format!("[Error ignored] {e}\n"));
            Ok(())
        }
        else {
            // dbg!("There was an error", &e);
            Err(e)
        }
    })
}
