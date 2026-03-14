use std::fmt::Display;

use crate::delegated::{ArchiveFormat, Compiler, DelegateApplicationDescription};
use crate::event::EventObserver;
#[cfg(target_os = "linux")]
use crate::runner::Runner;
#[cfg(target_os = "linux")]
use crate::runner::ExternRunner;
#[cfg(target_os = "linux")]
use cpclib_common::camino::Utf8Path;

pub const TWO_CDT_CMD: &str = "2cdt";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum TwoCdtVersion {
    #[default]
    V1
}

impl TwoCdtVersion {
    pub fn get_command(&self) -> &str {
        TWO_CDT_CMD
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        #[cfg(target_os = "windows")]
        let exec_fname: &'static str = "2cdt.exe";
        #[cfg(not(target_os = "windows"))]
        let exec_fname: &'static str = "2cdt";

        #[cfg(target_os = "linux")]
        let compile: Option<Compiler<E>> = {
            let install: Box<dyn Fn(&Utf8Path, &E) -> Result<(), String>> =
                Box::new(|_path: &Utf8Path, o: &E| -> Result<(), String> {
                    ExternRunner::default().inner_run(
                        &["gcc", "2cdt.c", "-O2", "-o", "2cdt"],
                        o
                    )
                });
            Some(Compiler::from(install))
        };
        #[cfg(not(target_os = "linux"))]
        let compile: Option<Compiler<E>> = None;

        DelegateApplicationDescription::builder()
            .download_fn_url("https://cpctech.cpcwiki.de/download/2cdt.zip")
            .folder("2cdt")
            .archive_format(ArchiveFormat::Zip)
            .exec_fname(exec_fname)
            .maybe_compile(compile)
            .build()
    }
}

impl Display for TwoCdtVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "2cdt")
    }
}
