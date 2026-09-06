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
use cpclib_runner::runner::twocdt::TwoCdtVersion;
use cpclib_runner::runner::vlink::Vlink;
use cpclib_runner::runner::{ExternRunner, Runner};

use crate::event::BndBuilderObserver;
use crate::runners::archive::ArchiveRunner;
use crate::runners::asmfmt::AsmFmtRunner;
use crate::runners::assembler::{Assembler, BasmRunner, OrgamsRunner};
use crate::runners::ay::YmCruncher;
#[cfg(feature = "basmdoc-generator")]
use crate::runners::basmdoc::BasmDocRunner;
use crate::runners::basmopt::BasmOptRunner;
use crate::runners::bndbuild::BndBuildRunner;
use crate::runners::cpc2img::CpcToImgRunner;
use crate::runners::cprcli::CprCliRunner;
use crate::runners::crunch::CrunchRunner;
use crate::runners::csl::CslRunner;
use crate::runners::disassembler::{BdasmRunner, Disassembler};
use crate::runners::disc::DiscManagerRunner;
use crate::runners::echo::EchoRunner;
use crate::runners::fade::FadeRunner;
use crate::runners::fs::cp::CpRunner;
use crate::runners::fs::mkdir::MkdirRunner;
use crate::runners::fs::mv::MvRunner;
use crate::runners::fs::rm::RmRunner;
use crate::runners::hideur::HideurRunner;
use crate::runners::hxcfe::HxcfeRunner;
use crate::runners::img2cpc::ImgToCpcRunner;
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

            InnerTask::Assembler(Assembler::Extern(extern_assembler), _) => {
                Some(extern_assembler.configuration::<E>())
            },
            InnerTask::Assembler(_, _) => None,

            InnerTask::YmCruncher(c, _) => {
                match c {
                    YmCruncher::Ayt => Some(AytVersion::default().configuration()),
                    YmCruncher::Miny => Some(MinimiserVersion::default().configuration()),
                    #[cfg(feature = "fap")]
                    YmCruncher::Fap => Some(FAPVersion::default().configuration())
                }
            },
            InnerTask::Convgeneric(_) => Some(ConvGenericVersion::default().configuration()),
            InnerTask::Disassembler(Disassembler::Extern(e), _) => Some(e.configuration()),
            InnerTask::Disassembler(_, _) => None,

            InnerTask::Grafx2(_) => Some(Grafx2Version::default().configuration()),

            InnerTask::HspCompiler(_) => Some(HspCompilerVersion::default().configuration()),
            InnerTask::ImpDsk(_) => Some(ImpDskVersion::default().configuration()),
            #[cfg(feature = "tape")]
            InnerTask::Cdt(crate::runners::cdt::CdtManager::TwoCdt, _) => {
                Some(TwoCdtVersion::default().configuration())
            },
            InnerTask::Martine(_) => Some(MartineVersion::default().configuration()),
            InnerTask::Tracker(t, _) => Some(t.configuration()),
            InnerTask::Vlink(_) => Some(Vlink.configuration()),

            _ => None
        }
    }
}

#[inline]
pub fn execute<E: BndBuilderObserver + 'static>(
    task: &InnerTask,
    observer: &Arc<E>
) -> Result<(), String> {
    execute_redirected(task, observer, None, None)
}

/// Same as [`execute`], but with an optional stdin source and/or raw stdout
/// destination - the entry point pipeline stages (see
/// `crate::shell_pipe::execute_pipe`) use to wire a `<file` redirection or
/// the previous stage's pipe output into whichever task kind ends up
/// running, and/or a `>`/`>>` redirection or the next stage's pipe input out
/// of it. Every match arm below threads both through as `run_redirected`
/// instead of `run`; for the overwhelming majority of task kinds (anything
/// that hasn't overridden `inner_run_redirected`) this is behaviourally
/// identical to `run`, since the default implementation just ignores both
/// and falls back to it.
pub(crate) fn execute_redirected<E: BndBuilderObserver + 'static>(
    task: &InnerTask,
    observer: &Arc<E>,
    stdin: Option<cpclib_runner::runner::TaskStdin>,
    stdout: Option<cpclib_runner::runner::TaskStdout>
) -> Result<(), String> {
    match task {
        InnerTask::Emulator(e, _) => {
            match e {
                crate::runners::emulator::Emulator::EmulatorProxy(e) => {
                    DelegatedRunner::<E>::new(
                        task.configuration::<E>().unwrap(),
                        e.get_command().to_owned()
                    )
                    .run_redirected(task.args(), observer, stdin, stdout)
                },
                crate::runners::emulator::Emulator::EmulatorFacade => {
                    EmulatorFacadeRunner::default().run_redirected(task.args(), observer, stdin, stdout)
                },
            }
        },
        InnerTask::Catalog(_args) => {
            crate::runners::disc::CatalogRunner::<E>::default().run_redirected(task.args(), observer, stdin, stdout)
        },
        InnerTask::Locomotive(_args) => {
            crate::runners::locomotive::LocomotiveRunner::<E>::default().run_redirected(task.args(), observer, stdin, stdout)
        },
        #[cfg(feature = "tape")]
        InnerTask::Cdt(cdt, _args) => {
            match cdt {
                crate::runners::cdt::CdtManager::Rtzx => {
                    crate::runners::cdt::RtzxRunner::<E>::default().run_redirected(task.args(), observer, stdin, stdout)
                },
                crate::runners::cdt::CdtManager::TwoCdt => {
                    DelegatedRunner::<E>::new(
                        task.configuration::<E>().unwrap(),
                        TwoCdtVersion::default().get_command().to_owned()
                    )
                    .run_redirected(task.args(), observer, stdin, stdout)
                },
            }
        },
        InnerTask::Assembler(a, _) => {
            match a {
                Assembler::Basm => BasmRunner::default().run_redirected(task.args(), observer, stdin, stdout),
                Assembler::Orgams => OrgamsRunner::default().run_redirected(task.args(), observer, stdin, stdout),
                Assembler::Extern(e) => {
                    DelegatedRunner::<E>::new(e.configuration(), a.get_command().to_owned())
                        .run_redirected(task.args(), observer, stdin, stdout)
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
                    .run_redirected(task.args(), observer, stdin, stdout)
                },
                YmCruncher::Miny => {
                    DelegatedRunner::<E>::new(
                        task.configuration::<E>().unwrap(),
                        c.get_command().to_owned()
                    )
                    .run_redirected(task.args(), observer, stdin, stdout)
                },
                #[cfg(feature = "fap")]
                YmCruncher::Fap => {
                    DelegatedRunner::<E>::new(
                        task.configuration::<E>().unwrap(),
                        c.get_command().to_owned()
                    )
                    .run_redirected(task.args(), observer, stdin, stdout)
                },
            }
        },

        InnerTask::Crunch(_) => CrunchRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::Disassembler(d, _) => {
            match d {
                crate::runners::disassembler::Disassembler::Bdasm => {
                    BdasmRunner::default().run_redirected(task.args(), observer, stdin, stdout)
                },
                crate::runners::disassembler::Disassembler::Extern(d) => {
                    DelegatedRunner::<E>::new(d.configuration(), d.get_command().to_owned())
                        .run_redirected(task.args(), observer, stdin, stdout)
                },
            }
        },
        InnerTask::SongConverter(d, _) => {
            DelegatedRunner::<E>::new(d.configuration(), d.get_command().to_owned())
                .run_redirected(task.args(), observer, stdin, stdout)
        },

        InnerTask::Tracker(d, _) => {
            DelegatedRunner::<E>::new(d.configuration(), d.get_command().to_owned())
                .run_redirected(task.args(), observer, stdin, stdout)
        },
        InnerTask::BndBuild(_) => BndBuildRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        #[cfg(feature = "basmdoc-generator")]
        InnerTask::BasmDoc(_) => BasmDocRunner::<E>::default().run_redirected(task.args(), observer, stdin, stdout),
        #[cfg(not(feature = "basmdoc-generator"))]
        InnerTask::BasmDoc(_) => {
            Err(
                "basmdoc support was not compiled into this build (missing the \
                 `basmdoc-generator` feature)"
                    .to_string()
            )
        },
        InnerTask::Cp(_) => CpRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::Mv(_) => MvRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::Disc(_) => DiscManagerRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::AsmFmt(_) => AsmFmtRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::BasmOpt(_) => BasmOptRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::Echo(_) => EchoRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::Extern(_) => ExternRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::Fade(_) => FadeRunner::<E>::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::Hideur(_) => HideurRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::Hxcfe(_) => HxcfeRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::Snapshot(_) => SnapshotRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::ImgToCpc(_) => ImgToCpcRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::CpcToImg(_) => CpcToImgRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::ImpDsk(_) => {
            DelegatedRunner::<E>::new(
                task.configuration().unwrap(),
                ImpDskVersion::default().get_command().to_owned()
            )
            .run_redirected(task.args(), observer, stdin, stdout)
        },
        InnerTask::HspCompiler(_) => {
            DelegatedRunner::<E>::new(
                task.configuration().unwrap(),
                HspCompilerVersion::default().get_command().to_owned()
            )
            .run_redirected(task.args(), observer, stdin, stdout)
        },
        InnerTask::Martine(_) => {
            // Martine v0.39 crashes without arguments (invalid log filename with parentheses)
            // Inject --help when no arguments provided to avoid crash
            let args = task.args();
            let safe_args = if args.trim().is_empty() {
                "--help"
            }
            else {
                args
            };

            DelegatedRunner::<E>::new(
                task.configuration().unwrap(),
                MartineVersion::default().get_command().to_owned()
            )
            .run_redirected(safe_args, observer, stdin, stdout)
        },
        InnerTask::Mkdir(_) => MkdirRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::Rm(_) => RmRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::Archive(_) => ArchiveRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::Xfer(_) => XferRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::Cpr(_) => CprCliRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::Csl(_) => CslRunner::default().run_redirected(task.args(), observer, stdin, stdout),
        InnerTask::Vlink(_) => {
            DelegatedRunner::<E>::new(
                task.configuration().unwrap(),
                Vlink.get_command().to_owned()
            )
            .run_redirected(task.args(), observer, stdin, stdout)
        },

        InnerTask::Grafx2(_) => {
            DelegatedRunner::<E>::new(
                task.configuration().unwrap(),
                Grafx2Version::default().get_command().to_owned()
            )
            .run_redirected(task.args(), observer, stdin, stdout)
        },
        InnerTask::Convgeneric(_standard_task_arguments) => {
            DelegatedRunner::<E>::new(
                task.configuration().unwrap(),
                ConvGenericVersion::default().get_command().to_owned()
            )
            .run_redirected(task.args(), observer, stdin, stdout)
        },
        InnerTask::Pipe(p) => {
            // Erase the observer type before recursing into the pipeline
            // executor: `execute_redirected` is generic over the observer
            // and a pipeline stage's own observer is a `RedirectedObserver`
            // wrapping whatever was passed in here, so without erasing this
            // type would grow one `RedirectedObserver<Arc<...>>` layer per
            // nesting level that the type-checker must be able to
            // monomorphize for - even though a `Pipe` never actually
            // contains another `Pipe` as a stage, the compiler doesn't know
            // that, and hits the recursion limit trying to instantiate an
            // unbounded chain. Erasing to one fixed `dyn` type here breaks
            // the cycle: `execute_pipe` and everything under it only ever
            // deals with this same concrete type, however deep the
            // (impossible in practice) nesting goes.
            let erased: Arc<dyn BndBuilderObserver + Send + Sync> = observer.clone();
            crate::shell_pipe::execute_pipe(p, erased)
        }
    }
    .or_else(|e| {
        if task.ignore_errors() {
            // Structured event (drives e.g. the CLI's red "[Error ignored]"
            // line and the LSP's warning diagnostics); observers that only
            // implement plain `update()`/`EventObserver` and ignore this
            // still see nothing here, unlike before - if you need a plain
            // stdout fallback for such an observer, add it there instead of
            // reverting this, to avoid the double-print this replaces.
            observer.emit_ignored_error(&e);
            Ok(())
        }
        else {
            // dbg!("There was an error", &e);
            Err(e)
        }
    })
}
